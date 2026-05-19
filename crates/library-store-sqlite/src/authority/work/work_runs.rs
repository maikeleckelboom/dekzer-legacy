use rusqlite::{OptionalExtension, Row, params};

use crate::authority::work::work_items::load_work_item_row;
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{WorkItemId, WorkItemState, WorkRunId, WorkRunOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartWorkRunInput {
    pub work_item_id: WorkItemId,
    pub adapter_key: String,
    pub adapter_version: String,
    pub started_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedWorkRun {
    pub work_run_id: WorkRunId,
    pub work_item_id: WorkItemId,
    pub adapter_key: String,
    pub adapter_version: String,
    pub started_at: i64,
    pub outcome: WorkRunOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishWorkRunInput {
    pub work_run_id: WorkRunId,
    pub finished_at: i64,
    pub outcome: WorkRunOutcome,
    pub failure_kind: Option<String>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedWorkRun {
    pub work_run_id: i64,
    pub work_item_id: i64,
    pub adapter_key: String,
    pub adapter_version: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub outcome: String,
    pub failure_kind: Option<String>,
    pub error_detail: Option<String>,
}

pub struct WorkRunsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> WorkRunsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn start_work_run(&self, input: &StartWorkRunInput) -> LibrarySqliteResult<StartedWorkRun> {
        let work_item = load_work_item_row(self.tx, input.work_item_id)?;
        if work_item.state != WorkItemState::Leased {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} must be leased before starting a run; found {}",
                input.work_item_id.get(),
                work_item.state.as_str()
            )));
        }
        if let Some(open_run) = load_open_work_run_for_work_item(self.tx, input.work_item_id)? {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work item {} already has unfinished work run {}",
                input.work_item_id.get(),
                open_run.work_run_id
            )));
        }

        self.tx.execute(
            "INSERT INTO WorkRuns (
                 work_item_id,
                 adapter_key,
                 adapter_version,
                 started_at,
                 finished_at,
                 outcome,
                 failure_kind,
                 error_detail
             )
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, NULL)",
            params![
                input.work_item_id.get(),
                input.adapter_key.as_str(),
                input.adapter_version.as_str(),
                input.started_at,
                WorkRunOutcome::Running.as_str(),
            ],
        )?;

        Ok(StartedWorkRun {
            work_run_id: parse_work_run_id(self.tx.last_insert_rowid())?,
            work_item_id: input.work_item_id,
            adapter_key: input.adapter_key.clone(),
            adapter_version: input.adapter_version.clone(),
            started_at: input.started_at,
            outcome: WorkRunOutcome::Running,
        })
    }

    pub fn finish_work_run(&self, input: &FinishWorkRunInput) -> LibrarySqliteResult<()> {
        let run = self.load_work_run(input.work_run_id)?;
        if run.finished_at.is_some() {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "work run {} is already finished",
                input.work_run_id.get()
            )));
        }
        if input.outcome == WorkRunOutcome::Running {
            return Err(LibrarySqliteError::WriteInvariant(
                "finished work runs must not retain outcome=running".to_string(),
            ));
        }
        self.tx.execute(
            "UPDATE WorkRuns
             SET finished_at = ?2,
                 outcome = ?3,
                 failure_kind = ?4,
                 error_detail = ?5
             WHERE work_run_id = ?1",
            params![
                input.work_run_id.get(),
                input.finished_at,
                input.outcome.as_str(),
                input.failure_kind.as_deref(),
                input.error_detail.as_deref(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn load_work_run(
        &self,
        work_run_id: WorkRunId,
    ) -> LibrarySqliteResult<PersistedWorkRun> {
        self.tx
            .query_row(
                "SELECT work_run_id,
                        work_item_id,
                        adapter_key,
                        adapter_version,
                        started_at,
                        finished_at,
                        outcome,
                        failure_kind,
                        error_detail
                 FROM WorkRuns
                 WHERE work_run_id = ?1",
                [work_run_id.get()],
                map_work_run_row,
            )
            .optional()?
            .ok_or_else(|| {
                LibrarySqliteError::WriteInvariant(format!(
                    "work run {} does not exist",
                    work_run_id.get()
                ))
            })
    }
}

pub(crate) fn load_open_work_run_for_work_item(
    tx: &AdmittedWrite<'_>,
    work_item_id: WorkItemId,
) -> LibrarySqliteResult<Option<PersistedWorkRun>> {
    tx.query_row(
        "SELECT work_run_id,
                work_item_id,
                adapter_key,
                adapter_version,
                started_at,
                finished_at,
                outcome,
                failure_kind,
                error_detail
         FROM WorkRuns
         WHERE work_item_id = ?1
           AND finished_at IS NULL
         ORDER BY work_run_id DESC
         LIMIT 1",
        [work_item_id.get()],
        map_work_run_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn load_latest_work_run_for_work_item(
    tx: &AdmittedWrite<'_>,
    work_item_id: WorkItemId,
) -> LibrarySqliteResult<Option<PersistedWorkRun>> {
    tx.query_row(
        "SELECT work_run_id,
                work_item_id,
                adapter_key,
                adapter_version,
                started_at,
                finished_at,
                outcome,
                failure_kind,
                error_detail
         FROM WorkRuns
         WHERE work_item_id = ?1
         ORDER BY work_run_id DESC
         LIMIT 1",
        [work_item_id.get()],
        map_work_run_row,
    )
    .optional()
    .map_err(Into::into)
}

fn parse_work_run_id(value: i64) -> LibrarySqliteResult<WorkRunId> {
    WorkRunId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("invalid WorkRuns.work_run_id value: {value}"))
    })
}

fn map_work_run_row(row: &Row<'_>) -> rusqlite::Result<PersistedWorkRun> {
    Ok(PersistedWorkRun {
        work_run_id: row.get(0)?,
        work_item_id: row.get(1)?,
        adapter_key: row.get(2)?,
        adapter_version: row.get(3)?,
        started_at: row.get(4)?,
        finished_at: row.get(5)?,
        outcome: row.get(6)?,
        failure_kind: row.get(7)?,
        error_detail: row.get(8)?,
    })
}
