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
    SourceLocationsAuthorityTx, SourceLocatorsAuthorityTx, SourceRootNavigationStateAuthorityTx,
    SourceRootNavigationWindowState, SourceStateAuthorityTx, SourcesAuthorityTx,
    UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocationInput,
    UpsertSourceLocatorInput, UpsertSourceRootNavigationStateInput, UpsertSourceScanStateInput,
    UpsertSourceStateInput, read_source_root_navigation_state,
};
use crate::authority::work::{QueueInspectSourceFileWorkInput, WorkItemAuthorityTx};
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
    pub requested_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRegistrationRootClass {
    NormalMusicRoot,
    BroadDriveRoot,
    SystemVolumeRoot,
    UserProfileRoot,
    CloudBackedRoot,
    NetworkRoot,
    ProtectedRoot,
    IndirectionRoot,
    UnknownRoot,
}

impl SourceRegistrationRootClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NormalMusicRoot => "normal_music_root",
            Self::BroadDriveRoot => "broad_drive_root",
            Self::SystemVolumeRoot => "system_volume_root",
            Self::UserProfileRoot => "user_profile_root",
            Self::CloudBackedRoot => "cloud_backed_root",
            Self::NetworkRoot => "network_root",
            Self::ProtectedRoot => "protected_root",
            Self::IndirectionRoot => "indirection_root",
            Self::UnknownRoot => "unknown_root",
        }
    }

    fn parse(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "normal_music_root" => Ok(Self::NormalMusicRoot),
            "broad_drive_root" => Ok(Self::BroadDriveRoot),
            "system_volume_root" => Ok(Self::SystemVolumeRoot),
            "user_profile_root" => Ok(Self::UserProfileRoot),
            "cloud_backed_root" => Ok(Self::CloudBackedRoot),
            "network_root" => Ok(Self::NetworkRoot),
            "protected_root" => Ok(Self::ProtectedRoot),
            "indirection_root" => Ok(Self::IndirectionRoot),
            "unknown_root" => Ok(Self::UnknownRoot),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "unexpected value {other:?} for root_admission_proposals.root_class"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRegistrationProposal {
    pub proposal_id: i64,
    pub root_class: SourceRegistrationRootClass,
    pub requested_path: PathBuf,
    pub resolved_path: Option<PathBuf>,
    pub confirmation_required_reason: String,
    pub suggested_root_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRegistrationRejection {
    pub root_class: SourceRegistrationRootClass,
    pub requested_path: PathBuf,
    pub resolved_path: Option<PathBuf>,
    pub rejection_reason: String,
    pub suggested_root_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterLocalRootResult {
    Registered(LocalRoot),
    ProposalRequired(SourceRegistrationProposal),
    Rejected(SourceRegistrationRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRoot {
    pub root_id: i64,
    pub admitted_root_path: PathBuf,
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
    Empty,
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
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(())
        })
    }

    pub fn upsert_source_scan_state(
        &self,
        input: UpsertSourceScanStateInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            SourceStateAuthorityTx::new(write).upsert_source_scan_state(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
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
            queue_inspect_source_file_work_for_observation(
                write,
                source_file_id,
                &input.relative_path,
                input.presence_state,
                input.size_bytes,
                input.mtime_ns,
                input.updated_at,
            )?;
            Ok(source_file_id)
        })
    }

    pub fn register_local_root(
        &self,
        input: RegisterLocalRootInput,
    ) -> LibrarySqliteResult<RegisterLocalRootResult> {
        match classify_source_registration_root(&input.requested_path) {
            SourceRegistrationAdmission::AdmitNormal { resolved_path } => {
                let resolved_root = self.bootstrap_root(&resolved_path)?;
                let _ = self.establish_root_navigation_window(resolved_root.root_id)?;
                Ok(RegisterLocalRootResult::Registered(LocalRoot {
                    root_id: resolved_root.root_id,
                    admitted_root_path: resolved_root.admitted_root_path,
                    availability: LocalRootAvailability::Available,
                }))
            }
            SourceRegistrationAdmission::RequireProposal {
                root_class,
                requested_path,
                resolved_path,
                confirmation_required_reason,
                suggested_roots,
            } => self
                .record_or_read_source_registration_proposal(
                    root_class,
                    requested_path,
                    resolved_path,
                    confirmation_required_reason,
                    suggested_roots,
                )
                .map(RegisterLocalRootResult::ProposalRequired),
            SourceRegistrationAdmission::Reject {
                root_class,
                requested_path,
                resolved_path,
                rejection_reason,
                suggested_roots,
            } => Ok(RegisterLocalRootResult::Rejected(
                SourceRegistrationRejection {
                    root_class,
                    requested_path,
                    resolved_path,
                    rejection_reason,
                    suggested_root_paths: suggested_roots,
                },
            )),
        }
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
                    admitted_root_path: PathBuf::from(row.get::<_, String>(1)?),
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

    fn record_or_read_source_registration_proposal(
        &self,
        root_class: SourceRegistrationRootClass,
        requested_path: PathBuf,
        resolved_path: Option<PathBuf>,
        confirmation_required_reason: String,
        suggested_roots: Vec<PathBuf>,
    ) -> LibrarySqliteResult<SourceRegistrationProposal> {
        let resolved_key = proposal_resolved_key(&requested_path, resolved_path.as_deref());
        let requested_path_text = path_to_text(&requested_path);
        let resolved_path_text = resolved_path.as_ref().map(|path| path_to_text(path));
        let suggested_roots_json = serde_json::to_string(
            &suggested_roots
                .iter()
                .map(|path| path_to_text(path))
                .collect::<Vec<_>>(),
        )
        .map_err(|error| {
            LibrarySqliteError::WriteInvariant(format!(
                "failed to encode source registration proposal suggestions: {error}"
            ))
        })?;

        self.with_write(|write| {
            if let Some(existing) =
                read_proposed_source_registration_proposal(write, &resolved_key)?
            {
                return Ok(existing);
            }

            let now_ms = unix_time_ms()?;
            write.execute(
                "INSERT INTO root_admission_proposals (
                     proposal_status,
                     root_class,
                     requested_path,
                     resolved_path,
                     confirmation_required_reason,
                     suggested_roots_json,
                     created_at,
                     updated_at
                 )
                 VALUES ('proposed', ?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                rusqlite::params![
                    root_class.as_str(),
                    requested_path_text,
                    resolved_path_text.as_deref(),
                    confirmation_required_reason,
                    suggested_roots_json,
                    now_ms,
                ],
            )?;
            let proposal_id = write.last_insert_rowid();
            Ok(SourceRegistrationProposal {
                proposal_id,
                root_class,
                requested_path,
                resolved_path,
                confirmation_required_reason,
                suggested_root_paths: suggested_roots,
            })
        })
    }

    pub fn classify_local_root_for_registration(
        path: impl AsRef<Path>,
    ) -> SourceRegistrationRootClass {
        match classify_source_registration_root(path.as_ref()) {
            SourceRegistrationAdmission::AdmitNormal { .. } => {
                SourceRegistrationRootClass::NormalMusicRoot
            }
            SourceRegistrationAdmission::RequireProposal { root_class, .. }
            | SourceRegistrationAdmission::Reject { root_class, .. } => root_class,
        }
    }

    pub fn root_scan_admission_class(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<SourceRegistrationRootClass> {
        let path = self.read_root_scan_path(root_id)?;
        Ok(Self::classify_local_root_for_registration(path))
    }

    #[allow(dead_code)]
    pub(crate) fn bootstrap_root(
        &self,
        path: impl AsRef<Path>,
    ) -> LibrarySqliteResult<ResolvedRoot> {
        let resolved_path = CanonicalSourcePath::resolve(path.as_ref())?;
        let resolved_root =
            self.with_source_lifecycle_tx(|tx| tx.bootstrap_root(&resolved_path))?;
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
        if let Some(establishment) = self.root_navigation_window_establishment_gate(root_id)? {
            return Ok(establishment);
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
                self.write_root_navigation_state_from_access_probe(root_id, &access_probe)?;
                self.sync_root_projection_state(&[root_id])?;
                return Ok(root_establishment_from_access_probe(root_id, &access_probe));
            }
        };

        let observed_at = unix_time_ms()?;
        let child_directories =
            match collect_immediate_root_child_directories(&effective_root, observed_at) {
                Ok(child_directories) => child_directories,
                Err(failure) => {
                    self.write_root_navigation_state(RootNavigationStateWrite {
                        root_id,
                        root_reach_state: root_navigation_window_state_for_establishment(
                            failure.state,
                        ),
                        immediate_child_directory_count: 0,
                        issue_kind: Some(failure.issue_kind),
                        detail: Some(failure.detail.clone()),
                        checked_at: Some(checked_at_ms),
                        updated_at: observed_at,
                    })?;
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
            SourceRootNavigationStateAuthorityTx::new(write).upsert_source_root_navigation_state(
                &UpsertSourceRootNavigationStateInput {
                    source_id: root_id,
                    root_reach_state: if child_directories.is_empty() {
                        SourceRootNavigationWindowState::Empty
                    } else {
                        SourceRootNavigationWindowState::Established
                    },
                    immediate_child_directory_count: i64::try_from(child_directories.len())
                        .map_err(|_| {
                            LibrarySqliteError::WriteInvariant(
                                "immediate root child directory count does not fit i64".to_string(),
                            )
                        })?,
                    issue_kind: None,
                    detail: None,
                    checked_at: Some(checked_at_ms),
                    updated_at: observed_at,
                },
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(())
        })?;

        Ok(RootNavigationWindowEstablishment {
            root_id,
            state: if child_directories.is_empty() {
                RootNavigationWindowEstablishmentState::Empty
            } else {
                RootNavigationWindowEstablishmentState::Established
            },
            immediate_child_directory_count: child_directories.len(),
            detail: None,
        })
    }

    fn root_navigation_window_establishment_gate(
        &self,
        root_id: i64,
    ) -> LibrarySqliteResult<Option<RootNavigationWindowEstablishment>> {
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
            return Ok(Some(RootNavigationWindowEstablishment {
                root_id,
                state: RootNavigationWindowEstablishmentState::NotRequired,
                immediate_child_directory_count: 0,
                detail: None,
            }));
        }

        let Some(root_navigation_state) = read_source_root_navigation_state(&connection, root_id)?
        else {
            return Ok(None);
        };
        if matches!(
            root_navigation_state.root_reach_state,
            SourceRootNavigationWindowState::Unknown
        ) {
            return Ok(None);
        }

        Ok(Some(root_establishment_from_root_navigation_state(
            root_navigation_state,
        )?))
    }

    fn write_root_navigation_state_from_access_probe(
        &self,
        root_id: i64,
        access_probe: &SourceAccessProbeResult,
    ) -> LibrarySqliteResult<()> {
        let issue_kind = access_probe.issue_kind();
        let Some(issue_kind) = issue_kind else {
            return Ok(());
        };
        self.write_root_navigation_state(RootNavigationStateWrite {
            root_id,
            root_reach_state: root_navigation_window_state_for_establishment(
                root_establishment_state_for_issue(issue_kind),
            ),
            immediate_child_directory_count: 0,
            issue_kind: Some(issue_kind),
            detail: access_probe.diagnostic_detail().map(str::to_string),
            checked_at: Some(access_probe.checked_at_ms()),
            updated_at: access_probe.checked_at_ms(),
        })
    }

    fn write_root_navigation_state(
        &self,
        input: RootNavigationStateWrite,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            SourceRootNavigationStateAuthorityTx::new(write).upsert_source_root_navigation_state(
                &UpsertSourceRootNavigationStateInput {
                    source_id: input.root_id,
                    root_reach_state: input.root_reach_state,
                    immediate_child_directory_count: input.immediate_child_directory_count,
                    issue_kind: input.issue_kind,
                    detail: input.detail,
                    checked_at: input.checked_at,
                    updated_at: input.updated_at,
                },
            )?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(())
        })
    }
}

