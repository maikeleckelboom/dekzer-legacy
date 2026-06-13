use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::authority::sources::format_source_identity_key;
use crate::authority::sources::{SourceAccessProbeResult, probe_source_access};
use crate::authority::write_lane::AdmittedWrite;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::SourceAccessIssueKind;

const ROOT_IDENTITY_KIND_DEGRADED_ENROLLMENT: &str = "degraded_enrollment";
const ERROR_DETAIL_SCAN_FAILED: &str = "scan_failed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub root_id: i64,
    pub canonical_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalSourcePath {
    path: PathBuf,
    path_text: String,
}

impl CanonicalSourcePath {
    pub(crate) fn resolve(path: &Path) -> LibrarySqliteResult<Self> {
        let canonical_path = std::fs::canonicalize(path).map_err(|source| {
            LibrarySqliteError::RootPathCanonicalization {
                path: path.to_path_buf(),
                source,
            }
        })?;
        let path_text = canonical_path
            .to_str()
            .ok_or_else(|| LibrarySqliteError::NonUtf8CanonicalRootPath(canonical_path.clone()))?
            .to_owned();

        Ok(Self {
            path: canonical_path,
            path_text,
        })
    }

    fn as_text(&self) -> &str {
        &self.path_text
    }

    fn identity_kind(&self) -> &'static str {
        ROOT_IDENTITY_KIND_DEGRADED_ENROLLMENT
    }

    fn identity_value(&self) -> &str {
        self.as_text()
    }

    fn resolved_root(&self, root_id: i64) -> ResolvedRoot {
        ResolvedRoot {
            root_id,
            canonical_path: self.path.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootIdentityKind {
    WindowsVolumeGuid,
    FilesystemUuid,
    MacVolumeUuid,
    DeviceSerialPartition,
}

impl RootIdentityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WindowsVolumeGuid => "windows_volume_guid",
            Self::FilesystemUuid => "filesystem_uuid",
            Self::MacVolumeUuid => "mac_volume_uuid",
            Self::DeviceSerialPartition => "device_serial_partition",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMountStatus {
    Unknown,
    Mounted,
    Unmounted,
    EjectRequested,
    EjectPending,
}

impl RootMountStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Mounted => "mounted",
            Self::Unmounted => "unmounted",
            Self::EjectRequested => "eject_requested",
            Self::EjectPending => "eject_pending",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAccessState {
    Unknown,
    Accessible,
    Missing,
    Blocked,
}

impl RootAccessState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Accessible => "accessible",
            Self::Missing => "missing",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootScanPhase {
    Idle,
    Scanning,
    Complete,
    Partial,
    BlockedUnavailable,
    InterruptedUnavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterRemovableRootInput {
    pub root_identity_kind: String,
    pub root_identity_value: String,
    pub label: Option<String>,
    pub locator_identity_kind: RootIdentityKind,
    pub locator_identity_value: String,
    pub relative_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootMountedInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub mount_root: PathBuf,
    pub observed_volume_label: Option<String>,
    pub filesystem_type: Option<String>,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootUnmountRequestedInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootEjectCancelledInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootUnmountPendingInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootUnmountedInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRootChangedInput {
    pub identity_kind: RootIdentityKind,
    pub identity_value: String,
    pub mount_root: Option<PathBuf>,
    pub observed_volume_label: Option<String>,
    pub filesystem_type: Option<String>,
    pub event_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootStatusDelta {
    pub root_id: i64,
    pub old_mount_status: RootMountStatus,
    pub new_mount_status: RootMountStatus,
    pub old_access_state: RootAccessState,
    pub new_access_state: RootAccessState,
    pub old_mount_epoch: i64,
    pub new_mount_epoch: i64,
    pub old_effective_path: Option<PathBuf>,
    pub new_effective_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceLocatorKind {
    AbsolutePath,
    RemovableVolume,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLocatorRecord {
    root_id: i64,
    locator_kind: SourceLocatorKind,
    absolute_path: Option<String>,
    identity_kind: Option<String>,
    identity_value: Option<String>,
    relative_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceStateRecord {
    root_id: i64,
    mount_status: RootMountStatus,
    access_state: RootAccessState,
    access_issue_kind: Option<SourceAccessIssueKind>,
    access_error_detail: Option<String>,
    access_checked_at: Option<i64>,
    mount_epoch: i64,
    mount_root: Option<String>,
    effective_path: Option<String>,
    observed_volume_label: Option<String>,
    filesystem_type: Option<String>,
    last_seen_at: Option<i64>,
    scan_phase: RootScanPhase,
    last_scan_started_at: Option<i64>,
    last_scan_finished_at: Option<i64>,
    last_successful_scan_at: Option<i64>,
    scan_issue_kind: Option<SourceAccessIssueKind>,
    error_detail: Option<String>,
    updated_at: i64,
}

impl SourceStateRecord {
    fn delta_to(&self, next: &Self) -> RootStatusDelta {
        RootStatusDelta {
            root_id: self.root_id,
            old_mount_status: self.mount_status,
            new_mount_status: next.mount_status,
            old_access_state: self.access_state,
            new_access_state: next.access_state,
            old_mount_epoch: self.mount_epoch,
            new_mount_epoch: next.mount_epoch,
            old_effective_path: self.effective_path.as_ref().map(PathBuf::from),
            new_effective_path: next.effective_path.as_ref().map(PathBuf::from),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MatchedSourceRecord {
    locator: SourceLocatorRecord,
    state: SourceStateRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingSourceIdentity {
    source_id: i64,
    display_name: String,
}

pub(crate) struct SourceLifecycleTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceLifecycleTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write mut AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn bootstrap_root(
        &mut self,
        canonical_path: &CanonicalSourcePath,
    ) -> LibrarySqliteResult<ResolvedRoot> {
        let root_id = self.upsert_source(
            canonical_path.identity_kind(),
            canonical_path.identity_value(),
            None,
            "internal",
            "system",
        )?;
        self.upsert_absolute_path_locator(root_id, canonical_path.as_text())?;
        self.initialize_source_state_if_missing(
            root_id,
            SourceLocatorKind::AbsolutePath,
            canonical_path.as_text(),
        )?;
        self.refresh_absolute_path_root(root_id, unix_time_ms()?)?;
        Ok(canonical_path.resolved_root(root_id))
    }

    pub(crate) fn register_removable_root(
        &mut self,
        input: &RegisterRemovableRootInput,
    ) -> LibrarySqliteResult<i64> {
        let root_id = self.upsert_source(
            input.root_identity_kind.as_str(),
            input.root_identity_value.as_str(),
            input.label.as_deref(),
            "removable_mounted",
            "device",
        )?;
        self.upsert_removable_locator(root_id, input)?;
        self.initialize_source_state_if_missing(root_id, SourceLocatorKind::RemovableVolume, "")?;
        Ok(root_id)
    }

    pub(crate) fn mark_root_scan_started(
        &mut self,
        root_id: i64,
        started_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        let mut next = record.state.clone();
        next.scan_phase = RootScanPhase::Scanning;
        next.last_scan_started_at = Some(started_at_ms);
        next.scan_issue_kind = None;
        next.error_detail = None;
        next.updated_at = started_at_ms;
        self.write_source_state(&next)
    }

    pub(crate) fn mark_root_scan_failed(
        &mut self,
        root_id: i64,
        failed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        let mut next = record.state.clone();
        next.scan_phase = RootScanPhase::Failed;
        next.last_scan_finished_at = Some(failed_at_ms);
        next.scan_issue_kind = Some(SourceAccessIssueKind::UnknownIo);
        next.error_detail = Some(ERROR_DETAIL_SCAN_FAILED.to_string());
        next.updated_at = failed_at_ms;
        self.write_source_state(&next)
    }

    pub(crate) fn mark_root_scan_completed(
        &mut self,
        root_id: i64,
        completed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        let mut next = record.state.clone();
        if next.scan_phase == RootScanPhase::Partial {
            next.last_scan_finished_at = Some(
                next.last_scan_finished_at
                    .unwrap_or(completed_at_ms)
                    .max(completed_at_ms),
            );
        } else {
            next.scan_phase = RootScanPhase::Complete;
            next.last_scan_finished_at = Some(
                next.last_scan_finished_at
                    .unwrap_or(completed_at_ms)
                    .max(completed_at_ms),
            );
            next.last_successful_scan_at = Some(
                next.last_successful_scan_at
                    .unwrap_or(completed_at_ms)
                    .max(completed_at_ms),
            );
            next.scan_issue_kind = None;
            next.error_detail = None;
        }
        next.updated_at = completed_at_ms;
        self.write_source_state(&next)
    }

    pub(crate) fn mark_root_scan_blocked(
        &mut self,
        root_id: i64,
        blocked_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        let mut next = record.state.clone();
        next.scan_phase = RootScanPhase::BlockedUnavailable;
        next.scan_issue_kind = Some(SourceAccessIssueKind::UnavailableMount);
        next.error_detail = None;
        next.updated_at = blocked_at_ms;
        self.write_source_state(&next)
    }

    pub(crate) fn apply_root_access_probe_result(
        &mut self,
        root_id: i64,
        probe: &SourceAccessProbeResult,
    ) -> LibrarySqliteResult<()> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        let (access_state, access_issue_kind, access_error_detail, access_checked_at) =
            source_access_fields(probe);
        let mut next = record.state.clone();
        next.access_state = access_state;
        next.access_issue_kind = access_issue_kind;
        next.access_error_detail = access_error_detail;
        next.access_checked_at = access_checked_at;
        if let SourceAccessProbeResult::Accessible { effective_root, .. } = probe {
            next.effective_path = Some(path_to_text(effective_root));
            next.last_seen_at = Some(probe.checked_at_ms());
        } else {
            next.scan_phase = RootScanPhase::BlockedUnavailable;
            next.scan_issue_kind = probe.issue_kind();
            next.error_detail = probe.diagnostic_detail().map(str::to_string);
        }
        next.updated_at = probe.checked_at_ms();
        self.write_source_state(&next)
    }

    pub(crate) fn interrupt_root_bound_work(
        &mut self,
        root_ids: &[i64],
        interrupted_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        for root_id in root_ids {
            let Some(record) = self.load_source_by_id(*root_id)? else {
                continue;
            };
            let mut next = record.state.clone();
            next.scan_phase = if next.scan_phase == RootScanPhase::Scanning {
                RootScanPhase::InterruptedUnavailable
            } else {
                RootScanPhase::BlockedUnavailable
            };
            next.scan_issue_kind = Some(SourceAccessIssueKind::IoInterrupted);
            next.error_detail = None;
            next.updated_at = interrupted_at_ms;
            self.write_source_state(&next)?;
        }
        Ok(())
    }

    pub(crate) fn resume_root_bound_work(
        &mut self,
        root_ids: &[i64],
        resumed_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        for root_id in root_ids {
            let Some(record) = self.load_source_by_id(*root_id)? else {
                continue;
            };
            if !self.source_is_runnable(record.state.root_id)? {
                continue;
            }
            let mut next = record.state.clone();
            if matches!(
                next.scan_phase,
                RootScanPhase::BlockedUnavailable | RootScanPhase::InterruptedUnavailable
            ) {
                next.scan_phase = RootScanPhase::Idle;
                next.scan_issue_kind = None;
                next.error_detail = None;
                next.updated_at = resumed_at_ms;
                self.write_source_state(&next)?;
            }
        }
        Ok(())
    }

    pub(crate) fn refresh_absolute_path_root(
        &mut self,
        root_id: i64,
        refreshed_at_ms: i64,
    ) -> LibrarySqliteResult<bool> {
        let matched = self
            .load_source_by_id(root_id)?
            .ok_or(LibrarySqliteError::MissingRoot(root_id))?;
        if matched.locator.locator_kind != SourceLocatorKind::AbsolutePath {
            return Ok(false);
        }

        let absolute_path = matched.locator.absolute_path.clone().unwrap_or_default();
        let access_probe = probe_source_access(Path::new(&absolute_path), refreshed_at_ms);
        let last_seen_at = if access_probe.is_accessible() {
            Some(refreshed_at_ms)
        } else {
            matched.state.last_seen_at
        };
        let (access_state, access_issue_kind, access_error_detail, access_checked_at) =
            source_access_fields(&access_probe);
        let next = SourceStateRecord {
            root_id,
            mount_status: RootMountStatus::Mounted,
            access_state,
            access_issue_kind,
            access_error_detail,
            access_checked_at,
            mount_epoch: 0,
            mount_root: None,
            effective_path: Some(absolute_path),
            observed_volume_label: matched.state.observed_volume_label.clone(),
            filesystem_type: matched.state.filesystem_type.clone(),
            last_seen_at,
            scan_phase: matched.state.scan_phase,
            last_scan_started_at: matched.state.last_scan_started_at,
            last_scan_finished_at: matched.state.last_scan_finished_at,
            last_successful_scan_at: matched.state.last_successful_scan_at,
            scan_issue_kind: matched.state.scan_issue_kind,
            error_detail: matched.state.error_detail.clone(),
            updated_at: refreshed_at_ms,
        };
        if next == matched.state {
            return Ok(false);
        }

        self.write_source_state(&next)?;
        Ok(true)
    }

    pub(crate) fn apply_root_mounted(
        &mut self,
        input: ApplyRootMountedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let matched = self.load_matched_sources(&input.identity_kind, &input.identity_value)?;
        let mount_root_text = path_to_text(&input.mount_root);
        let mut deltas = Vec::new();

        for record in matched {
            let effective_path = Some(compute_effective_path_text(
                &record.locator,
                &input.mount_root,
            ));
            let access_probe = probe_source_access(
                Path::new(effective_path.as_deref().unwrap_or_default()),
                input.event_at_ms,
            );
            let (access_state, access_issue_kind, access_error_detail, access_checked_at) =
                source_access_fields(&access_probe);
            let next = SourceStateRecord {
                root_id: record.state.root_id,
                mount_status: RootMountStatus::Mounted,
                access_state,
                access_issue_kind,
                access_error_detail,
                access_checked_at,
                mount_epoch: next_mount_epoch(
                    record.state.mount_epoch,
                    record.locator.locator_kind,
                    record.state.mount_status != RootMountStatus::Mounted
                        || record.state.mount_root.as_deref() != Some(mount_root_text.as_str()),
                ),
                mount_root: Some(mount_root_text.clone()),
                effective_path,
                observed_volume_label: input.observed_volume_label.clone(),
                filesystem_type: input.filesystem_type.clone(),
                last_seen_at: Some(input.event_at_ms),
                scan_phase: record.state.scan_phase,
                last_scan_started_at: record.state.last_scan_started_at,
                last_scan_finished_at: record.state.last_scan_finished_at,
                last_successful_scan_at: record.state.last_successful_scan_at,
                scan_issue_kind: record.state.scan_issue_kind,
                error_detail: record.state.error_detail.clone(),
                updated_at: input.event_at_ms,
            };
            if next == record.state {
                continue;
            }

            self.write_source_state(&next)?;
            deltas.push(record.state.delta_to(&next));
        }

        Ok(deltas)
    }

    pub(crate) fn apply_root_unmount_requested(
        &mut self,
        input: ApplyRootUnmountRequestedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        self.apply_source_presence_transition(
            input.identity_kind,
            input.identity_value,
            |state, _| {
                if matches!(
                    state.mount_status,
                    RootMountStatus::Unmounted
                        | RootMountStatus::EjectPending
                        | RootMountStatus::EjectRequested
                ) {
                    return None;
                }
                let mut next = state.clone();
                next.mount_status = RootMountStatus::EjectRequested;
                Some(next)
            },
            input.event_at_ms,
        )
    }

    pub(crate) fn apply_root_eject_cancelled(
        &mut self,
        input: ApplyRootEjectCancelledInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        self.apply_source_presence_transition(
            input.identity_kind,
            input.identity_value,
            |state, _| {
                if state.mount_status != RootMountStatus::EjectRequested {
                    return None;
                }
                let mut next = state.clone();
                next.mount_status = RootMountStatus::Mounted;
                Some(next)
            },
            input.event_at_ms,
        )
    }

    pub(crate) fn apply_root_unmount_pending(
        &mut self,
        input: ApplyRootUnmountPendingInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        self.apply_source_presence_transition(
            input.identity_kind,
            input.identity_value,
            |state, _| {
                if matches!(
                    state.mount_status,
                    RootMountStatus::Unmounted | RootMountStatus::EjectPending
                ) {
                    return None;
                }
                let mut next = state.clone();
                next.mount_status = RootMountStatus::EjectPending;
                Some(next)
            },
            input.event_at_ms,
        )
    }

    pub(crate) fn apply_root_unmounted(
        &mut self,
        input: ApplyRootUnmountedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        self.apply_source_presence_transition(
            input.identity_kind,
            input.identity_value,
            |state, locator| {
                if state.mount_status == RootMountStatus::Unmounted
                    || locator.locator_kind == SourceLocatorKind::AbsolutePath
                {
                    return None;
                }
                let mut next = state.clone();
                next.mount_status = RootMountStatus::Unmounted;
                next.access_state = RootAccessState::Unknown;
                next.access_issue_kind = Some(SourceAccessIssueKind::UnavailableMount);
                next.access_error_detail = None;
                next.access_checked_at = Some(input.event_at_ms);
                next.mount_epoch = state.mount_epoch + 1;
                next.mount_root = None;
                next.effective_path = None;
                Some(next)
            },
            input.event_at_ms,
        )
    }

    pub(crate) fn apply_root_changed(
        &mut self,
        input: ApplyRootChangedInput,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>> {
        let matched = self.load_matched_sources(&input.identity_kind, &input.identity_value)?;
        let mut deltas = Vec::new();

        for record in matched {
            if record.state.mount_status == RootMountStatus::Unmounted {
                continue;
            }

            let next_mount_root = input
                .mount_root
                .as_ref()
                .map(|mount_root| path_to_text(mount_root))
                .or_else(|| record.state.mount_root.clone());
            let mount_root_changed = next_mount_root != record.state.mount_root;
            let next_effective_path = if mount_root_changed {
                next_mount_root
                    .as_ref()
                    .map(|mount_root| compute_effective_path_from_text(&record.locator, mount_root))
            } else {
                record.state.effective_path.clone()
            };
            let access_probe = if mount_root_changed {
                next_effective_path
                    .as_deref()
                    .map(|path| probe_source_access(Path::new(path), input.event_at_ms))
            } else {
                None
            };
            let (
                next_access_state,
                next_access_issue_kind,
                next_access_error_detail,
                next_access_checked_at,
            ) = access_probe.as_ref().map(source_access_fields).unwrap_or((
                record.state.access_state,
                record.state.access_issue_kind,
                record.state.access_error_detail.clone(),
                record.state.access_checked_at,
            ));
            let next = SourceStateRecord {
                root_id: record.state.root_id,
                mount_status: record.state.mount_status,
                access_state: next_access_state,
                access_issue_kind: next_access_issue_kind,
                access_error_detail: next_access_error_detail,
                access_checked_at: next_access_checked_at,
                mount_epoch: record.state.mount_epoch,
                mount_root: next_mount_root,
                effective_path: next_effective_path,
                observed_volume_label: input
                    .observed_volume_label
                    .clone()
                    .or_else(|| record.state.observed_volume_label.clone()),
                filesystem_type: input
                    .filesystem_type
                    .clone()
                    .or_else(|| record.state.filesystem_type.clone()),
                last_seen_at: record.state.last_seen_at,
                scan_phase: record.state.scan_phase,
                last_scan_started_at: record.state.last_scan_started_at,
                last_scan_finished_at: record.state.last_scan_finished_at,
                last_successful_scan_at: record.state.last_successful_scan_at,
                scan_issue_kind: record.state.scan_issue_kind,
                error_detail: record.state.error_detail.clone(),
                updated_at: input.event_at_ms,
            };
            if next == record.state {
                continue;
            }

            self.write_source_state(&next)?;
            deltas.push(record.state.delta_to(&next));
        }

        Ok(deltas)
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }

    fn upsert_source(
        &self,
        identity_kind: &str,
        identity_value: &str,
        label: Option<&str>,
        source_class: &str,
        authority: &str,
    ) -> LibrarySqliteResult<i64> {
        let existing = self.load_existing_source_identity(identity_kind, identity_value)?;
        let changed_at = unix_time_ms()?;
        let display_name = label
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(identity_value)
            .to_string();
        let identity_key = format_source_identity_key(identity_kind, identity_value);

        if let Some(existing) = existing {
            self.tx().execute(
                "UPDATE sources
                 SET source_class = ?2,
                     authority = ?3,
                     display_name = ?4,
                     is_user_visible = 1,
                     updated_at = ?5
                 WHERE source_id = ?1",
                params![
                    existing.source_id,
                    source_class,
                    authority,
                    label
                        .filter(|value| !value.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or(existing.display_name),
                    changed_at,
                ],
            )?;
            return Ok(existing.source_id);
        }

        let source_navigation_order = self.next_source_order()?;
        self.tx().execute(
            "INSERT INTO sources (
                 source_class,
                 authority,
                 identity_key,
                 display_name,
                 medium_label,
                 is_user_visible,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, NULL, 1, ?5, ?5)",
            params![
                source_class,
                authority,
                identity_key,
                display_name,
                changed_at,
            ],
        )?;
        let source_id = self.tx().last_insert_rowid();
        self.tx().execute(
            "INSERT INTO source_navigation_user_order (
                 node_domain,
                 node_id,
                 parent_scope,
                 ordinal,
                 created_at,
                 updated_at
             )
             VALUES ('source', ?1, NULL, ?2, ?3, ?3)",
            params![source_id.to_string(), source_navigation_order, changed_at],
        )?;
        Ok(source_id)
    }

    fn load_existing_source_identity(
        &self,
        identity_kind: &str,
        identity_value: &str,
    ) -> LibrarySqliteResult<Option<ExistingSourceIdentity>> {
        self.tx()
            .query_row(
                "SELECT source_id, display_name
                 FROM sources
                 WHERE identity_key = ?1",
                [format_source_identity_key(identity_kind, identity_value)],
                |row| {
                    Ok(ExistingSourceIdentity {
                        source_id: row.get(0)?,
                        display_name: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn next_source_order(&self) -> LibrarySqliteResult<i64> {
        self.tx()
            .query_row(
                "SELECT COALESCE(MAX(ordinal), -1) + 1
                 FROM source_navigation_user_order
                 WHERE node_domain = 'source'
                   AND parent_scope IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn upsert_absolute_path_locator(
        &self,
        root_id: i64,
        absolute_path: &str,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO source_locators (
                 source_id,
                 locator_kind,
                 absolute_path,
                 device_identity_kind,
                 device_identity_value,
                 relative_suffix
             )
             VALUES (?1, 'absolute_path', ?2, NULL, NULL, '')
             ON CONFLICT(source_id) DO UPDATE
             SET locator_kind = excluded.locator_kind,
                 absolute_path = excluded.absolute_path,
                 device_identity_kind = excluded.device_identity_kind,
                 device_identity_value = excluded.device_identity_value,
                 relative_suffix = excluded.relative_suffix",
            params![root_id, absolute_path],
        )?;
        Ok(())
    }

    fn upsert_removable_locator(
        &self,
        root_id: i64,
        input: &RegisterRemovableRootInput,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO source_locators (
                 source_id,
                 locator_kind,
                 absolute_path,
                 device_identity_kind,
                 device_identity_value,
                 relative_suffix
             )
             VALUES (?1, 'removable_volume', NULL, ?2, ?3, ?4)
             ON CONFLICT(source_id) DO UPDATE
             SET locator_kind = excluded.locator_kind,
                 absolute_path = excluded.absolute_path,
                 device_identity_kind = excluded.device_identity_kind,
                 device_identity_value = excluded.device_identity_value,
                 relative_suffix = excluded.relative_suffix",
            params![
                root_id,
                input.locator_identity_kind.as_str(),
                input.locator_identity_value,
                input.relative_suffix,
            ],
        )?;
        Ok(())
    }

    fn initialize_source_state_if_missing(
        &self,
        root_id: i64,
        locator_kind: SourceLocatorKind,
        absolute_path: &str,
    ) -> LibrarySqliteResult<bool> {
        let now_ms = unix_time_ms()?;
        let (
            mount_status,
            access_state,
            access_issue_kind,
            access_error_detail,
            access_checked_at,
            effective_path,
            last_seen_at,
        ) = match locator_kind {
            SourceLocatorKind::AbsolutePath => {
                let access_probe = probe_source_access(Path::new(absolute_path), now_ms);
                let last_seen_at = if access_probe.is_accessible() {
                    Some(now_ms)
                } else {
                    None
                };
                let (access_state, access_issue_kind, access_error_detail, access_checked_at) =
                    source_access_fields(&access_probe);
                (
                    RootMountStatus::Mounted,
                    access_state,
                    access_issue_kind,
                    access_error_detail,
                    access_checked_at,
                    Some(absolute_path.to_string()),
                    last_seen_at,
                )
            }
            SourceLocatorKind::RemovableVolume => (
                RootMountStatus::Unknown,
                RootAccessState::Unknown,
                None,
                None,
                None,
                None,
                None,
            ),
        };
        let inserted = self.tx().execute(
            "INSERT INTO source_state (
                 source_id,
                 mount_status,
                 mount_epoch,
                 access_state,
                 access_issue_kind,
                 access_error_detail,
                 access_checked_at,
                 mount_root,
                 effective_path,
                 observed_volume_label,
                 filesystem_type,
                 last_seen_at,
                 updated_at
             )
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, NULL, ?7, NULL, NULL, ?8, ?9)
             ON CONFLICT(source_id) DO NOTHING",
            params![
                root_id,
                mount_status.as_str(),
                access_state.as_str(),
                access_issue_kind.map(SourceAccessIssueKind::as_str),
                access_error_detail,
                access_checked_at,
                effective_path,
                last_seen_at,
                now_ms,
            ],
        )?;
        self.tx().execute(
            "INSERT INTO source_scan_state (
                 source_id,
                 scan_phase,
                 last_scan_started_at,
                 last_scan_finished_at,
                 last_successful_scan_at,
                 scan_issue_kind,
                 error_detail,
                 updated_at
             )
             VALUES (?1, 'idle', NULL, NULL, NULL, NULL, NULL, ?2)
             ON CONFLICT(source_id) DO NOTHING",
            params![root_id, now_ms],
        )?;
        Ok(inserted > 0)
    }

    fn write_source_state(&self, next: &SourceStateRecord) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO source_state (
                 source_id,
                 mount_status,
                 mount_epoch,
                 access_state,
                 access_issue_kind,
                 access_error_detail,
                 access_checked_at,
                 mount_root,
                 effective_path,
                 observed_volume_label,
                 filesystem_type,
                 last_seen_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(source_id) DO UPDATE
             SET mount_status = excluded.mount_status,
                 mount_epoch = excluded.mount_epoch,
                 access_state = excluded.access_state,
                 access_issue_kind = excluded.access_issue_kind,
                 access_error_detail = excluded.access_error_detail,
                 access_checked_at = excluded.access_checked_at,
                 mount_root = excluded.mount_root,
                 effective_path = excluded.effective_path,
                 observed_volume_label = excluded.observed_volume_label,
                 filesystem_type = excluded.filesystem_type,
                 last_seen_at = excluded.last_seen_at,
                 updated_at = excluded.updated_at",
            params![
                next.root_id,
                next.mount_status.as_str(),
                next.mount_epoch,
                next.access_state.as_str(),
                next.access_issue_kind.map(SourceAccessIssueKind::as_str),
                next.access_error_detail.as_deref(),
                next.access_checked_at,
                next.mount_root.as_deref(),
                next.effective_path.as_deref(),
                next.observed_volume_label.as_deref(),
                next.filesystem_type.as_deref(),
                next.last_seen_at,
                next.updated_at,
            ],
        )?;
        self.tx().execute(
            "INSERT INTO source_scan_state (
                 source_id,
                 scan_phase,
                 last_scan_started_at,
                 last_scan_finished_at,
                 last_successful_scan_at,
                 scan_issue_kind,
                 error_detail,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(source_id) DO UPDATE
             SET scan_phase = excluded.scan_phase,
                 last_scan_started_at = excluded.last_scan_started_at,
                 last_scan_finished_at = excluded.last_scan_finished_at,
                 last_successful_scan_at = excluded.last_successful_scan_at,
                 scan_issue_kind = excluded.scan_issue_kind,
                 error_detail = excluded.error_detail,
                 updated_at = excluded.updated_at",
            params![
                next.root_id,
                source_scan_phase_value(next.scan_phase),
                next.last_scan_started_at,
                next.last_scan_finished_at,
                next.last_successful_scan_at,
                scan_issue_kind_value(next.scan_phase, next.scan_issue_kind),
                error_detail_value(next.scan_phase, next.error_detail.as_deref()),
                next.updated_at,
            ],
        )?;
        Ok(())
    }

    fn source_is_runnable(&self, root_id: i64) -> LibrarySqliteResult<bool> {
        let Some(record) = self.load_source_by_id(root_id)? else {
            return Err(LibrarySqliteError::MissingRoot(root_id));
        };
        Ok(record.state.mount_status == RootMountStatus::Mounted
            && record.state.access_state == RootAccessState::Accessible)
    }

    fn apply_source_presence_transition<F>(
        &mut self,
        identity_kind: RootIdentityKind,
        identity_value: String,
        mut transition: F,
        event_at_ms: i64,
    ) -> LibrarySqliteResult<Vec<RootStatusDelta>>
    where
        F: FnMut(&SourceStateRecord, &SourceLocatorRecord) -> Option<SourceStateRecord>,
    {
        let matched = self.load_matched_sources(&identity_kind, &identity_value)?;
        let mut deltas = Vec::new();

        for record in matched {
            let Some(mut next) = transition(&record.state, &record.locator) else {
                continue;
            };
            next.updated_at = event_at_ms;
            if next == record.state {
                continue;
            }

            self.write_source_state(&next)?;
            deltas.push(record.state.delta_to(&next));
        }

        Ok(deltas)
    }

    fn load_matched_sources(
        &self,
        identity_kind: &RootIdentityKind,
        identity_value: &str,
    ) -> LibrarySqliteResult<Vec<MatchedSourceRecord>> {
        let mut stmt = self.tx().prepare(
            "SELECT sl.source_id,
                    sl.locator_kind,
                    sl.absolute_path,
                    sl.device_identity_kind,
                    sl.device_identity_value,
                    sl.relative_suffix,
                    lss.mount_status,
                    lss.access_state,
                    lss.access_issue_kind,
                    lss.access_error_detail,
                    lss.access_checked_at,
                    lss.mount_epoch,
                    lss.mount_root,
                    lss.effective_path,
                    lss.observed_volume_label,
                    lss.filesystem_type,
                    lss.last_seen_at,
                    sss.scan_phase,
                    sss.last_scan_started_at,
                    sss.last_scan_finished_at,
                    sss.last_successful_scan_at,
                    sss.scan_issue_kind,
                    sss.error_detail,
                    CASE
                        WHEN sss.updated_at > lss.updated_at THEN sss.updated_at
                        ELSE lss.updated_at
                    END AS updated_at
             FROM source_locators sl
             JOIN source_state lss
               ON lss.source_id = sl.source_id
             JOIN source_scan_state sss
               ON sss.source_id = sl.source_id
             WHERE sl.device_identity_kind = ?1
               AND sl.device_identity_value = ?2
              ORDER BY sl.source_id ASC",
        )?;
        let rows = stmt.query_map(
            params![identity_kind.as_str(), identity_value],
            load_matched_source,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn load_source_by_id(&self, root_id: i64) -> LibrarySqliteResult<Option<MatchedSourceRecord>> {
        self.tx()
            .query_row(
                "SELECT sl.source_id,
                        sl.locator_kind,
                        sl.absolute_path,
                        sl.device_identity_kind,
                        sl.device_identity_value,
                        sl.relative_suffix,
                        lss.mount_status,
                        lss.access_state,
                        lss.access_issue_kind,
                        lss.access_error_detail,
                        lss.access_checked_at,
                        lss.mount_epoch,
                        lss.mount_root,
                        lss.effective_path,
                        lss.observed_volume_label,
                        lss.filesystem_type,
                        lss.last_seen_at,
                        sss.scan_phase,
                        sss.last_scan_started_at,
                        sss.last_scan_finished_at,
                        sss.last_successful_scan_at,
                        sss.scan_issue_kind,
                        sss.error_detail,
                        CASE
                            WHEN sss.updated_at > lss.updated_at THEN sss.updated_at
                            ELSE lss.updated_at
                        END AS updated_at
                 FROM source_locators sl
                 JOIN source_state lss
                   ON lss.source_id = sl.source_id
                 JOIN source_scan_state sss
                   ON sss.source_id = sl.source_id
                 WHERE sl.source_id = ?1",
                [root_id],
                load_matched_source,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn load_matched_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<MatchedSourceRecord> {
    let mount_status_text: String = row.get(6)?;
    let access_state_text: String = row.get(7)?;
    let access_issue_kind_text: Option<String> = row.get(8)?;
    let scan_phase_text: String = row.get(17)?;
    let scan_issue_kind_text: Option<String> = row.get(21)?;
    Ok(MatchedSourceRecord {
        locator: SourceLocatorRecord {
            root_id: row.get(0)?,
            locator_kind: parse_locator_kind(&row.get::<_, String>(1)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
            absolute_path: row.get(2)?,
            identity_kind: row.get(3)?,
            identity_value: row.get(4)?,
            relative_suffix: row.get(5)?,
        },
        state: SourceStateRecord {
            root_id: row.get(0)?,
            mount_status: parse_mount_status(&mount_status_text).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
            access_state: parse_access_state(&access_state_text).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
            access_issue_kind: parse_optional_source_access_issue_kind(
                "source_state.access_issue_kind",
                access_issue_kind_text.as_deref(),
                8,
            )?,
            access_error_detail: row.get(9)?,
            access_checked_at: row.get(10)?,
            mount_epoch: row.get(11)?,
            mount_root: row.get(12)?,
            effective_path: row.get(13)?,
            observed_volume_label: row.get(14)?,
            filesystem_type: row.get(15)?,
            last_seen_at: row.get(16)?,
            scan_phase: parse_scan_phase(&scan_phase_text, scan_issue_kind_text.as_deref())
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        17,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
            last_scan_started_at: row.get(18)?,
            last_scan_finished_at: row.get(19)?,
            last_successful_scan_at: row.get(20)?,
            scan_issue_kind: parse_optional_source_access_issue_kind(
                "source_scan_state.scan_issue_kind",
                scan_issue_kind_text.as_deref(),
                21,
            )?,
            error_detail: row.get(22)?,
            updated_at: row.get(23)?,
        },
    })
}

fn parse_locator_kind(value: &str) -> LibrarySqliteResult<SourceLocatorKind> {
    match value {
        "absolute_path" => Ok(SourceLocatorKind::AbsolutePath),
        "removable_volume" => Ok(SourceLocatorKind::RemovableVolume),
        other => Err(malformed_value("source_locators.locator_kind", other)),
    }
}

fn parse_mount_status(value: &str) -> LibrarySqliteResult<RootMountStatus> {
    match value {
        "unknown" => Ok(RootMountStatus::Unknown),
        "mounted" => Ok(RootMountStatus::Mounted),
        "unmounted" => Ok(RootMountStatus::Unmounted),
        "eject_requested" => Ok(RootMountStatus::EjectRequested),
        "eject_pending" => Ok(RootMountStatus::EjectPending),
        other => Err(malformed_value("source_state.mount_status", other)),
    }
}

fn parse_access_state(value: &str) -> LibrarySqliteResult<RootAccessState> {
    match value {
        "unknown" => Ok(RootAccessState::Unknown),
        "accessible" => Ok(RootAccessState::Accessible),
        "missing" => Ok(RootAccessState::Missing),
        "blocked" => Ok(RootAccessState::Blocked),
        other => Err(malformed_value("source_state.access_state", other)),
    }
}

fn parse_scan_phase(
    value: &str,
    scan_issue_kind: Option<&str>,
) -> LibrarySqliteResult<RootScanPhase> {
    match value {
        "idle" => Ok(RootScanPhase::Idle),
        "scanning" => Ok(RootScanPhase::Scanning),
        "complete" => Ok(RootScanPhase::Complete),
        "partial" => Ok(RootScanPhase::Partial),
        "blocked" => Ok(
            if scan_issue_kind == Some(SourceAccessIssueKind::IoInterrupted.as_str()) {
                RootScanPhase::InterruptedUnavailable
            } else {
                RootScanPhase::BlockedUnavailable
            },
        ),
        "failed" => Ok(RootScanPhase::Failed),
        other => Err(malformed_value("source_scan_state.scan_phase", other)),
    }
}

fn parse_optional_source_access_issue_kind(
    column: &str,
    value: Option<&str>,
    column_index: usize,
) -> rusqlite::Result<Option<SourceAccessIssueKind>> {
    value
        .map(|text| {
            SourceAccessIssueKind::parse(text).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    column_index,
                    rusqlite::types::Type::Text,
                    Box::new(malformed_value(column, text)),
                )
            })
        })
        .transpose()
}

fn source_scan_phase_value(phase: RootScanPhase) -> &'static str {
    match phase {
        RootScanPhase::Idle => "idle",
        RootScanPhase::Scanning => "scanning",
        RootScanPhase::Complete => "complete",
        RootScanPhase::Partial => "partial",
        RootScanPhase::BlockedUnavailable | RootScanPhase::InterruptedUnavailable => "blocked",
        RootScanPhase::Failed => "failed",
    }
}

fn scan_issue_kind_value(
    phase: RootScanPhase,
    current: Option<SourceAccessIssueKind>,
) -> Option<&'static str> {
    match phase {
        RootScanPhase::BlockedUnavailable => Some(
            current
                .unwrap_or(SourceAccessIssueKind::UnavailableMount)
                .as_str(),
        ),
        RootScanPhase::InterruptedUnavailable => Some(
            current
                .unwrap_or(SourceAccessIssueKind::IoInterrupted)
                .as_str(),
        ),
        RootScanPhase::Failed => Some(current.unwrap_or(SourceAccessIssueKind::UnknownIo).as_str()),
        RootScanPhase::Partial => {
            Some(current.unwrap_or(SourceAccessIssueKind::UnknownIo).as_str())
        }
        _ => None,
    }
}

fn error_detail_value(phase: RootScanPhase, current: Option<&str>) -> Option<String> {
    match phase {
        RootScanPhase::Failed => Some(current.unwrap_or(ERROR_DETAIL_SCAN_FAILED).to_string()),
        _ => None,
    }
}

fn availability_state_from_mount_and_access(
    mount_status: RootMountStatus,
    access_state: RootAccessState,
) -> &'static str {
    match mount_status {
        RootMountStatus::Unknown | RootMountStatus::Unmounted => "unavailable",
        RootMountStatus::Mounted => {
            if access_state == RootAccessState::Accessible {
                "available"
            } else {
                "degraded"
            }
        }
        RootMountStatus::EjectRequested | RootMountStatus::EjectPending => "degraded",
    }
}

fn malformed_value(column: &str, value: &str) -> LibrarySqliteError {
    LibrarySqliteError::MalformedSchemaState(format!("unexpected value {value:?} for {column}"))
}

fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn compute_effective_path_text(locator: &SourceLocatorRecord, mount_root: &Path) -> String {
    match locator.locator_kind {
        SourceLocatorKind::AbsolutePath => locator.absolute_path.clone().unwrap_or_default(),
        SourceLocatorKind::RemovableVolume => {
            if locator.relative_suffix.is_empty() {
                path_to_text(mount_root)
            } else {
                path_to_text(&mount_root.join(Path::new(&locator.relative_suffix)))
            }
        }
    }
}

fn compute_effective_path_from_text(locator: &SourceLocatorRecord, mount_root: &str) -> String {
    compute_effective_path_text(locator, Path::new(mount_root))
}

fn source_access_fields(
    probe: &SourceAccessProbeResult,
) -> (
    RootAccessState,
    Option<SourceAccessIssueKind>,
    Option<String>,
    Option<i64>,
) {
    match probe {
        SourceAccessProbeResult::Accessible { checked_at_ms, .. } => (
            RootAccessState::Accessible,
            None,
            None,
            Some(*checked_at_ms),
        ),
        SourceAccessProbeResult::Missing {
            issue_kind,
            diagnostic_detail,
            checked_at_ms,
        } => (
            RootAccessState::Missing,
            Some(*issue_kind),
            diagnostic_detail.clone(),
            Some(*checked_at_ms),
        ),
        SourceAccessProbeResult::Blocked {
            issue_kind,
            diagnostic_detail,
            checked_at_ms,
        } => (
            RootAccessState::Blocked,
            Some(*issue_kind),
            diagnostic_detail.clone(),
            Some(*checked_at_ms),
        ),
    }
}

fn next_mount_epoch(
    current_mount_epoch: i64,
    locator_kind: SourceLocatorKind,
    increment: bool,
) -> i64 {
    match locator_kind {
        SourceLocatorKind::AbsolutePath => 0,
        SourceLocatorKind::RemovableVolume => {
            if increment {
                current_mount_epoch + 1
            } else {
                current_mount_epoch
            }
        }
    }
}
