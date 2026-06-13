use std::collections::BTreeMap;

use rusqlite::{OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::store::source_file_hash::SOURCE_FILE_BLAKE3_ALGORITHM;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaterializeAttachmentsForSourceResult {
    pub attachments_created: usize,
    pub attachments_refreshed: usize,
    pub links_created: usize,
    pub links_replaced: usize,
    pub links_refreshed: usize,
    pub skipped_stale_facts: usize,
    pub skipped_no_blake3: usize,
    pub skipped_no_facts: usize,
    pub remaining_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttachmentMaterializationCandidate {
    source_file_id: i64,
    source_id: i64,
    file_kind: String,
    content_hash_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingAttachmentLink {
    attachment_id: i64,
    content_hash_value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFileAttachmentLinkChange {
    Created,
    Replaced,
    Refreshed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentAttachmentChange {
    Created { attachment_id: i64 },
    Refreshed { attachment_id: i64 },
}

impl ContentAttachmentChange {
    fn attachment_id(self) -> i64 {
        match self {
            ContentAttachmentChange::Created { attachment_id }
            | ContentAttachmentChange::Refreshed { attachment_id } => attachment_id,
        }
    }
}

impl SqliteDurableStore {
    pub fn materialize_attachments_for_source(
        &self,
        source_id: i64,
        limit: usize,
    ) -> LibrarySqliteResult<MaterializeAttachmentsForSourceResult> {
        let materialized_at = unix_time_ms()?;
        self.with_write(|write| {
            materialize_attachments_for_source(write, source_id, limit, materialized_at)
        })
    }
}

fn materialize_attachments_for_source(
    write: &mut AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
    materialized_at: i64,
) -> LibrarySqliteResult<MaterializeAttachmentsForSourceResult> {
    let mut result = read_attachment_materialization_skip_summary(write, source_id)?;
    let materializable_count = count_attachment_materialization_candidates(write, source_id)?;
    result.remaining_candidates = materializable_count.saturating_sub(limit);

    let mut attachment_ids_by_hash = BTreeMap::new();
    for candidate in read_attachment_materialization_candidates(write, source_id, limit)? {
        let attachment_id = match attachment_ids_by_hash.get(&candidate.content_hash_value) {
            Some(attachment_id) => *attachment_id,
            None => {
                let attachment_change = upsert_content_attachment(
                    write,
                    &candidate.content_hash_value,
                    materialized_at,
                )?;
                let attachment_id = attachment_change.attachment_id();
                attachment_ids_by_hash.insert(candidate.content_hash_value.clone(), attachment_id);
                match attachment_change {
                    ContentAttachmentChange::Created { .. } => result.attachments_created += 1,
                    ContentAttachmentChange::Refreshed { .. } => {
                        result.attachments_refreshed += 1;
                    }
                }
                attachment_id
            }
        };

        match materialize_source_file_attachment_link(
            write,
            &candidate,
            attachment_id,
            materialized_at,
        )? {
            SourceFileAttachmentLinkChange::Created => result.links_created += 1,
            SourceFileAttachmentLinkChange::Replaced => result.links_replaced += 1,
            SourceFileAttachmentLinkChange::Refreshed => result.links_refreshed += 1,
        }
    }

    Ok(result)
}

fn read_attachment_materialization_skip_summary(
    write: &AdmittedWrite<'_>,
    source_id: i64,
) -> LibrarySqliteResult<MaterializeAttachmentsForSourceResult> {
    write
        .query_row(
            "SELECT COALESCE(SUM(CASE
                    WHEN facts.source_file_id IS NULL THEN 1
                    ELSE 0
                END), 0),
                COALESCE(SUM(CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND NOT (
                         file.source_id = facts.basis_source_id
                         AND file.relative_path = facts.basis_relative_path
                         AND file.size_bytes IS facts.basis_size_bytes
                         AND file.mtime_ns IS facts.basis_mtime_ns
                         AND file.presence_state = facts.basis_presence_state
                     )
                    THEN 1 ELSE 0
                END), 0),
                COALESCE(SUM(CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND (
                         file.source_id = facts.basis_source_id
                         AND file.relative_path = facts.basis_relative_path
                         AND file.size_bytes IS facts.basis_size_bytes
                         AND file.mtime_ns IS facts.basis_mtime_ns
                         AND file.presence_state = facts.basis_presence_state
                     )
                     AND NOT (
                         facts.content_hash_algorithm = ?2
                         AND facts.content_hash_value IS NOT NULL
                     )
                    THEN 1 ELSE 0
                END), 0)
             FROM source_files file
             LEFT JOIN source_file_facts facts
               ON facts.source_file_id = file.source_file_id
             WHERE file.source_id = ?1",
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| {
                Ok(MaterializeAttachmentsForSourceResult {
                    skipped_no_facts: read_count(row, 0)?,
                    skipped_stale_facts: read_count(row, 1)?,
                    skipped_no_blake3: read_count(row, 2)?,
                    ..Default::default()
                })
            },
        )
        .map_err(Into::into)
}

fn count_attachment_materialization_candidates(
    write: &AdmittedWrite<'_>,
    source_id: i64,
) -> LibrarySqliteResult<usize> {
    write
        .query_row(
            "SELECT COUNT(*)
             FROM source_files file
             JOIN source_file_facts facts
               ON facts.source_file_id = file.source_file_id
             WHERE file.source_id = ?1
               AND file.source_id = facts.basis_source_id
               AND file.relative_path = facts.basis_relative_path
               AND file.size_bytes IS facts.basis_size_bytes
               AND file.mtime_ns IS facts.basis_mtime_ns
               AND file.presence_state = facts.basis_presence_state
               AND facts.content_hash_algorithm = ?2
               AND facts.content_hash_value IS NOT NULL",
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM],
            |row| read_count(row, 0),
        )
        .map_err(Into::into)
}

fn read_attachment_materialization_candidates(
    write: &AdmittedWrite<'_>,
    source_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Vec<AttachmentMaterializationCandidate>> {
    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let mut statement = write.prepare(
        "SELECT file.source_file_id,
                file.source_id,
                file.file_kind,
                facts.content_hash_value
         FROM source_files file
         JOIN source_file_facts facts
           ON facts.source_file_id = file.source_file_id
         LEFT JOIN source_file_attachment_links link
           ON link.source_file_id = file.source_file_id
         LEFT JOIN content_attachments attachment
           ON attachment.attachment_id = link.attachment_id
         WHERE file.source_id = ?1
           AND file.source_id = facts.basis_source_id
           AND file.relative_path = facts.basis_relative_path
           AND file.size_bytes IS facts.basis_size_bytes
           AND file.mtime_ns IS facts.basis_mtime_ns
           AND file.presence_state = facts.basis_presence_state
           AND facts.content_hash_algorithm = ?2
           AND facts.content_hash_value IS NOT NULL
         ORDER BY CASE
                    WHEN link.source_file_id IS NULL THEN 0
                    WHEN attachment.content_hash_value != facts.content_hash_value THEN 1
                    ELSE 2
                  END ASC,
                  lower(file.relative_path) ASC,
                  file.source_file_id ASC
         LIMIT ?3",
    )?;

    statement
        .query_map(
            params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM, limit_i64],
            |row| {
                Ok(AttachmentMaterializationCandidate {
                    source_file_id: row.get(0)?,
                    source_id: row.get(1)?,
                    file_kind: row.get(2)?,
                    content_hash_value: row.get(3)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn read_count(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

fn upsert_content_attachment(
    write: &AdmittedWrite<'_>,
    content_hash_value: &str,
    materialized_at: i64,
) -> LibrarySqliteResult<ContentAttachmentChange> {
    if let Some(attachment_id) = write
        .query_row(
            "SELECT attachment_id
             FROM content_attachments
             WHERE content_hash_algorithm = ?1
               AND content_hash_value = ?2",
            params![SOURCE_FILE_BLAKE3_ALGORITHM, content_hash_value],
            |row| row.get(0),
        )
        .optional()?
    {
        write.execute(
            "UPDATE content_attachments
             SET updated_at = ?3
             WHERE content_hash_algorithm = ?1
               AND content_hash_value = ?2",
            params![
                SOURCE_FILE_BLAKE3_ALGORITHM,
                content_hash_value,
                materialized_at,
            ],
        )?;
        return Ok(ContentAttachmentChange::Refreshed { attachment_id });
    }

    write.execute(
        "INSERT INTO content_attachments (
             content_hash_algorithm,
             content_hash_value,
             first_observed_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?3)
        ",
        params![
            SOURCE_FILE_BLAKE3_ALGORITHM,
            content_hash_value,
            materialized_at,
        ],
    )?;
    let attachment_id = write
        .query_row(
            "SELECT attachment_id
             FROM content_attachments
             WHERE content_hash_algorithm = ?1
               AND content_hash_value = ?2",
            params![SOURCE_FILE_BLAKE3_ALGORITHM, content_hash_value],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            LibrarySqliteError::MalformedSchemaState(
                "content attachment insert did not leave an addressable row".to_string(),
            )
        })?;
    Ok(ContentAttachmentChange::Created { attachment_id })
}

fn materialize_source_file_attachment_link(
    write: &AdmittedWrite<'_>,
    candidate: &AttachmentMaterializationCandidate,
    attachment_id: i64,
    materialized_at: i64,
) -> LibrarySqliteResult<SourceFileAttachmentLinkChange> {
    let existing_links = read_existing_attachment_links(write, candidate.source_file_id)?;
    if existing_links.iter().any(|link| {
        link.attachment_id == attachment_id
            && link.content_hash_value == candidate.content_hash_value
    }) {
        write.execute(
            "UPDATE source_file_attachment_links
             SET updated_at = ?3
             WHERE source_file_id = ?1
               AND attachment_id = ?2",
            params![candidate.source_file_id, attachment_id, materialized_at],
        )?;
        return Ok(SourceFileAttachmentLinkChange::Refreshed);
    }

    let replaced = !existing_links.is_empty();
    if replaced {
        write.execute(
            "DELETE FROM source_file_attachment_links
             WHERE source_file_id = ?1",
            [candidate.source_file_id],
        )?;
    }

    write.execute(
        "INSERT INTO source_file_attachment_links (
             attachment_id,
             source_file_id,
             source_id,
             file_kind,
             created_at,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            attachment_id,
            candidate.source_file_id,
            candidate.source_id,
            candidate.file_kind,
            materialized_at,
        ],
    )?;

    Ok(if replaced {
        SourceFileAttachmentLinkChange::Replaced
    } else {
        SourceFileAttachmentLinkChange::Created
    })
}

fn read_existing_attachment_links(
    write: &AdmittedWrite<'_>,
    source_file_id: i64,
) -> LibrarySqliteResult<Vec<ExistingAttachmentLink>> {
    write
        .prepare(
            "SELECT link.attachment_id,
                    attachment.content_hash_value
             FROM source_file_attachment_links link
             JOIN content_attachments attachment
               ON attachment.attachment_id = link.attachment_id
             WHERE link.source_file_id = ?1
             ORDER BY link.source_file_attachment_link_id ASC",
        )?
        .query_map([source_file_id], |row| {
            Ok(ExistingAttachmentLink {
                attachment_id: row.get(0)?,
                content_hash_value: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use crate::read_models::attachment_identity::{
        StoreAttachmentOccurrenceStatus, StoreSourceFileAttachmentLinkStatus,
        get_attachment_for_source_file, get_source_attachment_summary,
        get_source_files_for_attachment, get_source_files_for_attachment_limited,
    };
    use crate::{
        CommitAcceptedSourceFileFactsInput, CommitAcceptedSourceFileFactsMergePolicy,
        CompleteMachineWorkInput, ContentHashEvidence, FinishWorkRunInput,
        InspectSourceFilePromotionInput, QueueInspectSourceFileWorkInput, RecordArtifactInput,
        RecordInlineArtifactInput, RecordSourceFileObservationInput, StartWorkRunInput,
        UpsertSourceInput, UpsertSourceStateInput,
    };
    use library_domain::{
        ArtifactKind, ArtifactRole, SourceAccessIssueKind, SourceAccessState, SourceFileId,
        SourcePresenceState, WorkPriorityClass, WorkRunOutcome,
    };

    use super::{MaterializeAttachmentsForSourceResult, SqliteDurableStore};

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    struct AttachmentIdentityFixture {
        _tempdir: TempDir,
        store: SqliteDurableStore,
        source_id: i64,
        next_at: i64,
    }

    impl AttachmentIdentityFixture {
        fn new() -> Self {
            let tempdir = TempDir::new().expect("create tempdir");
            let db_path = tempdir.path().join("library.sqlite3");
            let store = SqliteDurableStore::open(&db_path).expect("open store");
            let source_id = store
                .upsert_source(UpsertSourceInput {
                    source_id: None,
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "attachment-identity-source".to_string(),
                    display_name: "Attachment Identity Source".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    source_navigation_order_ordinal: Some(0),
                    changed_at: 1,
                })
                .expect("upsert source");
            store
                .upsert_source_state(UpsertSourceStateInput {
                    source_id,
                    mount_status: "mounted".to_string(),
                    mount_epoch: 1,
                    access_state: SourceAccessState::Accessible,
                    access_issue_kind: None,
                    access_error_detail: None,
                    access_checked_at: Some(1),
                    mount_root: None,
                    effective_path: None,
                    observed_volume_label: None,
                    filesystem_type: None,
                    last_seen_at: Some(1),
                    updated_at: 1,
                })
                .expect("seed source state");
            Self {
                _tempdir: tempdir,
                store,
                source_id,
                next_at: 10,
            }
        }

        fn record_source_file(
            &mut self,
            source_file_id: i64,
            relative_path: &str,
            size_bytes: i64,
            mtime_ns: i64,
        ) {
            let changed_at = self.tick();
            self.store
                .record_source_file_observation(RecordSourceFileObservationInput {
                    source_file_id: Some(source_file_id),
                    source_id: self.source_id,
                    parent_source_directory_id: None,
                    name: relative_path
                        .rsplit('/')
                        .next()
                        .expect("relative path has file name")
                        .to_string(),
                    relative_path: relative_path.to_string(),
                    size_bytes: Some(size_bytes),
                    mtime_ns: Some(mtime_ns),
                    presence_state: SourcePresenceState::Present,
                    first_discovered_at: Some(changed_at),
                    observed_at: Some(changed_at),
                    presence_changed_at: changed_at,
                    updated_at: changed_at,
                })
                .expect("record source file");
        }

        fn record_source_file_presence(
            &mut self,
            source_file_id: i64,
            relative_path: &str,
            size_bytes: i64,
            mtime_ns: i64,
            presence_state: SourcePresenceState,
        ) {
            let changed_at = self.tick();
            self.store
                .record_source_file_observation(RecordSourceFileObservationInput {
                    source_file_id: Some(source_file_id),
                    source_id: self.source_id,
                    parent_source_directory_id: None,
                    name: relative_path
                        .rsplit('/')
                        .next()
                        .expect("relative path has file name")
                        .to_string(),
                    relative_path: relative_path.to_string(),
                    size_bytes: Some(size_bytes),
                    mtime_ns: Some(mtime_ns),
                    presence_state,
                    first_discovered_at: Some(changed_at),
                    observed_at: Some(changed_at),
                    presence_changed_at: changed_at,
                    updated_at: changed_at,
                })
                .expect("record source file presence");
        }

        fn set_source_state(
            &mut self,
            mount_status: &str,
            access_state: SourceAccessState,
            access_issue_kind: Option<SourceAccessIssueKind>,
        ) {
            let updated_at = self.tick();
            self.store
                .upsert_source_state(UpsertSourceStateInput {
                    source_id: self.source_id,
                    mount_status: mount_status.to_string(),
                    mount_epoch: 1,
                    access_state,
                    access_issue_kind,
                    access_error_detail: None,
                    access_checked_at: Some(updated_at),
                    mount_root: None,
                    effective_path: None,
                    observed_volume_label: None,
                    filesystem_type: None,
                    last_seen_at: Some(updated_at),
                    updated_at,
                })
                .expect("update source state");
        }

        fn commit_blake3_fact(&mut self, source_file_id: i64, hash_value: &str, media_kind: &str) {
            self.commit_source_fact(source_file_id, Some(("blake3", hash_value)), media_kind);
        }

        fn commit_source_fact(
            &mut self,
            source_file_id: i64,
            content_hash: Option<(&str, &str)>,
            media_kind: &str,
        ) {
            let queued_at = self.tick();
            let basis_fingerprint = format!("attachment-test:basis:{source_file_id}:{queued_at}");
            let queued = self
                .store
                .queue_inspect_source_file_work(QueueInspectSourceFileWorkInput {
                    source_file_id: source_file_domain_id(source_file_id),
                    basis_fingerprint: basis_fingerprint.clone(),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at,
                })
                .expect("queue inspect source work");
            let claimed = self
                .store
                .claim_machine_work_batch(crate::ClaimMachineWorkBatchInput {
                    limit: 64,
                    lease_duration_ms: 30_000,
                    claimed_at: queued_at + 1,
                })
                .expect("claim inspect source work")
                .into_iter()
                .find(|claimed| claimed.work_item_id == queued.work_item_id)
                .expect("queued inspect work claimed");
            let work_run = self
                .store
                .start_work_run(StartWorkRunInput {
                    work_item_id: claimed.work_item_id,
                    adapter_key: "test.attachment_identity".to_string(),
                    adapter_version: "1".to_string(),
                    started_at: queued_at + 2,
                })
                .expect("start work run");
            let artifact = self
                .store
                .record_inline_artifact(RecordInlineArtifactInput {
                    artifact: RecordArtifactInput {
                        work_run_id: work_run.work_run_id,
                        artifact_kind: ArtifactKind::InspectionResult,
                        artifact_role: ArtifactRole::PrimaryResult,
                        media_type: "application/json".to_string(),
                        basis_fingerprint: basis_fingerprint.clone(),
                        payload_hash: format!("hash:attachment-test:{source_file_id}:{queued_at}"),
                        created_at: queued_at + 3,
                    },
                    payload: b"{}".to_vec(),
                })
                .expect("record inspection artifact");
            self.store
                .inspect_source_file(InspectSourceFilePromotionInput {
                    source_file_facts: CommitAcceptedSourceFileFactsInput {
                        source_file_id: source_file_domain_id(source_file_id),
                        accepted_artifact_id: artifact.artifact_id,
                        basis_fingerprint,
                        observed_at_ms: queued_at + 4,
                        content_hash: content_hash.map(|(algorithm, value)| ContentHashEvidence {
                            algorithm: algorithm.to_string(),
                            value: value.to_string(),
                        }),
                        media_kind: media_kind.to_string(),
                        mime_type: None,
                        duration_ms: None,
                        sample_rate_hz: None,
                        channels: None,
                        bit_depth: None,
                        codec: None,
                        updated_at: queued_at + 4,
                    },
                    source_file_facts_merge_policy:
                        CommitAcceptedSourceFileFactsMergePolicy::replacement(),
                    rebuild_projection_domains: vec![],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("commit source facts through inspect_source_file");
            self.store
                .finish_work_run(FinishWorkRunInput {
                    work_run_id: work_run.work_run_id,
                    finished_at: queued_at + 5,
                    outcome: WorkRunOutcome::Completed,
                    failure_kind: None,
                    error_detail: None,
                })
                .expect("finish work run");
            self.store
                .complete_machine_work_item(CompleteMachineWorkInput {
                    work_item_id: claimed.work_item_id,
                    completed_at: queued_at + 6,
                })
                .expect("complete work item");
        }

        fn materialize(&self, limit: usize) -> MaterializeAttachmentsForSourceResult {
            self.store
                .materialize_attachments_for_source(self.source_id, limit)
                .expect("materialize attachments")
        }

        fn read_connection(&self) -> Connection {
            self.store.open_read_connection().expect("open read")
        }

        fn count_rows(&self, table: &str) -> i64 {
            let connection = self.read_connection();
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count rows")
        }

        fn attachment_id_for_hash(&self, hash_value: &str) -> i64 {
            let connection = self.read_connection();
            connection
                .query_row(
                    "SELECT attachment_id
                     FROM content_attachments
                     WHERE content_hash_algorithm = 'blake3'
                       AND content_hash_value = ?1",
                    [hash_value],
                    |row| row.get(0),
                )
                .expect("attachment id for hash")
        }

        fn link_count_for_source_file(&self, source_file_id: i64) -> i64 {
            let connection = self.read_connection();
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM source_file_attachment_links
                     WHERE source_file_id = ?1",
                    [source_file_id],
                    |row| row.get(0),
                )
                .expect("count links for source file")
        }

        fn link_count_for_attachment(&self, attachment_id: i64) -> i64 {
            let connection = self.read_connection();
            connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM source_file_attachment_links
                     WHERE attachment_id = ?1",
                    [attachment_id],
                    |row| row.get(0),
                )
                .expect("count links for attachment")
        }

        fn tick(&mut self) -> i64 {
            let at = self.next_at;
            self.next_at += 10;
            at
        }
    }

    #[test]
    fn current_blake3_fact_materializes_one_attachment_and_current_link() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 123, 456);
        fixture.commit_blake3_fact(100, HASH_A, "audio");

        let result = fixture.materialize(10);

        assert_eq!(
            result,
            MaterializeAttachmentsForSourceResult {
                attachments_created: 1,
                links_created: 1,
                ..Default::default()
            }
        );
        assert_eq!(fixture.count_rows("content_attachments"), 1);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 1);

        let connection = fixture.read_connection();
        let link = get_attachment_for_source_file(&connection, 100)
            .expect("read source-file attachment")
            .expect("attachment link exists");
        assert_eq!(link.source_file_id, 100);
        assert_eq!(link.source_id, fixture.source_id);
        assert_eq!(link.content_hash_algorithm, "blake3");
        assert_eq!(link.content_hash_value, HASH_A);
        assert_eq!(link.file_kind, "audio");
        assert_eq!(
            link.link_status,
            StoreSourceFileAttachmentLinkStatus::Current
        );
        assert!(link.created_at > 0);
        assert!(link.updated_at >= link.created_at);
    }

    #[test]
    fn link_pointing_at_different_attachment_hash_reads_as_stale() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 123, 456);
        fixture.commit_blake3_fact(100, HASH_A, "audio");

        let connection = fixture.read_connection();
        connection
            .execute(
                "INSERT INTO content_attachments (
                     attachment_id,
                     content_hash_algorithm,
                     content_hash_value,
                     first_observed_at,
                     updated_at
                 )
                 VALUES (999, 'blake3', ?1, 1, 1)",
                [HASH_B],
            )
            .expect("insert mismatched attachment");
        connection
            .execute(
                "INSERT INTO source_file_attachment_links (
                     attachment_id,
                     source_file_id,
                     source_id,
                     file_kind,
                     created_at,
                     updated_at
                 )
                 VALUES (999, 100, ?1, 'audio', 1, 1)",
                [fixture.source_id],
            )
            .expect("insert mismatched link");

        let link = get_attachment_for_source_file(&connection, 100)
            .expect("read source-file attachment")
            .expect("mismatched link exists");

        assert_eq!(link.content_hash_value, HASH_B);
        assert_eq!(link.link_status, StoreSourceFileAttachmentLinkStatus::Stale);

        let result = fixture.materialize(10);
        assert_eq!(result.attachments_created, 1);
        assert_eq!(result.links_replaced, 1);

        let replacement = get_attachment_for_source_file(&connection, 100)
            .expect("read replacement source-file attachment")
            .expect("replacement link exists");
        assert_eq!(replacement.content_hash_value, HASH_A);
        assert_eq!(
            replacement.link_status,
            StoreSourceFileAttachmentLinkStatus::Current
        );
    }

    #[test]
    fn same_blake3_value_across_two_source_files_shares_one_attachment() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a.flac", 10, 100);
        fixture.record_source_file(101, "Album/b.flac", 10, 101);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_A, "audio");

        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 1);
        assert_eq!(result.attachments_refreshed, 0);
        assert_eq!(result.links_created, 2);
        assert_eq!(fixture.count_rows("content_attachments"), 1);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 2);

        let attachment_id = fixture.attachment_id_for_hash(HASH_A);
        let connection = fixture.read_connection();
        let links = get_source_files_for_attachment(&connection, attachment_id)
            .expect("read attachment source files");
        assert_eq!(links.len(), 2);
        assert_eq!(
            links
                .iter()
                .map(|link| link.source_file_id)
                .collect::<Vec<_>>(),
            vec![100, 101]
        );
        assert_eq!(
            links
                .iter()
                .map(|link| link.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["Album/a.flac", "Album/b.flac"],
            "attachment occurrence read order is deterministic by source_id then source_file_id"
        );
        assert!(
            links
                .iter()
                .all(|link| link.link_status == StoreSourceFileAttachmentLinkStatus::Current)
        );
        assert!(
            links
                .iter()
                .all(|link| link.occurrence_status == StoreAttachmentOccurrenceStatus::Available)
        );

        let bounded = get_source_files_for_attachment_limited(&connection, attachment_id, 1)
            .expect("read bounded attachment source files")
            .expect("attachment exists");
        assert_eq!(bounded.source_file_links.len(), 1);
        assert_eq!(bounded.remaining_source_file_links, 1);
        assert_eq!(bounded.summary.total_occurrence_count, 2);
        assert_eq!(bounded.summary.available_occurrence_count, 2);
        assert_eq!(bounded.summary.unavailable_occurrence_count, 0);
        assert_eq!(bounded.summary.current_link_occurrence_count, 2);
        assert_eq!(bounded.summary.stale_link_occurrence_count, 0);
        assert_eq!(bounded.summary.distinct_source_count, 1);
        assert!(bounded.summary.has_multiple_occurrences);
    }

    #[test]
    fn attachment_occurrence_read_keeps_missing_and_offline_evidence_visible() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/available.flac", 10, 100);
        fixture.record_source_file(101, "Album/missing.flac", 11, 101);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_A, "audio");
        fixture.materialize(10);
        let attachment_id = fixture.attachment_id_for_hash(HASH_A);

        fixture.record_source_file_presence(
            101,
            "Album/missing.flac",
            11,
            101,
            SourcePresenceState::Missing,
        );

        let connection = fixture.read_connection();
        let links = get_source_files_for_attachment(&connection, attachment_id)
            .expect("read attachment source files after missing observation");
        assert_eq!(
            links
                .iter()
                .map(|link| (link.source_file_id, link.presence_state.as_str()))
                .collect::<Vec<_>>(),
            vec![(100, "present"), (101, "missing")],
            "missing source-file occurrence evidence must remain visible"
        );
        assert_eq!(
            links[1].occurrence_status,
            StoreAttachmentOccurrenceStatus::FileMissing
        );
        assert_eq!(
            links[1].link_status,
            StoreSourceFileAttachmentLinkStatus::Stale,
            "presence changes make the old attachment link stale without deleting it"
        );

        fixture.set_source_state("unmounted", SourceAccessState::Unknown, None);
        let offline = get_source_files_for_attachment_limited(&connection, attachment_id, 10)
            .expect("read attachment source files after source offline")
            .expect("attachment exists");
        assert_eq!(offline.source_file_links.len(), 2);
        assert_eq!(
            offline
                .source_file_links
                .iter()
                .map(|link| (link.source_file_id, link.presence_state.as_str()))
                .collect::<Vec<_>>(),
            vec![(100, "present"), (101, "missing")],
            "source unavailability must not erase persisted source-file presence evidence"
        );
        assert!(
            offline
                .source_file_links
                .iter()
                .all(|link| link.occurrence_status
                    == StoreAttachmentOccurrenceStatus::SourceUnavailable),
            "offline source occurrences must remain visible instead of collapsing into absence"
        );
        assert_eq!(offline.summary.total_occurrence_count, 2);
        assert_eq!(offline.summary.available_occurrence_count, 0);
        assert_eq!(offline.summary.unavailable_occurrence_count, 2);
        assert_eq!(offline.summary.current_link_occurrence_count, 1);
        assert_eq!(offline.summary.stale_link_occurrence_count, 1);
    }

    #[test]
    fn attachment_occurrence_read_does_not_require_occurrence_tables() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.materialize(10);
        let attachment_id = fixture.attachment_id_for_hash(HASH_A);
        let connection = fixture.read_connection();

        let read = get_source_files_for_attachment_limited(&connection, attachment_id, 10)
            .expect("read attachment source files")
            .expect("attachment exists");
        assert_eq!(read.summary.total_occurrence_count, 1);

        let occurrence_tables = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM sqlite_schema
                 WHERE type = 'table'
                   AND lower(name) LIKE '%occurrence%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count occurrence tables");
        assert_eq!(
            occurrence_tables, 0,
            "A-5 occurrence read must derive from existing attachment/source-file evidence"
        );
    }

    #[test]
    fn different_blake3_values_materialize_distinct_attachments() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a.flac", 10, 100);
        fixture.record_source_file(101, "Album/b.flac", 11, 101);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_B, "audio");

        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 2);
        assert_eq!(result.links_created, 2);
        assert_eq!(fixture.count_rows("content_attachments"), 2);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 2);
        assert_ne!(
            fixture.attachment_id_for_hash(HASH_A),
            fixture.attachment_id_for_hash(HASH_B)
        );
    }

    #[test]
    fn source_attachment_summary_counts_current_stale_and_missing_links() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a-current.flac", 10, 100);
        fixture.record_source_file(101, "Album/b-stale.flac", 11, 101);
        fixture.record_source_file(102, "Album/c-missing-link.flac", 12, 102);
        fixture.record_source_file(103, "Album/d-no-facts.flac", 13, 103);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_B, "audio");
        fixture.commit_blake3_fact(102, HASH_C, "audio");
        fixture.materialize(2);
        fixture.commit_blake3_fact(101, HASH_C, "audio");

        let connection = fixture.read_connection();
        let summary = get_source_attachment_summary(&connection, fixture.source_id)
            .expect("read source attachment summary")
            .expect("source exists");

        assert_eq!(summary.source_id, fixture.source_id);
        assert_eq!(summary.current_links_count, 1);
        assert_eq!(summary.stale_links_count, 1);
        assert_eq!(summary.source_files_with_current_blake3_facts_count, 3);
        assert_eq!(summary.source_files_with_attachment_links_count, 2);
        assert_eq!(summary.source_files_missing_attachment_links_count, 1);
    }

    #[test]
    fn source_attachment_summary_returns_none_for_unknown_source() {
        let fixture = AttachmentIdentityFixture::new();
        let connection = fixture.read_connection();

        assert!(
            get_source_attachment_summary(&connection, 99_999)
                .expect("read missing source attachment summary")
                .is_none()
        );
    }

    #[test]
    fn bounded_materialization_prioritizes_unlinked_rows_before_refreshes() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a.flac", 10, 100);
        fixture.record_source_file(101, "Album/b.flac", 11, 101);
        fixture.record_source_file(102, "Album/c.flac", 12, 102);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_B, "audio");
        fixture.commit_blake3_fact(102, HASH_C, "audio");

        let first = fixture.materialize(2);
        assert_eq!(first.links_created, 2);
        assert_eq!(first.remaining_candidates, 1);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 2);
        assert_eq!(fixture.link_count_for_source_file(102), 0);

        let second = fixture.materialize(2);
        assert_eq!(second.links_created, 1);
        assert_eq!(second.links_refreshed, 1);
        assert_eq!(
            fixture.count_rows("source_file_attachment_links"),
            3,
            "a later bounded run must reach unlinked rows before refreshing current links"
        );
        assert_eq!(fixture.link_count_for_source_file(102), 1);
    }

    #[test]
    fn bounded_materialization_counts_skips_without_consuming_candidate_window() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a.flac", 10, 100);
        fixture.record_source_file(101, "Album/b.flac", 11, 101);
        fixture.record_source_file(102, "Album/no-facts.flac", 12, 102);
        fixture.record_source_file(103, "Album/no-blake3.flac", 13, 103);
        fixture.record_source_file(104, "Album/stale.flac", 14, 104);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_B, "audio");
        fixture.commit_source_fact(103, Some(("sha256", "fixture-sha")), "audio");
        fixture.commit_blake3_fact(104, HASH_C, "audio");
        fixture.record_source_file(104, "Album/stale.flac", 15, 105);

        let result = fixture.materialize(1);

        assert_eq!(result.links_created, 1);
        assert_eq!(result.remaining_candidates, 1);
        assert_eq!(result.skipped_no_facts, 1);
        assert_eq!(result.skipped_no_blake3, 1);
        assert_eq!(result.skipped_stale_facts, 1);
        assert_eq!(fixture.link_count_for_source_file(100), 1);
        assert_eq!(fixture.link_count_for_source_file(101), 0);
        assert_eq!(
            fixture.count_rows("source_file_attachment_links"),
            1,
            "bounded materialization must process only the requested candidate window"
        );
    }

    #[test]
    fn stale_observed_fact_is_skipped_without_creating_a_link() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/stale.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.record_source_file(100, "Album/stale.flac", 11, 101);

        let result = fixture.materialize(10);

        assert_eq!(result.skipped_stale_facts, 1);
        assert_eq!(result.links_created, 0);
        assert_eq!(fixture.count_rows("content_attachments"), 0);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 0);
    }

    #[test]
    fn non_blake3_observed_fact_is_skipped_without_creating_a_link() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/sha.flac", 10, 100);
        fixture.commit_source_fact(100, Some(("sha256", "fixture-sha")), "audio");

        let result = fixture.materialize(10);

        assert_eq!(result.skipped_no_blake3, 1);
        assert_eq!(result.links_created, 0);
        assert_eq!(fixture.count_rows("content_attachments"), 0);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 0);
    }

    #[test]
    fn source_file_without_facts_is_skipped_without_creating_a_link() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/no-facts.flac", 10, 100);

        let result = fixture.materialize(10);

        assert_eq!(result.skipped_no_facts, 1);
        assert_eq!(result.links_created, 0);
        assert_eq!(fixture.count_rows("content_attachments"), 0);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 0);
    }

    #[test]
    fn same_hash_materialization_refreshes_existing_attachment_and_link() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.materialize(10);

        let result = fixture.materialize(10);

        assert_eq!(
            result,
            MaterializeAttachmentsForSourceResult {
                attachments_refreshed: 1,
                links_refreshed: 1,
                ..Default::default()
            }
        );
        assert_eq!(fixture.count_rows("content_attachments"), 1);
        assert_eq!(fixture.link_count_for_source_file(100), 1);
    }

    #[test]
    fn existing_attachment_for_new_source_file_counts_as_refresh_not_creation() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/a.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.materialize(10);

        fixture.record_source_file(101, "Album/b.flac", 11, 101);
        fixture.commit_blake3_fact(101, HASH_A, "audio");
        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 0);
        assert_eq!(result.attachments_refreshed, 1);
        assert_eq!(result.links_created, 1);
        assert_eq!(result.links_refreshed, 1);
        assert_eq!(fixture.count_rows("content_attachments"), 1);
        assert_eq!(fixture.count_rows("source_file_attachment_links"), 2);
    }

    #[test]
    fn schema_rejects_multiple_current_attachment_links_for_one_source_file() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.materialize(10);

        let connection = fixture.read_connection();
        connection
            .execute(
                "INSERT INTO content_attachments (
                     attachment_id,
                     content_hash_algorithm,
                     content_hash_value,
                     first_observed_at,
                     updated_at
                 )
                 VALUES (999, 'blake3', ?1, 1, 1)",
                [HASH_B],
            )
            .expect("insert second attachment");

        connection
            .execute(
                "INSERT INTO source_file_attachment_links (
                     attachment_id,
                     source_file_id,
                     source_id,
                     file_kind,
                     created_at,
                     updated_at
                 )
                 VALUES (999, 100, ?1, 'audio', 1, 1)",
                [fixture.source_id],
            )
            .expect_err("source_file_id must have only one attachment link");
        assert_eq!(fixture.link_count_for_source_file(100), 1);
    }

    #[test]
    fn store_rejects_attachment_link_source_id_drift() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        let error = fixture
            .store
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
                     VALUES (2, 'internal', 'system', 'source:attachment-drift',
                             'Attachment Drift Source', 1, 1)",
                    [],
                )?;
                write.execute(
                    "INSERT INTO content_attachments (
                         attachment_id,
                         content_hash_algorithm,
                         content_hash_value,
                         first_observed_at,
                         updated_at
                     )
                     VALUES (999, 'blake3', ?1, 1, 1)",
                    [HASH_A],
                )?;
                write.execute(
                    "INSERT INTO source_file_attachment_links (
                         attachment_id,
                         source_file_id,
                         source_id,
                         file_kind,
                         created_at,
                         updated_at
                     )
                     VALUES (999, 100, 2, 'audio', 1, 1)",
                    [],
                )?;
                Ok(())
            })
            .expect_err("store must reject mismatched source-file attachment link source_id");

        let detail = format!("{error:?}");
        assert!(
            detail.contains("source_file_attachment_links.source_id must match"),
            "unexpected mismatch error: {detail}"
        );
        assert_eq!(fixture.link_count_for_source_file(100), 0);
    }

    #[test]
    fn hash_change_replaces_source_file_link_and_preserves_old_attachment() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.materialize(10);
        let old_attachment_id = fixture.attachment_id_for_hash(HASH_A);

        fixture.commit_blake3_fact(100, HASH_B, "audio");
        let connection = fixture.read_connection();
        let stale_links = get_source_files_for_attachment(&connection, old_attachment_id)
            .expect("read stale old attachment links");
        assert_eq!(stale_links.len(), 1);
        assert_eq!(
            stale_links[0].link_status,
            StoreSourceFileAttachmentLinkStatus::Stale
        );

        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 1);
        assert_eq!(result.links_replaced, 1);
        assert_eq!(fixture.count_rows("content_attachments"), 2);
        assert_eq!(
            fixture.count_rows("source_file_attachment_links"),
            1,
            "old source-file link must be deleted during replacement"
        );
        assert_eq!(
            fixture
                .read_connection()
                .query_row(
                    "SELECT COUNT(*)
                     FROM content_attachments
                     WHERE attachment_id = ?1",
                    [old_attachment_id],
                    |row| row.get::<_, i64>(0)
                )
                .expect("old attachment still exists"),
            1
        );
        assert_eq!(
            fixture.link_count_for_attachment(old_attachment_id),
            0,
            "old attachment row is preserved but orphaned after replacement"
        );
        let connection = fixture.read_connection();
        let link = get_attachment_for_source_file(&connection, 100)
            .expect("read source-file attachment")
            .expect("replacement link exists");
        assert_eq!(link.content_hash_value, HASH_B);
        assert_eq!(
            link.link_status,
            StoreSourceFileAttachmentLinkStatus::Current
        );
    }

    #[test]
    fn cue_and_adjacent_audio_materialize_as_separate_attachments_without_pairing() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.record_source_file(101, "Album/album.cue", 11, 101);
        fixture.commit_blake3_fact(100, HASH_A, "audio");
        fixture.commit_blake3_fact(101, HASH_B, "cue_sheet");

        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 2);
        assert_eq!(result.links_created, 2);
        let audio_attachment_id = fixture.attachment_id_for_hash(HASH_A);
        let cue_attachment_id = fixture.attachment_id_for_hash(HASH_B);
        assert_ne!(audio_attachment_id, cue_attachment_id);

        let connection = fixture.read_connection();
        let audio_links = get_source_files_for_attachment(&connection, audio_attachment_id)
            .expect("read audio attachment links");
        let cue_links = get_source_files_for_attachment(&connection, cue_attachment_id)
            .expect("read cue attachment links");
        assert_eq!(
            audio_links
                .iter()
                .map(|link| link.source_file_id)
                .collect::<Vec<_>>(),
            vec![100]
        );
        assert_eq!(
            cue_links
                .iter()
                .map(|link| link.source_file_id)
                .collect::<Vec<_>>(),
            vec![101]
        );
        assert_eq!(cue_links[0].file_kind, "cue_sheet");
    }

    fn source_file_domain_id(value: i64) -> SourceFileId {
        SourceFileId::new(value).expect("positive source file id")
    }
}
