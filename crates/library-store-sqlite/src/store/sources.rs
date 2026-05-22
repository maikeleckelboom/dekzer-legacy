use std::path::{Path, PathBuf};

use crate::authority::roots::{
    ApplyRootChangedInput, ApplyRootEjectCancelledInput, ApplyRootMountedInput,
    ApplyRootUnmountPendingInput, ApplyRootUnmountRequestedInput, ApplyRootUnmountedInput,
    CanonicalSourcePath, RegisterRemovableRootInput, ResolvedRoot, RootStatusDelta,
};
use crate::authority::sources::DeleteSourceLocationInput;
use crate::authority::sources::{
    RecordSourceFileObservationInput, SourceDirectoriesAuthorityTx, SourceFilesAuthorityTx,
    SourceLocationsAuthorityTx, SourceLocatorsAuthorityTx, SourceStateAuthorityTx,
    SourcesAuthorityTx, UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocationInput,
    UpsertSourceLocatorInput, UpsertSourceScanStateInput, UpsertSourceStateInput,
};
use crate::authority::work::{QueueInspectSourceWorkInput, WorkItemsAuthorityTx};
use crate::authority::write_lane::AdmittedWrite;
use crate::browse_media::classify_relative_path_file_kind;
use crate::publication;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{ProjectionDomain, SourceFileId, SourcePresenceState, WorkPriorityClass};

