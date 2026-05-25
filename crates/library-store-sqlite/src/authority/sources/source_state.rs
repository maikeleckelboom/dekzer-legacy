use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{SourceAccessIssueKind, SourceAccessState, SourceScanPhase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceStateInput {
    pub source_id: i64,
    pub mount_status: String,
    pub mount_epoch: i64,
    pub access_state: SourceAccessState,
    pub access_issue_kind: Option<SourceAccessIssueKind>,
    pub access_error_detail: Option<String>,
    pub access_checked_at: Option<i64>,
    pub mount_root: Option<String>,
    pub effective_path: Option<String>,
    pub observed_volume_label: Option<String>,
    pub filesystem_type: Option<String>,
    pub last_seen_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceScanStateInput {
    pub source_id: i64,
    pub scan_phase: SourceScanPhase,
    pub last_scan_started_at: Option<i64>,
    pub last_scan_finished_at: Option<i64>,
    pub last_successful_scan_at: Option<i64>,
    pub scan_issue_kind: Option<SourceAccessIssueKind>,
    pub error_detail: Option<String>,
    pub updated_at: i64,
}

pub struct SourceStateAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceStateAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source_state(&self, input: &UpsertSourceStateInput) -> LibrarySqliteResult<()> {
        self.tx.execute(
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
                input.source_id,
                input.mount_status,
                input.mount_epoch,
                input.access_state.as_str(),
                input.access_issue_kind.map(SourceAccessIssueKind::as_str),
                input.access_error_detail.as_deref(),
                input.access_checked_at,
                input.mount_root.as_deref(),
                input.effective_path.as_deref(),
                input.observed_volume_label.as_deref(),
                input.filesystem_type.as_deref(),
                input.last_seen_at,
                input.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_source_scan_state(
        &self,
        input: &UpsertSourceScanStateInput,
    ) -> LibrarySqliteResult<()> {
        self.tx.execute(
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
                input.source_id,
                input.scan_phase.as_str(),
                input.last_scan_started_at,
                input.last_scan_finished_at,
                input.last_successful_scan_at,
                input.scan_issue_kind.map(SourceAccessIssueKind::as_str),
                input.error_detail.as_deref(),
                input.updated_at,
            ],
        )?;
        Ok(())
    }
}