struct RootNavigationStateWrite {
    root_id: i64,
    root_reach_state: SourceRootNavigationWindowState,
    immediate_child_directory_count: i64,
    issue_kind: Option<SourceAccessIssueKind>,
    detail: Option<String>,
    checked_at: Option<i64>,
    updated_at: i64,
}

fn root_establishment_from_root_navigation_state(
    state: crate::authority::sources::SourceRootNavigationStateRecord,
) -> LibrarySqliteResult<RootNavigationWindowEstablishment> {
    let immediate_child_directory_count =
        usize::try_from(state.immediate_child_directory_count).map_err(|_| {
            LibrarySqliteError::MalformedSchemaState(format!(
                "source_root_navigation_state.immediate_child_directory_count does not fit usize: {}",
                state.immediate_child_directory_count
            ))
        })?;
    Ok(RootNavigationWindowEstablishment {
        root_id: state.source_id,
        state: match state.root_reach_state {
            SourceRootNavigationWindowState::Established => {
                RootNavigationWindowEstablishmentState::Established
            }
            SourceRootNavigationWindowState::Empty => RootNavigationWindowEstablishmentState::Empty,
            SourceRootNavigationWindowState::Missing => {
                RootNavigationWindowEstablishmentState::Missing
            }
            SourceRootNavigationWindowState::Blocked => {
                RootNavigationWindowEstablishmentState::Blocked
            }
            SourceRootNavigationWindowState::Failed => {
                RootNavigationWindowEstablishmentState::Failed
            }
            SourceRootNavigationWindowState::Unknown => {
                RootNavigationWindowEstablishmentState::NotRequired
            }
        },
        immediate_child_directory_count,
        detail: state.detail,
    })
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
    issue_kind: SourceAccessIssueKind,
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
                issue_kind: SourceAccessIssueKind::InvalidPath,
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
        issue_kind,
        detail: error.to_string(),
    }
}

