use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::store::source_file_hash::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::time::unix_time_ms;

use super::SqliteDurableStore;

pub const DEFAULT_TRACK_IDENTITY_DECISION_LIMIT: usize = 4;
pub const MAX_TRACK_IDENTITY_DECISION_LIMIT: usize = 128;
const TRACK_IDENTITY_CANDIDATE_KIND: &str = "exact_primary_media_content";
const TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS: &str = "current_primary_media_exact_blake3";
const TRACK_IDENTITY_DECISION_STATE_ACCEPTED: &str = "accepted";
const TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0: &str = "system_exact_content_v0";
const TRACK_IDENTITY_DECISION_BASIS_SYSTEM_EXACT_CONTENT_V0: &str =
    "active_exact_content_candidate_current_evidence_v0";
const TRACK_IDENTITY_DECISION_REASON_SYSTEM_EXACT_CONTENT_V0: &str =
    "accepted active exact-content track identity candidate from current primary-media evidence";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProduceTrackIdentityDecisionsForSourceResult {
    pub decisions_created: usize,
    pub decision_evidence_created: usize,
    pub skipped_stale_candidates: usize,
    pub skipped_existing_current_decisions: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackIdentityDecisionProductionCandidate {
    track_identity_candidate_id: i64,
    candidate_kind: String,
    candidate_evidence_basis: String,
    evidence_key_algorithm: String,
    evidence_key_value: String,
    candidate_status: String,
}

impl SqliteDurableStore {
    pub fn produce_track_identity_decisions_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<ProduceTrackIdentityDecisionsForSourceResult> {
        let decided_at = unix_time_ms()?;
        self.with_write(|write| {
            produce_track_identity_decisions_for_source(write, source_id, limit, decided_at)
        })
    }

    pub fn count_track_identity_decision_production_candidates(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        count_track_identity_decision_production_candidates(&connection, source_id)
    }
}

pub fn effective_track_identity_decision_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_TRACK_IDENTITY_DECISION_LIMIT)
        .clamp(1, MAX_TRACK_IDENTITY_DECISION_LIMIT)
}

fn produce_track_identity_decisions_for_source(
    write: &mut AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
    decided_at: i64,
) -> LibrarySqliteResult<ProduceTrackIdentityDecisionsForSourceResult> {
    let skipped_stale_candidates = count_stale_track_identity_candidates(write, source_id)?;
    let skipped_existing_current_decisions =
        count_existing_current_system_decisions(write, source_id)?;
    let candidates = read_track_identity_decision_production_candidates(write, source_id)?;
    let mut result = ProduceTrackIdentityDecisionsForSourceResult {
        skipped_stale_candidates,
        skipped_existing_current_decisions,
        remaining_candidates: candidates.len().saturating_sub(limit),
        ..Default::default()
    };

    for candidate in candidates.into_iter().take(limit) {
        let track_identity_decision_id =
            insert_system_exact_content_decision(write, &candidate, decided_at)?;
        result.decisions_created += 1;
        result.decision_evidence_created += insert_decision_evidence_snapshot(
            write,
            track_identity_decision_id,
            candidate.track_identity_candidate_id,
            decided_at,
        )?;
    }

    result.remaining_candidates =
        count_track_identity_decision_production_candidates(write, source_id)?;
    Ok(result)
}

fn count_track_identity_decision_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    read_track_identity_decision_production_candidates(connection, source_id).map(|rows| rows.len())
}

