use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use rusqlite::{OptionalExtension, params};

use crate::authority::sources::{
    RecordSourceFileObservationInput, SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx,
    UpsertSourceDirectoryInput,
};
use crate::authority::work::{
    QueueInspectSourceWorkInput, QueueRebindSourceWorkInput, RebindSourceWorkAuthorityTx,
    WorkItemsAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::browse_media::{
    LibraryTreeRowAdmission, classify_relative_path_file_kind,
    library_tree_row_admission_predicate_sql_for_column,
};
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{SourceAccessIssueKind, SourceFileId, SourcePresenceState, WorkPriorityClass};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiscoveredLocationKind {
    Folder,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryBatch {
    pub root_id: i64,
    pub scan_started_at_ms: i64,
    pub files: Vec<DiscoveredFileInput>,
}

impl DiscoveryBatch {
    pub(crate) fn validate(&self) -> LibrarySqliteResult<()> {
        self.require_non_empty()?;
        self.require_unique_canonical_paths()
    }

    pub(crate) fn require_non_empty(&self) -> LibrarySqliteResult<()> {
        if self.files.is_empty() {
            Err(LibrarySqliteError::EmptyDiscoveryBatch)
        } else {
            Ok(())
        }
    }

    fn require_unique_canonical_paths(&self) -> LibrarySqliteResult<()> {
        let mut seen_paths = HashSet::with_capacity(self.files.len());

        for file in &self.files {
            if !seen_paths.insert(file.canonical_path.as_str()) {
                return Err(LibrarySqliteError::DuplicateDiscoveryCanonicalPath(
                    file.canonical_path.clone(),
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFileInput {
    pub canonical_path: String,
    pub file_size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredLocationInput {
    pub canonical_path: String,
    pub display_name: String,
    pub kind: DiscoveredLocationKind,
    pub file_size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEnumerationOutcome {
    pub relative_path: String,
    pub outcome: DirectoryEnumerationOutcomeKind,
    pub issue_kind: Option<SourceAccessIssueKind>,
    pub diagnostic_detail: Option<String>,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryEnumerationOutcomeKind {
    Enumerated,
    Blocked,
    Missing,
    Failed,
}

impl DiscoveredLocationInput {
    pub(crate) fn folder(
        canonical_path: impl Into<String>,
        display_name: impl Into<String>,
        observed_at_ms: i64,
    ) -> Self {
        Self {
            canonical_path: canonical_path.into(),
            display_name: display_name.into(),
            kind: DiscoveredLocationKind::Folder,
            file_size_bytes: None,
            modified_at_ns: None,
            observed_at_ms,
        }
    }

    pub(crate) fn file(
        canonical_path: impl Into<String>,
        display_name: impl Into<String>,
        file_size_bytes: Option<i64>,
        modified_at_ns: Option<i64>,
        observed_at_ms: i64,
    ) -> Self {
        Self {
            canonical_path: canonical_path.into(),
            display_name: display_name.into(),
            kind: DiscoveredLocationKind::File,
            file_size_bytes,
            modified_at_ns,
            observed_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryCommitResult {
    pub scan_run_id: i64,
    pub files: Vec<DiscoveredFileCommitResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFileCommitResult {
    pub file_id: i64,
    pub track_id: Option<i64>,
    pub canonical_path: String,
    pub needs_probe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveryChunkCommitResult {
    pub files: Vec<DiscoveredFileCommitResult>,
    pub files_seen: usize,
    pub files_new: usize,
    pub changed_track_ids: Vec<i64>,
    pub observed_file_paths: Vec<String>,
    pub observed_directory_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveryFinalizeResult {
    pub files_gone: usize,
}

pub(crate) struct DiscoveryTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingSourceFileRow {
    source_file_id: i64,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
    presence_state: SourcePresenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessedFile {
    source_file_id: i64,
    is_new_file: bool,
    needs_probe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceScanIncompleteCoverage {
    scan_issue_kind: String,
    error_detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresentSourceFileRow {
    source_file_id: i64,
    relative_path: String,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
}

impl<'write, 'conn> DiscoveryTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write mut AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn start_scan_session(
        &mut self,
        root_id: i64,
        started_at_ms: i64,
    ) -> LibrarySqliteResult<i64> {
        self.require_root(root_id)?;
        let rows_changed = self.tx().execute(
            "UPDATE source_scan_state
             SET scan_phase = 'scanning',
                 scan_issue_kind = NULL,
                 error_detail = NULL,
                 last_scan_started_at = ?2,
                 last_scan_finished_at = NULL,
                 updated_at = ?2
             WHERE source_id = ?1",
            params![root_id, started_at_ms],
        )?;
        if rows_changed == 0 {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        }
        self.tx().execute(
            "UPDATE source_directories
             SET dir_scan_state = 'pending',
                 dir_scan_issue_kind = NULL,
                 dir_scan_error_detail = NULL,
                 dir_scan_updated_at = ?2,
                 updated_at = ?2
             WHERE source_id = ?1
               AND presence_state = 'present'",
            params![root_id, started_at_ms],
        )?;

        Ok(started_at_ms.max(1))
    }

    pub(crate) fn commit_discovery_chunk(
        &mut self,
        _scan_run_id: i64,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.commit_chunk(root_id, locations, Some("scanning"))
    }

    pub(crate) fn commit_import_chunk(
        &mut self,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.commit_chunk(root_id, locations, None)
    }

    pub(crate) fn commit_directory_enumeration_outcome(
        &mut self,
        root_id: i64,
        outcome: &DirectoryEnumerationOutcome,
    ) -> LibrarySqliteResult<()> {
        self.require_root(root_id)?;
        match outcome.outcome {
            DirectoryEnumerationOutcomeKind::Enumerated => {
                if outcome.relative_path.is_empty() {
                    return Ok(());
                }
                self.ensure_source_directory_chain(
                    root_id,
                    &outcome.relative_path,
                    None,
                    outcome.observed_at_ms,
                    Some("complete"),
                )?;
            }
            DirectoryEnumerationOutcomeKind::Blocked | DirectoryEnumerationOutcomeKind::Failed => {
                if outcome.relative_path.is_empty() {
                    return Ok(());
                }
                let scan_state = match outcome.outcome {
                    DirectoryEnumerationOutcomeKind::Blocked => "blocked",
                    DirectoryEnumerationOutcomeKind::Failed => "failed",
                    DirectoryEnumerationOutcomeKind::Enumerated
                    | DirectoryEnumerationOutcomeKind::Missing => unreachable!(),
                };
                self.ensure_source_directory_chain_with_issue(
                    root_id,
                    &outcome.relative_path,
                    outcome.observed_at_ms,
                    scan_state,
                    outcome.issue_kind,
                    outcome.diagnostic_detail.clone(),
                )?;
            }
            DirectoryEnumerationOutcomeKind::Missing => {
                if outcome.relative_path.is_empty() {
                    return Ok(());
                }
                self.mark_directory_missing(
                    root_id,
                    &outcome.relative_path,
                    outcome.observed_at_ms,
                )?;
            }
        }
        Ok(())
    }

    fn commit_chunk(
        &mut self,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
        directory_scan_state: Option<&str>,
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.require_root(root_id)?;
        if locations.is_empty() {
            return Ok(DiscoveryChunkCommitResult {
                files: Vec::new(),
                files_seen: 0,
                files_new: 0,
                changed_track_ids: Vec::new(),
                observed_file_paths: Vec::new(),
                observed_directory_paths: Vec::new(),
            });
        }

        let ordered_locations = order_chunk_locations(locations)?;
        let mut committed_files = Vec::new();
        let mut observed_file_paths = Vec::new();
        let mut observed_directory_paths = Vec::new();
        let mut files_seen = 0_usize;
        let mut files_new = 0_usize;

        for location in &ordered_locations {
            match location.kind {
                DiscoveredLocationKind::Folder => {
                    if location.canonical_path.is_empty() {
                        continue;
                    }
                    self.upsert_directory_presence(
                        root_id,
                        &location.canonical_path,
                        location.modified_at_ns,
                        location.observed_at_ms,
                        directory_scan_state,
                    )?;
                    observed_directory_paths.push(location.canonical_path.clone());
                }
                DiscoveredLocationKind::File => {
                    let processed =
                        self.process_discovered_file(root_id, location, directory_scan_state)?;
                    observed_file_paths.push(location.canonical_path.clone());
                    if processed.is_new_file {
                        files_new += 1;
                    }
                    files_seen += 1;
                    committed_files.push(DiscoveredFileCommitResult {
                        file_id: processed.source_file_id,
                        track_id: None,
                        canonical_path: location.canonical_path.clone(),
                        needs_probe: processed.needs_probe,
                    });
                }
            }
        }

        Ok(DiscoveryChunkCommitResult {
            files: committed_files,
            files_seen,
            files_new,
            changed_track_ids: Vec::new(),
            observed_file_paths,
            observed_directory_paths,
        })
    }

    pub(crate) fn finalize_scan(
        &mut self,
        root_id: i64,
        observed_file_paths: &[String],
        observed_directory_paths: &[String],
    ) -> LibrarySqliteResult<DiscoveryFinalizeResult> {
        self.require_root(root_id)?;
        let now_ms = unix_time_ms()?;
        let unproven_prefixes = self.load_unproven_directory_prefixes(root_id)?;
        self.mark_missing_directories(
            root_id,
            observed_directory_paths,
            &unproven_prefixes,
            now_ms,
        )?;
        let missing_file_ids =
            self.mark_missing_files(root_id, observed_file_paths, &unproven_prefixes, now_ms)?;
        self.reconcile_directory_coverage_facts(root_id, now_ms)?;

        let state_rows_changed = self.tx().execute(
            "UPDATE source_state
             SET last_seen_at = CASE
                     WHEN mount_status = 'mounted' THEN ?2
                     ELSE last_seen_at
                 END,
                 updated_at = ?2
             WHERE source_id = ?1",
            params![root_id, now_ms],
        )?;

        let descendant_coverage = self.load_source_scan_incomplete_coverage(root_id)?;
        let scan_rows_changed = match descendant_coverage {
            Some(ref cov) => self.tx().execute(
                "UPDATE source_scan_state
                     SET scan_phase = 'partial',
                         last_scan_finished_at = ?2,
                         last_successful_scan_at = last_successful_scan_at,
                         scan_issue_kind = ?3,
                         error_detail = ?4,
                         updated_at = ?2
                     WHERE source_id = ?1",
                params![root_id, now_ms, &cov.scan_issue_kind, &cov.error_detail,],
            )?,
            None => self.tx().execute(
                "UPDATE source_scan_state
                     SET scan_phase = 'complete',
                         last_scan_finished_at = ?2,
                         last_successful_scan_at = ?2,
                         scan_issue_kind = NULL,
                         error_detail = NULL,
                         updated_at = ?2
                     WHERE source_id = ?1",
                params![root_id, now_ms],
            )?,
        };

        if state_rows_changed == 0 || scan_rows_changed == 0 {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        }

        Ok(DiscoveryFinalizeResult {
            files_gone: missing_file_ids.len(),
        })
    }

    pub(crate) fn root_display_name(&self, root_id: i64) -> LibrarySqliteResult<String> {
        let raw: String = self
            .tx()
            .query_row(
                "SELECT COALESCE(
                        NULLIF(trim(ls.display_name), ''),
                        NULLIF(trim(sl.absolute_path), ''),
                        ls.identity_key
                    )
                 FROM sources ls
                 LEFT JOIN source_locators sl
                   ON sl.source_id = ls.source_id
                 WHERE ls.source_id = ?1",
                [root_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(LibrarySqliteError::MissingRoot(root_id))?;

        Ok(path_leaf_name_from_text(&raw))
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }

    fn require_root(&self, root_id: i64) -> LibrarySqliteResult<()> {
        let root_exists: Option<i64> = self
            .tx()
            .query_row(
                "SELECT source_id
                 FROM sources
                 WHERE source_id = ?1",
                [root_id],
                |row| row.get(0),
            )
            .optional()?;

        match root_exists {
            Some(_) => Ok(()),
            None => Err(LibrarySqliteError::MissingRoot(root_id)),
        }
    }

    fn lookup_source_file(
        &self,
        root_id: i64,
        relative_path: &str,
    ) -> LibrarySqliteResult<Option<ExistingSourceFileRow>> {
        self.tx()
            .query_row(
                "SELECT source_file_id,
                        size_bytes,
                        mtime_ns,
                        presence_state
                 FROM source_files
                 WHERE source_id = ?1
                   AND relative_path = ?2",
                params![root_id, relative_path],
                |row| {
                    Ok(ExistingSourceFileRow {
                        source_file_id: row.get(0)?,
                        size_bytes: row.get(1)?,
                        mtime_ns: row.get(2)?,
                        presence_state: source_presence_state_from_row(&row.get::<_, String>(3)?),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn file_needs_inspection(
        &self,
        existing: Option<&ExistingSourceFileRow>,
        relative_path: &str,
        size_bytes: Option<i64>,
        mtime_ns: Option<i64>,
    ) -> LibrarySqliteResult<bool> {
        if classify_relative_path_file_kind(relative_path) != "audio" {
            return Ok(false);
        }

        let Some(existing) = existing else {
            return Ok(true);
        };

        if existing.presence_state != SourcePresenceState::Present
            || existing.size_bytes != size_bytes
            || existing.mtime_ns != mtime_ns
        {
            return Ok(true);
        }

        let has_facts = self.tx().query_row(
            "SELECT EXISTS(
                     SELECT 1
                     FROM SourceFacts
                     WHERE source_file_id = ?1
                 )",
            [existing.source_file_id],
            |row| row.get::<_, i64>(0),
        )? != 0;
        Ok(!has_facts)
    }

    fn process_discovered_file(
        &self,
        root_id: i64,
        location: &DiscoveredLocationInput,
        directory_scan_state: Option<&str>,
    ) -> LibrarySqliteResult<ProcessedFile> {
        let existing = self.lookup_source_file(root_id, &location.canonical_path)?;
        let parent_source_directory_id = self.ensure_source_directory_chain(
            root_id,
            parent_relative_path(&location.canonical_path),
            None,
            location.observed_at_ms,
            directory_scan_state,
        )?;
        let needs_probe = self.file_needs_inspection(
            existing.as_ref(),
            &location.canonical_path,
            location.file_size_bytes,
            location.modified_at_ns,
        )?;

        let source_file_id = SourceFilesAuthorityTx::new(self.tx).record_source_file_observation(
            &RecordSourceFileObservationInput {
                source_file_id: existing.as_ref().map(|row| row.source_file_id),
                source_id: root_id,
                parent_source_directory_id,
                name: location.display_name.clone(),
                relative_path: location.canonical_path.clone(),
                size_bytes: location.file_size_bytes,
                mtime_ns: location.modified_at_ns,
                presence_state: SourcePresenceState::Present,
                first_discovered_at: existing.as_ref().map(|_| location.observed_at_ms),
                observed_at: Some(location.observed_at_ms),
                presence_changed_at: location.observed_at_ms,
                updated_at: location.observed_at_ms,
            },
        )?;

        if needs_probe {
            let result = WorkItemsAuthorityTx::new(self.tx).queue_inspect_source_work(
                &QueueInspectSourceWorkInput {
                    source_file_id: source_file_domain_id(source_file_id)?,
                    basis_fingerprint: observation_basis_fingerprint(
                        source_file_id,
                        &location.canonical_path,
                        location.file_size_bytes,
                        location.modified_at_ns,
                        location.observed_at_ms,
                    ),
                    priority_class: WorkPriorityClass::Interactive,
                    queued_at: location.observed_at_ms,
                },
            )?;
            let _ = result;
        }

        Ok(ProcessedFile {
            source_file_id,
            is_new_file: existing.is_none(),
            needs_probe,
        })
    }

    fn ensure_source_directory_chain(
        &self,
        root_id: i64,
        relative_path: &str,
        directory_mtime_ns: Option<i64>,
        changed_at: i64,
        directory_scan_state: Option<&str>,
    ) -> LibrarySqliteResult<Option<i64>> {
        if relative_path.is_empty() {
            return Ok(None);
        }

        let mut parent_source_directory_id = None;
        let mut running_relative_path = String::new();
        for segment in relative_path.split('/') {
            if running_relative_path.is_empty() {
                running_relative_path.push_str(segment);
            } else {
                running_relative_path.push('/');
                running_relative_path.push_str(segment);
            }
            parent_source_directory_id = Some(
                SourceDirectoriesAuthorityTx::new(self.tx).upsert_source_directory(
                    &UpsertSourceDirectoryInput {
                        source_directory_id: None,
                        source_id: root_id,
                        parent_source_directory_id,
                        name: segment.to_string(),
                        relative_path: running_relative_path.clone(),
                        presence_state: SourcePresenceState::Present,
                        dir_scan_state: directory_scan_state.map(str::to_string),
                        dir_scan_issue_kind: None,
                        dir_scan_error_detail: None,
                        scanned_at: None,
                        mtime_ns: if running_relative_path == relative_path {
                            directory_mtime_ns
                        } else {
                            None
                        },
                        first_created_at: Some(changed_at),
                        changed_at,
                    },
                )?,
            );
        }

        Ok(parent_source_directory_id)
    }

    fn ensure_source_directory_chain_with_issue(
        &self,
        root_id: i64,
        relative_path: &str,
        changed_at: i64,
        final_directory_scan_state: &str,
        issue_kind: Option<SourceAccessIssueKind>,
        error_detail: Option<String>,
    ) -> LibrarySqliteResult<Option<i64>> {
        if relative_path.is_empty() {
            return Ok(None);
        }

        let effective_issue_kind = if matches!(final_directory_scan_state, "blocked" | "failed") {
            Some(issue_kind.unwrap_or(SourceAccessIssueKind::UnknownIo))
        } else {
            issue_kind
        };

        let mut parent_source_directory_id = None;
        let mut running_relative_path = String::new();
        for segment in relative_path.split('/') {
            if running_relative_path.is_empty() {
                running_relative_path.push_str(segment);
            } else {
                running_relative_path.push('/');
                running_relative_path.push_str(segment);
            }
            let is_final_directory = running_relative_path == relative_path;
            parent_source_directory_id = Some(
                SourceDirectoriesAuthorityTx::new(self.tx).upsert_source_directory(
                    &UpsertSourceDirectoryInput {
                        source_directory_id: None,
                        source_id: root_id,
                        parent_source_directory_id,
                        name: segment.to_string(),
                        relative_path: running_relative_path.clone(),
                        presence_state: SourcePresenceState::Present,
                        dir_scan_state: Some(
                            if is_final_directory {
                                final_directory_scan_state
                            } else {
                                "complete"
                            }
                            .to_string(),
                        ),
                        dir_scan_issue_kind: is_final_directory
                            .then_some(effective_issue_kind)
                            .flatten(),
                        dir_scan_error_detail: if is_final_directory {
                            error_detail.clone()
                        } else {
                            None
                        },
                        scanned_at: None,
                        mtime_ns: None,
                        first_created_at: Some(changed_at),
                        changed_at,
                    },
                )?,
            );
        }

        Ok(parent_source_directory_id)
    }

    fn mark_directory_missing(
        &self,
        root_id: i64,
        relative_path: &str,
        observed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let rows_changed = self.tx().execute(
            "UPDATE source_directories
             SET presence_state = 'missing',
                 dir_scan_state = 'complete',
                 dir_scan_issue_kind = NULL,
                 dir_scan_error_detail = NULL,
                 dir_scan_updated_at = ?3,
                 updated_at = ?3
             WHERE source_id = ?1
               AND relative_path = ?2",
            params![root_id, relative_path, observed_at_ms],
        )?;
        if rows_changed == 0 {
            self.ensure_source_directory_chain_with_issue(
                root_id,
                relative_path,
                observed_at_ms,
                "complete",
                None,
                None,
            )?;
            self.tx().execute(
                "UPDATE source_directories
                 SET presence_state = 'missing',
                     dir_scan_updated_at = ?3,
                     updated_at = ?3
                 WHERE source_id = ?1
                   AND relative_path = ?2",
                params![root_id, relative_path, observed_at_ms],
            )?;
        }
        Ok(())
    }

    fn upsert_directory_presence(
        &self,
        root_id: i64,
        relative_path: &str,
        modified_at_ns: Option<i64>,
        changed_at: i64,
        directory_scan_state: Option<&str>,
    ) -> LibrarySqliteResult<()> {
        let _ = self.ensure_source_directory_chain(
            root_id,
            relative_path,
            modified_at_ns,
            changed_at,
            directory_scan_state,
        )?;
        Ok(())
    }

    fn mark_missing_directories(
        &self,
        root_id: i64,
        observed_directory_paths: &[String],
        unproven_prefixes: &[String],
        now_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let observed_directory_paths = observed_directory_paths
            .iter()
            .filter(|path| !path.is_empty())
            .cloned()
            .collect::<HashSet<_>>();
        let mut stmt = self.tx().prepare(
            "SELECT source_directory_id, relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND dir_scan_state NOT IN ('blocked', 'failed')
             ORDER BY relative_path DESC, source_directory_id DESC",
        )?;
        let rows = stmt
            .query_map([root_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for (source_directory_id, relative_path) in rows {
            if observed_directory_paths.contains(&relative_path) {
                continue;
            }
            if is_inside_unproven_subtree(&relative_path, unproven_prefixes) {
                continue;
            }
            self.tx().execute(
                "UPDATE source_directories
                 SET presence_state = 'missing',
                     dir_scan_state = 'complete',
                     dir_scan_issue_kind = NULL,
                     dir_scan_error_detail = NULL,
                     dir_scan_updated_at = ?2,
                     updated_at = ?2
                 WHERE source_directory_id = ?1",
                params![source_directory_id, now_ms],
            )?;
        }

        Ok(())
    }

    fn mark_missing_files(
        &self,
        root_id: i64,
        observed_file_paths: &[String],
        unproven_prefixes: &[String],
        now_ms: i64,
    ) -> LibrarySqliteResult<Vec<i64>> {
        let observed_file_paths = observed_file_paths.iter().cloned().collect::<HashSet<_>>();
        let mut stmt = self.tx().prepare(
            "SELECT source_file_id,
                    relative_path,
                    size_bytes,
                    mtime_ns
             FROM source_files
             WHERE source_id = ?1
               AND presence_state = 'present'
             ORDER BY relative_path ASC, source_file_id ASC",
        )?;
        let rows = stmt
            .query_map([root_id], |row| {
                Ok(PresentSourceFileRow {
                    source_file_id: row.get(0)?,
                    relative_path: row.get(1)?,
                    size_bytes: row.get(2)?,
                    mtime_ns: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut missing_file_ids = Vec::new();
        for row in rows {
            if observed_file_paths.contains(&row.relative_path) {
                continue;
            }
            if is_inside_unproven_subtree(&row.relative_path, unproven_prefixes) {
                continue;
            }

            self.tx().execute(
                "UPDATE source_files
                 SET presence_state = 'missing',
                     last_presence_change_at = ?2,
                     updated_at = ?2
                 WHERE source_file_id = ?1",
                params![row.source_file_id, now_ms],
            )?;
            self.queue_rebind_work_if_needed(&row, now_ms)?;
            missing_file_ids.push(row.source_file_id);
        }

        Ok(missing_file_ids)
    }

    fn load_unproven_directory_prefixes(&self, root_id: i64) -> LibrarySqliteResult<Vec<String>> {
        let mut stmt = self.tx().prepare(
            "SELECT relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND dir_scan_state IN ('blocked', 'failed')",
        )?;
        let rows = stmt
            .query_map([root_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn load_source_scan_incomplete_coverage(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<Option<SourceScanIncompleteCoverage>> {
        let failed_issue: Option<(Option<String>, Option<String>)> = self
            .tx()
            .query_row(
                "SELECT dir_scan_issue_kind, dir_scan_error_detail
                 FROM source_directories
                 WHERE source_id = ?1
                   AND presence_state = 'present'
                   AND dir_scan_state = 'failed'
                 ORDER BY relative_path ASC
                 LIMIT 1",
                [root_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;

        if let Some((maybe_issue_kind, _error_detail)) = failed_issue {
            let issue_kind = maybe_issue_kind.unwrap_or_else(|| "unknown_io".to_string());
            return Ok(Some(SourceScanIncompleteCoverage {
                scan_issue_kind: issue_kind,
                error_detail: "descendant directory coverage is incomplete".to_string(),
            }));
        }

        let blocked_issue: Option<(Option<String>, Option<String>)> = self
            .tx()
            .query_row(
                "SELECT dir_scan_issue_kind, dir_scan_error_detail
                 FROM source_directories
                 WHERE source_id = ?1
                   AND presence_state = 'present'
                   AND dir_scan_state = 'blocked'
                 ORDER BY relative_path ASC
                 LIMIT 1",
                [root_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;

        if let Some((maybe_issue_kind, _error_detail)) = blocked_issue {
            let issue_kind = maybe_issue_kind.unwrap_or_else(|| "unknown_io".to_string());
            return Ok(Some(SourceScanIncompleteCoverage {
                scan_issue_kind: issue_kind,
                error_detail: "descendant directory coverage is incomplete".to_string(),
            }));
        }

        let has_pending_or_scanning: Option<i64> = self
            .tx()
            .query_row(
                "SELECT 1
                 FROM source_directories
                 WHERE source_id = ?1
                   AND presence_state = 'present'
                   AND dir_scan_state IN ('pending', 'scanning')
                 LIMIT 1",
                [root_id],
                |row| row.get(0),
            )
            .optional()?;

        if has_pending_or_scanning.is_some() {
            return Ok(Some(SourceScanIncompleteCoverage {
                scan_issue_kind: "unknown_io".to_string(),
                error_detail: "incomplete directory coverage remains after scan finalization"
                    .to_string(),
            }));
        }

        Ok(None)
    }

    fn queue_rebind_work_if_needed(
        &self,
        row: &PresentSourceFileRow,
        queued_at: i64,
    ) -> LibrarySqliteResult<()> {
        let should_queue = self.tx().query_row(
            "SELECT EXISTS(
                     SELECT 1
                     FROM SourceFacts
                     WHERE source_file_id = ?1
                 )
                 OR EXISTS(
                     SELECT 1
                     FROM SourceSegmentSets
                     WHERE source_file_id = ?1
                 )",
            [row.source_file_id],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !should_queue {
            return Ok(());
        }

        let result = RebindSourceWorkAuthorityTx::new(self.tx).queue_rebind_source_work(
            &QueueRebindSourceWorkInput {
                source_file_id: source_file_domain_id(row.source_file_id)?,
                basis_fingerprint: observation_basis_fingerprint(
                    row.source_file_id,
                    &row.relative_path,
                    row.size_bytes,
                    row.mtime_ns,
                    queued_at,
                ),
                priority_class: WorkPriorityClass::Interactive,
                queued_at,
            },
        )?;
        let _ = result;
        Ok(())
    }

    fn reconcile_directory_coverage_facts(
        &self,
        root_id: i64,
        completed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "UPDATE source_directories
             SET has_child_directories = 0,
                 has_primary_media_descendant = 0,
                 has_image_media_descendant = 0
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND dir_scan_state IN ('pending', 'scanning', 'complete')",
            [root_id],
        )?;

        self.tx().execute(
            "UPDATE source_directories
             SET has_child_directories = 1
             WHERE source_id = ?1
               AND source_directory_id IN (
                   SELECT DISTINCT child.parent_source_directory_id
                   FROM source_directories child
                   WHERE child.source_id = ?1
                     AND child.presence_state = 'present'
                     AND child.parent_source_directory_id IS NOT NULL
               )",
            [root_id],
        )?;

        let primary_media_predicate = library_tree_row_admission_predicate_sql_for_column(
            LibraryTreeRowAdmission::Performance,
            "f.media_class",
        );
        self.tx().execute(
            &format!(
                "WITH RECURSIVE media_up(source_directory_id) AS (
                 SELECT DISTINCT f.parent_source_directory_id
                 FROM source_files f
                 WHERE f.source_id = ?1
                   AND f.presence_state = 'present'
                   AND f.parent_source_directory_id IS NOT NULL
                   AND {primary_media_predicate}
                 UNION
                 SELECT d.parent_source_directory_id
                 FROM source_directories d
                 JOIN media_up b ON d.source_directory_id = b.source_directory_id
                 WHERE d.source_id = ?1
                   AND d.presence_state = 'present'
                   AND d.parent_source_directory_id IS NOT NULL
             )
             UPDATE source_directories
             SET has_primary_media_descendant = 1
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND source_directory_id IN media_up"
            ),
            [root_id],
        )?;

        self.tx().execute(
            "WITH RECURSIVE image_up(source_directory_id) AS (
                 SELECT DISTINCT f.parent_source_directory_id
                 FROM source_files f
                 WHERE f.source_id = ?1
                   AND f.presence_state = 'present'
                   AND f.parent_source_directory_id IS NOT NULL
                   AND f.media_class = 'image'
                 UNION
                 SELECT d.parent_source_directory_id
                 FROM source_directories d
                 JOIN image_up b ON d.source_directory_id = b.source_directory_id
                 WHERE d.source_id = ?1
                   AND d.presence_state = 'present'
                   AND d.parent_source_directory_id IS NOT NULL
             )
             UPDATE source_directories
             SET has_image_media_descendant = 1
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND source_directory_id IN image_up",
            [root_id],
        )?;

        self.tx().execute(
            "UPDATE source_directories
             SET dir_scan_state = 'complete',
                 dir_scan_issue_kind = NULL,
                 dir_scan_error_detail = NULL,
                 dir_scan_updated_at = ?2,
                 scanned_at = ?2,
                 updated_at = ?2
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND dir_scan_state IN ('pending', 'scanning', 'complete')
               AND NOT EXISTS (
                   SELECT 1
                   FROM source_directories blocked_ancestor
                   WHERE blocked_ancestor.source_id = source_directories.source_id
                     AND blocked_ancestor.presence_state = 'present'
                     AND blocked_ancestor.dir_scan_state IN ('blocked', 'failed')
                     AND source_directories.relative_path COLLATE BINARY >= blocked_ancestor.relative_path || '/'
                     AND source_directories.relative_path COLLATE BINARY < blocked_ancestor.relative_path || char(48)
               )",
            params![root_id, completed_at_ms],
        )?;

        Ok(())
    }
}

fn source_file_domain_id(source_file_id: i64) -> LibrarySqliteResult<SourceFileId> {
    SourceFileId::new(source_file_id).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "source_files.source_file_id is not a domain id: {source_file_id}"
        ))
    })
}

fn source_presence_state_from_row(value: &str) -> SourcePresenceState {
    match value {
        "present" => SourcePresenceState::Present,
        "missing" => SourcePresenceState::Missing,
        _ => SourcePresenceState::Removed,
    }
}

pub(crate) fn build_discovered_locations_from_files(
    root_display_name: &str,
    files: &[DiscoveredFileInput],
) -> Vec<DiscoveredLocationInput> {
    let mut locations = BTreeMap::new();
    let root_observed_at_ms = files
        .iter()
        .map(|file| file.observed_at_ms)
        .min()
        .unwrap_or(0);
    locations.insert(
        String::new(),
        DiscoveredLocationInput::folder("", root_display_name, root_observed_at_ms),
    );

    for file in files {
        let mut parent_path = String::new();
        let segments = file
            .canonical_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        for segment in segments
            .iter()
            .take(segments.len().saturating_sub(1))
            .copied()
        {
            if !parent_path.is_empty() {
                parent_path.push('/');
            }
            parent_path.push_str(segment);
            locations.entry(parent_path.clone()).or_insert_with(|| {
                DiscoveredLocationInput::folder(
                    parent_path.clone(),
                    segment.to_string(),
                    file.observed_at_ms,
                )
            });
        }

        let display_name = segments
            .last()
            .copied()
            .unwrap_or(file.canonical_path.as_str())
            .to_string();
        locations.insert(
            file.canonical_path.clone(),
            DiscoveredLocationInput::file(
                file.canonical_path.clone(),
                display_name,
                file.file_size_bytes,
                file.modified_at_ns,
                file.observed_at_ms,
            ),
        );
    }

    locations.into_values().collect()
}

fn order_chunk_locations(
    locations: &[DiscoveredLocationInput],
) -> LibrarySqliteResult<Vec<DiscoveredLocationInput>> {
    let mut ordered = locations.to_vec();
    ordered.sort_by(|left, right| {
        location_depth(&left.canonical_path)
            .cmp(&location_depth(&right.canonical_path))
            .then_with(|| kind_order(&left.kind).cmp(&kind_order(&right.kind)))
            .then_with(|| left.canonical_path.cmp(&right.canonical_path))
    });

    let mut seen_paths = HashSet::with_capacity(ordered.len());
    for location in &ordered {
        if !seen_paths.insert(location.canonical_path.as_str()) {
            return Err(LibrarySqliteError::DuplicateDiscoveryCanonicalPath(
                location.canonical_path.clone(),
            ));
        }
    }

    Ok(ordered)
}

fn kind_order(kind: &DiscoveredLocationKind) -> u8 {
    match kind {
        DiscoveredLocationKind::Folder => 0,
        DiscoveredLocationKind::File => 1,
    }
}

fn location_depth(canonical_path: &str) -> usize {
    if canonical_path.is_empty() {
        0
    } else {
        canonical_path.bytes().filter(|byte| *byte == b'/').count() + 1
    }
}

fn parent_relative_path(relative_path: &str) -> &str {
    relative_path
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

fn path_leaf_name_from_text(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|segment| segment.to_str())
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn observation_basis_fingerprint(
    source_file_id: i64,
    relative_path: &str,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
    updated_at: i64,
) -> String {
    let size_fragment = size_bytes
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let mtime_fragment = mtime_ns
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    format!(
        "source-observation:file:{source_file_id}:path:{relative_path}:size:{size_fragment}:mtime:{mtime_fragment}:updated:{updated_at}"
    )
}

fn is_inside_unproven_subtree(relative_path: &str, unproven_prefixes: &[String]) -> bool {
    unproven_prefixes.iter().any(|prefix| {
        if relative_path == prefix.as_str() {
            return true;
        }
        relative_path.len() > prefix.len()
            && relative_path.as_bytes()[prefix.len()] == b'/'
            && relative_path.starts_with(prefix.as_str())
    })
}

#[cfg(test)]
mod tests {
    use crate::authority::ingest::{
        DirectoryEnumerationOutcome, DirectoryEnumerationOutcomeKind, DiscoveryTx,
    };
    use crate::authority::sources::{
        RecordSourceFileObservationInput, SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx,
        SourceLocatorsAuthorityTx, SourceStateAuthorityTx, SourcesAuthorityTx,
        UpsertSourceDirectoryInput, UpsertSourceLocatorInput, UpsertSourceScanStateInput,
        UpsertSourceStateInput,
    };
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{
        SourceAccessIssueKind, SourceAccessState, SourcePresenceState, SourceScanPhase,
    };
    use rusqlite::OptionalExtension;

    fn setup_source_with_directory(connection: &mut rusqlite::Connection) -> (i64, i64) {
        let source_id = admit_write(connection, |write| {
            let source_id = SourcesAuthorityTx::new(write)
                .upsert_source(&crate::authority::sources::UpsertSourceInput {
                    source_id: Some(1),
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "media-class-test".to_string(),
                    display_name: "Test Source".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    browser_order_ordinal: Some(0),
                    changed_at: 10,
                })
                .expect("upsert source");
            SourceLocatorsAuthorityTx::new(write)
                .upsert_source_locator(&UpsertSourceLocatorInput {
                    source_id,
                    locator: crate::authority::sources::SourceLocatorInput::AbsolutePath {
                        absolute_path: "/test/music".to_string(),
                    },
                })
                .expect("upsert source locator");
            SourceStateAuthorityTx::new(write)
                .upsert_source_state(&UpsertSourceStateInput {
                    source_id,
                    mount_status: "mounted".to_string(),
                    mount_epoch: 0,
                    access_state: SourceAccessState::Accessible,
                    access_issue_kind: None,
                    access_error_detail: None,
                    access_checked_at: Some(10),
                    mount_root: Some("/test/music".to_string()),
                    effective_path: Some("/test/music".to_string()),
                    observed_volume_label: None,
                    filesystem_type: None,
                    last_seen_at: Some(10),
                    updated_at: 10,
                })
                .expect("upsert source state");
            SourceStateAuthorityTx::new(write)
                .upsert_source_scan_state(&UpsertSourceScanStateInput {
                    source_id,
                    scan_phase: SourceScanPhase::Idle,
                    last_scan_started_at: Some(10),
                    last_scan_finished_at: Some(11),
                    last_successful_scan_at: Some(11),
                    scan_issue_kind: None,
                    error_detail: None,
                    updated_at: 11,
                })
                .expect("upsert scan state");
            Ok(source_id)
        })
        .expect("write source");

        let parent_dir_id = admit_write(connection, |write| {
            let dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(10),
                    source_id,
                    parent_source_directory_id: None,
                    name: "albums".to_string(),
                    relative_path: "albums".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert parent directory");
            Ok(dir_id)
        })
        .expect("write directory");

        (source_id, parent_dir_id)
    }

    fn insert_file(
        connection: &mut rusqlite::Connection,
        source_id: i64,
        parent_dir_id: Option<i64>,
        name: &str,
        relative_path: &str,
    ) -> i64 {
        admit_write(connection, |write| {
            let file_id = SourceFilesAuthorityTx::new(write)
                .record_source_file_observation(&RecordSourceFileObservationInput {
                    source_file_id: None,
                    source_id,
                    parent_source_directory_id: parent_dir_id,
                    name: name.to_string(),
                    relative_path: relative_path.to_string(),
                    size_bytes: Some(1000),
                    mtime_ns: Some(500),
                    presence_state: SourcePresenceState::Present,
                    first_discovered_at: Some(15),
                    observed_at: Some(15),
                    presence_changed_at: 15,
                    updated_at: 15,
                })
                .expect("record source file observation");
            Ok(file_id)
        })
        .expect("write file")
    }

    fn read_media_class(connection: &rusqlite::Connection, relative_path: &str) -> String {
        connection
            .query_row(
                "SELECT media_class FROM source_files WHERE relative_path = ?1",
                [relative_path],
                |row| row.get::<_, String>(0),
            )
            .expect("read media_class")
    }

    fn read_file_kind(connection: &rusqlite::Connection, relative_path: &str) -> String {
        connection
            .query_row(
                "SELECT file_kind FROM source_files WHERE relative_path = ?1",
                [relative_path],
                |row| row.get::<_, String>(0),
            )
            .expect("read file_kind")
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DirectoryFacts {
        has_child_directories: bool,
        has_primary_media_descendant: bool,
        has_image_media_descendant: bool,
        dir_scan_state: String,
        scanned_at: Option<i64>,
    }

    fn read_directory_facts(
        connection: &rusqlite::Connection,
        relative_path: &str,
    ) -> DirectoryFacts {
        connection
            .query_row(
                "SELECT has_child_directories,
                        has_primary_media_descendant,
                        has_image_media_descendant,
                        dir_scan_state,
                        scanned_at
                 FROM source_directories
                 WHERE relative_path = ?1",
                [relative_path],
                |row| {
                    Ok(DirectoryFacts {
                        has_child_directories: row.get(0)?,
                        has_primary_media_descendant: row.get(1)?,
                        has_image_media_descendant: row.get(2)?,
                        dir_scan_state: row.get(3)?,
                        scanned_at: row.get(4)?,
                    })
                },
            )
            .expect("read directory facts")
    }

    fn run_coverage_finalization(
        connection: &mut rusqlite::Connection,
        root_id: i64,
        observed_directory_paths: &[String],
        observed_file_paths: &[String],
    ) {
        admit_write(connection, |write| {
            DiscoveryTx::new(write)
                .finalize_scan(root_id, observed_file_paths, observed_directory_paths)
                .expect("finalize scan to reconcile directory coverage facts");
            Ok(())
        })
        .expect("write directory coverage facts");
    }

    #[test]
    fn start_scan_session_clears_finished_timestamp_on_rescan() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            SourceStateAuthorityTx::new(write)
                .upsert_source_scan_state(&UpsertSourceScanStateInput {
                    source_id,
                    scan_phase: SourceScanPhase::Complete,
                    last_scan_started_at: Some(100),
                    last_scan_finished_at: Some(150),
                    last_successful_scan_at: Some(150),
                    scan_issue_kind: Some(SourceAccessIssueKind::UnknownIo),
                    error_detail: Some("stale issue from prior scan".to_string()),
                    updated_at: 150,
                })
                .expect("set completed scan state");
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(parent_dir_id),
                    source_id,
                    parent_source_directory_id: None,
                    name: "albums".to_string(),
                    relative_path: "albums".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: Some("blocked".to_string()),
                    dir_scan_issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                    dir_scan_error_detail: Some("stale directory issue".to_string()),
                    scanned_at: Some(150),
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 150,
                })
                .expect("set stale directory scan issue");
            Ok(())
        })
        .expect("prepare completed scan state");

        let scan_run_id = admit_write(&mut connection, |write| {
            DiscoveryTx::new(write).start_scan_session(source_id, 200)
        })
        .expect("start scan session");
        assert_eq!(scan_run_id, 200);

        let (
            scan_phase,
            last_scan_started_at,
            last_scan_finished_at,
            scan_issue_kind,
            error_detail,
        ): (
            String,
            Option<i64>,
            Option<i64>,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT scan_phase,
                        last_scan_started_at,
                        last_scan_finished_at,
                        scan_issue_kind,
                        error_detail
                 FROM source_scan_state
                 WHERE source_id = ?1",
                [source_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read scan state");
        let (dir_scan_state, dir_scan_issue_kind, dir_scan_error_detail): (
            String,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT dir_scan_state, dir_scan_issue_kind, dir_scan_error_detail
                 FROM source_directories
                 WHERE source_directory_id = ?1",
                [parent_dir_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read directory scan state");

        assert_eq!(scan_phase, "scanning");
        assert_eq!(last_scan_started_at, Some(200));
        assert_eq!(last_scan_finished_at, None);
        assert_eq!(scan_issue_kind, None);
        assert_eq!(error_detail, None);
        assert_eq!(dir_scan_state, "pending");
        assert_eq!(dir_scan_issue_kind, None);
        assert_eq!(dir_scan_error_detail, None);
    }

    #[test]
    fn blocked_directory_outcome_survives_scan_finalization() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit blocked directory outcome");
            Ok(())
        })
        .expect("write blocked directory outcome");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let facts = read_directory_facts(&connection, "albums/locked");
        assert_eq!(facts.dir_scan_state, "blocked");
    }

    #[test]
    fn wma_file_stores_audio_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "track.wma",
            "albums/track.wma",
        );
        assert_eq!(read_media_class(&connection, "albums/track.wma"), "audio");
    }

    #[test]
    fn alac_file_stores_audio_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "track.alac",
            "albums/track.alac",
        );
        assert_eq!(read_media_class(&connection, "albums/track.alac"), "audio");
    }

    #[test]
    fn mp4_file_stores_video_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "clip.mp4",
            "albums/clip.mp4",
        );
        assert_eq!(read_media_class(&connection, "albums/clip.mp4"), "video");
    }

    #[test]
    fn png_file_stores_image_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "cover.png",
            "albums/cover.png",
        );
        assert_eq!(read_media_class(&connection, "albums/cover.png"), "image");
    }

    #[test]
    fn unknown_extension_stores_none_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "data.xyz",
            "albums/data.xyz",
        );
        assert_eq!(read_media_class(&connection, "albums/data.xyz"), "none");
    }

    #[test]
    fn cue_file_stores_fine_file_kind_and_unsupported_media_class() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, parent_dir_id) = setup_source_with_directory(&mut connection);
        insert_file(
            &mut connection,
            source_id,
            Some(parent_dir_id),
            "album.cue",
            "albums/album.cue",
        );
        assert_eq!(
            read_media_class(&connection, "albums/album.cue"),
            "unsupported"
        );
        assert_eq!(read_file_kind(&connection, "albums/album.cue"), "cue_sheet");
    }

    #[test]
    fn wma_and_alac_media_policy_marks_directory_as_having_primary_media_descendants() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _root_dir_id) = setup_source_with_directory(&mut connection);

        let nested_dir_id = admit_write(&mut connection, |write| {
            let dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(20),
                    source_id,
                    parent_source_directory_id: None,
                    name: "nested".to_string(),
                    relative_path: "nested".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert nested directory");
            Ok(dir_id)
        })
        .expect("write nested directory");

        insert_file(
            &mut connection,
            source_id,
            Some(nested_dir_id),
            "track.wma",
            "nested/track.wma",
        );
        insert_file(
            &mut connection,
            source_id,
            Some(nested_dir_id),
            "track.alac",
            "nested/track.alac",
        );

        assert_eq!(read_media_class(&connection, "nested/track.wma"), "audio");
        assert_eq!(read_media_class(&connection, "nested/track.alac"), "audio");
        assert!(
            read_directory_facts(&connection, "nested").has_primary_media_descendant,
            ".wma and .alac media_class values must feed primary media descendant facts"
        );
        assert!(!read_directory_facts(&connection, "nested").has_image_media_descendant);

        run_coverage_finalization(
            &mut connection,
            source_id,
            &["nested".to_string()],
            &[
                "nested/track.wma".to_string(),
                "nested/track.alac".to_string(),
            ],
        );

        let facts = read_directory_facts(&connection, "nested");
        assert!(facts.has_primary_media_descendant);
        assert!(!facts.has_image_media_descendant);
        assert_eq!(facts.dir_scan_state, "complete");
        assert!(facts.scanned_at.is_some());
    }

    #[test]
    fn sibling_folder_with_image_has_image_media_descendant_only() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _root_dir_id) = setup_source_with_directory(&mut connection);

        let docs_dir_id = admit_write(&mut connection, |write| {
            let dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(30),
                    source_id,
                    parent_source_directory_id: None,
                    name: "docs".to_string(),
                    relative_path: "docs".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert docs directory");
            Ok(dir_id)
        })
        .expect("write docs directory");

        insert_file(
            &mut connection,
            source_id,
            Some(docs_dir_id),
            "readme.txt",
            "docs/readme.txt",
        );
        insert_file(
            &mut connection,
            source_id,
            Some(docs_dir_id),
            "cover.png",
            "docs/cover.png",
        );

        let before_finalization = read_directory_facts(&connection, "docs");
        assert!(!before_finalization.has_primary_media_descendant);
        assert!(before_finalization.has_image_media_descendant);
        assert_eq!(before_finalization.dir_scan_state, "pending");
        assert_eq!(before_finalization.scanned_at, None);

        run_coverage_finalization(
            &mut connection,
            source_id,
            &["docs".to_string()],
            &["docs/readme.txt".to_string(), "docs/cover.png".to_string()],
        );

        let after_finalization = read_directory_facts(&connection, "docs");
        assert!(!after_finalization.has_primary_media_descendant);
        assert!(after_finalization.has_image_media_descendant);
        assert_eq!(after_finalization.dir_scan_state, "complete");
        assert!(after_finalization.scanned_at.is_some());
    }

    #[test]
    fn folder_with_only_unsupported_files_has_no_primary_or_image_media_descendants() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _root_dir_id) = setup_source_with_directory(&mut connection);

        let misc_dir_id = admit_write(&mut connection, |write| {
            let dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(31),
                    source_id,
                    parent_source_directory_id: None,
                    name: "misc".to_string(),
                    relative_path: "misc".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert misc directory");
            Ok(dir_id)
        })
        .expect("write misc directory");

        insert_file(
            &mut connection,
            source_id,
            Some(misc_dir_id),
            "readme.txt",
            "misc/readme.txt",
        );
        insert_file(
            &mut connection,
            source_id,
            Some(misc_dir_id),
            "archive.zip",
            "misc/archive.zip",
        );

        let before_finalization = read_directory_facts(&connection, "misc");
        assert!(!before_finalization.has_primary_media_descendant);
        assert!(!before_finalization.has_image_media_descendant);
        assert_eq!(before_finalization.dir_scan_state, "pending");
        assert_eq!(before_finalization.scanned_at, None);

        run_coverage_finalization(
            &mut connection,
            source_id,
            &["misc".to_string()],
            &[
                "misc/readme.txt".to_string(),
                "misc/archive.zip".to_string(),
            ],
        );

        let after_finalization = read_directory_facts(&connection, "misc");
        assert!(!after_finalization.has_primary_media_descendant);
        assert!(!after_finalization.has_image_media_descendant);
        assert_eq!(after_finalization.dir_scan_state, "complete");
        assert!(after_finalization.scanned_at.is_some());
    }

    #[test]
    fn nested_audio_files_propagate_primary_media_descendant_facts_to_ancestors() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, root_dir_id) = setup_source_with_directory(&mut connection);

        let deep_dir_id = admit_write(&mut connection, |write| {
            let mid_dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(40),
                    source_id,
                    parent_source_directory_id: Some(root_dir_id),
                    name: "mid".to_string(),
                    relative_path: "albums/mid".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert mid directory");
            let deep_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(41),
                    source_id,
                    parent_source_directory_id: Some(mid_dir_id),
                    name: "deep".to_string(),
                    relative_path: "albums/mid/deep".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(12),
                    changed_at: 12,
                })
                .expect("upsert deep directory");
            Ok(deep_id)
        })
        .expect("write directories");

        insert_file(
            &mut connection,
            source_id,
            Some(deep_dir_id),
            "song.flac",
            "albums/mid/deep/song.flac",
        );

        assert!(read_directory_facts(&connection, "albums").has_primary_media_descendant);
        assert!(read_directory_facts(&connection, "albums/mid").has_primary_media_descendant);
        assert!(read_directory_facts(&connection, "albums/mid/deep").has_primary_media_descendant);

        run_coverage_finalization(
            &mut connection,
            source_id,
            &[
                "albums".to_string(),
                "albums/mid".to_string(),
                "albums/mid/deep".to_string(),
            ],
            &["albums/mid/deep/song.flac".to_string()],
        );

        assert_eq!(
            read_directory_facts(&connection, "albums/mid/deep").dir_scan_state,
            "complete"
        );
        assert_eq!(
            read_directory_facts(&connection, "albums/mid").dir_scan_state,
            "complete"
        );
        assert_eq!(
            read_directory_facts(&connection, "albums").dir_scan_state,
            "complete"
        );
    }

    #[test]
    fn child_directory_observation_marks_immediate_parent_child_directory_fact() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, root_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(50),
                    source_id,
                    parent_source_directory_id: Some(root_dir_id),
                    name: "1998".to_string(),
                    relative_path: "albums/1998".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert child directory");
            Ok(())
        })
        .expect("write child directory");

        assert!(
            read_directory_facts(&connection, "albums").has_child_directories,
            "observing albums/1998 must mark albums as having immediate child directories"
        );
    }

    #[test]
    fn blocked_subtree_does_not_mark_descendant_files_missing() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            let locked_dir_id = SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(60),
                    source_id,
                    parent_source_directory_id: None,
                    name: "Locked".to_string(),
                    relative_path: "Music/Locked".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert locked directory");
            SourceFilesAuthorityTx::new(write)
                .record_source_file_observation(&RecordSourceFileObservationInput {
                    source_file_id: None,
                    source_id,
                    parent_source_directory_id: Some(locked_dir_id),
                    name: "track.flac".to_string(),
                    relative_path: "Music/Locked/track.flac".to_string(),
                    size_bytes: Some(1000),
                    mtime_ns: Some(500),
                    presence_state: SourcePresenceState::Present,
                    first_discovered_at: Some(20),
                    observed_at: Some(20),
                    presence_changed_at: 20,
                    updated_at: 20,
                })
                .expect("record track file");
            Ok(())
        })
        .expect("write locked directory and file");

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "Music/Locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit blocked outcome");
            Ok(())
        })
        .expect("write blocked outcome");

        run_coverage_finalization(&mut connection, source_id, &["Music".to_string()], &[]);

        let locked_facts = read_directory_facts(&connection, "Music/Locked");
        assert_eq!(locked_facts.dir_scan_state, "blocked");

        let file_presence: String = connection
            .query_row(
                "SELECT presence_state FROM source_files WHERE relative_path = 'Music/Locked/track.flac'",
                [],
                |row| row.get(0),
            )
            .expect("read file presence");
        assert_eq!(
            file_presence, "present",
            "file under blocked directory must not be marked missing"
        );
    }

    #[test]
    fn blocked_subtree_does_not_mark_descendant_directories_missing() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(70),
                    source_id,
                    parent_source_directory_id: None,
                    name: "Locked".to_string(),
                    relative_path: "Music/Locked".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert locked directory");
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(71),
                    source_id,
                    parent_source_directory_id: None,
                    name: "Subfolder".to_string(),
                    relative_path: "Music/Locked/Subfolder".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert subfolder directory");
            Ok(())
        })
        .expect("write directories");

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "Music/Locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit blocked outcome");
            Ok(())
        })
        .expect("write blocked outcome");

        run_coverage_finalization(&mut connection, source_id, &["Music".to_string()], &[]);

        let locked_facts = read_directory_facts(&connection, "Music/Locked");
        assert_eq!(locked_facts.dir_scan_state, "blocked");

        let subfolder_presence: String = connection
            .query_row(
                "SELECT presence_state FROM source_directories WHERE relative_path = 'Music/Locked/Subfolder'",
                [],
                |row| row.get(0),
            )
            .expect("read subfolder presence");
        assert_eq!(
            subfolder_presence, "present",
            "directory under blocked ancestor must not be marked missing"
        );

        let subfolder_facts = read_directory_facts(&connection, "Music/Locked/Subfolder");
        assert_ne!(
            subfolder_facts.dir_scan_state, "complete",
            "unobserved directory under blocked subtree must not be finalized as complete"
        );
    }

    #[test]
    fn explicit_missing_outcome_still_marks_missing() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(80),
                    source_id,
                    parent_source_directory_id: None,
                    name: "Album".to_string(),
                    relative_path: "Music/Album".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert album directory");
            Ok(())
        })
        .expect("write album directory");

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "Music/Album".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Missing,
                        issue_kind: Some(SourceAccessIssueKind::Missing),
                        diagnostic_detail: Some("directory not found".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit missing outcome");
            Ok(())
        })
        .expect("write missing outcome");

        run_coverage_finalization(&mut connection, source_id, &["Music".to_string()], &[]);

        let album_presence: String = connection
            .query_row(
                "SELECT presence_state FROM source_directories WHERE relative_path = 'Music/Album'",
                [],
                |row| row.get(0),
            )
            .expect("read album presence");
        assert_eq!(
            album_presence, "missing",
            "explicit missing outcome must mark directory as missing"
        );
    }

    #[test]
    fn previously_blocked_source_candidate_path_is_available() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");

        let source_id: i64 = admit_write(&mut connection, |write| {
            let source_id = SourcesAuthorityTx::new(write)
                .upsert_source(&crate::authority::sources::UpsertSourceInput {
                    source_id: Some(100),
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "reprobe-test".to_string(),
                    display_name: "Reprobe Test".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    browser_order_ordinal: Some(0),
                    changed_at: 10,
                })
                .expect("upsert source");
            SourceLocatorsAuthorityTx::new(write)
                .upsert_source_locator(&UpsertSourceLocatorInput {
                    source_id,
                    locator: crate::authority::sources::SourceLocatorInput::AbsolutePath {
                        absolute_path: "/test/reprobe".to_string(),
                    },
                })
                .expect("upsert source locator");
            SourceStateAuthorityTx::new(write)
                .upsert_source_state(&UpsertSourceStateInput {
                    source_id,
                    mount_status: "mounted".to_string(),
                    mount_epoch: 0,
                    access_state: SourceAccessState::Blocked,
                    access_issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                    access_error_detail: Some("permission denied".to_string()),
                    access_checked_at: Some(10),
                    mount_root: None,
                    effective_path: Some("/test/reprobe".to_string()),
                    observed_volume_label: None,
                    filesystem_type: None,
                    last_seen_at: None,
                    updated_at: 10,
                })
                .expect("upsert blocked source state");
            SourceStateAuthorityTx::new(write)
                .upsert_source_scan_state(&UpsertSourceScanStateInput {
                    source_id,
                    scan_phase: SourceScanPhase::Blocked,
                    last_scan_started_at: None,
                    last_scan_finished_at: None,
                    last_successful_scan_at: None,
                    scan_issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                    error_detail: None,
                    updated_at: 10,
                })
                .expect("upsert scan state");
            Ok(source_id)
        })
        .expect("write blocked source");

        let candidate_path: Option<String> = connection
            .query_row(
                "SELECT COALESCE(ss.effective_path, sl.absolute_path) AS root_path
                 FROM sources s
                 JOIN source_locators sl
                   ON sl.source_id = s.source_id
                 JOIN source_state ss
                   ON ss.source_id = s.source_id
                 WHERE s.source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .optional()
            .expect("query candidate path")
            .flatten();

        assert_eq!(
            candidate_path,
            Some("/test/reprobe".to_string()),
            "blocked source must still yield a candidate path for fresh re-probe"
        );
    }

    #[test]
    fn finalized_scan_with_blocked_descendant_becomes_partial_not_complete() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit blocked directory outcome");
            Ok(())
        })
        .expect("write blocked directory outcome");

        let previous_successful_at: Option<i64> = connection
            .query_row(
                "SELECT last_successful_scan_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_successful_scan_at");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(scan_phase, "partial");

        let last_scan_finished_at: Option<i64> = connection
            .query_row(
                "SELECT last_scan_finished_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_scan_finished_at");
        assert!(last_scan_finished_at.is_some());

        let last_successful_scan_at: Option<i64> = connection
            .query_row(
                "SELECT last_successful_scan_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_successful_scan_at");
        assert_eq!(
            last_successful_scan_at, previous_successful_at,
            "partial finalization must not update last_successful_scan_at"
        );

        let scan_issue_kind: Option<String> = connection
            .query_row(
                "SELECT scan_issue_kind FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan_issue_kind");
        assert!(
            scan_issue_kind.is_some(),
            "partial scan phase must have a non-null scan_issue_kind"
        );

        let locked_facts = read_directory_facts(&connection, "albums/locked");
        assert_eq!(locked_facts.dir_scan_state, "blocked");
    }

    #[test]
    fn finalized_scan_with_failed_descendant_becomes_partial_not_complete() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/corrupt".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Failed,
                        issue_kind: Some(SourceAccessIssueKind::UnknownIo),
                        diagnostic_detail: Some("read error".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit failed directory outcome");
            Ok(())
        })
        .expect("write failed directory outcome");

        let previous_successful_at: Option<i64> = connection
            .query_row(
                "SELECT last_successful_scan_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_successful_scan_at");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(scan_phase, "partial");

        let scan_issue_kind: Option<String> = connection
            .query_row(
                "SELECT scan_issue_kind FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan_issue_kind");
        assert_eq!(scan_issue_kind.as_deref(), Some("unknown_io"));

        let last_successful_scan_at: Option<i64> = connection
            .query_row(
                "SELECT last_successful_scan_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_successful_scan_at");
        assert_eq!(
            last_successful_scan_at, previous_successful_at,
            "partial finalization must not update last_successful_scan_at"
        );
    }

    #[test]
    fn fully_covered_scan_becomes_complete() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(scan_phase, "complete");

        let last_successful_scan_at: Option<i64> = connection
            .query_row(
                "SELECT last_successful_scan_at FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read last_successful_scan_at");
        assert!(last_successful_scan_at.is_some());

        let scan_issue_kind: Option<String> = connection
            .query_row(
                "SELECT scan_issue_kind FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan_issue_kind");
        assert!(scan_issue_kind.is_none());

        let error_detail: Option<String> = connection
            .query_row(
                "SELECT error_detail FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read error_detail");
        assert!(error_detail.is_none());
    }

    #[test]
    fn partial_scan_prefers_failed_descendant_issue_over_blocked() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/blocked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit blocked directory outcome");
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/failed".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Failed,
                        issue_kind: Some(SourceAccessIssueKind::TimedOut),
                        diagnostic_detail: Some("timed out".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit failed directory outcome");
            Ok(())
        })
        .expect("write directory outcomes");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(scan_phase, "partial");

        let scan_issue_kind: Option<String> = connection
            .query_row(
                "SELECT scan_issue_kind FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan_issue_kind");
        assert_eq!(
            scan_issue_kind.as_deref(),
            Some("timed_out"),
            "partial scan must prefer failed descendant issue over blocked"
        );
    }

    #[test]
    fn blocked_directory_without_issue_receives_fallback_unknown_io() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/blocked_no_issue".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: None,
                        diagnostic_detail: Some("blocked reason".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit blocked outcome without issue");
            Ok(())
        })
        .expect("write blocked outcome without issue");

        let dir_issue: Option<String> = connection
            .query_row(
                "SELECT dir_scan_issue_kind FROM source_directories WHERE relative_path = 'albums/blocked_no_issue'",
                [],
                |row| row.get(0),
            )
            .expect("read dir_scan_issue_kind");
        assert_eq!(
            dir_issue.as_deref(),
            Some("unknown_io"),
            "blocked directory without explicit issue must receive fallback unknown_io"
        );
    }

    #[test]
    fn failed_directory_without_issue_receives_fallback_unknown_io() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/failed_no_issue".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Failed,
                        issue_kind: None,
                        diagnostic_detail: Some("failed reason".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit failed outcome without issue");
            Ok(())
        })
        .expect("write failed outcome without issue");

        let dir_issue: Option<String> = connection
            .query_row(
                "SELECT dir_scan_issue_kind FROM source_directories WHERE relative_path = 'albums/failed_no_issue'",
                [],
                |row| row.get(0),
            )
            .expect("read dir_scan_issue_kind");
        assert_eq!(
            dir_issue.as_deref(),
            Some("unknown_io"),
            "failed directory without explicit issue must receive fallback unknown_io"
        );
    }

    #[test]
    fn schema_rejects_blocked_directory_without_issue_kind() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");

        let source_id = admit_write(&mut connection, |write| {
            let source_id = SourcesAuthorityTx::new(write)
                .upsert_source(&crate::authority::sources::UpsertSourceInput {
                    source_id: Some(1),
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "check-test".to_string(),
                    display_name: "Check Test".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    browser_order_ordinal: Some(0),
                    changed_at: 10,
                })
                .expect("upsert source");
            Ok(source_id)
        })
        .expect("write source");

        let result = connection.execute(
            "INSERT INTO source_directories (
                 source_directory_id,
                 source_id,
                 parent_source_directory_id,
                 name,
                 relative_path,
                 presence_state,
                 dir_scan_state,
                 dir_scan_updated_at,
                 created_at,
                 updated_at
             )
             VALUES (60, ?1, NULL, 'Locked', 'Locked', 'present', 'blocked', 1, 1, 1)",
            [source_id],
        );

        assert!(
            result.is_err(),
            "blocked directory without dir_scan_issue_kind must be rejected by schema CHECK"
        );
    }

    #[test]
    fn schema_rejects_failed_directory_without_issue_kind() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");

        let source_id = admit_write(&mut connection, |write| {
            let source_id = SourcesAuthorityTx::new(write)
                .upsert_source(&crate::authority::sources::UpsertSourceInput {
                    source_id: Some(1),
                    source_class: "internal".to_string(),
                    authority: "system".to_string(),
                    identity_kind: "filesystem_uuid".to_string(),
                    identity_value: "check-test-2".to_string(),
                    display_name: "Check Test 2".to_string(),
                    medium_label: None,
                    is_user_visible: true,
                    browser_order_ordinal: Some(0),
                    changed_at: 10,
                })
                .expect("upsert source");
            Ok(source_id)
        })
        .expect("write source");

        let result = connection.execute(
            "INSERT INTO source_directories (
                 source_directory_id,
                 source_id,
                 parent_source_directory_id,
                 name,
                 relative_path,
                 presence_state,
                 dir_scan_state,
                 dir_scan_updated_at,
                 created_at,
                 updated_at
             )
             VALUES (61, ?1, NULL, 'Failed', 'Failed', 'present', 'failed', 1, 1, 1)",
            [source_id],
        );

        assert!(
            result.is_err(),
            "failed directory without dir_scan_issue_kind must be rejected by schema CHECK"
        );
    }

    #[test]
    fn blocked_directory_remains_blocked_after_finalization() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 30,
                    },
                )
                .expect("commit blocked outcome");
            Ok(())
        })
        .expect("write blocked outcome");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let locked_facts = read_directory_facts(&connection, "albums/locked");
        assert_eq!(locked_facts.dir_scan_state, "blocked");
        assert_eq!(
            locked_facts.scanned_at, None,
            "blocked directory should not have scanned_at set during finalization"
        );

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(scan_phase, "partial");
    }

    #[test]
    fn finalized_scan_with_pending_descendant_under_blocked_subtree_becomes_partial() {
        let mut connection =
            rusqlite::Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        let (source_id, _parent_dir_id) = setup_source_with_directory(&mut connection);

        admit_write(&mut connection, |write| {
            DiscoveryTx::new(write)
                .commit_directory_enumeration_outcome(
                    source_id,
                    &DirectoryEnumerationOutcome {
                        relative_path: "albums/locked".to_string(),
                        outcome: DirectoryEnumerationOutcomeKind::Blocked,
                        issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                        diagnostic_detail: Some("permission denied".to_string()),
                        observed_at_ms: 20,
                    },
                )
                .expect("commit blocked outcome");
            Ok(())
        })
        .expect("write blocked outcome");

        admit_write(&mut connection, |write| {
            SourceDirectoriesAuthorityTx::new(write)
                .upsert_source_directory(&UpsertSourceDirectoryInput {
                    source_directory_id: Some(90),
                    source_id,
                    parent_source_directory_id: None,
                    name: "sub".to_string(),
                    relative_path: "albums/locked/sub".to_string(),
                    presence_state: SourcePresenceState::Present,
                    dir_scan_state: None,
                    dir_scan_issue_kind: None,
                    dir_scan_error_detail: None,
                    scanned_at: None,
                    mtime_ns: None,
                    first_created_at: Some(20),
                    changed_at: 20,
                })
                .expect("upsert child directory under blocked");
            Ok(())
        })
        .expect("write child directory");

        run_coverage_finalization(&mut connection, source_id, &["albums".to_string()], &[]);

        let scan_phase: String = connection
            .query_row(
                "SELECT scan_phase FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan phase");
        assert_eq!(
            scan_phase, "partial",
            "scan with pending descendant under blocked subtree must be partial, not complete"
        );

        let scan_issue_kind: Option<String> = connection
            .query_row(
                "SELECT scan_issue_kind FROM source_scan_state WHERE source_id = ?1",
                [source_id],
                |row| row.get(0),
            )
            .expect("read scan_issue_kind");
        assert!(
            scan_issue_kind.is_some(),
            "partial scan must have non-null scan_issue_kind"
        );
    }
}