use super::{SqliteDurableStore, bootstrap::open_connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterLocalRootInput {
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRoot {
    pub root_id: i64,
    pub canonical_path: PathBuf,
    pub availability: LocalRootAvailability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRootAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLocalRootsResult {
    pub roots: Vec<LocalRoot>,
}

impl SqliteDurableStore {
    pub fn upsert_source(&self, input: UpsertSourceInput) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            let source_id = SourcesAuthorityTx::new(write).upsert_source(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(source_id)
        })
    }

    pub fn upsert_source_locator(
        &self,
        input: UpsertSourceLocatorInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            SourceLocatorsAuthorityTx::new(write).upsert_source_locator(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(())
        })
    }

    pub fn upsert_source_state(&self, input: UpsertSourceStateInput) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            SourceStateAuthorityTx::new(write).upsert_source_state(&input)?;
            publication::reseed_projection_domains(
                write,
                &[
                    ProjectionDomain::Navigation,
                    ProjectionDomain::LibraryBrowser,
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_source_scan_state(
        &self,
        input: UpsertSourceScanStateInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            SourceStateAuthorityTx::new(write).upsert_source_scan_state(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(())
        })
    }

    pub fn upsert_source_directory(
        &self,
        input: UpsertSourceDirectoryInput,
    ) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            SourceDirectoriesAuthorityTx::new(write).upsert_source_directory(&input)
        })
    }

    pub fn upsert_source_location(
        &self,
        input: UpsertSourceLocationInput,
    ) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            let source_location_id =
                SourceLocationsAuthorityTx::new(write).upsert_source_location(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(source_location_id)
        })
    }

    pub fn delete_source_location(
        &self,
        input: DeleteSourceLocationInput,
    ) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let deleted = SourceLocationsAuthorityTx::new(write).delete_source_location(&input)?;
            if deleted {
                publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            }
            Ok(deleted)
        })
    }

    pub fn record_source_file_observation(
        &self,
        input: RecordSourceFileObservationInput,
    ) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            let source_file_id =
                SourceFilesAuthorityTx::new(write).record_source_file_observation(&input)?;
            queue_inspect_source_work_for_observation(
                write,
                source_file_id,
                &input.relative_path,
                input.presence_state,
                input.size_bytes,
                input.mtime_ns,
                input.updated_at,
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(source_file_id)
        })
    }

    pub fn register_local_root(
        &self,
        input: RegisterLocalRootInput,
    ) -> LibrarySqliteResult<LocalRoot> {
        let resolved_root = self.bootstrap_root(&input.absolute_path)?;
        Ok(LocalRoot {
            root_id: resolved_root.root_id,
            canonical_path: resolved_root.canonical_path,
            availability: LocalRootAvailability::Available,
        })
    }

    pub fn read_local_roots(&self) -> LibrarySqliteResult<ReadLocalRootsResult> {
        let connection = open_connection(&self.path)?;
        let mut statement = connection.prepare(
            "SELECT sl.source_id, sl.absolute_path, lss.resolution_status
             FROM source_locators sl
             LEFT JOIN source_state lss
               ON lss.source_id = sl.source_id
             WHERE sl.locator_kind = 'absolute_path'
             ORDER BY sl.source_id ASC",
        )?;
        let roots = statement
            .query_map([], |row| {
                let resolution_status: Option<String> = row.get(2)?;
                let availability = match resolution_status.as_deref() {
                    Some("resolved") => LocalRootAvailability::Available,
                    _ => LocalRootAvailability::Unavailable,
                };
                Ok(LocalRoot {
                    root_id: row.get(0)?,
                    canonical_path: PathBuf::from(row.get::<_, String>(1)?),
                    availability,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ReadLocalRootsResult { roots })
    }

    #[allow(dead_code)]
    pub(crate) fn bootstrap_root(
        &self,
        path: impl AsRef<Path>,
    ) -> LibrarySqliteResult<ResolvedRoot> {
        let canonical_path = CanonicalSourcePath::resolve(path.as_ref())?;
        let resolved_root =
            self.with_source_lifecycle_tx(|tx| tx.bootstrap_root(&canonical_path))?;
        self.sync_root_projection_state(&[resolved_root.root_id])?;
        Ok(resolved_root)
    }

    #[allow(dead_code)]
    pub(crate) fn register_removable_root(
        &self,
        input: RegisterRemovableRootInput,
    ) -> LibrarySqliteResult<i64> {
        let root_id = self.with_source_lifecycle_tx(|tx| tx.register_removable_root(&input))?;
        self.sync_root_projection_state(&[root_id])?;
        Ok(root_id)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_mounted(
        &self,
        input: ApplyRootMountedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_mounted(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_unmount_requested(
        &self,
        input: ApplyRootUnmountRequestedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_unmount_requested(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_eject_cancelled(
        &self,
        input: ApplyRootEjectCancelledInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_eject_cancelled(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_unmount_pending(
        &self,
        input: ApplyRootUnmountPendingInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_unmount_pending(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_unmounted(
        &self,
        input: ApplyRootUnmountedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_unmounted(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn apply_root_changed(
        &self,
        input: ApplyRootChangedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let deltas = self.with_source_lifecycle_tx(|tx| tx.apply_root_changed(input))?;
        self.sync_root_projection_state(&root_ids_from_deltas(&deltas))?;
        Ok(deltas)
    }

    #[allow(dead_code)]
    pub(crate) fn mark_root_scan_started(
        &self,
        root_id: i64,
        started_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.with_source_lifecycle_tx(|tx| tx.mark_root_scan_started(root_id, started_at_ms))?;
        self.sync_root_projection_state(&[root_id])
    }

    #[allow(dead_code)]
    pub(crate) fn mark_root_scan_failed(
        &self,
        root_id: i64,
        failed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.with_source_lifecycle_tx(|tx| tx.mark_root_scan_failed(root_id, failed_at_ms))?;
        self.sync_root_projection_state(&[root_id])
    }

    #[allow(dead_code)]
    pub(crate) fn mark_root_scan_completed(
        &self,
        root_id: i64,
        completed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.with_source_lifecycle_tx(|tx| tx.mark_root_scan_completed(root_id, completed_at_ms))?;
        self.sync_root_projection_state(&[root_id])
    }

    #[allow(dead_code)]
    pub(crate) fn interrupt_root_bound_work(
        &self,
        root_ids: &[i64],
        interrupted_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.with_source_lifecycle_tx(|tx| {
            tx.interrupt_root_bound_work(root_ids, interrupted_at_ms)
        })?;
        self.sync_root_projection_state(root_ids)
    }

    #[allow(dead_code)]
    pub(crate) fn resume_root_bound_work(
        &self,
        root_ids: &[i64],
        resumed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.with_source_lifecycle_tx(|tx| tx.resume_root_bound_work(root_ids, resumed_at_ms))?;
        self.sync_root_projection_state(root_ids)
    }

    #[allow(dead_code)]
    pub(crate) fn refresh_root_availability_from_filesystem(&self) -> LibrarySqliteResult<()> {
        let connection = open_connection(&self.path)?;
        let mut statement = connection.prepare(
            "SELECT source_id
             FROM source_locators
             WHERE locator_kind = 'absolute_path'
             ORDER BY source_id ASC",
        )?;
        let root_ids = statement
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let refreshed_at_ms = unix_time_ms()?;

        self.with_source_lifecycle_tx(|tx| {
            for root_id in &root_ids {
                tx.refresh_absolute_path_root(*root_id, refreshed_at_ms)?;
            }

            Ok(())
        })?;
        self.sync_root_projection_state(&root_ids)
    }
}

fn dedup_ids(ids: &[i64]) -> Vec<i64> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn root_ids_from_deltas(deltas: &[RootStatusDelta]) -> Vec<i64> {
    dedup_ids(&deltas.iter().map(|delta| delta.root_id).collect::<Vec<_>>())
}

fn queue_inspect_source_work_for_observation(
    write: &AdmittedWrite<'_>,
    source_file_id: i64,
    relative_path: &str,
    presence_state: SourcePresenceState,
    size_bytes: Option<i64>,
    mtime_ns: Option<i64>,
    updated_at: i64,
) -> LibrarySqliteResult<bool> {
    if presence_state != SourcePresenceState::Present
        || classify_relative_path_file_kind(relative_path) != "audio"
    {
        return Ok(false);
    }

    let result = WorkItemsAuthorityTx::new(write).queue_inspect_source_work(
        &QueueInspectSourceWorkInput {
            source_file_id: SourceFileId::new(source_file_id).ok_or_else(|| {
                LibrarySqliteError::WriteInvariant(format!(
                    "recorded source_files.source_file_id is not a domain id: {source_file_id}"
                ))
            })?,
            basis_fingerprint: source_observation_basis_fingerprint(
                source_file_id,
                relative_path,
                size_bytes,
                mtime_ns,
                updated_at,
            ),
            priority_class: WorkPriorityClass::Interactive,
            queued_at: updated_at,
        },
    )?;
    Ok(result.created)
}

pub(super) fn source_observation_basis_fingerprint(
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
