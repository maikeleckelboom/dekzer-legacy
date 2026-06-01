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
struct AttachmentMaterializationRow {
    source_file_id: i64,
    source_id: i64,
    file_kind: String,
    content_hash_value: Option<String>,
    disposition: AttachmentMaterializationDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttachmentMaterializationDisposition {
    Materializable,
    StaleFacts,
    NoBlake3,
    NoFacts,
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
    let rows = read_attachment_materialization_rows(write, source_id)?;
    let mut result = MaterializeAttachmentsForSourceResult::default();
    let mut candidates = Vec::new();

    for row in rows {
        match row.disposition {
            AttachmentMaterializationDisposition::Materializable => {
                let content_hash_value = row.content_hash_value.ok_or_else(|| {
                    LibrarySqliteError::MalformedSchemaState(
                        "materializable attachment row is missing content_hash_value".to_string(),
                    )
                })?;
                candidates.push(AttachmentMaterializationCandidate {
                    source_file_id: row.source_file_id,
                    source_id: row.source_id,
                    file_kind: row.file_kind,
                    content_hash_value,
                });
            }
            AttachmentMaterializationDisposition::StaleFacts => {
                result.skipped_stale_facts += 1;
            }
            AttachmentMaterializationDisposition::NoBlake3 => {
                result.skipped_no_blake3 += 1;
            }
            AttachmentMaterializationDisposition::NoFacts => {
                result.skipped_no_facts += 1;
            }
        }
    }

    let materializable_count = candidates.len();
    result.remaining_candidates = materializable_count.saturating_sub(limit);

    let mut attachment_ids_by_hash = BTreeMap::new();
    for candidate in candidates.into_iter().take(limit) {
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

fn read_attachment_materialization_rows(
    write: &AdmittedWrite<'_>,
    source_id: i64,
) -> LibrarySqliteResult<Vec<AttachmentMaterializationRow>> {
    let mut statement = write.prepare(
        "SELECT file.source_file_id,
                file.source_id,
                file.file_kind,
                facts.content_hash_value,
                CASE
                    WHEN facts.source_file_id IS NULL THEN 3
                    WHEN NOT (
                        file.source_id = facts.basis_source_id
                        AND file.relative_path = facts.basis_relative_path
                        AND file.size_bytes IS facts.basis_size_bytes
                        AND file.mtime_ns IS facts.basis_mtime_ns
                        AND file.presence_state = facts.basis_presence_state
                    ) THEN 3
                    WHEN NOT (
                        facts.content_hash_algorithm = ?2
                        AND facts.content_hash_value IS NOT NULL
                    ) THEN 3
                    WHEN link.source_file_id IS NULL THEN 0
                    WHEN attachment.content_hash_value != facts.content_hash_value THEN 1
                    ELSE 2
                END AS materialization_priority,
                CASE
                    WHEN facts.source_file_id IS NULL THEN 'no_facts'
                    WHEN NOT (
                        file.source_id = facts.basis_source_id
                        AND file.relative_path = facts.basis_relative_path
                        AND file.size_bytes IS facts.basis_size_bytes
                        AND file.mtime_ns IS facts.basis_mtime_ns
                        AND file.presence_state = facts.basis_presence_state
                    ) THEN 'stale_facts'
                    WHEN facts.content_hash_algorithm = ?2
                     AND facts.content_hash_value IS NOT NULL THEN 'materializable'
                    ELSE 'no_blake3'
                END AS attachment_materialization_disposition
         FROM source_files file
         LEFT JOIN SourceFacts facts
           ON facts.source_file_id = file.source_file_id
         LEFT JOIN source_file_attachment_links link
           ON link.source_file_id = file.source_file_id
         LEFT JOIN content_attachments attachment
           ON attachment.attachment_id = link.attachment_id
         WHERE file.source_id = ?1
         ORDER BY materialization_priority ASC,
                  lower(file.relative_path) ASC,
                  file.source_file_id ASC",
    )?;

    statement
        .query_map(params![source_id, SOURCE_FILE_BLAKE3_ALGORITHM], |row| {
            let raw_disposition = row.get::<_, String>(5)?;
            let disposition = match raw_disposition.as_str() {
                "materializable" => AttachmentMaterializationDisposition::Materializable,
                "stale_facts" => AttachmentMaterializationDisposition::StaleFacts,
                "no_blake3" => AttachmentMaterializationDisposition::NoBlake3,
                _ => AttachmentMaterializationDisposition::NoFacts,
            };
            Ok(AttachmentMaterializationRow {
                source_file_id: row.get(0)?,
                source_id: row.get(1)?,
                file_kind: row.get(2)?,
                content_hash_value: row.get(3)?,
                disposition,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
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
        StoreSourceFileAttachmentLinkStatus, get_attachment_for_source_file,
        get_source_attachment_summary, get_source_files_for_attachment,
    };
    use crate::{
        CommitAcceptedSourceFactsInput, CommitAcceptedSourceFactsMergePolicy,
        CompleteMachineWorkInput, ContentHashEvidence, FinishWorkRunInput,
        InspectSourcePromotionInput, QueueInspectSourceWorkInput, RecordArtifactInput,
        RecordInlineArtifactInput, RecordSourceFileObservationInput, StartWorkRunInput,
        UpsertSourceInput,
    };
    use library_domain::{
        ArtifactKind, ArtifactRole, SourceFileId, SourcePresenceState, WorkPriorityClass,
        WorkRunOutcome,
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
                    browser_order_ordinal: Some(0),
                    changed_at: 1,
                })
                .expect("upsert source");
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
                .queue_inspect_source_work(QueueInspectSourceWorkInput {
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
                .inspect_source(InspectSourcePromotionInput {
                    source_facts: CommitAcceptedSourceFactsInput {
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
                    source_facts_merge_policy: CommitAcceptedSourceFactsMergePolicy::replacement(),
                    rebuild_projection_domains: vec![],
                    rebuild_priority: WorkPriorityClass::Interactive,
                })
                .expect("commit source facts through inspect_source");
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

        fn count_table_if_exists(&self, table: &str) -> Option<i64> {
            let connection = self.read_connection();
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM sqlite_schema
                     WHERE type IN ('table', 'view')
                       AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("check table exists");
            if exists == 0 {
                return None;
            }
            Some(
                connection
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .expect("count optional table"),
            )
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
        assert!(
            links
                .iter()
                .all(|link| link.link_status == StoreSourceFileAttachmentLinkStatus::Current)
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

    #[test]
    fn materialization_creates_no_browser_segments_tracks_or_prep_rows() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_A, "audio");

        fixture.materialize(10);

        assert_eq!(fixture.count_rows("LibraryAssets"), 0);
        assert_eq!(fixture.count_rows("LibraryAssetAttachments"), 0);
        assert_eq!(fixture.count_rows("LibraryBrowserRows"), 0);
        assert_eq!(fixture.count_rows("SourceSegmentSets"), 0);
        assert_eq!(fixture.count_rows("SourceSegments"), 0);
        assert_eq!(fixture.count_rows("PrepAssignments"), 0);
        assert_eq!(fixture.count_rows("ResolvedLibraryAssetPrepTargets"), 0);
        for absent_or_future_table in [
            "Tracks",
            "TrackRows",
            "LibraryTracks",
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
    }

    #[test]
    fn equivalence_fingerprint_is_not_attachment_identity() {
        let mut fixture = AttachmentIdentityFixture::new();
        fixture.record_source_file(100, "Album/track.flac", 10, 100);
        fixture.commit_blake3_fact(100, HASH_C, "audio");
        fixture
            .read_connection()
            .execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'eq:not-content-identity', 1, 1)",
                [],
            )
            .expect("insert legacy library asset");

        let result = fixture.materialize(10);

        assert_eq!(result.attachments_created, 1);
        assert_eq!(fixture.count_rows("LibraryAssets"), 1);
        assert_eq!(fixture.count_rows("LibraryAssetAttachments"), 0);
        let connection = fixture.read_connection();
        let content_hash_value: String = connection
            .query_row(
                "SELECT content_hash_value
                 FROM content_attachments",
                [],
                |row| row.get(0),
            )
            .expect("read content attachment hash");
        assert_eq!(content_hash_value, HASH_C);
        assert_ne!(content_hash_value, "eq:not-content-identity");
    }

    fn source_file_domain_id(value: i64) -> SourceFileId {
        SourceFileId::new(value).expect("positive source file id")
    }
}