fn count_stale_track_identity_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(DISTINCT candidate.track_identity_candidate_id)
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             WHERE evidence.source_id = ?1
               AND candidate.status != 'active'",
            [source_id],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn count_existing_current_system_decisions(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            "SELECT COUNT(DISTINCT decision.track_identity_candidate_id)
             FROM track_identity_decisions decision
             JOIN track_identity_decision_evidence evidence
               ON evidence.track_identity_decision_id = decision.track_identity_decision_id
             WHERE evidence.source_id = ?1
               AND decision.decision_source = ?2
               AND decision.superseded_by_decision_id IS NULL",
            params![
                source_id,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            ],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn read_track_identity_decision_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<Vec<TrackIdentityDecisionProductionCandidate>> {
    connection
        .prepare(&format!(
            "SELECT DISTINCT candidate.track_identity_candidate_id,
                    candidate.candidate_kind,
                    candidate.evidence_basis,
                    candidate.evidence_key_algorithm,
                    candidate.evidence_key_value,
                    candidate.status
             FROM track_identity_candidates candidate
             JOIN track_identity_candidate_evidence evidence
               ON evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
             JOIN source_files file
               ON file.source_file_id = evidence.source_file_id
             LEFT JOIN SourceFacts facts
               ON facts.source_file_id = evidence.source_file_id
             LEFT JOIN source_file_attachment_links link
               ON link.source_file_attachment_link_id =
                  evidence.source_file_attachment_link_id
             LEFT JOIN content_attachments attachment
               ON attachment.attachment_id = evidence.attachment_id
             WHERE evidence.source_id = ?1
               AND candidate.status = 'active'
               AND candidate.candidate_kind = ?3
               AND candidate.evidence_basis = ?4
               AND candidate.evidence_key_algorithm = ?2
               AND {current_evidence_predicate}
               AND NOT EXISTS (
                   SELECT 1
                   FROM track_identity_decisions decision
                   WHERE decision.track_identity_candidate_id =
                         candidate.track_identity_candidate_id
                     AND decision.decision_source = ?5
                     AND decision.superseded_by_decision_id IS NULL
               )
             ORDER BY candidate.track_identity_candidate_id ASC",
            current_evidence_predicate = CURRENT_TRACK_IDENTITY_CANDIDATE_EVIDENCE_PREDICATE,
        ))?
        .query_map(
            params![
                source_id,
                SOURCE_FILE_BLAKE3_ALGORITHM,
                TRACK_IDENTITY_CANDIDATE_KIND,
                TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            ],
            |row| {
                Ok(TrackIdentityDecisionProductionCandidate {
                    track_identity_candidate_id: row.get(0)?,
                    candidate_kind: row.get(1)?,
                    candidate_evidence_basis: row.get(2)?,
                    evidence_key_algorithm: row.get(3)?,
                    evidence_key_value: row.get(4)?,
                    candidate_status: row.get(5)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn insert_system_exact_content_decision(
    write: &mut AdmittedWrite<'_>,
    candidate: &TrackIdentityDecisionProductionCandidate,
    decided_at: i64,
) -> LibrarySqliteResult<i64> {
    write.execute(
        "INSERT INTO track_identity_decisions (
             track_identity_candidate_id,
             decision_state,
             decision_source,
             decision_basis,
             decision_reason,
             candidate_kind,
             candidate_evidence_basis,
             candidate_status_at_decision,
             evidence_key_algorithm,
             evidence_key_value,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            candidate.track_identity_candidate_id,
            TRACK_IDENTITY_DECISION_STATE_ACCEPTED,
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            TRACK_IDENTITY_DECISION_BASIS_SYSTEM_EXACT_CONTENT_V0,
            TRACK_IDENTITY_DECISION_REASON_SYSTEM_EXACT_CONTENT_V0,
            candidate.candidate_kind,
            candidate.candidate_evidence_basis,
            candidate.candidate_status,
            candidate.evidence_key_algorithm,
            candidate.evidence_key_value,
            decided_at,
        ],
    )?;

    write
        .query_row(
            "SELECT track_identity_decision_id
             FROM track_identity_decisions
             WHERE track_identity_candidate_id = ?1
               AND decision_source = ?2
               AND superseded_by_decision_id IS NULL",
            params![
                candidate.track_identity_candidate_id,
                TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
            ],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn insert_decision_evidence_snapshot(
    write: &mut AdmittedWrite<'_>,
    track_identity_decision_id: i64,
    track_identity_candidate_id: i64,
    decided_at: i64,
) -> LibrarySqliteResult<usize> {
    let predicate = CURRENT_TRACK_IDENTITY_CANDIDATE_EVIDENCE_PREDICATE.replace("?2", "?4");
    write
        .execute(
            &format!(
                "INSERT INTO track_identity_decision_evidence (
                     track_identity_decision_id,
                     track_identity_candidate_id,
                     track_identity_candidate_member_id,
                     track_identity_candidate_evidence_id,
                     primary_media_candidate_id,
                     attachment_id,
                     source_file_attachment_link_id,
                     source_file_id,
                     source_id,
                     evidence_basis_fingerprint,
                     content_hash_algorithm,
                     content_hash_value,
                     probe_accepted_artifact_id,
                     created_at,
                     updated_at
                 )
                 SELECT ?1,
                        evidence.track_identity_candidate_id,
                        member.track_identity_candidate_member_id,
                        evidence.track_identity_candidate_evidence_id,
                        evidence.primary_media_candidate_id,
                        evidence.attachment_id,
                        evidence.source_file_attachment_link_id,
                        evidence.source_file_id,
                        evidence.source_id,
                        evidence.evidence_basis_fingerprint,
                        evidence.content_hash_algorithm,
                        evidence.content_hash_value,
                        evidence.probe_accepted_artifact_id,
                        ?3,
                        ?3
                 FROM track_identity_candidate_evidence evidence
                 JOIN track_identity_candidate_members member
                   ON member.track_identity_candidate_id =
                      evidence.track_identity_candidate_id
                  AND member.primary_media_candidate_id = evidence.primary_media_candidate_id
                 JOIN source_files file
                   ON file.source_file_id = evidence.source_file_id
                 LEFT JOIN SourceFacts facts
                   ON facts.source_file_id = evidence.source_file_id
                 LEFT JOIN source_file_attachment_links link
                   ON link.source_file_attachment_link_id =
                      evidence.source_file_attachment_link_id
                 LEFT JOIN content_attachments attachment
                   ON attachment.attachment_id = evidence.attachment_id
                 WHERE evidence.track_identity_candidate_id = ?2
                   AND {predicate}
                 ORDER BY evidence.source_id ASC,
                          evidence.source_file_id ASC",
            ),
            params![
                track_identity_decision_id,
                track_identity_candidate_id,
                decided_at,
                SOURCE_FILE_BLAKE3_ALGORITHM,
            ],
        )
        .map_err(Into::into)
}

fn read_count(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

const CURRENT_TRACK_IDENTITY_CANDIDATE_EVIDENCE_PREDICATE: &str =
    "file.source_id = facts.basis_source_id
    AND file.presence_state = 'present'
    AND file.media_class = 'audio'
    AND file.file_kind = 'audio'
    AND facts.source_file_id IS NOT NULL
    AND file.relative_path = facts.basis_relative_path
    AND file.size_bytes IS facts.basis_size_bytes
    AND file.mtime_ns IS facts.basis_mtime_ns
    AND file.presence_state = facts.basis_presence_state
    AND evidence.content_hash_algorithm = ?2
    AND facts.content_hash_algorithm = evidence.content_hash_algorithm
    AND facts.content_hash_value = evidence.content_hash_value
    AND link.source_file_attachment_link_id = evidence.source_file_attachment_link_id
    AND link.source_file_id = evidence.source_file_id
    AND link.source_id = evidence.source_id
    AND link.attachment_id = evidence.attachment_id
    AND attachment.content_hash_algorithm = evidence.content_hash_algorithm
    AND attachment.content_hash_value = evidence.content_hash_value
    AND facts.media_kind = 'audio'
    AND (
        facts.mime_type IS NOT NULL
        OR facts.duration_ms IS NOT NULL
        OR facts.sample_rate_hz IS NOT NULL
        OR facts.channels IS NOT NULL
        OR facts.bit_depth IS NOT NULL
        OR facts.codec IS NOT NULL
    )
    AND facts.accepted_artifact_id = evidence.probe_accepted_artifact_id";

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::contents::{
        StoreContentsMediaClass, StoreContentsReadPolicy, StoreContentsRecursion,
        StoreContentsRowProfile, StoreContentsScope, StoreContentsState,
    };
    use crate::read_models::track_identity_decisions::{
        StoreTrackIdentityDecisionCurrentStatus, StoreTrackIdentityDecisionState,
    };
    use crate::{ProduceTrackIdentityDecisionsForSourceResult, SqliteDurableStore};

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    struct TrackIdentityDecisionFixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
        source_id: i64,
    }

    impl TrackIdentityDecisionFixture {
        fn new() -> Self {
            let tempdir = TempDir::new().expect("create tempdir");
            let db_path = tempdir.path().join("library.sqlite3");
            let store = SqliteDurableStore::open(&db_path).expect("open store");
            let fixture = Self {
                _tempdir: tempdir,
                store,
                source_id: 1,
            };
            fixture.insert_source();
            fixture
        }

        fn insert_source(&self) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO sources (
                             source_id,
                             source_class,
                             authority,
                             identity_key,
                             display_name,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, 'internal', 'system', 'source:track-decision-test', 'Track Decision Test', 1, 1)",
                        [self.source_id],
                    )?;
                    write.execute(
                        "INSERT INTO source_state (
                             source_id,
                             mount_status,
                             mount_epoch,
                             access_state,
                             access_checked_at,
                             effective_path,
                             updated_at
                         )
                         VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 1)",
                        [self.source_id],
                    )?;
                    write.execute(
                        "INSERT INTO source_scan_state (
                             source_id,
                             scan_phase,
                             last_scan_started_at,
                             last_scan_finished_at,
                             last_successful_scan_at,
                             updated_at
                         )
                         VALUES (?1, 'complete', 1, 2, 2, 2)",
                        [self.source_id],
                    )?;
                    Ok(())
                })
                .expect("insert source");
        }

        fn insert_source_file(&self, source_file_id: i64, relative_path: &str) {
            let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
            let file_kind = crate::browse_media::file_kind_str_from_path(relative_path);
            let media_class = crate::browse_media::media_class_str_from_path(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             name,
                             relative_path,
                             size_bytes,
                             mtime_ns,
                             file_kind,
                             media_class,
                             presence_state,
                             first_discovered_at,
                             last_observed_at,
                             last_presence_change_at,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, 10, 100, ?5, ?6, 'present', 1, 1, 1, 1, 1)",
                        params![
                            source_file_id,
                            self.source_id,
                            file_name,
                            relative_path,
                            file_kind,
                            media_class
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn commit_current_facts(&self, source_file_id: i64, hash_value: &str) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO WorkItems (
                             work_item_id,
                             subject_kind,
                             subject_id,
                             work_kind,
                             basis_fingerprint,
                             state,
                             priority_class,
                             created_at,
                             updated_at
                         )
                         VALUES (1, 'source_file', 'fixture', 'inspect_source', 'fixture', 'completed', 'interactive', 1, 1)",
                        [],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO WorkRuns (
                             work_run_id,
                             work_item_id,
                             adapter_key,
                             adapter_version,
                             started_at,
                             outcome
                         )
                         VALUES (1, 1, 'test.track_identity_decision', '1', 1, 'ok')",
                        [],
                    )?;
                    let artifact_id = 10_000 + source_file_id;
                    write.execute(
                        "INSERT OR REPLACE INTO Artifacts (
                             artifact_id,
                             work_run_id,
                             subject_kind,
                             subject_id,
                             artifact_kind,
                             artifact_role,
                             adapter_key,
                             adapter_version,
                             basis_fingerprint,
                             media_type,
                             storage_kind,
                             payload_hash,
                             created_at
                         )
                         VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test.track_identity_decision', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
                        params![
                            artifact_id,
                            source_file_id.to_string(),
                            format!("basis:{source_file_id}"),
                            format!("payload:{source_file_id}")
                        ],
                    )?;
                    let (relative_path, size_bytes, mtime_ns, presence_state): (
                        String,
                        Option<i64>,
                        Option<i64>,
                        String,
                    ) = write.query_row(
                        "SELECT relative_path, size_bytes, mtime_ns, presence_state
                         FROM source_files
                         WHERE source_file_id = ?1",
                        [source_file_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )?;
                    write.execute(
                        "INSERT INTO SourceFacts (
                             source_file_id,
                             fact_kind,
                             basis_fingerprint,
                             basis_source_id,
                             basis_relative_path,
                             basis_size_bytes,
                             basis_mtime_ns,
                             basis_presence_state,
                             observed_at_ms,
                             content_hash_algorithm,
                             content_hash_value,
                             media_kind,
                             mime_type,
                             duration_ms,
                             sample_rate_hz,
                             channels,
                             bit_depth,
                             codec,
                             updated_at,
                             accepted_artifact_id
                         )
                         VALUES (?1, 'source_inspection', ?2, ?3, ?4, ?5, ?6, ?7, 1, 'blake3', ?8, 'audio', 'audio/wav', 100, 44100, 2, 16, 'pcm', 1, ?9)
                         ON CONFLICT(source_file_id) DO UPDATE SET
                             basis_fingerprint = excluded.basis_fingerprint,
                             basis_source_id = excluded.basis_source_id,
                             basis_relative_path = excluded.basis_relative_path,
                             basis_size_bytes = excluded.basis_size_bytes,
                             basis_mtime_ns = excluded.basis_mtime_ns,
                             basis_presence_state = excluded.basis_presence_state,
                             content_hash_algorithm = excluded.content_hash_algorithm,
                             content_hash_value = excluded.content_hash_value,
                             media_kind = excluded.media_kind,
                             mime_type = excluded.mime_type,
                             duration_ms = excluded.duration_ms,
                             sample_rate_hz = excluded.sample_rate_hz,
                             channels = excluded.channels,
                             bit_depth = excluded.bit_depth,
                             codec = excluded.codec,
                             updated_at = excluded.updated_at,
                             accepted_artifact_id = excluded.accepted_artifact_id",
                        params![
                            source_file_id,
                            format!("basis:{source_file_id}"),
                            self.source_id,
                            relative_path,
                            size_bytes,
                            mtime_ns,
                            presence_state,
                            hash_value,
                            artifact_id,
                        ],
                    )?;
                    Ok(())
                })
                .expect("commit facts");
        }

        fn link_attachment(&self, source_file_id: i64, hash_value: &str) -> i64 {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO content_attachments (
                             content_hash_algorithm,
                             content_hash_value,
                             first_observed_at,
                             updated_at
                         )
                         VALUES ('blake3', ?1, 1, 1)",
                        [hash_value],
                    )?;
                    let attachment_id: i64 = write.query_row(
                        "SELECT attachment_id
                         FROM content_attachments
                         WHERE content_hash_algorithm = 'blake3'
                           AND content_hash_value = ?1",
                        [hash_value],
                        |row| row.get(0),
                    )?;
                    let file_kind: String = write.query_row(
                        "SELECT file_kind
                         FROM source_files
                         WHERE source_file_id = ?1",
                        [source_file_id],
                        |row| row.get(0),
                    )?;
                    write.execute(
                        "INSERT OR REPLACE INTO source_file_attachment_links (
                             attachment_id,
                             source_file_id,
                             source_id,
                             file_kind,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, 1, 1)",
                        params![attachment_id, source_file_id, self.source_id, file_kind],
                    )?;
                    Ok(attachment_id)
                })
                .expect("link attachment")
        }

        fn promote_and_candidate(&self) {
            self.store
                .promote_primary_media_for_source(self.source_id, 10)
                .expect("promote primary media");
            self.store
                .produce_track_identity_candidates_for_source(self.source_id, 10)
                .expect("produce track identity candidates");
        }

        fn produce_decisions(&self, limit: usize) -> ProduceTrackIdentityDecisionsForSourceResult {
            self.store
                .produce_track_identity_decisions_for_source(self.source_id, limit)
                .expect("produce track identity decisions")
        }

        fn count_rows(&self, table: &str) -> i64 {
            self.store
                .open_read_connection()
                .expect("open read")
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count rows")
        }

        fn count_table_if_exists(&self, table: &str) -> Option<i64> {
            let connection = self.store.open_read_connection().expect("open read");
            let exists = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM sqlite_master
                     WHERE type = 'table'
                       AND name = ?1",
                    [table],
                    |row| row.get::<_, i64>(0),
                )
                .expect("check table")
                > 0;
            exists.then(|| {
                connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .expect("count table")
            })
        }

        fn change_file_basis(&self, source_file_id: i64) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE source_files
                         SET size_bytes = size_bytes + 1,
                             mtime_ns = mtime_ns + 1,
                             updated_at = updated_at + 1
                         WHERE source_file_id = ?1",
                        [source_file_id],
                    )?;
                    Ok(())
                })
                .expect("change file basis");
        }
    }

    #[test]
    fn active_candidate_receives_system_exact_content_decision_with_provenance() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();

        let result = fixture.produce_decisions(10);

        assert_eq!(result.decisions_created, 1);
        assert_eq!(result.decision_evidence_created, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 1);
        assert_eq!(fixture.count_rows("track_identity_decision_evidence"), 1);

        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        let candidate = &candidates[0];
        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        assert_eq!(decisions.len(), 1);
        let decision = &decisions[0];
        assert_eq!(
            decision.track_identity_candidate_id,
            candidate.track_identity_candidate_id
        );
        assert_eq!(
            decision.decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(decision.decision_source, "system_exact_content_v0");
        assert_eq!(
            decision.current_status,
            StoreTrackIdentityDecisionCurrentStatus::Current
        );
        assert_eq!(decision.evidence_key_algorithm, "blake3");
        assert_eq!(decision.evidence_key_value, HASH_A);
        assert!(
            decision
                .proves
                .contains("exact-content track identity candidate")
        );
        assert!(decision.does_not_prove.contains("canonical track identity"));
        assert_eq!(decision.evidence.len(), 1);
        let evidence = &decision.evidence[0];
        assert_eq!(
            evidence.track_identity_candidate_member_id,
            candidate.members[0].track_identity_candidate_member_id
        );
        assert_eq!(
            evidence.track_identity_candidate_evidence_id,
            candidate.evidence[0].track_identity_candidate_evidence_id
        );
        assert_eq!(evidence.attachment_id, attachment_id);
        assert_eq!(evidence.source_file_id, 100);
        assert_eq!(evidence.content_hash_value, HASH_A);
    }

    #[test]
    fn stale_candidate_cannot_receive_new_accepted_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/stale.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("mark candidate stale");

        let result = fixture.produce_decisions(10);

        assert_eq!(result.decisions_created, 0);
        assert_eq!(result.decision_evidence_created, 0);
        assert_eq!(result.skipped_stale_candidates, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 0);
    }

    #[test]
    fn repeated_decision_production_does_not_duplicate_current_decision() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();

        let first = fixture.produce_decisions(10);
        let second = fixture.produce_decisions(10);

        assert_eq!(first.decisions_created, 1);
        assert_eq!(second.decisions_created, 0);
        assert_eq!(second.skipped_existing_current_decisions, 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 1);
        assert_eq!(fixture.count_rows("track_identity_decision_evidence"), 1);
    }

    #[test]
    fn decision_production_does_not_group_by_path_or_equivalence_fingerprint() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/track copy.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_B);
        fixture.commit_current_facts(100, HASH_A);
        fixture.commit_current_facts(101, HASH_B);
        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO LibraryAssets (
                         library_asset_id,
                         equivalence_fingerprint,
                         created_at,
                         updated_at
                     )
                     VALUES (1, 'same-looking-track', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .expect("insert legacy asset");
        fixture.promote_and_candidate();

        fixture.produce_decisions(10);

        assert_eq!(fixture.count_rows("track_identity_candidates"), 2);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 2);
        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        let mut hashes = decisions
            .iter()
            .map(|decision| decision.evidence_key_value.as_str())
            .collect::<Vec<_>>();
        hashes.sort_unstable();
        assert_eq!(hashes, vec![HASH_A, HASH_B]);
    }

    #[test]
    fn decision_production_leaves_contents_cue_metadata_and_prep_surfaces_unchanged() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/album.cue");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.promote_and_candidate();
        fixture.produce_decisions(10);

        let result = fixture
            .store
            .read_contents(
                StoreContentsScope::Source {
                    source_id: fixture.source_id,
                },
                StoreContentsReadPolicy {
                    media_classes: vec![
                        StoreContentsMediaClass::Audio,
                        StoreContentsMediaClass::Video,
                        StoreContentsMediaClass::Image,
                        StoreContentsMediaClass::Unsupported,
                    ],
                    row_profile: StoreContentsRowProfile::SourceFile,
                },
                StoreContentsRecursion::Recursive,
                10,
                None,
            )
            .expect("read contents");
        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|row| row.primary_media.is_none()));

        for table in [
            "LibraryAssetAttachments",
            "LibraryBrowserRows",
            "SourceSegmentSets",
            "SourceSegments",
            "PrepAssignments",
            "ResolvedLibraryAssetPrepTargets",
            "Playlists",
            "PlaylistEntries",
        ] {
            assert_eq!(fixture.count_rows(table), 0, "{table} must remain empty");
        }
        for absent_or_future_table in [
            "Tracks",
            "TrackRows",
            "LibraryTracks",
            "CueAudioAssociations",
            "Waveforms",
            "Stems",
            "PrepRows",
            "PreparationRows",
        ] {
            assert!(
                matches!(
                    fixture.count_table_if_exists(absent_or_future_table),
                    None | Some(0)
                ),
                "{absent_or_future_table} must be absent or empty"
            );
        }

        let columns = fixture
            .store
            .open_read_connection()
            .expect("open read")
            .prepare("PRAGMA table_info(track_identity_decisions)")
            .expect("prepare pragma")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query pragma")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns");
        for forbidden in ["title", "artist", "album", "bpm", "key"] {
            assert!(
                !columns.iter().any(|column| column == forbidden),
                "decision schema must not contain metadata column {forbidden}"
            );
        }
    }

    #[test]
    fn decision_evidence_snapshot_includes_only_current_evidence_rows() {
        let fixture = TrackIdentityDecisionFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/track copy.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_A);
        fixture.commit_current_facts(100, HASH_A);
        fixture.commit_current_facts(101, HASH_A);
        fixture.promote_and_candidate();

        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            2,
            "both source files share the same hash and group into one candidate"
        );

        fixture.change_file_basis(100);
        fixture
            .store
            .produce_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("re-produce candidates to mark file 100 evidence stale");

        assert_eq!(
            fixture.count_rows("track_identity_candidate_evidence"),
            2,
            "stale evidence row is preserved but no longer current"
        );

        let result = fixture.produce_decisions(10);
        assert_eq!(result.decisions_created, 1);
        assert_eq!(
            result.decision_evidence_created, 1,
            "only the current evidence row must be snapshotted"
        );

        let decisions = fixture
            .store
            .read_track_identity_decisions_for_source(fixture.source_id, 10)
            .expect("read decisions");
        assert_eq!(decisions.len(), 1);
        let evidence = &decisions[0].evidence;
        assert_eq!(
            evidence.len(),
            1,
            "decision evidence must contain only the current supporting evidence row"
        );
        assert_eq!(
            evidence[0].source_file_id, 101,
            "snapshot evidence must reference the still-current source file"
        );
    }
}
