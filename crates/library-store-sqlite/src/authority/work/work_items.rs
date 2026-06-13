use rusqlite::{OptionalExtension, params};

use crate::authority::work::work_runs::{
    load_latest_work_run_for_work_item, load_open_work_run_for_work_item,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    MachineWorkKind, ProjectionDomain, SourceFileId, WorkItemId, WorkItemState, WorkPriorityClass,
    WorkRunOutcome, WorkSubject, WorkSubjectKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineWorkKey {
    pub subject: WorkSubject,
    pub work_kind: MachineWorkKind,
    pub basis_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueMachineWorkInput {
    pub key: MachineWorkKey,
    pub priority_class: WorkPriorityClass,
    pub queued_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueInspectSourceWorkInput {
    pub source_file_id: SourceFileId,
    pub basis_fingerprint: String,
    pub priority_class: WorkPriorityClass,
    pub queued_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRebuildProjectionWorkInput {
    pub projection_domain: ProjectionDomain,
    pub basis_fingerprint: String,
    pub priority_class: WorkPriorityClass,
    pub queued_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueMachineWorkResult {
    pub work_item_id: WorkItemId,
    pub created: bool,
    pub state: WorkItemState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimMachineWorkBatchInput {
    pub limit: usize,
    pub lease_duration_ms: i64,
    pub claimed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaimSpecificMachineWorkInput {
    pub work_item_id: WorkItemId,
    pub lease_duration_ms: i64,
    pub claimed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedMachineWorkItem {
    pub work_item_id: WorkItemId,
    pub subject: WorkSubject,
    pub work_kind: MachineWorkKind,
    pub priority_class: WorkPriorityClass,
    pub basis_fingerprint: String,
    pub state: WorkItemState,
    pub leased_until: Option<i64>,
    pub attempt_count: i64,
    pub blocked_reason: Option<String>,
    pub failure_kind: Option<String>,
    pub error_detail: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteMachineWorkInput {
    pub work_item_id: WorkItemId,
    pub completed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailMachineWorkInput {
    pub work_item_id: WorkItemId,
    pub failed_at: i64,
    pub failure_kind: String,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMachineWorkInput {
    pub work_item_id: WorkItemId,
    pub blocked_at: i64,
    pub blocked_reason: String,
    pub error_detail: Option<String>,
}

pub struct WorkItemsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> WorkItemsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn queue_machine_work(
        &self,
        input: &QueueMachineWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        validate_work_key(&input.key)?;

        if let Some((work_item_id, state)) = self.find_active_work_item(&input.key)? {
            let next_state = if state == WorkItemState::Blocked {
                WorkItemState::Queued
            } else {
                state
            };
            self.tx.execute(
                "UPDATE WorkItems
                 SET priority_class = ?2,
                     state = ?3,
                     leased_until = CASE WHEN ?3 = 'queued' THEN NULL ELSE leased_until END,
                     blocked_reason = CASE WHEN ?3 = 'queued' THEN NULL ELSE blocked_reason END,
                     failure_kind = CASE WHEN ?3 = 'queued' THEN NULL ELSE failure_kind END,
                     error_detail = CASE WHEN ?3 = 'queued' THEN NULL ELSE error_detail END,
                     updated_at = ?4
                 WHERE work_item_id = ?1",
                params![
                    work_item_id.get(),
                    input.priority_class.as_str(),
                    next_state.as_str(),
                    input.queued_at,
                ],
            )?;
            return Ok(QueueMachineWorkResult {
                work_item_id,
                created: false,
                state: next_state,
            });
        }

        let subject_id = input.key.subject.storage_id();
        self.tx.execute(
            "INSERT INTO WorkItems (
                 subject_kind,
                 subject_id,
                 work_kind,
                 priority_class,
                 basis_fingerprint,
                 state,
                 leased_until,
                 attempt_count,
                 blocked_reason,
                 failure_kind,
                 error_detail,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'queued', NULL, 0, NULL, NULL, NULL, ?6, ?6)",
            params![
                input.key.subject.kind().as_str(),
                subject_id.as_str(),
                input.key.work_kind.as_str(),
                input.priority_class.as_str(),
                input.key.basis_fingerprint.as_str(),
                input.queued_at,
            ],
        )?;

        Ok(QueueMachineWorkResult {
            work_item_id: parse_work_item_id(self.tx.last_insert_rowid())?,
            created: true,
            state: WorkItemState::Queued,
        })
    }

    pub fn queue_inspect_source_work(
        &self,
        input: &QueueInspectSourceWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.queue_machine_work(&QueueMachineWorkInput {
            key: MachineWorkKey {
                subject: WorkSubject::SourceFile(input.source_file_id),
                work_kind: MachineWorkKind::InspectSource,
                basis_fingerprint: input.basis_fingerprint.clone(),
            },
            priority_class: input.priority_class,
            queued_at: input.queued_at,
        })
    }

    pub fn queue_rebuild_projection_work(
        &self,
        input: &QueueRebuildProjectionWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.queue_machine_work(&QueueMachineWorkInput {
            key: MachineWorkKey {
                subject: WorkSubject::ProjectionDomain(input.projection_domain),
                work_kind: MachineWorkKind::RebuildProjection,
                basis_fingerprint: input.basis_fingerprint.clone(),
            },
            priority_class: input.priority_class,
            queued_at: input.queued_at,
        })
    }

    pub fn claim_machine_work_batch(
        &self,
        input: &ClaimMachineWorkBatchInput,
    ) -> LibrarySqliteResult<Vec<ClaimedMachineWorkItem>> {
        if input.limit == 0 {
            return Ok(Vec::new());
        }

        let lease_until = input.claimed_at + input.lease_duration_ms;
        let claimed_ids = self
            .tx
            .prepare(
                "SELECT work_item_id
                 FROM WorkItems
                 WHERE state = 'queued'
                    OR (state = 'leased' AND leased_until IS NOT NULL AND leased_until <= ?1)
                 ORDER BY CASE priority_class
                              WHEN 'urgent' THEN 0
                              WHEN 'interactive' THEN 1
                              ELSE 2
                          END,
                          created_at ASC,
                          work_item_id ASC
                 LIMIT ?2",
            )?
            .query_map(params![input.claimed_at, input.limit as i64], |row| {
                row.get(0)
            })?
            .collect::<Result<Vec<i64>, _>>()?;

        let mut claimed = Vec::with_capacity(claimed_ids.len());
        for work_item_id in claimed_ids {
            self.tx.execute(
                "UPDATE WorkItems
                 SET state = 'leased',
                     leased_until = ?2,
                     attempt_count = attempt_count + 1,
                     blocked_reason = NULL,
                     failure_kind = NULL,
                     error_detail = NULL,
                     updated_at = ?3
                 WHERE work_item_id = ?1",
                params![work_item_id, lease_until, input.claimed_at],
            )?;
            claimed.push(self.load_work_item(parse_work_item_id(work_item_id)?)?);
        }

        Ok(claimed)
    }

    pub(crate) fn claim_specific_machine_work(
        &self,
        input: &ClaimSpecificMachineWorkInput,
    ) -> LibrarySqliteResult<ClaimedMachineWorkItem> {
        let lease_until = input.claimed_at + input.lease_duration_ms;
        let changed = self.tx.execute(
            "UPDATE WorkItems
             SET state = 'leased',
                 leased_until = ?2,
                 attempt_count = attempt_count + 1,
                 blocked_reason = NULL,
                 failure_kind = NULL,
                 error_detail = NULL,
                 updated_at = ?3
             WHERE work_item_id = ?1
               AND (
                   state = 'queued'
                   OR (state = 'leased' AND leased_until IS NOT NULL AND leased_until <= ?3)
               )",
            params![input.work_item_id.get(), lease_until, input.claimed_at],
        )?;
        if changed != 1 {
            let work_item = load_work_item_row(self.tx, input.work_item_id)?;
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} must be queued or lease-expired before specific claim; found {}",
                input.work_item_id.get(),
                work_item.state.as_str()
            )));
        }

        self.load_work_item(input.work_item_id)
    }

    pub fn complete_machine_work_item(
        &self,
        input: &CompleteMachineWorkInput,
    ) -> LibrarySqliteResult<()> {
        self.require_terminal_transition_preconditions(
            input.work_item_id,
            WorkItemState::Completed,
        )?;
        self.require_successful_completion_run(input.work_item_id)?;
        self.persist_terminal_state(
            input.work_item_id,
            WorkItemState::Completed,
            input.completed_at,
            None,
            None,
            None,
        )
    }

    pub fn fail_machine_work_item(&self, input: &FailMachineWorkInput) -> LibrarySqliteResult<()> {
        self.require_terminal_transition_preconditions(input.work_item_id, WorkItemState::Failed)?;
        self.persist_terminal_state(
            input.work_item_id,
            WorkItemState::Failed,
            input.failed_at,
            None,
            Some(input.failure_kind.as_str()),
            input.error_detail.as_deref(),
        )
    }

    pub fn block_machine_work_item(
        &self,
        input: &BlockMachineWorkInput,
    ) -> LibrarySqliteResult<()> {
        self.require_terminal_transition_preconditions(input.work_item_id, WorkItemState::Blocked)?;
        self.persist_terminal_state(
            input.work_item_id,
            WorkItemState::Blocked,
            input.blocked_at,
            Some(input.blocked_reason.as_str()),
            None,
            input.error_detail.as_deref(),
        )
    }

    pub(crate) fn load_work_item(
        &self,
        work_item_id: WorkItemId,
    ) -> LibrarySqliteResult<ClaimedMachineWorkItem> {
        load_work_item_row(self.tx, work_item_id)
    }

    fn persist_terminal_state(
        &self,
        work_item_id: WorkItemId,
        state: WorkItemState,
        changed_at: i64,
        blocked_reason: Option<&str>,
        failure_kind: Option<&str>,
        error_detail: Option<&str>,
    ) -> LibrarySqliteResult<()> {
        self.tx.execute(
            "UPDATE WorkItems
             SET state = ?2,
                 leased_until = NULL,
                 blocked_reason = ?3,
                 failure_kind = ?4,
                 error_detail = ?5,
                 updated_at = ?6
            WHERE work_item_id = ?1",
            params![
                work_item_id.get(),
                state.as_str(),
                blocked_reason,
                failure_kind,
                error_detail,
                changed_at,
            ],
        )?;
        Ok(())
    }

    fn require_terminal_transition_preconditions(
        &self,
        work_item_id: WorkItemId,
        next_state: WorkItemState,
    ) -> LibrarySqliteResult<()> {
        let work_item = load_work_item_row(self.tx, work_item_id)?;
        if work_item.state != WorkItemState::Leased {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} must be leased before transitioning to {}; found {}",
                work_item_id.get(),
                next_state.as_str(),
                work_item.state.as_str()
            )));
        }
        if let Some(open_run) = load_open_work_run_for_work_item(self.tx, work_item_id)? {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} cannot transition to {} while work run {} is still open",
                work_item_id.get(),
                next_state.as_str(),
                open_run.work_run_id
            )));
        }
        Ok(())
    }

    fn require_successful_completion_run(
        &self,
        work_item_id: WorkItemId,
    ) -> LibrarySqliteResult<()> {
        match load_latest_work_run_for_work_item(self.tx, work_item_id)? {
            Some(run)
                if run.finished_at.is_some()
                    && run.outcome == WorkRunOutcome::Completed.as_str() =>
            {
                Ok(())
            }
            Some(run) => Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} cannot transition to completed because latest work run {} finished with outcome={}",
                work_item_id.get(),
                run.work_run_id,
                run.outcome
            ))),
            None => Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} cannot transition to completed without a finished successful work run",
                work_item_id.get()
            ))),
        }
    }

    fn find_active_work_item(
        &self,
        key: &MachineWorkKey,
    ) -> LibrarySqliteResult<Option<(WorkItemId, WorkItemState)>> {
        let subject_id = key.subject.storage_id();
        let row = self
            .tx
            .query_row(
                "SELECT work_item_id, state
                 FROM WorkItems
                 WHERE subject_kind = ?1
                   AND subject_id = ?2
                   AND work_kind = ?3
                   AND basis_fingerprint = ?4
                   AND state IN ('queued', 'leased', 'blocked')",
                params![
                    key.subject.kind().as_str(),
                    subject_id.as_str(),
                    key.work_kind.as_str(),
                    key.basis_fingerprint.as_str(),
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        row.map(|(work_item_id, state)| {
            Ok((parse_work_item_id(work_item_id)?, parse_state(&state)?))
        })
        .transpose()
    }
}

pub(crate) fn load_work_item_row(
    tx: &AdmittedWrite<'_>,
    work_item_id: WorkItemId,
) -> LibrarySqliteResult<ClaimedMachineWorkItem> {
    let raw = tx
        .query_row(
            "SELECT work_item_id,
                subject_kind,
                subject_id,
                work_kind,
                priority_class,
                basis_fingerprint,
                state,
                leased_until,
                attempt_count,
                blocked_reason,
                failure_kind,
                error_detail,
                created_at,
                updated_at
         FROM WorkItems
         WHERE work_item_id = ?1",
            [work_item_id.get()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, i64>(13)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| {
            LibrarySqliteError::WriteInvariant(format!(
                "work item {} does not exist",
                work_item_id.get()
            ))
        })?;

    let subject_kind = parse_subject_kind(&raw.1)?;
    Ok(ClaimedMachineWorkItem {
        work_item_id: parse_work_item_id(raw.0)?,
        subject: parse_work_subject(subject_kind, &raw.2)?,
        work_kind: parse_work_kind(&raw.3)?,
        priority_class: parse_priority_class(&raw.4)?,
        basis_fingerprint: raw.5,
        state: parse_state(&raw.6)?,
        leased_until: raw.7,
        attempt_count: raw.8,
        blocked_reason: raw.9,
        failure_kind: raw.10,
        error_detail: raw.11,
        created_at: raw.12,
        updated_at: raw.13,
    })
}

fn validate_work_key(key: &MachineWorkKey) -> LibrarySqliteResult<()> {
    if key.basis_fingerprint.trim().is_empty() {
        return Err(LibrarySqliteError::WriteInvariant(
            "machine work basis_fingerprint must not be empty".to_string(),
        ));
    }

    match key.work_kind {
        MachineWorkKind::InspectSource => {
            if !matches!(key.subject, WorkSubject::SourceFile(_)) {
                return Err(LibrarySqliteError::WriteInvariant(format!(
                    "{} work must target subject_kind=source_file",
                    key.work_kind.as_str()
                )));
            }
        }
        MachineWorkKind::RebuildProjection => {
            if !matches!(key.subject, WorkSubject::ProjectionDomain(_)) {
                return Err(LibrarySqliteError::WriteInvariant(
                    "rebuild_projection work must target subject_kind=projection_domain"
                        .to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn parse_work_item_id(value: i64) -> LibrarySqliteResult<WorkItemId> {
    WorkItemId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("invalid WorkItems.work_item_id value: {value}"))
    })
}

fn parse_subject_kind(value: &str) -> LibrarySqliteResult<WorkSubjectKind> {
    WorkSubjectKind::parse(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("unknown WorkItems.subject_kind value: {value}"))
    })
}

fn parse_work_subject(
    subject_kind: WorkSubjectKind,
    subject_id: &str,
) -> LibrarySqliteResult<WorkSubject> {
    WorkSubject::parse(subject_kind, subject_id).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid WorkItems.subject_id value for {}: {subject_id}",
            subject_kind.as_str()
        ))
    })
}

fn parse_work_kind(value: &str) -> LibrarySqliteResult<MachineWorkKind> {
    MachineWorkKind::parse(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("unknown WorkItems.work_kind value: {value}"))
    })
}

fn parse_priority_class(value: &str) -> LibrarySqliteResult<WorkPriorityClass> {
    WorkPriorityClass::parse(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "unknown WorkItems.priority_class value: {value}"
        ))
    })
}

fn parse_state(value: &str) -> LibrarySqliteResult<WorkItemState> {
    WorkItemState::parse(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("unknown WorkItems.state value: {value}"))
    })
}
