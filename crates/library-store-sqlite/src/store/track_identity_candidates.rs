use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::store::source_file_hash::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::time::unix_time_ms;

use super::SqliteDurableStore;

pub const DEFAULT_TRACK_IDENTITY_CANDIDATE_LIMIT: usize = 4;
pub const MAX_TRACK_IDENTITY_CANDIDATE_LIMIT: usize = 128;
const TRACK_IDENTITY_CANDIDATE_KIND: &str = "exact_primary_media_content";
const TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS: &str = "current_primary_media_exact_blake3";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProduceTrackIdentityCandidatesForSourceResult {
    pub candidates_created: usize,
    pub candidates_refreshed: usize,
    pub members_created: usize,
    pub members_refreshed: usize,
    pub evidence_created: usize,
    pub evidence_refreshed: usize,
    pub candidates_marked_stale: usize,
    pub skipped_stale_primary_media_candidates: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackIdentityCandidateProductionRow {
    primary_media_candidate_id: i64,
    attachment_id: i64,
    evidence_source_file_id: i64,
    evidence_basis_fingerprint: String,
    content_hash_value: String,
    source_file_attachment_link_id: i64,
    source_file_id: i64,
    source_id: i64,
    probe_accepted_artifact_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpsertChange {
    Created,
    Refreshed,
}

impl SqliteDurableStore {
    pub fn produce_track_identity_candidates_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<ProduceTrackIdentityCandidatesForSourceResult> {
        let produced_at = unix_time_ms()?;
        self.with_write(|write| {
            produce_track_identity_candidates_for_source(write, source_id, limit, produced_at)
        })
    }

    pub fn count_track_identity_candidate_production_candidates(
        &self,
        source_id: i64,
    ) -> LibrarySqliteResult<usize> {
        let connection = self.open_read_connection()?;
        count_track_identity_candidate_production_candidates(&connection, source_id)
    }
}

pub fn effective_track_identity_candidate_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_TRACK_IDENTITY_CANDIDATE_LIMIT)
        .clamp(1, MAX_TRACK_IDENTITY_CANDIDATE_LIMIT)
}

fn produce_track_identity_candidates_for_source(
    write: &mut AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
    produced_at: i64,
) -> LibrarySqliteResult<ProduceTrackIdentityCandidatesForSourceResult> {
    let skipped_stale_primary_media_candidates =
        count_stale_primary_media_candidates(write, source_id)?;
    let production_candidates =
        read_track_identity_candidate_production_candidates(write, source_id)?;

    let mut result = ProduceTrackIdentityCandidatesForSourceResult {
        skipped_stale_primary_media_candidates,
        remaining_candidates: production_candidates.len().saturating_sub(limit),
        ..Default::default()
    };

    for production in production_candidates.into_iter().take(limit) {
        let (track_identity_candidate_id, candidate_change) =
            upsert_track_identity_candidate(write, &production, produced_at)?;
        match candidate_change {
            UpsertChange::Created => result.candidates_created += 1,
            UpsertChange::Refreshed => result.candidates_refreshed += 1,
        }

        match upsert_track_identity_candidate_member(
            write,
            track_identity_candidate_id,
            &production,
            produced_at,
        )? {
            UpsertChange::Created => result.members_created += 1,
            UpsertChange::Refreshed => result.members_refreshed += 1,
        }

        match upsert_track_identity_candidate_evidence(
            write,
            track_identity_candidate_id,
            &production,
            produced_at,
        )? {
            UpsertChange::Created => result.evidence_created += 1,
            UpsertChange::Refreshed => result.evidence_refreshed += 1,
        }
    }

    result.candidates_marked_stale =
        mark_track_identity_candidates_without_current_members_stale(write, produced_at)?;
    result.remaining_candidates =
        count_track_identity_candidate_production_candidates(write, source_id)?;
    Ok(result)
}

fn count_track_identity_candidate_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    read_track_identity_candidate_production_candidates(connection, source_id)
        .map(|rows| rows.len())
}

