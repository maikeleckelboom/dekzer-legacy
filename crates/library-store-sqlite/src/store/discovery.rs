use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::authority::ingest::{
    DirectoryEnumerationOutcome, DiscoveredFileCommitResult, DiscoveredFileInput,
    DiscoveredLocationInput, DiscoveryBatch, DiscoveryChunkCommitResult, DiscoveryCommitResult,
    DiscoveryFinalizeResult, FilesystemWalkEntries, build_discovered_locations_from_files,
};
use crate::authority::sources::{SourceAccessProbeResult, probe_source_access};
use crate::time::unix_time_ms;
use crate::work_control::{
    MountEpochStamp, ROOT_SCAN_SOURCE_MEDIA_WRITE_POLICY, admit_source_bound_work,
    load_mount_epoch_stamp_for_file, load_mount_epoch_stamp_for_root,
};
use crate::{CanonicalError, CanonicalErrorCode, LibrarySqliteError, LibrarySqliteResult};

use super::SqliteDurableStore;

const DISCOVERY_LOCATION_CHUNK_SIZE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ExplicitImportResult {
    pub files: Vec<DiscoveredFileCommitResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct RootScanMaterializationResult {
    pub scan_run_id: i64,
    pub discovered_file_count: usize,
    pub queued_source_work_items: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum RootScanObservation {
    HierarchyPublished {
        root_id: i64,
        scan_run_id: i64,
        reason: RootScanHierarchyObservationReason,
    },
    SourceWorkQueued {
        root_id: i64,
        queued_work_items: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootScanHierarchyObservationReason {
    ChunkCommitted,
    Finalized,
}

impl SqliteDurableStore {
    pub fn run_root_scan(
        &self,
        root_id: i64,
        scan_started_at_ms: i64,
    ) -> LibrarySqliteResult<RootScanMaterializationResult> {
        let root_path = self.read_root_scan_path(root_id)?;
        self.execute_root_scan_materialization(root_id, root_path, scan_started_at_ms)
    }

    pub(crate) fn read_mount_epoch_stamp_for_root(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<MountEpochStamp> {
        let connection = self.open_read_connection()?;
        load_mount_epoch_stamp_for_root(&connection, root_id)
    }

    #[allow(dead_code)]
    pub(crate) fn read_mount_epoch_stamp_for_file(
        &self,
        file_id: i64,
    ) -> LibrarySqliteResult<MountEpochStamp> {
        let connection = self.open_read_connection()?;
        load_mount_epoch_stamp_for_file(&connection, file_id)
    }

    #[allow(dead_code)]
    pub(crate) fn commit_discovery(
        &self,
        batch: DiscoveryBatch,
    ) -> LibrarySqliteResult<DiscoveryCommitResult> {
        batch.validate()?;
        let root_display_name = self.read_root_display_name(batch.root_id)?;
        let result = self.commit_discovery_locations(
            batch.root_id,
            batch.scan_started_at_ms,
            build_discovered_locations_from_files(&root_display_name, &batch.files),
            |_, _| {},
        )?;
        self.reseed_navigation_and_library_browser_projections()?;
        Ok(result)
    }

    #[allow(dead_code)]
    pub(crate) fn import_files(
        &self,
        root_id: i64,
        files: Vec<DiscoveredFileInput>,
    ) -> LibrarySqliteResult<ExplicitImportResult> {
        if files.is_empty() {
            return Ok(ExplicitImportResult { files: Vec::new() });
        }

        DiscoveryBatch {
            root_id,
            scan_started_at_ms: 0,
            files: files.clone(),
        }
        .validate()?;
        let root_display_name = self.read_root_display_name(root_id)?;
        let result = self.with_discovery_tx(|tx| {
            tx.commit_import_chunk(
                root_id,
                &build_discovered_locations_from_files(&root_display_name, &files),
            )
        })?;
        let changed_file_ids = result
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        if !changed_file_ids.is_empty() {
            self.reseed_navigation_and_library_browser_projections()?;
        }

        Ok(ExplicitImportResult {
            files: result.files,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn commit_empty_scan(
        &self,
        root_id: i64,
        scan_started_at_ms: i64,
    ) -> LibrarySqliteResult<DiscoveryCommitResult> {
        let root_display_name = self.read_root_display_name(root_id)?;
        let result = self.commit_discovery_locations(
            root_id,
            scan_started_at_ms,
            vec![DiscoveredLocationInput::folder(
                "",
                root_display_name,
                scan_started_at_ms,
            )],
            |_, _| {},
        )?;
        self.reseed_navigation_and_library_browser_projections()?;
        Ok(result)
    }

    #[allow(dead_code)]
    pub(crate) fn execute_root_scan_materialization(
        &self,
        root_id: i64,
        root_path: impl AsRef<Path>,
        scan_started_at_ms: i64,
    ) -> LibrarySqliteResult<RootScanMaterializationResult> {
        self.execute_root_scan_materialization_with_observer(
            root_id,
            root_path.as_ref(),
            scan_started_at_ms,
            |_| {},
        )
    }

    #[allow(dead_code)]
    pub(crate) fn execute_root_scan_materialization_with_observer<F>(
        &self,
        root_id: i64,
        root_path: &Path,
        scan_started_at_ms: i64,
        mut observe: F,
    ) -> LibrarySqliteResult<RootScanMaterializationResult>
    where
        F: FnMut(RootScanObservation),
    {
        let access_checked_at_ms = unix_time_ms()?;
        let access_probe = probe_source_access(root_path, access_checked_at_ms);
        self.with_source_lifecycle_tx(|tx| {
            tx.apply_root_access_probe_result(root_id, &access_probe)
        })?;
        if !access_probe.is_accessible() {
            return Err(root_scan_access_error(root_id, &access_probe));
        }

        let scan_stamp = self.read_mount_epoch_stamp_for_root(root_id)?;
        let scan_token = match admit_source_bound_work(&self.source_admission_gate, root_id) {
            Ok(token) => token,
            Err(error) => {
                let blocked_at_ms = unix_time_ms()?;
                self.with_source_lifecycle_tx(|tx| {
                    tx.mark_root_scan_blocked(root_id, blocked_at_ms)
                })?;
                return Err(error);
            }
        };
        let scan_run_id = self.start_discovery_scan_session(root_id, scan_started_at_ms)?;
        let mut discovered_files = Vec::new();
        let mut observed_file_paths = HashSet::new();
        let mut observed_directory_paths = HashSet::new();
        let mut pending_chunk = Vec::with_capacity(DISCOVERY_LOCATION_CHUNK_SIZE);

        for location in FilesystemWalkEntries::from_source_media_root(
            root_path,
            ROOT_SCAN_SOURCE_MEDIA_WRITE_POLICY,
        ) {
            if scan_token.is_cancelled() {
                let interrupted_at_ms = unix_time_ms()?;
                self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
                return Err(LibrarySqliteError::RootWorkCancelled { root_id });
            }
            let location = match location {
                Ok(location) => location,
                Err(error) => {
                    if !pending_chunk.is_empty() {
                        let chunk_result =
                            self.commit_discovery_chunk_tx(scan_run_id, root_id, &pending_chunk)?;
                        extend_discovery_scan_progress(
                            &mut discovered_files,
                            &mut observed_file_paths,
                            &mut observed_directory_paths,
                            chunk_result,
                        );
                        pending_chunk.clear();
                    }
                    self.commit_directory_enumeration_outcome_tx(root_id, error.into_outcome())?;
                    continue;
                }
            };
            pending_chunk.push(location);
            if pending_chunk.len() < DISCOVERY_LOCATION_CHUNK_SIZE {
                continue;
            }
            if let Err(error) = validate_root_scan_stamp(self, scan_stamp) {
                let interrupted_at_ms = unix_time_ms()?;
                self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
                return Err(error);
            }

            let chunk_result =
                self.commit_discovery_chunk_tx(scan_run_id, root_id, &pending_chunk)?;
            extend_discovery_scan_progress(
                &mut discovered_files,
                &mut observed_file_paths,
                &mut observed_directory_paths,
                chunk_result,
            );
            pending_chunk.clear();
            observe(RootScanObservation::HierarchyPublished {
                root_id,
                scan_run_id,
                reason: RootScanHierarchyObservationReason::ChunkCommitted,
            });
        }

        if !pending_chunk.is_empty() {
            if scan_token.is_cancelled() {
                let interrupted_at_ms = unix_time_ms()?;
                self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
                return Err(LibrarySqliteError::RootWorkCancelled { root_id });
            }
            if let Err(error) = validate_root_scan_stamp(self, scan_stamp) {
                let interrupted_at_ms = unix_time_ms()?;
                self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
                return Err(error);
            }
            let chunk_result =
                self.commit_discovery_chunk_tx(scan_run_id, root_id, &pending_chunk)?;
            extend_discovery_scan_progress(
                &mut discovered_files,
                &mut observed_file_paths,
                &mut observed_directory_paths,
                chunk_result,
            );
            observe(RootScanObservation::HierarchyPublished {
                root_id,
                scan_run_id,
                reason: RootScanHierarchyObservationReason::ChunkCommitted,
            });
        }

        if scan_token.is_cancelled() {
            let interrupted_at_ms = unix_time_ms()?;
            self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
            return Err(LibrarySqliteError::RootWorkCancelled { root_id });
        }
        if let Err(error) = validate_root_scan_stamp(self, scan_stamp) {
            let interrupted_at_ms = unix_time_ms()?;
            self.interrupt_root_bound_work(&[root_id], interrupted_at_ms)?;
            return Err(error);
        }
        let _finalize_result = self.finalize_discovery_scan_tx(
            root_id,
            &observed_file_paths.into_iter().collect::<Vec<_>>(),
            &observed_directory_paths.into_iter().collect::<Vec<_>>(),
        )?;

        // Root scan completion boundary: discovery finalization is complete.
        // Any later source work is outside the completion semantics for this scan.
        self.mark_root_scan_completed(root_id, unix_time_ms()?)?;

        observe(RootScanObservation::HierarchyPublished {
            root_id,
            scan_run_id,
            reason: RootScanHierarchyObservationReason::Finalized,
        });
        self.reseed_navigation_and_library_browser_projections()?;
        let queued_source_work_items = discovered_files
            .iter()
            .filter(|file| file.needs_probe)
            .count();
        if queued_source_work_items > 0 {
            observe(RootScanObservation::SourceWorkQueued {
                root_id,
                queued_work_items: queued_source_work_items,
            });
        }

        Ok(RootScanMaterializationResult {
            scan_run_id,
            discovered_file_count: discovered_files.len(),
            queued_source_work_items,
        })
    }

    fn read_root_display_name(&self, root_id: i64) -> LibrarySqliteResult<String> {
        self.with_discovery_tx(|tx| tx.root_display_name(root_id))
    }

    fn read_root_scan_path(&self, root_id: i64) -> LibrarySqliteResult<PathBuf> {
        let connection = self.open_read_connection()?;
        let Some((root_path, access_state)) = connection
            .query_row(
                "SELECT COALESCE(ss.effective_path, sl.absolute_path) AS root_path,
                        ss.access_state
                 FROM sources s
                 JOIN source_locators sl
                   ON sl.source_id = s.source_id
                 JOIN source_state ss
                   ON ss.source_id = s.source_id
                 WHERE s.source_id = ?1",
                [root_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };

        if access_state != "accessible" {
            return Err(LibrarySqliteError::Canonical(CanonicalError::new(
                CanonicalErrorCode::NotFound,
                format!("root {root_id} is not accessible for scan: access_state={access_state}"),
            )));
        }

        let Some(root_path) = root_path else {
            return Err(LibrarySqliteError::Canonical(CanonicalError::new(
                CanonicalErrorCode::NotFound,
                format!("root {root_id} has no effective path for scan"),
            )));
        };

        Ok(PathBuf::from(root_path))
    }

    fn start_discovery_scan_session(
        &self,
        root_id: i64,
        scan_started_at_ms: i64,
    ) -> LibrarySqliteResult<i64> {
        self.with_discovery_tx(|tx| tx.start_scan_session(root_id, scan_started_at_ms))
    }

    fn commit_discovery_chunk_tx(
        &self,
        scan_run_id: i64,
        root_id: i64,
        locations: &[DiscoveredLocationInput],
    ) -> LibrarySqliteResult<DiscoveryChunkCommitResult> {
        self.with_discovery_tx(|tx| tx.commit_discovery_chunk(scan_run_id, root_id, locations))
    }

    fn finalize_discovery_scan_tx(
        &self,
        root_id: i64,
        observed_file_paths: &[String],
        observed_directory_paths: &[String],
    ) -> LibrarySqliteResult<DiscoveryFinalizeResult> {
        self.with_discovery_tx(|tx| {
            tx.finalize_scan(root_id, observed_file_paths, observed_directory_paths)
        })
    }

    fn commit_directory_enumeration_outcome_tx(
        &self,
        root_id: i64,
        outcome: DirectoryEnumerationOutcome,
    ) -> LibrarySqliteResult<()> {
        self.with_discovery_tx(|tx| tx.commit_directory_enumeration_outcome(root_id, &outcome))
    }

    fn commit_discovery_locations<I, F>(
        &self,
        root_id: i64,
        scan_started_at_ms: i64,
        locations: I,
        mut observe_progress: F,
    ) -> LibrarySqliteResult<DiscoveryCommitResult>
    where
        I: IntoIterator<Item = DiscoveredLocationInput>,
        F: FnMut(i64, RootScanHierarchyObservationReason),
    {
        let scan_run_id = self.start_discovery_scan_session(root_id, scan_started_at_ms)?;
        let mut discovered_files = Vec::new();
        let mut observed_file_paths = HashSet::new();
        let mut observed_directory_paths = HashSet::new();
        let mut pending_chunk = Vec::with_capacity(DISCOVERY_LOCATION_CHUNK_SIZE);

        for location in locations {
            pending_chunk.push(location);
            if pending_chunk.len() < DISCOVERY_LOCATION_CHUNK_SIZE {
                continue;
            }

            let chunk_result =
                self.commit_discovery_chunk_tx(scan_run_id, root_id, &pending_chunk)?;
            extend_discovery_scan_progress(
                &mut discovered_files,
                &mut observed_file_paths,
                &mut observed_directory_paths,
                chunk_result,
            );
            pending_chunk.clear();
            observe_progress(
                scan_run_id,
                RootScanHierarchyObservationReason::ChunkCommitted,
            );
        }

        if !pending_chunk.is_empty() {
            let chunk_result =
                self.commit_discovery_chunk_tx(scan_run_id, root_id, &pending_chunk)?;
            extend_discovery_scan_progress(
                &mut discovered_files,
                &mut observed_file_paths,
                &mut observed_directory_paths,
                chunk_result,
            );
            observe_progress(
                scan_run_id,
                RootScanHierarchyObservationReason::ChunkCommitted,
            );
        }

        let _finalize_result = self.finalize_discovery_scan_tx(
            root_id,
            &observed_file_paths.into_iter().collect::<Vec<_>>(),
            &observed_directory_paths.into_iter().collect::<Vec<_>>(),
        )?;
        observe_progress(scan_run_id, RootScanHierarchyObservationReason::Finalized);

        Ok(DiscoveryCommitResult {
            scan_run_id,
            files: discovered_files,
        })
    }
}

fn extend_discovery_scan_progress(
    discovered_files: &mut Vec<DiscoveredFileCommitResult>,
    observed_file_paths: &mut HashSet<String>,
    observed_directory_paths: &mut HashSet<String>,
    chunk_result: DiscoveryChunkCommitResult,
) {
    observed_file_paths.extend(chunk_result.observed_file_paths);
    observed_directory_paths.extend(chunk_result.observed_directory_paths);
    discovered_files.extend(chunk_result.files);
}

fn root_scan_access_error(root_id: i64, probe: &SourceAccessProbeResult) -> LibrarySqliteError {
    let issue = probe
        .issue_kind()
        .map(|kind| kind.as_str())
        .unwrap_or("unknown_io");
    let detail = probe
        .diagnostic_detail()
        .unwrap_or("source root is not accessible");
    LibrarySqliteError::Canonical(CanonicalError::new(
        CanonicalErrorCode::NotFound,
        format!("scan blocked for root {root_id}: issue={issue}: {detail}"),
    ))
}

fn validate_root_scan_stamp(
    durable_store: &SqliteDurableStore,
    stamp: MountEpochStamp,
) -> LibrarySqliteResult<()> {
    let current = durable_store.read_mount_epoch_stamp_for_root(stamp.root_id)?;
    if current.mount_epoch != stamp.mount_epoch {
        return Err(LibrarySqliteError::StaleMountEpochStamp {
            root_id: stamp.root_id,
            admitted_mount_epoch: stamp.mount_epoch,
            current_mount_epoch: current.mount_epoch,
        });
    }

    Ok(())
}
