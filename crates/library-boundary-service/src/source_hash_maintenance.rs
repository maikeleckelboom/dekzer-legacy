use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use library_boundary_protocol as protocol;
use library_store_sqlite::{
    HashSourceFileBlake3BatchInput, SourceFileBlake3HashAdmissionScope, SqliteDurableStore,
};

use crate::session_events::LibraryBoundaryEventStream;
use crate::snapshot_read_protocol::map_maintained_read_model_revisions;
use crate::source_file_hash_protocol::map_hash_lifecycle_source_failure;

pub(crate) const SOURCE_HASH_MAINTENANCE_BATCH_LIMIT: usize = 8;

#[derive(Debug, Clone)]
pub(crate) struct SourceHashMaintenanceController {
    state: Arc<Mutex<SourceHashMaintenanceState>>,
    stop_requested: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct SourceHashMaintenanceState {
    pending_source_ids: HashSet<i64>,
    active_source_ids: HashSet<i64>,
    #[cfg(test)]
    completed_runs: Vec<SourceHashMaintenanceRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SourceHashMaintenanceRun {
    pub(crate) source_id: i64,
    pub(crate) passes: usize,
    pub(crate) hashed_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) failed_count: usize,
    pub(crate) remaining_candidates: usize,
    pub(crate) source_failure: Option<protocol::HashSourceFilesBlake3SourceFailure>,
    pub(crate) stopped: bool,
}

impl SourceHashMaintenanceController {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SourceHashMaintenanceState::default())),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn request_source(&self, source_id: i64) -> bool {
        if self.stop_requested.load(Ordering::Acquire) {
            return false;
        }

        let mut state = self.state.lock().expect("hash maintenance state poisoned");
        if state.active_source_ids.contains(&source_id) {
            return false;
        }
        state.pending_source_ids.insert(source_id)
    }

    pub(crate) fn run_queued(
        &self,
        store: &SqliteDurableStore,
        events: &LibraryBoundaryEventStream,
    ) -> protocol::ProtocolResult<Vec<SourceHashMaintenanceRun>> {
        let mut runs = Vec::new();
        while !self.stop_requested.load(Ordering::Acquire) {
            let Some(source_id) = self.take_next_source() else {
                break;
            };
            let run = self.run_source(store, events, source_id);
            self.finish_source(source_id);
            let run = run?;
            self.record_completed_run_for_test(run.clone());
            runs.push(run);
        }
        Ok(runs)
    }

    #[cfg(test)]
    pub(crate) fn completed_runs_for_test(&self) -> Vec<SourceHashMaintenanceRun> {
        self.state
            .lock()
            .expect("hash maintenance state poisoned")
            .completed_runs
            .clone()
    }

    pub(crate) fn stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
        let mut state = self.state.lock().expect("hash maintenance state poisoned");
        state.pending_source_ids.clear();
    }

    fn take_next_source(&self) -> Option<i64> {
        let mut state = self.state.lock().expect("hash maintenance state poisoned");
        let source_id = state.pending_source_ids.iter().next().copied()?;
        state.pending_source_ids.remove(&source_id);
        state.active_source_ids.insert(source_id);
        Some(source_id)
    }

    fn finish_source(&self, source_id: i64) {
        let mut state = self.state.lock().expect("hash maintenance state poisoned");
        state.active_source_ids.remove(&source_id);
    }

    #[cfg(test)]
    fn record_completed_run_for_test(&self, run: SourceHashMaintenanceRun) {
        self.state
            .lock()
            .expect("hash maintenance state poisoned")
            .completed_runs
            .push(run);
    }

    #[cfg(not(test))]
    fn record_completed_run_for_test(&self, _run: SourceHashMaintenanceRun) {}

    fn run_source(
        &self,
        store: &SqliteDurableStore,
        events: &LibraryBoundaryEventStream,
        source_id: i64,
    ) -> protocol::ProtocolResult<SourceHashMaintenanceRun> {
        let mut run = SourceHashMaintenanceRun {
            source_id,
            passes: 0,
            hashed_count: 0,
            skipped_count: 0,
            failed_count: 0,
            remaining_candidates: 0,
            source_failure: None,
            stopped: false,
        };

        loop {
            if self.stop_requested.load(Ordering::Acquire) {
                run.stopped = true;
                return Ok(run);
            }

            let lifecycle = store
                .read_source_lifecycle(source_id)
                .map_err(crate::service::map_store_error)?;
            let source_failure = map_hash_lifecycle_source_failure(lifecycle.as_ref())
                .map_err(crate::service::map_store_error)?;
            if let Some(source_failure) = source_failure {
                run.source_failure = Some(source_failure);
                return Ok(run);
            }

            let result = store
                .hash_source_file_blake3_batch(HashSourceFileBlake3BatchInput {
                    scope: SourceFileBlake3HashAdmissionScope::Source { source_id },
                    limit: Some(SOURCE_HASH_MAINTENANCE_BATCH_LIMIT),
                    observed_at_ms: crate::service::unix_time_ms()?,
                })
                .map_err(crate::service::map_store_error)?;

            run.passes += 1;
            run.hashed_count += result.hashed_count;
            run.skipped_count += result.skipped_count;
            run.failed_count += result.failed_count;
            run.remaining_candidates = result.remaining_candidates;

            publish_maintained_snapshot_invalidations(store, events)?;

            if result.remaining_candidates == 0 {
                return Ok(run);
            }

            if result.hashed_count == 0 {
                return Ok(run);
            }
        }
    }
}

fn publish_maintained_snapshot_invalidations(
    store: &SqliteDurableStore,
    events: &LibraryBoundaryEventStream,
) -> protocol::ProtocolResult<()> {
    let revisions = store
        .read_maintained_read_model_revisions()
        .map_err(crate::service::map_store_error)?;
    events.publish_revisions(map_maintained_read_model_revisions(revisions));
    Ok(())
}
