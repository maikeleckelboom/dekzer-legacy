use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{SourceResolutionStatus, SourceScanPhase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceStateInput {
    pub source_id: i64,
    pub mount_status: String,
    pub mount_epoch: i64,
    pub resolution_status: SourceResolutionStatus,
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
    pub blocked_reason: Option<String>,
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
                 resolution_status,
                 mount_root,
                 effective_path,
                 observed_volume_label,
                 filesystem_type,
                 last_seen_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(source_id) DO UPDATE
             SET mount_status = excluded.mount_status,
                 mount_epoch = excluded.mount_epoch,
                 resolution_status = excluded.resolution_status,
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
                input.resolution_status.as_str(),
                input.mount_root,
                input.effective_path,
                input.observed_volume_label,
                input.filesystem_type,
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
                 blocked_reason,
                 error_detail,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(source_id) DO UPDATE
             SET scan_phase = excluded.scan_phase,
                 last_scan_started_at = excluded.last_scan_started_at,
                 last_scan_finished_at = excluded.last_scan_finished_at,
                 last_successful_scan_at = excluded.last_successful_scan_at,
                 blocked_reason = excluded.blocked_reason,
                 error_detail = excluded.error_detail,
                 updated_at = excluded.updated_at",
            params![
                input.source_id,
                input.scan_phase.as_str(),
                input.last_scan_started_at,
                input.last_scan_finished_at,
                input.last_successful_scan_at,
                input.blocked_reason,
                input.error_detail,
                input.updated_at,
            ],
        )?;
        Ok(())
    }
}
