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
use crate::browse_media::classify_relative_path_file_kind;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{SourceFileId, SourcePresenceState, WorkPriorityClass};

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
                 blocked_reason = NULL,
                 error_detail = NULL,
                 last_scan_started_at = ?2,
                 updated_at = ?2
             WHERE source_id = ?1",
            params![root_id, started_at_ms],
        )?;
        if rows_changed == 0 {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        }

        Ok(started_at_ms.max(1))
    }

    pub(crate) fn commit_discovery_chunk(
        &mut self,
        _scan_run_id: i64,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.commit_chunk(root_id, locations)
    }

    pub(crate) fn commit_import_chunk(
        &mut self,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.commit_chunk(root_id, locations)
    }

    fn commit_chunk(
        &mut self,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
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
                        location.observed_at_ms,
                    )?;
                    observed_directory_paths.push(location.canonical_path.clone());
                }
                DiscoveredLocationKind::File => {
                    let processed = self.process_discovered_file(root_id, location)?;
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
        self.mark_missing_directories(root_id, observed_directory_paths, now_ms)?;
        let missing_file_ids = self.mark_missing_files(root_id, observed_file_paths, now_ms)?;
        self.recompute_directory_browseability(root_id)?;

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
        let scan_rows_changed = self.tx().execute(
            "UPDATE source_scan_state
             SET scan_phase = 'idle',
                 last_scan_finished_at = ?2,
                 last_successful_scan_at = ?2,
                 blocked_reason = NULL,
                 error_detail = NULL,
                 updated_at = ?2
             WHERE source_id = ?1",
            params![root_id, now_ms],
        )?;
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
    ) -> LibrarySqliteResult<ProcessedFile> {
        let existing = self.lookup_source_file(root_id, &location.canonical_path)?;
        let parent_source_directory_id = self.ensure_source_directory_chain(
            root_id,
            parent_relative_path(&location.canonical_path),
            location.observed_at_ms,
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
        changed_at: i64,
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
                        first_created_at: Some(changed_at),
                        changed_at,
                    },
                )?,
            );
        }

        Ok(parent_source_directory_id)
    }

    fn upsert_directory_presence(
        &self,
        root_id: i64,
        relative_path: &str,
        changed_at: i64,
    ) -> LibrarySqliteResult<()> {
        let _ = self.ensure_source_directory_chain(root_id, relative_path, changed_at)?;
        Ok(())
    }

    fn mark_missing_directories(
        &self,
        root_id: i64,
        observed_directory_paths: &[String],
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
            self.tx().execute(
                "UPDATE source_directories
                 SET presence_state = 'missing',
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

    fn recompute_directory_browseability(&self, root_id: i64) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "UPDATE source_directories
             SET media_browseability = 'unknown'
             WHERE source_id = ?1",
            [root_id],
        )?;

        self.tx().execute(
            "UPDATE source_directories
             SET media_browseability = 'browseable'
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND source_directory_id IN (
                   SELECT DISTINCT f.parent_source_directory_id
                   FROM source_files f
                   WHERE f.source_id = ?1
                     AND f.presence_state = 'present'
                     AND f.parent_source_directory_id IS NOT NULL
                     AND f.media_class IN ('audio', 'video')
               )",
            [root_id],
        )?;

        self.tx().execute(
            "WITH RECURSIVE browseable_up(source_directory_id) AS (
                 SELECT DISTINCT f.parent_source_directory_id
                 FROM source_files f
                 WHERE f.source_id = ?1
                   AND f.presence_state = 'present'
                   AND f.parent_source_directory_id IS NOT NULL
                   AND f.media_class IN ('audio', 'video')
                 UNION
                 SELECT d.parent_source_directory_id
                 FROM source_directories d
                 JOIN browseable_up b ON d.source_directory_id = b.source_directory_id
                 WHERE d.source_id = ?1
                   AND d.presence_state = 'present'
                   AND d.parent_source_directory_id IS NOT NULL
             )
             UPDATE source_directories
             SET media_browseability = 'browseable'
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND source_directory_id IN browseable_up",
            [root_id],
        )?;

        self.tx().execute(
            "UPDATE source_directories
             SET media_browseability = 'empty'
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND media_browseability = 'unknown'",
            [root_id],
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

#[cfg(test)]
mod tests {
    use crate::authority::ingest::DiscoveryTx;
    use crate::authority::sources::{
        RecordSourceFileObservationInput, SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx,
        SourceLocatorsAuthorityTx, SourceStateAuthorityTx, SourcesAuthorityTx,
        UpsertSourceDirectoryInput, UpsertSourceLocatorInput, UpsertSourceScanStateInput,
        UpsertSourceStateInput,
    };
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{SourcePresenceState, SourceResolutionStatus, SourceScanPhase};

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
                    resolution_status: SourceResolutionStatus::Resolved,
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
                    blocked_reason: None,
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

    fn read_browseability(
        connection: &rusqlite::Connection,
        relative_path: &str,
    ) -> Option<String> {
        connection
            .query_row(
                "SELECT media_browseability FROM source_directories WHERE relative_path = ?1",
                [relative_path],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }

    fn run_browseability_recompute(
        connection: &mut rusqlite::Connection,
        root_id: i64,
        observed_directory_paths: &[String],
        observed_file_paths: &[String],
    ) {
        admit_write(connection, |write| {
            DiscoveryTx::new(write)
                .finalize_scan(root_id, observed_file_paths, observed_directory_paths)
                .expect("finalize scan to recompute browseability");
            Ok(())
        })
        .expect("write browseability");
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
    fn png_file_stores_unsupported_media_class() {
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
        assert_eq!(
            read_media_class(&connection, "albums/cover.png"),
            "unsupported"
        );
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
    fn directory_with_wma_and_alac_files_marks_ancestors_browseable() {
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

        run_browseability_recompute(
            &mut connection,
            source_id,
            &["nested".to_string()],
            &[
                "nested/track.wma".to_string(),
                "nested/track.alac".to_string(),
            ],
        );

        assert_eq!(
            read_browseability(&connection, "nested"),
            Some("browseable".to_string()),
            "directory with .wma and .alac should be browseable"
        );
    }

    #[test]
    fn directory_with_only_unsupported_files_is_empty() {
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

        run_browseability_recompute(
            &mut connection,
            source_id,
            &["docs".to_string()],
            &["docs/readme.txt".to_string(), "docs/cover.png".to_string()],
        );

        assert_eq!(
            read_browseability(&connection, "docs"),
            Some("empty".to_string()),
            "directory with only unsupported files should be empty"
        );
    }

    #[test]
    fn nested_audio_files_propagate_browseability_to_ancestors() {
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

        run_browseability_recompute(
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
            read_browseability(&connection, "albums/mid/deep"),
            Some("browseable".to_string()),
            "deep directory with audio file should be browseable"
        );
        assert_eq!(
            read_browseability(&connection, "albums/mid"),
            Some("browseable".to_string()),
            "mid directory should inherit browseability from deep child"
        );
        assert_eq!(
            read_browseability(&connection, "albums"),
            Some("browseable".to_string()),
            "root directory should inherit browseability through nested children"
        );
    }
}
