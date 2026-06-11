use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;

use crate::authority::roots::{
    ApplyRootChangedInput, ApplyRootEjectCancelledInput, ApplyRootMountedInput,
    ApplyRootUnmountPendingInput, ApplyRootUnmountRequestedInput, ApplyRootUnmountedInput,
    CanonicalSourcePath, RegisterRemovableRootInput, ResolvedRoot, RootStatusDelta,
};
use crate::authority::sources::DeleteSourceLocationInput;
use crate::authority::sources::{
    EstablishRootChildDirectoryInput, SourceAccessProbeResult, probe_source_access,
    source_access_issue_kind_from_io_error,
};
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
use library_domain::{
    ProjectionDomain, SourceAccessIssueKind, SourceFileId, SourcePresenceState, WorkPriorityClass,
};

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
pub struct RootNavigationWindowEstablishment {
    pub root_id: i64,
    pub state: RootNavigationWindowEstablishmentState,
    pub immediate_child_directory_count: usize,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootNavigationWindowEstablishmentState {
    Established,
    NotRequired,
    Missing,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLocalRootsResult {
    pub roots: Vec<LocalRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnregisterLocalRootInput {
    pub root_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnregisterLocalRootResult {
    pub unregistered: bool,
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
        let _ = self.establish_root_navigation_window(resolved_root.root_id)?;
        Ok(LocalRoot {
            root_id: resolved_root.root_id,
            canonical_path: resolved_root.canonical_path,
            availability: LocalRootAvailability::Available,
        })
    }

    pub fn read_local_roots(&self) -> LibrarySqliteResult<ReadLocalRootsResult> {
        let connection = open_connection(&self.path)?;
        let mut statement = connection.prepare(
            "SELECT sl.source_id, sl.absolute_path, lss.access_state
             FROM source_locators sl
             JOIN sources s ON s.source_id = sl.source_id
             LEFT JOIN source_state lss
               ON lss.source_id = sl.source_id
             WHERE sl.locator_kind = 'absolute_path'
               AND s.is_user_visible = 1
             ORDER BY sl.source_id ASC",
        )?;
        let roots = statement
            .query_map([], |row| {
                let access_state: Option<String> = row.get(2)?;
                let availability = match access_state.as_deref() {
                    Some("accessible") => LocalRootAvailability::Available,
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

    pub fn unregister_local_root(
        &self,
        input: UnregisterLocalRootInput,
    ) -> LibrarySqliteResult<UnregisterLocalRootResult> {
        if input.root_id <= 0 {
            return Ok(UnregisterLocalRootResult {
                unregistered: false,
            });
        }
        self.with_write(|write| {
            let locator_kind: Option<String> = write
                .query_row(
                    "SELECT sl.locator_kind
                     FROM source_locators sl
                     JOIN sources s ON s.source_id = sl.source_id
                     WHERE sl.source_id = ?1",
                    [input.root_id],
                    |row| row.get(0),
                )
                .optional()?;
            let locator_kind = match locator_kind {
                Some(kind) => kind,
                None => return Ok(UnregisterLocalRootResult { unregistered: false }),
            };
            if locator_kind != "absolute_path" {
                return Ok(UnregisterLocalRootResult { unregistered: false });
            }
            let changed_at = unix_time_ms()?;
            write.execute(
                "UPDATE sources SET is_user_visible = 0, updated_at = ?2 WHERE source_id = ?1 AND is_user_visible = 1",
                rusqlite::params![input.root_id, changed_at],
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(UnregisterLocalRootResult { unregistered: true })
        })
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

    pub fn establish_root_navigation_window(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<RootNavigationWindowEstablishment> {
        if !self.root_navigation_window_establishment_required(root_id)? {
            return Ok(RootNavigationWindowEstablishment {
                root_id,
                state: RootNavigationWindowEstablishmentState::NotRequired,
                immediate_child_directory_count: 0,
                detail: None,
            });
        }

        let root_path = self.read_root_scan_path(root_id)?;
        let checked_at_ms = unix_time_ms()?;
        let access_probe = probe_source_access(&root_path, checked_at_ms);
        self.with_source_lifecycle_tx(|tx| {
            tx.apply_root_access_probe_result(root_id, &access_probe)
        })?;

        let effective_root = match &access_probe {
            SourceAccessProbeResult::Accessible { effective_root, .. } => effective_root.clone(),
            _ => {
                self.sync_root_projection_state(&[root_id])?;
                return Ok(root_establishment_from_access_probe(root_id, &access_probe));
            }
        };

        let observed_at = unix_time_ms()?;
        let child_directories =
            match collect_immediate_root_child_directories(&effective_root, observed_at) {
                Ok(child_directories) => child_directories,
                Err(failure) => {
                    self.sync_root_projection_state(&[root_id])?;
                    return Ok(RootNavigationWindowEstablishment {
                        root_id,
                        state: failure.state,
                        immediate_child_directory_count: 0,
                        detail: Some(failure.detail),
                    });
                }
            };
        let observed_relative_paths = child_directories
            .iter()
            .map(|directory| directory.relative_path.clone())
            .collect::<HashSet<_>>();

        self.with_write(|write| {
            let directories = SourceDirectoriesAuthorityTx::new(write);
            for directory in &child_directories {
                directories.establish_root_child_directory(&EstablishRootChildDirectoryInput {
                    source_id: root_id,
                    name: directory.name.clone(),
                    relative_path: directory.relative_path.clone(),
                    mtime_ns: directory.mtime_ns,
                    observed_at,
                })?;
            }
            let _ = directories.mark_absent_root_child_directories_missing(
                root_id,
                &observed_relative_paths,
                observed_at,
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(())
        })?;

        Ok(RootNavigationWindowEstablishment {
            root_id,
            state: RootNavigationWindowEstablishmentState::Established,
            immediate_child_directory_count: child_directories.len(),
            detail: None,
        })
    }

    fn root_navigation_window_establishment_required(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<bool> {
        let connection = self.open_read_connection()?;
        let scan_phase = connection
            .query_row(
                "SELECT scan_phase
                 FROM source_scan_state
                 WHERE source_id = ?1",
                [root_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if !matches!(scan_phase.as_deref(), None | Some("idle")) {
            return Ok(false);
        }

        let has_immediate_root_directory_rows = connection.query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM source_directories
                 WHERE source_id = ?1
                   AND parent_source_directory_id IS NULL
                   AND relative_path <> ''
             )",
            [root_id],
            |row| row.get::<_, bool>(0),
        )?;
        Ok(!has_immediate_root_directory_rows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImmediateRootChildDirectory {
    name: String,
    relative_path: String,
    mtime_ns: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootNavigationWindowReadFailure {
    state: RootNavigationWindowEstablishmentState,
    detail: String,
}

fn collect_immediate_root_child_directories(
    root_path: &Path,
    _observed_at: i64,
) -> Result<Vec<ImmediateRootChildDirectory>, RootNavigationWindowReadFailure> {
    let entries = std::fs::read_dir(root_path).map_err(root_window_read_failure_from_io)?;
    let mut child_directories = Vec::new();

    for entry in entries {
        let entry = entry.map_err(root_window_read_failure_from_io)?;
        let file_type = entry
            .file_type()
            .map_err(root_window_read_failure_from_io)?;
        if !file_type.is_dir() {
            continue;
        }

        let name = entry
            .file_name()
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| RootNavigationWindowReadFailure {
                state: RootNavigationWindowEstablishmentState::Failed,
                detail: format!("non-UTF-8 child directory name under {root_path:?}"),
            })?;
        if name.is_empty() {
            continue;
        }

        let mtime_ns = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_nanos()).ok());
        child_directories.push(ImmediateRootChildDirectory {
            relative_path: name.clone(),
            name,
            mtime_ns,
        });
    }

    child_directories.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(child_directories)
}

fn root_window_read_failure_from_io(error: std::io::Error) -> RootNavigationWindowReadFailure {
    let issue_kind = source_access_issue_kind_from_io_error(&error);
    RootNavigationWindowReadFailure {
        state: root_establishment_state_for_issue(issue_kind),
        detail: error.to_string(),
    }
}

fn root_establishment_from_access_probe(
    root_id: i64,
    probe: &SourceAccessProbeResult,
) -> RootNavigationWindowEstablishment {
    match probe {
        SourceAccessProbeResult::Accessible { .. } => RootNavigationWindowEstablishment {
            root_id,
            state: RootNavigationWindowEstablishmentState::Established,
            immediate_child_directory_count: 0,
            detail: None,
        },
        SourceAccessProbeResult::Missing {
            diagnostic_detail, ..
        } => RootNavigationWindowEstablishment {
            root_id,
            state: RootNavigationWindowEstablishmentState::Missing,
            immediate_child_directory_count: 0,
            detail: diagnostic_detail.clone(),
        },
        SourceAccessProbeResult::Blocked {
            issue_kind,
            diagnostic_detail,
            ..
        } => RootNavigationWindowEstablishment {
            root_id,
            state: root_establishment_state_for_issue(*issue_kind),
            immediate_child_directory_count: 0,
            detail: diagnostic_detail.clone(),
        },
    }
}

fn root_establishment_state_for_issue(
    issue_kind: SourceAccessIssueKind,
) -> RootNavigationWindowEstablishmentState {
    match issue_kind {
        SourceAccessIssueKind::Missing => RootNavigationWindowEstablishmentState::Missing,
        SourceAccessIssueKind::PermissionDenied
        | SourceAccessIssueKind::PrivacyPermissionRequired
        | SourceAccessIssueKind::UnavailableMount
        | SourceAccessIssueKind::ResourceBusy
        | SourceAccessIssueKind::StaleNetworkHandle
        | SourceAccessIssueKind::SymlinkLoop
        | SourceAccessIssueKind::SymlinkEscapeBlocked
        | SourceAccessIssueKind::UnsupportedPath
        | SourceAccessIssueKind::NotDirectory => RootNavigationWindowEstablishmentState::Blocked,
        SourceAccessIssueKind::InvalidPath
        | SourceAccessIssueKind::IoInterrupted
        | SourceAccessIssueKind::TimedOut
        | SourceAccessIssueKind::UnknownIo => RootNavigationWindowEstablishmentState::Failed,
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