fn root_navigation_window_state_for_establishment(
    state: RootNavigationWindowEstablishmentState,
) -> SourceRootNavigationWindowState {
    match state {
        RootNavigationWindowEstablishmentState::Established => {
            SourceRootNavigationWindowState::Established
        }
        RootNavigationWindowEstablishmentState::Empty => SourceRootNavigationWindowState::Empty,
        RootNavigationWindowEstablishmentState::Missing => SourceRootNavigationWindowState::Missing,
        RootNavigationWindowEstablishmentState::Blocked => SourceRootNavigationWindowState::Blocked,
        RootNavigationWindowEstablishmentState::Failed => SourceRootNavigationWindowState::Failed,
        RootNavigationWindowEstablishmentState::NotRequired => {
            SourceRootNavigationWindowState::Unknown
        }
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

fn queue_inspect_source_file_work_for_observation(
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

    let result = WorkItemAuthorityTx::new(write).queue_inspect_source_file_work(
        &QueueInspectSourceFileWorkInput {
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceRegistrationAdmission {
    AdmitNormal {
        resolved_path: PathBuf,
    },
    RequireProposal {
        root_class: SourceRegistrationRootClass,
        requested_path: PathBuf,
        resolved_path: Option<PathBuf>,
        confirmation_required_reason: String,
        suggested_roots: Vec<PathBuf>,
    },
    Reject {
        root_class: SourceRegistrationRootClass,
        requested_path: PathBuf,
        resolved_path: Option<PathBuf>,
        rejection_reason: String,
        suggested_roots: Vec<PathBuf>,
    },
}

fn classify_source_registration_root(path: &Path) -> SourceRegistrationAdmission {
    let requested_path = path.to_path_buf();
    let resolved_path = std::fs::canonicalize(path).ok();
    let classification_path = resolved_path.as_deref().unwrap_or(path);
    let suggested_roots = deterministic_suggested_music_roots();

    if is_indirection_root(path) {
        if resolved_path
            .as_deref()
            .is_some_and(is_protected_windows_location)
        {
            return SourceRegistrationAdmission::Reject {
                root_class: SourceRegistrationRootClass::ProtectedRoot,
                requested_path,
                resolved_path,
                rejection_reason:
                    "indirection roots that resolve to protected locations cannot be registered"
                        .to_string(),
                suggested_roots,
            };
        }

        return SourceRegistrationAdmission::RequireProposal {
            root_class: SourceRegistrationRootClass::IndirectionRoot,
            requested_path,
            resolved_path,
            confirmation_required_reason:
                "indirection roots require a later scan-plan confirmation before admission"
                    .to_string(),
            suggested_roots,
        };
    }

    if is_protected_windows_location(classification_path) {
        return SourceRegistrationAdmission::Reject {
            root_class: SourceRegistrationRootClass::ProtectedRoot,
            requested_path,
            resolved_path,
            rejection_reason: "protected roots cannot be registered as sources".to_string(),
            suggested_roots,
        };
    }

    if is_network_root(classification_path) {
        return proposal_required(
            SourceRegistrationRootClass::NetworkRoot,
            requested_path,
            resolved_path,
            "network roots require a later latency-aware scan-plan confirmation",
            suggested_roots,
        );
    }

    if is_cloud_backed_root(classification_path) {
        return proposal_required(
            SourceRegistrationRootClass::CloudBackedRoot,
            requested_path,
            resolved_path,
            "cloud-backed roots require a later provider-aware scan-plan confirmation",
            suggested_roots,
        );
    }

    if is_user_profile_root(classification_path) {
        return proposal_required(
            SourceRegistrationRootClass::UserProfileRoot,
            requested_path,
            resolved_path,
            "user profile roots require a later scan-plan confirmation",
            suggested_roots,
        );
    }

    if is_system_drive_root(classification_path) {
        let resolved_path = resolved_path.or_else(|| normalized_windows_drive_root(path));
        return proposal_required(
            SourceRegistrationRootClass::SystemVolumeRoot,
            requested_path,
            resolved_path,
            "system volume roots require a later scan-plan confirmation",
            suggested_roots,
        );
    }

    if is_windows_drive_root(classification_path) {
        let resolved_path = resolved_path.or_else(|| normalized_windows_drive_root(path));
        return proposal_required(
            SourceRegistrationRootClass::BroadDriveRoot,
            requested_path,
            resolved_path,
            "drive roots require a later broad-root scan-plan confirmation",
            suggested_roots,
        );
    }

    if is_unknown_risky_root(classification_path) {
        return proposal_required(
            SourceRegistrationRootClass::UnknownRoot,
            requested_path,
            resolved_path,
            "unknown root selections require a later scan-plan confirmation",
            suggested_roots,
        );
    }

    match resolved_path {
        Some(resolved_path) => SourceRegistrationAdmission::AdmitNormal { resolved_path },
        None => SourceRegistrationAdmission::Reject {
            root_class: SourceRegistrationRootClass::UnknownRoot,
            requested_path,
            resolved_path: None,
            rejection_reason: "root path could not be safely canonicalized for registration"
                .to_string(),
            suggested_roots,
        },
    }
}

fn proposal_required(
    root_class: SourceRegistrationRootClass,
    requested_path: PathBuf,
    resolved_path: Option<PathBuf>,
    confirmation_required_reason: &str,
    suggested_roots: Vec<PathBuf>,
) -> SourceRegistrationAdmission {
    SourceRegistrationAdmission::RequireProposal {
        root_class,
        requested_path,
        resolved_path,
        confirmation_required_reason: confirmation_required_reason.to_string(),
        suggested_roots,
    }
}

fn read_proposed_source_registration_proposal(
    connection: &rusqlite::Connection,
    resolved_key: &str,
) -> LibrarySqliteResult<Option<SourceRegistrationProposal>> {
    connection
        .query_row(
            "SELECT root_admission_proposal_id,
                    root_class,
                    requested_path,
                    resolved_path,
                    confirmation_required_reason,
                    suggested_roots_json
             FROM root_admission_proposals
             WHERE proposal_status = 'proposed'
               AND COALESCE(resolved_path, requested_path) = ?1",
            [resolved_key],
            |row| {
                let root_class_text: String = row.get(1)?;
                let requested_path: String = row.get(2)?;
                let resolved_path: Option<String> = row.get(3)?;
                let suggested_roots_json: String = row.get(5)?;
                let suggested_roots = serde_json::from_str::<Vec<String>>(&suggested_roots_json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>();

                let root_class =
                    SourceRegistrationRootClass::parse(&root_class_text).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;

                Ok(SourceRegistrationProposal {
                    proposal_id: row.get(0)?,
                    root_class,
                    requested_path: PathBuf::from(requested_path),
                    resolved_path: resolved_path.map(PathBuf::from),
                    confirmation_required_reason: row.get(4)?,
                    suggested_root_paths: suggested_roots,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn proposal_resolved_key(requested_path: &Path, resolved_path: Option<&Path>) -> String {
    path_to_text(resolved_path.unwrap_or(requested_path))
}

fn is_indirection_root(path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn is_network_root(path: &Path) -> bool {
    let text = comparable_path_text(path);
    text.starts_with("//") && !text.starts_with("//?/")
}

fn is_cloud_backed_root(path: &Path) -> bool {
    let path_text = comparable_path_text(path);
    for var_name in [
        "OneDrive",
        "OneDriveConsumer",
        "OneDriveCommercial",
        "Dropbox",
        "iCloudDrive",
    ] {
        let Ok(value) = std::env::var(var_name) else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        let cloud_root = comparable_path_text(Path::new(&value));
        if path_text == cloud_root || path_text.starts_with(&(cloud_root + "/")) {
            return true;
        }
    }
    false
}

fn is_user_profile_root(path: &Path) -> bool {
    let Ok(user_profile) = std::env::var("USERPROFILE") else {
        return false;
    };
    if user_profile.trim().is_empty() {
        return false;
    }
    comparable_path_text(path) == comparable_path_text(Path::new(&user_profile))
}

fn is_system_drive_root(path: &Path) -> bool {
    let Some(drive_letter) = windows_drive_root_letter(path) else {
        return false;
    };
    let system_drive = std::env::var("SystemDrive")
        .ok()
        .and_then(|value| value.chars().find(|c| c.is_ascii_alphabetic()))
        .unwrap_or('C')
        .to_ascii_uppercase();
    drive_letter == system_drive && is_windows_drive_root(path)
}

fn is_windows_drive_root(path: &Path) -> bool {
    windows_drive_root_letter(path).is_some()
}

fn windows_drive_root_letter(path: &Path) -> Option<char> {
    let text = comparable_path_text(path);
    let bytes = text.as_bytes();
    if bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Some((bytes[0] as char).to_ascii_uppercase());
    }
    if bytes.len() == 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        return Some((bytes[0] as char).to_ascii_uppercase());
    }
    None
}

fn normalized_windows_drive_root(path: &Path) -> Option<PathBuf> {
    windows_drive_root_letter(path).map(|letter| PathBuf::from(format!("{letter}:\\")))
}

fn is_unknown_risky_root(path: &Path) -> bool {
    comparable_path_text(path) == "/"
}

fn is_protected_windows_location(path: &Path) -> bool {
    let text = comparable_path_text(path);
    let without_drive = if text.len() >= 3
        && text.as_bytes()[0].is_ascii_alphabetic()
        && text.as_bytes()[1] == b':'
        && text.as_bytes()[2] == b'/'
    {
        &text[3..]
    } else {
        text.as_str()
    };
    let first_segment = without_drive.split('/').find(|segment| !segment.is_empty());
    matches!(
        first_segment,
        Some(
            "windows"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "system volume information"
                | "recovery"
                | "windowsapps"
        )
    ) || without_drive
        .split('/')
        .any(|segment| segment == "windowsapps")
}

fn deterministic_suggested_music_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(user_profile) = std::env::var("USERPROFILE")
        && !user_profile.trim().is_empty()
    {
        roots.push(Path::new(&user_profile).join("Music"));
    }
    roots
}

fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn comparable_path_text(path: &Path) -> String {
    let mut text = path_to_text(path).replace('\\', "/");
    if let Some(stripped) = text.strip_prefix("//?/") {
        text = stripped.to_string();
    }
    while text.len() > 3 && text.ends_with('/') {
        text.pop();
    }
    text.to_ascii_lowercase()
}