fn count_stale_primary_media_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*)
                 FROM primary_media_candidates pmc
                 JOIN source_files file
                   ON file.source_file_id = pmc.evidence_source_file_id
                 LEFT JOIN SourceFacts facts
                   ON facts.source_file_id = pmc.evidence_source_file_id
                 LEFT JOIN source_file_attachment_links link
                   ON link.source_file_id = pmc.evidence_source_file_id
                  AND link.attachment_id = pmc.attachment_id
                 LEFT JOIN content_attachments attachment
                   ON attachment.attachment_id = pmc.attachment_id
                 WHERE file.source_id = ?1
                   AND NOT ({current_primary_media_predicate})",
                current_primary_media_predicate = CURRENT_PRIMARY_MEDIA_PREDICATE,
            ),
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn read_track_identity_candidate_production_candidates(
    connection: &rusqlite::Connection,
    source_id: i64,
) -> LibrarySqliteResult<Vec<TrackIdentityCandidateProductionRow>> {
    connection
        .prepare(&format!(
            "WITH current_primary_media AS (
                 SELECT pmc.primary_media_candidate_id,
                        pmc.attachment_id,
                        pmc.evidence_source_file_id,
                        pmc.evidence_basis_fingerprint,
                        attachment.content_hash_value,
                        file.relative_path
                 FROM primary_media_candidates pmc
                 JOIN source_files file
                   ON file.source_file_id = pmc.evidence_source_file_id
                 JOIN SourceFacts facts
                   ON facts.source_file_id = pmc.evidence_source_file_id
                 JOIN source_file_attachment_links link
                   ON link.source_file_id = pmc.evidence_source_file_id
                  AND link.attachment_id = pmc.attachment_id
                 JOIN content_attachments attachment
                   ON attachment.attachment_id = pmc.attachment_id
                 WHERE file.source_id = ?1
                   AND {current_primary_media_predicate}
             ),
             current_attachment_evidence AS (
                 SELECT current.primary_media_candidate_id,
                        current.attachment_id,
                        current.evidence_source_file_id,
                        current.evidence_basis_fingerprint,
                        current.content_hash_value,
                        current.relative_path,
                        link.source_file_attachment_link_id,
                        link.source_file_id,
                        link.source_id,
                        facts.accepted_artifact_id,
                        ROW_NUMBER() OVER (
                            PARTITION BY current.primary_media_candidate_id
                            ORDER BY lower(file.relative_path) ASC,
                                     file.source_file_id ASC
                        ) AS evidence_rank
                 FROM current_primary_media current
                 JOIN source_file_attachment_links link
                   ON link.attachment_id = current.attachment_id
                 JOIN source_files file
                   ON file.source_file_id = link.source_file_id
                 JOIN SourceFacts facts
                   ON facts.source_file_id = link.source_file_id
                 JOIN content_attachments attachment
                   ON attachment.attachment_id = link.attachment_id
                 WHERE {current_attachment_link_predicate}
             )
             SELECT evidence.primary_media_candidate_id,
                    evidence.attachment_id,
                    evidence.evidence_source_file_id,
                    evidence.evidence_basis_fingerprint,
                    evidence.content_hash_value,
                    evidence.source_file_attachment_link_id,
                    evidence.source_file_id,
                    evidence.source_id,
                    evidence.accepted_artifact_id
             FROM current_attachment_evidence evidence
             LEFT JOIN track_identity_candidates candidate
               ON candidate.candidate_kind = ?3
              AND candidate.evidence_basis = ?4
              AND candidate.evidence_key_algorithm = ?2
              AND candidate.evidence_key_value = evidence.content_hash_value
             LEFT JOIN track_identity_candidate_members member
               ON member.primary_media_candidate_id = evidence.primary_media_candidate_id
             LEFT JOIN track_identity_candidate_evidence stored_evidence
               ON stored_evidence.track_identity_candidate_id = candidate.track_identity_candidate_id
              AND stored_evidence.primary_media_candidate_id = evidence.primary_media_candidate_id
              AND stored_evidence.source_file_id = evidence.source_file_id
             WHERE candidate.track_identity_candidate_id IS NULL
                OR candidate.status != 'active'
                OR member.track_identity_candidate_member_id IS NULL
                OR member.track_identity_candidate_id != candidate.track_identity_candidate_id
                OR member.attachment_id != evidence.attachment_id
                OR member.evidence_source_file_id != evidence.evidence_source_file_id
                OR member.evidence_basis_fingerprint != evidence.evidence_basis_fingerprint
                OR member.content_hash_algorithm != ?2
                OR member.content_hash_value != evidence.content_hash_value
                OR stored_evidence.track_identity_candidate_evidence_id IS NULL
                OR stored_evidence.attachment_id != evidence.attachment_id
                OR stored_evidence.source_file_attachment_link_id != evidence.source_file_attachment_link_id
                OR stored_evidence.source_id != evidence.source_id
                OR stored_evidence.evidence_basis_fingerprint != evidence.evidence_basis_fingerprint
                OR stored_evidence.content_hash_algorithm != ?2
                OR stored_evidence.content_hash_value != evidence.content_hash_value
                OR stored_evidence.probe_accepted_artifact_id != evidence.accepted_artifact_id
             ORDER BY lower(evidence.relative_path) ASC,
                      evidence.primary_media_candidate_id ASC,
                      evidence.evidence_rank ASC",
            current_primary_media_predicate = CURRENT_PRIMARY_MEDIA_PREDICATE,
            current_attachment_link_predicate = CURRENT_ATTACHMENT_LINK_PREDICATE,
        ))?
        .query_map(
            params![
                source_id,
                SOURCE_FILE_BLAKE3_ALGORITHM,
                TRACK_IDENTITY_CANDIDATE_KIND,
                TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
            ],
            |row| {
                Ok(TrackIdentityCandidateProductionRow {
                    primary_media_candidate_id: row.get(0)?,
                    attachment_id: row.get(1)?,
                    evidence_source_file_id: row.get(2)?,
                    evidence_basis_fingerprint: row.get(3)?,
                    content_hash_value: row.get(4)?,
                    source_file_attachment_link_id: row.get(5)?,
                    source_file_id: row.get(6)?,
                    source_id: row.get(7)?,
                    probe_accepted_artifact_id: row.get(8)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn upsert_track_identity_candidate(
    write: &mut AdmittedWrite<'_>,
    production: &TrackIdentityCandidateProductionRow,
    produced_at: i64,
) -> LibrarySqliteResult<(i64, UpsertChange)> {
    let existing_id: Option<i64> = write
        .query_row(
            "SELECT track_identity_candidate_id
             FROM track_identity_candidates
             WHERE candidate_kind = ?1
               AND evidence_basis = ?2
               AND evidence_key_algorithm = ?3
               AND evidence_key_value = ?4",
            params![
                TRACK_IDENTITY_CANDIDATE_KIND,
                TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
                SOURCE_FILE_BLAKE3_ALGORITHM,
                production.content_hash_value,
            ],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(track_identity_candidate_id) = existing_id {
        write.execute(
            "UPDATE track_identity_candidates
             SET status = 'active',
                 updated_at = ?2
             WHERE track_identity_candidate_id = ?1",
            params![track_identity_candidate_id, produced_at],
        )?;
        return Ok((track_identity_candidate_id, UpsertChange::Refreshed));
    }

    write.execute(
        "INSERT INTO track_identity_candidates (
             candidate_kind,
             evidence_basis,
             evidence_key_algorithm,
             evidence_key_value,
             status,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)",
        params![
            TRACK_IDENTITY_CANDIDATE_KIND,
            TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
            SOURCE_FILE_BLAKE3_ALGORITHM,
            production.content_hash_value,
            produced_at,
        ],
    )?;

    let track_identity_candidate_id = write.query_row(
        "SELECT track_identity_candidate_id
         FROM track_identity_candidates
         WHERE candidate_kind = ?1
           AND evidence_basis = ?2
           AND evidence_key_algorithm = ?3
           AND evidence_key_value = ?4",
        params![
            TRACK_IDENTITY_CANDIDATE_KIND,
            TRACK_IDENTITY_CANDIDATE_EVIDENCE_BASIS,
            SOURCE_FILE_BLAKE3_ALGORITHM,
            production.content_hash_value,
        ],
        |row| row.get(0),
    )?;
    Ok((track_identity_candidate_id, UpsertChange::Created))
}

fn upsert_track_identity_candidate_member(
    write: &mut AdmittedWrite<'_>,
    track_identity_candidate_id: i64,
    production: &TrackIdentityCandidateProductionRow,
    produced_at: i64,
) -> LibrarySqliteResult<UpsertChange> {
    let existing_id: Option<i64> = write
        .query_row(
            "SELECT track_identity_candidate_member_id
             FROM track_identity_candidate_members
             WHERE primary_media_candidate_id = ?1",
            [production.primary_media_candidate_id],
            |row| row.get(0),
        )
        .optional()?;

    write.execute(
        "INSERT INTO track_identity_candidate_members (
             track_identity_candidate_id,
             primary_media_candidate_id,
             attachment_id,
             evidence_source_file_id,
             evidence_basis_fingerprint,
             content_hash_algorithm,
             content_hash_value,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(primary_media_candidate_id) DO UPDATE SET
             track_identity_candidate_id = excluded.track_identity_candidate_id,
             attachment_id = excluded.attachment_id,
             evidence_source_file_id = excluded.evidence_source_file_id,
             evidence_basis_fingerprint = excluded.evidence_basis_fingerprint,
             content_hash_algorithm = excluded.content_hash_algorithm,
             content_hash_value = excluded.content_hash_value,
             updated_at = excluded.updated_at",
        params![
            track_identity_candidate_id,
            production.primary_media_candidate_id,
            production.attachment_id,
            production.evidence_source_file_id,
            production.evidence_basis_fingerprint,
            SOURCE_FILE_BLAKE3_ALGORITHM,
            production.content_hash_value,
            produced_at,
        ],
    )?;

    Ok(match existing_id {
        Some(_) => UpsertChange::Refreshed,
        None => UpsertChange::Created,
    })
}

fn upsert_track_identity_candidate_evidence(
    write: &mut AdmittedWrite<'_>,
    track_identity_candidate_id: i64,
    production: &TrackIdentityCandidateProductionRow,
    produced_at: i64,
) -> LibrarySqliteResult<UpsertChange> {
    let existing_id: Option<i64> = write
        .query_row(
            "SELECT track_identity_candidate_evidence_id
             FROM track_identity_candidate_evidence
             WHERE track_identity_candidate_id = ?1
               AND primary_media_candidate_id = ?2
               AND source_file_id = ?3",
            params![
                track_identity_candidate_id,
                production.primary_media_candidate_id,
                production.source_file_id,
            ],
            |row| row.get(0),
        )
        .optional()?;

    write.execute(
        "INSERT INTO track_identity_candidate_evidence (
             track_identity_candidate_id,
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
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
         ON CONFLICT(track_identity_candidate_id, primary_media_candidate_id, source_file_id)
         DO UPDATE SET
             attachment_id = excluded.attachment_id,
             source_file_attachment_link_id = excluded.source_file_attachment_link_id,
             source_id = excluded.source_id,
             evidence_basis_fingerprint = excluded.evidence_basis_fingerprint,
             content_hash_algorithm = excluded.content_hash_algorithm,
             content_hash_value = excluded.content_hash_value,
             probe_accepted_artifact_id = excluded.probe_accepted_artifact_id,
             updated_at = excluded.updated_at",
        params![
            track_identity_candidate_id,
            production.primary_media_candidate_id,
            production.attachment_id,
            production.source_file_attachment_link_id,
            production.source_file_id,
            production.source_id,
            production.evidence_basis_fingerprint,
            SOURCE_FILE_BLAKE3_ALGORITHM,
            production.content_hash_value,
            production.probe_accepted_artifact_id,
            produced_at,
        ],
    )?;

    Ok(match existing_id {
        Some(_) => UpsertChange::Refreshed,
        None => UpsertChange::Created,
    })
}

fn mark_track_identity_candidates_without_current_members_stale(
    write: &mut AdmittedWrite<'_>,
    produced_at: i64,
) -> LibrarySqliteResult<usize> {
    let changed = write.execute(
        &format!(
            "UPDATE track_identity_candidates
             SET status = 'stale',
                 updated_at = ?1
             WHERE status = 'active'
               AND NOT EXISTS (
                   SELECT 1
                   FROM track_identity_candidate_members member
                   JOIN primary_media_candidates pmc
                     ON pmc.primary_media_candidate_id = member.primary_media_candidate_id
                   JOIN source_files file
                     ON file.source_file_id = pmc.evidence_source_file_id
                   JOIN SourceFacts facts
                     ON facts.source_file_id = pmc.evidence_source_file_id
                   JOIN source_file_attachment_links link
                     ON link.source_file_id = pmc.evidence_source_file_id
                    AND link.attachment_id = pmc.attachment_id
                   JOIN content_attachments attachment
                     ON attachment.attachment_id = pmc.attachment_id
                   WHERE member.track_identity_candidate_id =
                         track_identity_candidates.track_identity_candidate_id
                     AND {current_primary_media_predicate}
               )",
            current_primary_media_predicate = CURRENT_PRIMARY_MEDIA_PREDICATE,
        ),
        params![produced_at, SOURCE_FILE_BLAKE3_ALGORITHM],
    )?;
    Ok(changed)
}

fn read_count(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

// Candidate production validates source rows before stored candidate evidence exists.
// Stored evidence currentness is shared by read status and decision snapshot predicates.
const CURRENT_PRIMARY_MEDIA_PREDICATE: &str = "file.presence_state = 'present'
    AND file.file_class = 'audio'
    AND file.file_kind = 'audio'
    AND facts.source_file_id IS NOT NULL
    AND file.source_id = facts.basis_source_id
    AND file.relative_path = facts.basis_relative_path
    AND file.size_bytes IS facts.basis_size_bytes
    AND file.mtime_ns IS facts.basis_mtime_ns
    AND file.presence_state = facts.basis_presence_state
    AND facts.content_hash_algorithm = ?2
    AND facts.content_hash_value IS NOT NULL
    AND facts.media_kind = 'audio'
    AND (
        facts.mime_type IS NOT NULL
        OR facts.duration_ms IS NOT NULL
        OR facts.sample_rate_hz IS NOT NULL
        OR facts.channels IS NOT NULL
        OR facts.bit_depth IS NOT NULL
        OR facts.codec IS NOT NULL
    )
    AND link.source_file_id IS NOT NULL
    AND attachment.content_hash_algorithm = facts.content_hash_algorithm
    AND attachment.content_hash_value = facts.content_hash_value";

const CURRENT_ATTACHMENT_LINK_PREDICATE: &str = "file.presence_state = 'present'
    AND file.source_id = facts.basis_source_id
    AND file.relative_path = facts.basis_relative_path
    AND file.size_bytes IS facts.basis_size_bytes
    AND file.mtime_ns IS facts.basis_mtime_ns
    AND file.presence_state = facts.basis_presence_state
    AND facts.content_hash_algorithm = attachment.content_hash_algorithm
    AND facts.content_hash_algorithm = ?2
    AND facts.content_hash_value = attachment.content_hash_value
    AND facts.media_kind = 'audio'
    AND (
        facts.mime_type IS NOT NULL
        OR facts.duration_ms IS NOT NULL
        OR facts.sample_rate_hz IS NOT NULL
        OR facts.channels IS NOT NULL
        OR facts.bit_depth IS NOT NULL
        OR facts.codec IS NOT NULL
    )";

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::read_models::contents::{
        StoreContentsFileClass, StoreContentsReadPolicy, StoreContentsScope,
        StoreContentsScopeDepth, StoreContentsState,
    };
    use crate::read_models::track_identity_candidates::{
        StoreTrackIdentityCandidateEvidenceStatus, StoreTrackIdentityCandidateStatus,
    };
    use crate::{ProduceTrackIdentityCandidatesForSourceResult, SqliteDurableStore};

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    struct TrackIdentityCandidateFixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
        source_id: i64,
    }

    impl TrackIdentityCandidateFixture {
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
                         VALUES (?1, 'internal', 'system', 'source:track-identity-test', 'Track Identity Test', 1, 1)",
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
            let file_class = crate::browse_media::file_class_str_from_path(relative_path);
            let name_browse_sort_key =
                crate::browse_sort_key::compute_name_browse_sort_key(file_name);
            let relative_path_browse_sort_key =
                crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO source_files (
                             source_file_id,
                             source_id,
                             name,
                             name_browse_sort_key,
                             relative_path_browse_sort_key,
                             relative_path,
                             size_bytes,
                             mtime_ns,
                             file_kind,
                             file_class,
                             presence_state,
                             first_discovered_at,
                             last_observed_at,
                             last_presence_change_at,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 10, 100, ?7, ?8, 'present', 1, 1, 1, 1, 1)",
                        params![
                            source_file_id,
                            self.source_id,
                            file_name,
                            name_browse_sort_key,
                            relative_path_browse_sort_key,
                            relative_path,
                            file_kind,
                            file_class,
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn commit_current_facts(
            &self,
            source_file_id: i64,
            hash_value: &str,
            media_kind: &str,
            with_probe: bool,
        ) {
            self.commit_current_facts_with_artifact_id(
                source_file_id,
                hash_value,
                media_kind,
                with_probe,
                10_000 + source_file_id,
            );
        }

        fn commit_current_facts_with_artifact_id(
            &self,
            source_file_id: i64,
            hash_value: &str,
            media_kind: &str,
            with_probe: bool,
            artifact_id: i64,
        ) {
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
                         VALUES (1, 1, 'test.track_identity_candidate', '1', 1, 'ok')",
                        [],
                    )?;
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
                         VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test.track_identity_candidate', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
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
                         VALUES (?1, 'source_inspection', ?2, ?3, ?4, ?5, ?6, ?7, 1, 'blake3', ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 1, ?16)
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
                            media_kind,
                            with_probe.then_some("audio/wav"),
                            with_probe.then_some(100_i64),
                            with_probe.then_some(44_100_i64),
                            with_probe.then_some(2_i64),
                            with_probe.then_some(16_i64),
                            with_probe.then_some("pcm"),
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

        fn promote(&self, limit: usize) {
            self.store
                .promote_primary_media_for_source(self.source_id, limit)
                .expect("promote primary media");
        }

        fn produce(&self, limit: usize) -> ProduceTrackIdentityCandidatesForSourceResult {
            self.store
                .produce_track_identity_candidates_for_source(self.source_id, limit)
                .expect("produce track identity candidates")
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

        fn single_evidence_status(&self) -> StoreTrackIdentityCandidateEvidenceStatus {
            let candidates = self
                .store
                .read_track_identity_candidates_for_source(self.source_id, 10)
                .expect("read candidates");
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].evidence.len(), 1);
            candidates[0].evidence[0].evidence_status
        }
    }

    #[test]
    fn current_primary_media_candidate_produces_track_identity_candidate() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        let attachment_id = fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);

        let result = fixture.produce(10);

        assert_eq!(result.candidates_created, 1);
        assert_eq!(result.members_created, 1);
        assert_eq!(result.evidence_created, 1);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_members"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 1);

        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.status, StoreTrackIdentityCandidateStatus::Active);
        assert_eq!(candidate.evidence_key_algorithm, "blake3");
        assert_eq!(candidate.evidence_key_value, HASH_A);
        assert_eq!(candidate.members[0].attachment_id, attachment_id);
        assert_eq!(candidate.evidence[0].source_file_id, 100);
        assert_eq!(
            candidate.evidence[0].evidence_status,
            StoreTrackIdentityCandidateEvidenceStatus::Current
        );
        assert!(
            candidate
                .does_not_prove
                .contains("canonical track identity")
        );
    }

    #[test]
    fn stale_primary_media_candidate_is_skipped_and_existing_candidate_becomes_stale() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/stale.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);
        fixture.change_file_basis(100);

        let result = fixture.produce(10);

        assert_eq!(result.skipped_stale_primary_media_candidates, 1);
        assert_eq!(result.candidates_created, 0);
        assert_eq!(result.candidates_refreshed, 0);
        assert_eq!(result.candidates_marked_stale, 1);
        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        assert_eq!(
            candidates[0].status,
            StoreTrackIdentityCandidateStatus::Stale
        );
        assert_eq!(
            candidates[0].evidence[0].evidence_status,
            StoreTrackIdentityCandidateEvidenceStatus::Stale
        );
    }

    #[test]
    fn evidence_read_status_is_stale_when_probe_fields_are_removed() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/no-probe.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        fixture.commit_current_facts(100, HASH_A, "audio", false);

        assert_eq!(
            fixture.single_evidence_status(),
            StoreTrackIdentityCandidateEvidenceStatus::Stale
        );
    }

    #[test]
    fn evidence_read_status_is_stale_when_media_kind_is_not_audio() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/not-audio.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        fixture.commit_current_facts(100, HASH_A, "video", true);

        assert_eq!(
            fixture.single_evidence_status(),
            StoreTrackIdentityCandidateEvidenceStatus::Stale
        );
    }

    #[test]
    fn evidence_read_status_is_stale_when_probe_artifact_changes() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/probe-artifact.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        fixture.commit_current_facts_with_artifact_id(100, HASH_A, "audio", true, 20_000);

        assert_eq!(
            fixture.single_evidence_status(),
            StoreTrackIdentityCandidateEvidenceStatus::Stale
        );
    }

    #[test]
    fn stale_attachment_link_evidence_prevents_candidate_refresh() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/hash-changed.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        fixture.commit_current_facts(100, HASH_B, "audio", true);
        let result = fixture.produce(10);

        assert_eq!(result.skipped_stale_primary_media_candidates, 1);
        assert_eq!(result.candidates_refreshed, 0);
        assert_eq!(result.evidence_refreshed, 0);
        assert_eq!(result.candidates_marked_stale, 1);
        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        assert_eq!(
            candidates[0].status,
            StoreTrackIdentityCandidateStatus::Stale
        );
        assert_eq!(
            candidates[0].evidence[0].evidence_status,
            StoreTrackIdentityCandidateEvidenceStatus::Stale
        );
    }

    #[test]
    fn duplicate_same_hash_source_files_group_as_evidence_not_canonical_track_identity() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/a.wav");
        fixture.insert_source_file(101, "Album/b.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.commit_current_facts(101, HASH_A, "audio", true);
        fixture.promote(10);

        fixture.produce(10);

        assert_eq!(fixture.count_rows("track_identity_candidates"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_members"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 2);
    }

    #[test]
    fn different_hashes_do_not_group_by_similar_path_names() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/track copy.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.link_attachment(101, HASH_B);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.commit_current_facts(101, HASH_B, "audio", true);
        fixture.promote(10);

        fixture.produce(10);

        assert_eq!(fixture.count_rows("track_identity_candidates"), 2);
        let candidates = fixture
            .store
            .read_track_identity_candidates_for_source(fixture.source_id, 10)
            .expect("read candidates");
        let mut hashes = candidates
            .iter()
            .map(|candidate| candidate.evidence_key_value.as_str())
            .collect::<Vec<_>>();
        hashes.sort_unstable();
        assert_eq!(hashes, vec![HASH_A, HASH_B]);
    }

    #[test]
    fn candidate_production_respects_explicit_limit() {
        let fixture = TrackIdentityCandidateFixture::new();
        for (source_file_id, path, hash) in [
            (100, "Album/track-a.wav", HASH_A),
            (101, "Album/track-b.wav", HASH_B),
            (102, "Album/track-c.wav", HASH_C),
        ] {
            fixture.insert_source_file(source_file_id, path);
            fixture.link_attachment(source_file_id, hash);
            fixture.commit_current_facts(source_file_id, hash, "audio", true);
        }
        fixture.promote(10);

        let first = fixture.produce(2);

        assert_eq!(first.candidates_created, 2);
        assert_eq!(first.remaining_candidates, 1);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 2);

        let second = fixture.produce(2);

        assert_eq!(second.candidates_created, 1);
        assert_eq!(second.remaining_candidates, 0);
        assert_eq!(fixture.count_rows("track_identity_candidates"), 3);
    }

    #[test]
    fn source_file_contents_surface_remains_default_after_candidate_production() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        let result = fixture
            .store
            .read_contents(
                StoreContentsScope::Source {
                    source_id: fixture.source_id,
                },
                StoreContentsReadPolicy::SourceFileInventory {
                    file_classes: vec![
                        StoreContentsFileClass::Audio,
                        StoreContentsFileClass::Video,
                        StoreContentsFileClass::Image,
                        StoreContentsFileClass::Unsupported,
                    ],
                },
                StoreContentsScopeDepth::Recursive,
                10,
                None,
            )
            .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert!(result.rows[0].primary_media.is_none());
    }

    #[test]
    fn candidate_production_writes_only_track_identity_candidates() {
        let fixture = TrackIdentityCandidateFixture::new();
        fixture.insert_source_file(100, "Album/track.wav");
        fixture.insert_source_file(101, "Album/album.cue");
        fixture.link_attachment(100, HASH_A);
        fixture.commit_current_facts(100, HASH_A, "audio", true);
        fixture.promote(10);
        fixture.produce(10);

        assert_eq!(fixture.count_rows("track_identity_candidates"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_members"), 1);
        assert_eq!(fixture.count_rows("track_identity_candidate_evidence"), 1);
        assert_eq!(fixture.count_rows("track_identity_decisions"), 0);
    }
}
