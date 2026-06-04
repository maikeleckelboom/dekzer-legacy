use crate::LibrarySqliteResult;
use crate::read_models::track_identity_review::{
    ReviewCandidate, ReviewState, read_track_identity_review_candidates,
};
use crate::store::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn read_track_identity_review_candidates(
        &self,
        source_id: Option<i64>,
        review_state_filter: Option<ReviewState>,
        limit: usize,
    ) -> LibrarySqliteResult<Vec<ReviewCandidate>> {
        let connection = self.open_read_connection()?;
        read_track_identity_review_candidates(&connection, source_id, review_state_filter, limit)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::TempDir;

    use crate::browse_sort_key::{
        compute_name_browse_sort_key, compute_relative_path_browse_sort_key,
    };
    use crate::read_models::track_identity_candidates::StoreTrackIdentityCandidateStatus;
    use crate::read_models::track_identity_decisions::{
        StoreTrackIdentityDecisionState, StoreTrackIdentityUserBlockingDecisionState,
    };
    use crate::read_models::track_identity_review::ReviewState;
    use crate::store::{
        SqliteDurableStore, TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
        TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
    };

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    struct Fixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
    }

    impl Fixture {
        fn new() -> Self {
            let tempdir = TempDir::new().expect("create tempdir");
            let store = SqliteDurableStore::open(tempdir.path().join("library.sqlite"))
                .expect("open store");
            Self {
                _tempdir: tempdir,
                store,
            }
        }

        fn read(
            &self,
            state: Option<ReviewState>,
            limit: usize,
        ) -> Vec<crate::StoreTrackIdentityReviewCandidate> {
            self.store
                .read_track_identity_review_candidates(None, state, limit)
                .expect("read review candidates")
        }

        fn read_for_source(
            &self,
            source_id: i64,
            state: Option<ReviewState>,
            limit: usize,
        ) -> Vec<crate::StoreTrackIdentityReviewCandidate> {
            self.store
                .read_track_identity_review_candidates(Some(source_id), state, limit)
                .expect("read source review candidates")
        }

        fn insert_candidate_with_current_evidence(
            &self,
            candidate_id: i64,
            source_id: i64,
            source_file_id: i64,
            hash: &str,
        ) {
            self.insert_source(source_id, &format!("Source {source_id}"));
            self.insert_source_file(source_id, source_file_id);
            self.insert_current_facts(source_id, source_file_id, hash);
            self.insert_attachment(source_file_id, hash);
            self.insert_primary_media_candidate(source_file_id);
            self.insert_candidate(candidate_id, hash, "active");
            self.insert_member(candidate_id, source_file_id, hash);
            self.insert_evidence(candidate_id, source_id, source_file_id, hash);
        }

        fn add_current_source_evidence(
            &self,
            candidate_id: i64,
            source_id: i64,
            source_file_id: i64,
            hash: &str,
        ) {
            self.insert_source(source_id, &format!("Source {source_id}"));
            self.insert_source_file(source_id, source_file_id);
            self.insert_current_facts(source_id, source_file_id, hash);
            self.insert_attachment(source_file_id, hash);
            self.insert_evidence(candidate_id, source_id, source_file_id, hash);
        }

        fn insert_source(&self, source_id: i64, display_name: &str) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO sources (
                             source_id, source_class, authority, identity_key,
                             display_name, created_at, updated_at
                         )
                         VALUES (?1, 'internal', 'system', ?2, ?3, 1, 1)",
                        params![source_id, format!("source:{source_id}"), display_name],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO source_state (
                             source_id, mount_status, mount_epoch, access_state,
                             access_checked_at, effective_path, updated_at
                         )
                         VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 1)",
                        [source_id],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO source_scan_state (
                             source_id, scan_phase, last_scan_started_at,
                             last_scan_finished_at, last_successful_scan_at, updated_at
                         )
                         VALUES (?1, 'complete', 1, 2, 2, 2)",
                        [source_id],
                    )?;
                    Ok(())
                })
                .expect("insert source");
        }

        fn insert_source_file(&self, source_id: i64, source_file_id: i64) {
            let name = format!("track-{source_file_id}.wav");
            let relative_path = format!("Album/{name}");
            let name_key = compute_name_browse_sort_key(&name);
            let relative_path_key = compute_relative_path_browse_sort_key(&relative_path);
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO source_files (
                             source_file_id, source_id, parent_source_directory_id,
                             name, name_browse_sort_key, relative_path_browse_sort_key,
                             relative_path, size_bytes, mtime_ns, file_kind, media_class,
                             presence_state, first_discovered_at, last_observed_at,
                             last_presence_change_at, created_at, updated_at
                         )
                         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 10, 100,
                                 'audio', 'audio', 'present', 1, 1, 1, 1, 1)",
                        params![
                            source_file_id,
                            source_id,
                            name,
                            name_key,
                            relative_path_key,
                            relative_path
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert source file");
        }

        fn insert_current_facts(&self, source_id: i64, source_file_id: i64, hash: &str) {
            self.insert_artifact(source_file_id);
            let relative_path = format!("Album/track-{source_file_id}.wav");
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO SourceFacts (
                             source_file_id, fact_kind, basis_fingerprint,
                             basis_source_id, basis_relative_path, basis_size_bytes,
                             basis_mtime_ns, basis_presence_state, observed_at_ms,
                             content_hash_algorithm, content_hash_value, media_kind,
                             mime_type, duration_ms, sample_rate_hz, channels,
                             bit_depth, codec, updated_at, accepted_artifact_id
                         )
                         VALUES (?1, 'source_inspection', ?2, ?3, ?4, 10, 100,
                                 'present', 1, 'blake3', ?5, 'audio', 'audio/wav',
                                 100, 44100, 2, 16, 'pcm', 1, ?6)
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
                            basis(source_file_id),
                            source_id,
                            relative_path,
                            hash,
                            artifact_id(source_file_id)
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert current facts");
        }

        fn insert_artifact(&self, source_file_id: i64) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO WorkItems (
                             work_item_id, subject_kind, subject_id, work_kind,
                             basis_fingerprint, state, priority_class, created_at, updated_at
                         )
                         VALUES (?1, 'source_file', ?2, 'inspect_source',
                                 ?3, 'completed', 'interactive', 1, 1)",
                        params![
                            source_file_id,
                            format!("fixture:{source_file_id}"),
                            basis(source_file_id)
                        ],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO WorkRuns (
                             work_run_id, work_item_id, adapter_key, adapter_version,
                             started_at, outcome
                         )
                         VALUES (?1, ?1, 'test.review', '1', 1, 'ok')",
                        [source_file_id],
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO Artifacts (
                             artifact_id, work_run_id, subject_kind, subject_id,
                             artifact_kind, artifact_role, adapter_key, adapter_version,
                             basis_fingerprint, media_type, storage_kind, payload_hash,
                             created_at
                         )
                         VALUES (?1, ?2, 'source_file', ?3, 'inspection_result',
                                 'primary_result', 'test.review', '1', ?4,
                                 'application/json', 'inline_payload', ?5, 1)",
                        params![
                            artifact_id(source_file_id),
                            source_file_id,
                            source_file_id.to_string(),
                            basis(source_file_id),
                            format!("payload:{source_file_id}")
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert artifact");
        }

        fn insert_attachment(&self, source_file_id: i64, hash: &str) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT OR IGNORE INTO content_attachments (
                             content_hash_algorithm, content_hash_value, first_observed_at,
                             updated_at
                         )
                         VALUES ('blake3', ?1, 1, 1)",
                        [hash],
                    )?;
                    let attachment_id: i64 = write.query_row(
                        "SELECT attachment_id
                         FROM content_attachments
                         WHERE content_hash_algorithm = 'blake3'
                           AND content_hash_value = ?1",
                        [hash],
                        |row| row.get(0),
                    )?;
                    let source_id: i64 = write.query_row(
                        "SELECT source_id FROM source_files WHERE source_file_id = ?1",
                        [source_file_id],
                        |row| row.get(0),
                    )?;
                    write.execute(
                        "INSERT OR IGNORE INTO source_file_attachment_links (
                             source_file_attachment_link_id, attachment_id, source_file_id,
                             source_id, file_kind, created_at, updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, 'audio', 1, 1)",
                        params![
                            link_id(source_file_id),
                            attachment_id,
                            source_file_id,
                            source_id
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert attachment");
        }

        fn insert_primary_media_candidate(&self, source_file_id: i64) {
            self.store
                .with_write(|write| {
                    let attachment_id = attachment_id_for_source_file(write, source_file_id)?;
                    write.execute(
                        "INSERT OR IGNORE INTO primary_media_candidates (
                             primary_media_candidate_id, attachment_id, evidence_source_file_id,
                             evidence_basis_fingerprint, media_kind, mime_type, duration_ms,
                             sample_rate_hz, channels, bit_depth, codec, created_at, updated_at
                        )
                         VALUES (?1, ?2, ?3, ?4, 'audio', 'audio/wav', 100,
                                 44100, 2, 16, 'pcm', 1, 1)",
                        params![
                            primary_media_id(source_file_id),
                            attachment_id,
                            source_file_id,
                            basis(source_file_id)
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert primary media candidate");
        }

        fn insert_candidate(&self, candidate_id: i64, hash: &str, status: &str) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "INSERT INTO track_identity_candidates (
                             track_identity_candidate_id, candidate_kind, evidence_basis,
                             evidence_key_algorithm, evidence_key_value, status,
                             created_at, updated_at
                         )
                         VALUES (?1, 'exact_primary_media_content',
                                 'current_primary_media_exact_blake3', 'blake3',
                                 ?2, ?3, ?4, ?4)",
                        params![candidate_id, hash, status, candidate_id * 10],
                    )?;
                    Ok(())
                })
                .expect("insert candidate");
        }

        fn insert_member(&self, candidate_id: i64, source_file_id: i64, hash: &str) {
            self.store
                .with_write(|write| {
                    let attachment_id = attachment_id_for_source_file(write, source_file_id)?;
                    write.execute(
                        "INSERT OR IGNORE INTO track_identity_candidate_members (
                             track_identity_candidate_member_id, track_identity_candidate_id,
                             primary_media_candidate_id, attachment_id, evidence_source_file_id,
                             evidence_basis_fingerprint, content_hash_algorithm,
                             content_hash_value, created_at, updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'blake3', ?7, 1, 1)",
                        params![
                            member_id(source_file_id),
                            candidate_id,
                            primary_media_id(source_file_id),
                            attachment_id,
                            source_file_id,
                            basis(source_file_id),
                            hash
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert member");
        }

        fn insert_evidence(
            &self,
            candidate_id: i64,
            source_id: i64,
            source_file_id: i64,
            hash: &str,
        ) {
            self.store
                .with_write(|write| {
                    let attachment_id = attachment_id_for_source_file(write, source_file_id)?;
                    let primary_media_candidate_id =
                        primary_media_id_for_candidate(write, candidate_id)?;
                    write.execute(
                        "INSERT OR IGNORE INTO track_identity_candidate_evidence (
                             track_identity_candidate_evidence_id, track_identity_candidate_id,
                             primary_media_candidate_id, attachment_id,
                             source_file_attachment_link_id, source_file_id, source_id,
                             evidence_basis_fingerprint, content_hash_algorithm,
                             content_hash_value, probe_accepted_artifact_id,
                             created_at, updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'blake3', ?9,
                                 ?10, 1, 1)",
                        params![
                            evidence_id(source_file_id),
                            candidate_id,
                            primary_media_candidate_id,
                            attachment_id,
                            link_id(source_file_id),
                            source_file_id,
                            source_id,
                            basis(source_file_id),
                            hash,
                            artifact_id(source_file_id)
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert evidence");
        }

        fn insert_decision(
            &self,
            decision_id: i64,
            candidate_id: i64,
            decision_state: &str,
            decision_source: &str,
        ) {
            self.store
                .with_write(|write| {
                    let (kind, evidence_basis, status, key_algorithm, key_value): (
                        String,
                        String,
                        String,
                        String,
                        String,
                    ) = write.query_row(
                        "SELECT candidate_kind, evidence_basis, status,
                                evidence_key_algorithm, evidence_key_value
                         FROM track_identity_candidates
                         WHERE track_identity_candidate_id = ?1",
                        [candidate_id],
                        |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        },
                    )?;
                    let basis = if decision_source
                        == TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0
                    {
                        "active_exact_content_candidate_current_evidence_v0"
                    } else {
                        "explicit_user_local_decision_v0"
                    };
                    write.execute(
                        "INSERT INTO track_identity_decisions (
                             track_identity_decision_id, track_identity_candidate_id,
                             decision_state, decision_source, decision_basis,
                             decision_reason, candidate_kind, candidate_evidence_basis,
                             candidate_status_at_decision, evidence_key_algorithm,
                             evidence_key_value, superseded_by_decision_id,
                             created_at, updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, 'fixture decision', ?6, ?7,
                                 ?8, ?9, ?10, NULL, ?11, ?11)",
                        params![
                            decision_id,
                            candidate_id,
                            decision_state,
                            decision_source,
                            basis,
                            kind,
                            evidence_basis,
                            status,
                            key_algorithm,
                            key_value,
                            decision_id * 10
                        ],
                    )?;
                    Ok(())
                })
                .expect("insert decision");
        }

        fn mark_candidate_stale(&self, candidate_id: i64) {
            self.store
                .with_write(|write| {
                    write.execute(
                        "UPDATE track_identity_candidates
                         SET status = 'stale',
                             updated_at = updated_at + 1
                         WHERE track_identity_candidate_id = ?1",
                        [candidate_id],
                    )?;
                    Ok(())
                })
                .expect("mark candidate stale");
        }
    }

    fn basis(source_file_id: i64) -> String {
        format!("basis:{source_file_id}")
    }

    fn artifact_id(source_file_id: i64) -> i64 {
        10_000 + source_file_id
    }

    fn attachment_id_for_source_file(
        connection: &rusqlite::Connection,
        source_file_id: i64,
    ) -> rusqlite::Result<i64> {
        connection.query_row(
            "SELECT attachment_id
             FROM source_file_attachment_links
             WHERE source_file_id = ?1",
            [source_file_id],
            |row| row.get(0),
        )
    }

    fn primary_media_id_for_candidate(
        connection: &rusqlite::Connection,
        candidate_id: i64,
    ) -> rusqlite::Result<i64> {
        connection.query_row(
            "SELECT primary_media_candidate_id
             FROM track_identity_candidate_members
             WHERE track_identity_candidate_id = ?1
             ORDER BY track_identity_candidate_member_id ASC
             LIMIT 1",
            [candidate_id],
            |row| row.get(0),
        )
    }

    fn link_id(source_file_id: i64) -> i64 {
        30_000 + source_file_id
    }

    fn primary_media_id(source_file_id: i64) -> i64 {
        40_000 + source_file_id
    }

    fn member_id(source_file_id: i64) -> i64 {
        50_000 + source_file_id
    }

    fn evidence_id(source_file_id: i64) -> i64 {
        60_000 + source_file_id
    }

    #[test]
    fn empty_read_returns_empty_candidate_list() {
        let fixture = Fixture::new();

        let candidates = fixture.read(None, 10);

        assert!(candidates.is_empty());
    }

    #[test]
    fn candidate_with_no_decision_returns_needs_user_decision() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);

        let candidates = fixture.read(None, 10);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].review_state, ReviewState::NeedsUserDecision);
        assert_eq!(candidates[0].effective_decision, None);
        assert_eq!(candidates[0].evidence_summary.member_count, 1);
        assert_eq!(candidates[0].evidence_summary.evidence_count, 1);
        assert_eq!(candidates[0].evidence_summary.current_evidence_count, 1);
        assert_eq!(candidates[0].source_summary.source_count, 1);
    }

    #[test]
    fn system_accepted_decision_returns_system_accepted() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_decision(
            1,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
        );

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.review_state, ReviewState::SystemAccepted);
        let decision = candidate.effective_decision.expect("effective decision");
        assert_eq!(decision.decision_id, 1);
        assert_eq!(
            decision.decision_state,
            StoreTrackIdentityDecisionState::Accepted
        );
        assert_eq!(
            decision.decision_source,
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0
        );
    }

    #[test]
    fn user_accepted_decision_returns_user_accepted_and_takes_precedence() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_decision(
            1,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
        );
        fixture.insert_decision(
            2,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.review_state, ReviewState::UserAccepted);
        let decision = candidate.effective_decision.expect("effective decision");
        assert_eq!(decision.decision_id, 2);
        assert_eq!(
            decision.decision_source,
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0
        );
        assert_eq!(
            decision.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::None
        );
    }

    #[test]
    fn user_rejected_decision_returns_user_rejected() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_decision(
            1,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
        );
        fixture.insert_decision(
            2,
            1,
            "rejected",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.review_state, ReviewState::UserRejected);
        let decision = candidate.effective_decision.expect("effective decision");
        assert_eq!(
            decision.user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Rejected
        );
        assert_eq!(decision.masked_system_decision_id, Some(1));
    }

    #[test]
    fn user_deferred_decision_returns_user_deferred() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_decision(
            1,
            1,
            "deferred",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.review_state, ReviewState::UserDeferred);
        assert_eq!(
            candidate
                .effective_decision
                .expect("effective decision")
                .user_blocking_decision_state,
            StoreTrackIdentityUserBlockingDecisionState::Deferred
        );
    }

    #[test]
    fn stale_effective_decision_returns_stale_decision() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_decision(
            1,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );
        fixture.mark_candidate_stale(1);

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.review_state, ReviewState::StaleDecision);
        assert_eq!(
            candidate.candidate_status,
            StoreTrackIdentityCandidateStatus::Stale
        );
    }

    #[test]
    fn source_filter_includes_candidates_with_evidence_from_that_source() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);

        let candidates = fixture.read_for_source(1, None, 10);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_id, 1);
    }

    #[test]
    fn source_filter_excludes_candidates_without_evidence_from_that_source() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_source(2, "Other Source");

        let candidates = fixture.read_for_source(2, None, 10);

        assert!(candidates.is_empty());
    }

    #[test]
    fn multi_source_candidate_reports_source_count_greater_than_one() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.add_current_source_evidence(1, 2, 200, HASH_A);

        let candidate = fixture.read(None, 10).remove(0);

        assert_eq!(candidate.source_summary.source_count, 2);
        assert_eq!(candidate.source_summary.source_samples.len(), 2);
    }

    #[test]
    fn limit_is_respected() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_candidate_with_current_evidence(2, 1, 101, HASH_B);
        fixture.insert_candidate_with_current_evidence(3, 1, 102, HASH_C);

        let candidates = fixture.read(None, 2);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].candidate_id, 1);
        assert_eq!(candidates[1].candidate_id, 2);
    }

    #[test]
    fn review_state_filter_is_respected() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_candidate_with_current_evidence(2, 1, 101, HASH_B);
        fixture.insert_decision(
            1,
            2,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );

        let candidates = fixture.read(Some(ReviewState::UserAccepted), 10);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_id, 2);
        assert_eq!(candidates[0].review_state, ReviewState::UserAccepted);
    }

    #[test]
    fn review_read_does_not_use_decision_source_scope_for_live_membership() {
        let fixture = Fixture::new();
        fixture.insert_candidate_with_current_evidence(1, 1, 100, HASH_A);
        fixture.insert_source(2, "Decision Scope Only");
        fixture.insert_decision(
            1,
            1,
            "accepted",
            TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0,
        );
        fixture
            .store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO track_identity_decision_source_scope (
                         track_identity_decision_id, track_identity_candidate_id,
                         source_id, scope_basis, created_at, updated_at
                     )
                     VALUES (1, 1, 2, 'candidate_source_provenance_v0', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .expect("insert decision source scope");

        let candidates = fixture.read_for_source(2, None, 10);

        assert!(
            candidates.is_empty(),
            "source-filtered review reads must use candidate evidence provenance"
        );
    }
}
