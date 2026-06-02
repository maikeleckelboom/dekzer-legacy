use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use library_boundary_protocol as protocol;
use library_store_sqlite::{
    HashSourceFileBlake3BatchInput, MaterializeAttachmentsForSourceResult,
    ProbeSourceFileMediaBatchInput, ProduceTrackIdentityCandidatesForSourceResult,
    PromotePrimaryMediaForSourceResult, SourceFileBlake3HashAdmissionScope,
    SourceFileMediaProbeAdmissionScope, SqliteDurableStore,
};

use crate::session_events::LibraryBoundaryEventStream;
use crate::snapshot_read_protocol::map_maintained_read_model_revisions;
use crate::source_file_hash_protocol::map_hash_lifecycle_source_failure;

pub(crate) const SOURCE_HASH_MAINTENANCE_BATCH_LIMIT: usize = 8;
pub(crate) const SOURCE_PROBE_MAINTENANCE_BATCH_LIMIT: usize = 4;
pub(crate) const SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT: usize = 4;
pub(crate) const SOURCE_ATTACHMENT_MATERIALIZATION_MAX_LIMIT: usize = 128;
pub(crate) const SOURCE_PRIMARY_MEDIA_PROMOTION_BATCH_LIMIT: usize = 4;
pub(crate) const SOURCE_TRACK_IDENTITY_CANDIDATE_BATCH_LIMIT: usize = 4;

#[derive(Debug, Clone)]
pub(crate) struct SourceMaintenanceController {
    state: Arc<Mutex<SourceMaintenanceState>>,
    stop_requested: Arc<AtomicBool>,
    #[cfg(test)]
    stop_before_attachment_materialization_for_test: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct SourceMaintenanceState {
    pending_source_ids: BTreeSet<i64>,
    active_source_ids: HashSet<i64>,
    last_runs: HashMap<i64, SourceMaintenanceRun>,
    #[cfg(test)]
    completed_runs: Vec<SourceMaintenanceRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SourceMaintenanceRun {
    pub(crate) source_id: i64,
    pub(crate) status: protocol::SourceMaintenanceRunStatus,
    pub(crate) effective_limits: protocol::SourceMaintenanceEffectiveLimits,
    pub(crate) hash: protocol::SourceMaintenanceHashSummary,
    pub(crate) attachment_materialization:
        protocol::SourceMaintenanceAttachmentMaterializationSummary,
    pub(crate) probe: protocol::SourceMaintenanceProbeSummary,
    pub(crate) primary_media_promotion: protocol::SourceMaintenancePrimaryMediaPromotionSummary,
    pub(crate) track_identity_candidates: protocol::SourceMaintenanceTrackIdentityCandidateSummary,
    pub(crate) remaining_hash_candidates: usize,
    pub(crate) remaining_probe_candidates: usize,
    pub(crate) remaining_primary_media_promotion_candidates: usize,
    pub(crate) remaining_track_identity_candidate_production_candidates: usize,
    pub(crate) attachment_links: Option<protocol::SourceMaintenanceAttachmentLinkSummary>,
    pub(crate) source_failure: Option<protocol::SourceMaintenanceSourceFailure>,
    pub(crate) stopped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SourceMaintenanceSnapshot {
    pub(crate) source_id: i64,
    pub(crate) status: protocol::SourceMaintenanceSnapshotStatus,
    pub(crate) remaining_hash_candidates: usize,
    pub(crate) remaining_probe_candidates: usize,
    pub(crate) remaining_primary_media_promotion_candidates: usize,
    pub(crate) remaining_track_identity_candidate_production_candidates: usize,
    pub(crate) attachment_links: Option<protocol::SourceMaintenanceAttachmentLinkSummary>,
    pub(crate) source_failure: Option<protocol::SourceMaintenanceSourceFailure>,
    pub(crate) last_run: Option<protocol::SourceMaintenanceLastRunSummary>,
}

impl SourceMaintenanceController {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SourceMaintenanceState::default())),
            stop_requested: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            stop_before_attachment_materialization_for_test: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn request_source(&self, source_id: i64) -> bool {
        if self.stop_requested.load(Ordering::Acquire) {
            return false;
        }

        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        if state.active_source_ids.contains(&source_id) {
            return false;
        }
        state.pending_source_ids.insert(source_id)
    }

    pub(crate) fn run_queued(
        &self,
        store: &SqliteDurableStore,
        events: &LibraryBoundaryEventStream,
    ) -> protocol::ProtocolResult<Vec<SourceMaintenanceRun>> {
        let mut runs = Vec::new();
        while !self.stop_requested.load(Ordering::Acquire) {
            let Some(source_id) = self.take_next_source() else {
                break;
            };
            let run = self.run_source(
                store,
                events,
                SourceMaintenanceRunInput {
                    source_id,
                    hash_limit: Some(SOURCE_HASH_MAINTENANCE_BATCH_LIMIT),
                    attachment_limit: Some(SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT),
                    probe_limit: Some(SOURCE_PROBE_MAINTENANCE_BATCH_LIMIT),
                    promotion_limit: Some(SOURCE_PRIMARY_MEDIA_PROMOTION_BATCH_LIMIT),
                    identity_candidate_limit: Some(SOURCE_TRACK_IDENTITY_CANDIDATE_BATCH_LIMIT),
                },
            );
            self.finish_source(source_id);
            let run = run?;
            self.record_completed_run(run.clone());
            runs.push(run);
        }
        Ok(runs)
    }

    pub(crate) fn run_manual(
        &self,
        store: &SqliteDurableStore,
        events: &LibraryBoundaryEventStream,
        input: SourceMaintenanceRunInput,
    ) -> protocol::ProtocolResult<SourceMaintenanceRun> {
        let effective_limits = effective_limits_for_input(&input);
        {
            let mut state = self
                .state
                .lock()
                .expect("source maintenance state poisoned");
            if state.active_source_ids.contains(&input.source_id) {
                return Ok(empty_run(input.source_id, effective_limits));
            }
            state.pending_source_ids.remove(&input.source_id);
            state.active_source_ids.insert(input.source_id);
        }
        let run = self.run_source(store, events, input.clone());
        self.finish_source(input.source_id);
        let run = run?;
        self.record_completed_run(run.clone());
        Ok(run)
    }

    pub(crate) fn read_snapshot(
        &self,
        store: &SqliteDurableStore,
        source_id: i64,
    ) -> protocol::ProtocolResult<SourceMaintenanceSnapshot> {
        let lifecycle = store
            .read_source_lifecycle(source_id)
            .map_err(crate::service::map_store_error)?;
        let source_failure = maintenance_source_failure(lifecycle.as_ref())
            .map_err(crate::service::map_store_error)?;
        let is_active = self
            .state
            .lock()
            .expect("source maintenance state poisoned")
            .active_source_ids
            .contains(&source_id);
        let status = if is_active {
            protocol::SourceMaintenanceSnapshotStatus::Running
        } else {
            snapshot_status_for_source_failure(source_failure.as_ref())
        };

        let remaining_hash_candidates =
            count_hash_candidates(store, source_id, source_failure.as_ref())?;
        let remaining_probe_candidates =
            count_probe_candidates(store, source_id, source_failure.as_ref())?;
        let remaining_primary_media_promotion_candidates =
            count_primary_media_promotion_candidates(store, source_id, source_failure.as_ref())?;
        let remaining_track_identity_candidate_production_candidates =
            count_track_identity_candidate_production_candidates(
                store,
                source_id,
                source_failure.as_ref(),
            )?;
        let attachment_links = read_attachment_link_summary(store, source_id)
            .map_err(crate::service::map_store_error)?;
        let last_run = self
            .state
            .lock()
            .expect("source maintenance state poisoned")
            .last_runs
            .get(&source_id)
            .map(last_run_summary);

        Ok(SourceMaintenanceSnapshot {
            source_id,
            status,
            remaining_hash_candidates,
            remaining_probe_candidates,
            remaining_primary_media_promotion_candidates,
            remaining_track_identity_candidate_production_candidates,
            attachment_links,
            source_failure,
            last_run,
        })
    }

    #[cfg(test)]
    pub(crate) fn completed_runs_for_test(&self) -> Vec<SourceMaintenanceRun> {
        self.state
            .lock()
            .expect("source maintenance state poisoned")
            .completed_runs
            .clone()
    }

    #[cfg(test)]
    pub(crate) fn pending_source_ids_for_test(&self) -> Vec<i64> {
        self.state
            .lock()
            .expect("source maintenance state poisoned")
            .pending_source_ids
            .iter()
            .copied()
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn active_source_ids_for_test(&self) -> Vec<i64> {
        let mut source_ids = self
            .state
            .lock()
            .expect("source maintenance state poisoned")
            .active_source_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        source_ids.sort_unstable();
        source_ids
    }

    #[cfg(test)]
    pub(crate) fn set_source_active_for_test(&self, source_id: i64, active: bool) {
        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        if active {
            state.active_source_ids.insert(source_id);
        } else {
            state.active_source_ids.remove(&source_id);
        }
    }

    pub(crate) fn stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        state.pending_source_ids.clear();
    }

    #[cfg(test)]
    pub(crate) fn request_stop_before_attachment_materialization_for_test(&self) {
        self.stop_before_attachment_materialization_for_test
            .store(true, Ordering::Release);
    }

    fn take_next_source(&self) -> Option<i64> {
        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        let source_id = state.pending_source_ids.iter().next().copied()?;
        state.pending_source_ids.remove(&source_id);
        state.active_source_ids.insert(source_id);
        Some(source_id)
    }

    fn finish_source(&self, source_id: i64) {
        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        state.active_source_ids.remove(&source_id);
    }

    fn record_completed_run(&self, run: SourceMaintenanceRun) {
        let mut state = self
            .state
            .lock()
            .expect("source maintenance state poisoned");
        state.last_runs.insert(run.source_id, run.clone());
        #[cfg(test)]
        state.completed_runs.push(run);
    }

    fn run_source(
        &self,
        store: &SqliteDurableStore,
        events: &LibraryBoundaryEventStream,
        input: SourceMaintenanceRunInput,
    ) -> protocol::ProtocolResult<SourceMaintenanceRun> {
        let effective_limits = effective_limits_for_input(&input);
        let mut run = empty_run(input.source_id, effective_limits);

        if self.stop_requested.load(Ordering::Acquire) {
            run.stopped = true;
            run.status = protocol::SourceMaintenanceRunStatus::Skipped;
            return Ok(run);
        }

        let lifecycle = store
            .read_source_lifecycle(input.source_id)
            .map_err(crate::service::map_store_error)?;
        let source_failure = maintenance_source_failure(lifecycle.as_ref())
            .map_err(crate::service::map_store_error)?;
        if let Some(source_failure) = source_failure {
            run.source_failure = Some(source_failure);
            run.status = protocol::SourceMaintenanceRunStatus::Failed;
            return Ok(run);
        }

        let hash_result = store
            .hash_source_file_blake3_batch(HashSourceFileBlake3BatchInput {
                scope: SourceFileBlake3HashAdmissionScope::Source {
                    source_id: input.source_id,
                },
                limit: Some(effective_limits.hash_limit),
                observed_at_ms: crate::service::unix_time_ms()?,
            })
            .map_err(crate::service::map_store_error)?;
        run.hash = protocol::SourceMaintenanceHashSummary {
            effective_limit: hash_result.effective_limit,
            hashed_count: hash_result.hashed_count,
            skipped_count: hash_result.skipped_count,
            failed_count: hash_result.failed_count,
            remaining_candidates: hash_result.remaining_candidates,
        };
        run.remaining_hash_candidates = hash_result.remaining_candidates;
        publish_maintained_snapshot_invalidations(store, events)?;

        #[cfg(test)]
        if self
            .stop_before_attachment_materialization_for_test
            .swap(false, Ordering::AcqRel)
        {
            self.stop();
        }

        if self.stop_requested.load(Ordering::Acquire) {
            run.stopped = true;
            run.status = protocol::SourceMaintenanceRunStatus::Partial;
            return Ok(run);
        }

        let attachment_result = store
            .materialize_attachments_for_source(input.source_id, effective_limits.attachment_limit)
            .map_err(crate::service::map_store_error)?;
        run.attachment_materialization = map_attachment_materialization_summary(
            effective_limits.attachment_limit,
            attachment_result,
        );
        run.attachment_links = read_attachment_link_summary(store, input.source_id)
            .map_err(crate::service::map_store_error)?;
        publish_maintained_snapshot_invalidations(store, events)?;

        if self.stop_requested.load(Ordering::Acquire) {
            run.stopped = true;
            run.status = protocol::SourceMaintenanceRunStatus::Partial;
            return Ok(run);
        }

        let probe_result = store
            .probe_source_file_media_batch(ProbeSourceFileMediaBatchInput {
                scope: SourceFileMediaProbeAdmissionScope::Source {
                    source_id: input.source_id,
                },
                limit: Some(effective_limits.probe_limit),
                observed_at_ms: crate::service::unix_time_ms()?,
            })
            .map_err(crate::service::map_store_error)?;
        run.probe = protocol::SourceMaintenanceProbeSummary {
            effective_limit: probe_result.effective_limit,
            probed_count: probe_result.probed_count,
            skipped_count: probe_result.skipped_count,
            failed_count: probe_result.failed_count,
            remaining_candidates: probe_result.remaining_candidates,
        };
        run.remaining_probe_candidates = probe_result.remaining_candidates;
        publish_maintained_snapshot_invalidations(store, events)?;

        if self.stop_requested.load(Ordering::Acquire) {
            run.stopped = true;
            run.status = protocol::SourceMaintenanceRunStatus::Partial;
            return Ok(run);
        }

        let promotion_result = store
            .promote_primary_media_for_source(input.source_id, effective_limits.promotion_limit)
            .map_err(crate::service::map_store_error)?;
        run.primary_media_promotion =
            map_primary_media_promotion_summary(effective_limits.promotion_limit, promotion_result);
        run.remaining_primary_media_promotion_candidates =
            run.primary_media_promotion.remaining_candidates;
        publish_maintained_snapshot_invalidations(store, events)?;

        if self.stop_requested.load(Ordering::Acquire) {
            run.stopped = true;
            run.status = protocol::SourceMaintenanceRunStatus::Partial;
            return Ok(run);
        }

        let track_identity_result = store
            .produce_track_identity_candidates_for_source(
                input.source_id,
                effective_limits.identity_candidate_limit,
            )
            .map_err(crate::service::map_store_error)?;
        run.track_identity_candidates = map_track_identity_candidate_summary(
            effective_limits.identity_candidate_limit,
            track_identity_result,
        );
        run.remaining_track_identity_candidate_production_candidates =
            run.track_identity_candidates.remaining_candidates;
        publish_maintained_snapshot_invalidations(store, events)?;

        run.status = run_status(&run);
        Ok(run)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceMaintenanceRunInput {
    pub(crate) source_id: i64,
    pub(crate) hash_limit: Option<usize>,
    pub(crate) attachment_limit: Option<usize>,
    pub(crate) probe_limit: Option<usize>,
    pub(crate) promotion_limit: Option<usize>,
    pub(crate) identity_candidate_limit: Option<usize>,
}

fn effective_limits_for_input(
    input: &SourceMaintenanceRunInput,
) -> protocol::SourceMaintenanceEffectiveLimits {
    protocol::SourceMaintenanceEffectiveLimits {
        hash_limit: library_store_sqlite::effective_hash_batch_limit(input.hash_limit),
        attachment_limit: effective_attachment_materialization_limit(input.attachment_limit),
        probe_limit: library_store_sqlite::effective_media_probe_batch_limit(input.probe_limit),
        promotion_limit: library_store_sqlite::effective_primary_media_promotion_limit(
            input.promotion_limit,
        ),
        identity_candidate_limit: library_store_sqlite::effective_track_identity_candidate_limit(
            input.identity_candidate_limit,
        ),
    }
}

pub(crate) fn effective_attachment_materialization_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT)
        .clamp(1, SOURCE_ATTACHMENT_MATERIALIZATION_MAX_LIMIT)
}

fn empty_run(
    source_id: i64,
    effective_limits: protocol::SourceMaintenanceEffectiveLimits,
) -> SourceMaintenanceRun {
    SourceMaintenanceRun {
        source_id,
        status: protocol::SourceMaintenanceRunStatus::Skipped,
        effective_limits,
        hash: protocol::SourceMaintenanceHashSummary {
            effective_limit: effective_limits.hash_limit,
            hashed_count: 0,
            skipped_count: 0,
            failed_count: 0,
            remaining_candidates: 0,
        },
        attachment_materialization: protocol::SourceMaintenanceAttachmentMaterializationSummary {
            effective_limit: effective_limits.attachment_limit,
            attachments_created: 0,
            attachments_refreshed: 0,
            links_created: 0,
            links_replaced: 0,
            links_refreshed: 0,
            skipped_stale_facts: 0,
            skipped_no_blake3: 0,
            skipped_no_facts: 0,
            remaining_candidates: 0,
        },
        probe: protocol::SourceMaintenanceProbeSummary {
            effective_limit: effective_limits.probe_limit,
            probed_count: 0,
            skipped_count: 0,
            failed_count: 0,
            remaining_candidates: 0,
        },
        primary_media_promotion: protocol::SourceMaintenancePrimaryMediaPromotionSummary {
            effective_limit: effective_limits.promotion_limit,
            promoted_count: 0,
            refreshed_count: 0,
            skipped_unusable_source: 0,
            skipped_unsupported_media_kind: 0,
            skipped_no_facts: 0,
            skipped_stale_facts: 0,
            skipped_no_blake3: 0,
            skipped_no_probe_facts: 0,
            skipped_missing_attachment_link: 0,
            skipped_stale_attachment_link: 0,
            remaining_candidates: 0,
        },
        track_identity_candidates: protocol::SourceMaintenanceTrackIdentityCandidateSummary {
            effective_limit: effective_limits.identity_candidate_limit,
            candidates_created: 0,
            candidates_refreshed: 0,
            members_created: 0,
            members_refreshed: 0,
            evidence_created: 0,
            evidence_refreshed: 0,
            candidates_marked_stale: 0,
            skipped_stale_primary_media_candidates: 0,
            remaining_candidates: 0,
        },
        remaining_hash_candidates: 0,
        remaining_probe_candidates: 0,
        remaining_primary_media_promotion_candidates: 0,
        remaining_track_identity_candidate_production_candidates: 0,
        attachment_links: None,
        source_failure: None,
        stopped: false,
    }
}

fn run_status(run: &SourceMaintenanceRun) -> protocol::SourceMaintenanceRunStatus {
    if run.source_failure.is_some() || run.hash.failed_count > 0 || run.probe.failed_count > 0 {
        return protocol::SourceMaintenanceRunStatus::Partial;
    }
    if run.remaining_hash_candidates > 0
        || run.remaining_probe_candidates > 0
        || run.attachment_materialization.remaining_candidates > 0
        || run.remaining_primary_media_promotion_candidates > 0
        || run.remaining_track_identity_candidate_production_candidates > 0
    {
        return protocol::SourceMaintenanceRunStatus::Partial;
    }
    protocol::SourceMaintenanceRunStatus::Completed
}

fn last_run_summary(run: &SourceMaintenanceRun) -> protocol::SourceMaintenanceLastRunSummary {
    protocol::SourceMaintenanceLastRunSummary {
        status: run.status,
        hash: run.hash,
        attachment_materialization: run.attachment_materialization,
        probe: run.probe,
        primary_media_promotion: run.primary_media_promotion,
        track_identity_candidates: run.track_identity_candidates,
        remaining_hash_candidates: run.remaining_hash_candidates,
        remaining_probe_candidates: run.remaining_probe_candidates,
        remaining_primary_media_promotion_candidates: run
            .remaining_primary_media_promotion_candidates,
        remaining_track_identity_candidate_production_candidates: run
            .remaining_track_identity_candidate_production_candidates,
        source_failure: run.source_failure.clone(),
    }
}

fn map_primary_media_promotion_summary(
    effective_limit: usize,
    result: PromotePrimaryMediaForSourceResult,
) -> protocol::SourceMaintenancePrimaryMediaPromotionSummary {
    protocol::SourceMaintenancePrimaryMediaPromotionSummary {
        effective_limit,
        promoted_count: result.promoted_count,
        refreshed_count: result.refreshed_count,
        skipped_unusable_source: result.skipped_unusable_source,
        skipped_unsupported_media_kind: result.skipped_unsupported_media_kind,
        skipped_no_facts: result.skipped_no_facts,
        skipped_stale_facts: result.skipped_stale_facts,
        skipped_no_blake3: result.skipped_no_blake3,
        skipped_no_probe_facts: result.skipped_no_probe_facts,
        skipped_missing_attachment_link: result.skipped_missing_attachment_link,
        skipped_stale_attachment_link: result.skipped_stale_attachment_link,
        remaining_candidates: result.remaining_candidates,
    }
}

fn map_attachment_materialization_summary(
    effective_limit: usize,
    result: MaterializeAttachmentsForSourceResult,
) -> protocol::SourceMaintenanceAttachmentMaterializationSummary {
    protocol::SourceMaintenanceAttachmentMaterializationSummary {
        effective_limit,
        attachments_created: result.attachments_created,
        attachments_refreshed: result.attachments_refreshed,
        links_created: result.links_created,
        links_replaced: result.links_replaced,
        links_refreshed: result.links_refreshed,
        skipped_stale_facts: result.skipped_stale_facts,
        skipped_no_blake3: result.skipped_no_blake3,
        skipped_no_facts: result.skipped_no_facts,
        remaining_candidates: result.remaining_candidates,
    }
}

fn map_track_identity_candidate_summary(
    effective_limit: usize,
    result: ProduceTrackIdentityCandidatesForSourceResult,
) -> protocol::SourceMaintenanceTrackIdentityCandidateSummary {
    protocol::SourceMaintenanceTrackIdentityCandidateSummary {
        effective_limit,
        candidates_created: result.candidates_created,
        candidates_refreshed: result.candidates_refreshed,
        members_created: result.members_created,
        members_refreshed: result.members_refreshed,
        evidence_created: result.evidence_created,
        evidence_refreshed: result.evidence_refreshed,
        candidates_marked_stale: result.candidates_marked_stale,
        skipped_stale_primary_media_candidates: result.skipped_stale_primary_media_candidates,
        remaining_candidates: result.remaining_candidates,
    }
}

fn maintenance_source_failure(
    lifecycle: Option<&library_store_sqlite::StoreSourceLifecycle>,
) -> library_store_sqlite::LibrarySqliteResult<Option<protocol::SourceMaintenanceSourceFailure>> {
    map_hash_lifecycle_source_failure(lifecycle).map(|failure| failure.map(map_source_failure))
}

fn map_source_failure(
    failure: protocol::HashSourceFilesBlake3SourceFailure,
) -> protocol::SourceMaintenanceSourceFailure {
    match failure {
        protocol::HashSourceFilesBlake3SourceFailure::SourceNotFound => {
            protocol::SourceMaintenanceSourceFailure::SourceNotFound
        }
        protocol::HashSourceFilesBlake3SourceFailure::SourceUnavailable(failure) => {
            protocol::SourceMaintenanceSourceFailure::SourceUnavailable(
                protocol::SourceMaintenanceSourceUnavailableFailure {
                    mount_status: failure.mount_status,
                    access_state: failure.access_state,
                    access_issue_kind: failure.access_issue_kind,
                },
            )
        }
        protocol::HashSourceFilesBlake3SourceFailure::SourceRootMissing(failure) => {
            protocol::SourceMaintenanceSourceFailure::SourceRootMissing(
                protocol::SourceMaintenanceSourceRootMissingFailure {
                    detail: failure.detail,
                },
            )
        }
        protocol::HashSourceFilesBlake3SourceFailure::SourceRootBlocked(failure) => {
            protocol::SourceMaintenanceSourceFailure::SourceRootBlocked(
                protocol::SourceMaintenanceSourceRootBlockedFailure {
                    access_issue_kind: failure.access_issue_kind,
                    detail: failure.detail,
                },
            )
        }
    }
}

fn snapshot_status_for_source_failure(
    source_failure: Option<&protocol::SourceMaintenanceSourceFailure>,
) -> protocol::SourceMaintenanceSnapshotStatus {
    match source_failure {
        None => protocol::SourceMaintenanceSnapshotStatus::Idle,
        Some(protocol::SourceMaintenanceSourceFailure::SourceNotFound) => {
            protocol::SourceMaintenanceSnapshotStatus::Failed
        }
        Some(protocol::SourceMaintenanceSourceFailure::SourceUnavailable(_)) => {
            protocol::SourceMaintenanceSnapshotStatus::Unavailable
        }
        Some(protocol::SourceMaintenanceSourceFailure::SourceRootMissing(_)) => {
            protocol::SourceMaintenanceSnapshotStatus::Unavailable
        }
        Some(protocol::SourceMaintenanceSourceFailure::SourceRootBlocked(_)) => {
            protocol::SourceMaintenanceSnapshotStatus::Blocked
        }
    }
}

fn count_hash_candidates(
    store: &SqliteDurableStore,
    source_id: i64,
    source_failure: Option<&protocol::SourceMaintenanceSourceFailure>,
) -> protocol::ProtocolResult<usize> {
    if source_failure.is_some() {
        return Ok(0);
    }
    store
        .count_source_file_blake3_hash_candidates(SourceFileBlake3HashAdmissionScope::Source {
            source_id,
        })
        .map_err(crate::service::map_store_error)
}

fn count_probe_candidates(
    store: &SqliteDurableStore,
    source_id: i64,
    source_failure: Option<&protocol::SourceMaintenanceSourceFailure>,
) -> protocol::ProtocolResult<usize> {
    if source_failure.is_some() {
        return Ok(0);
    }
    store
        .count_source_file_media_probe_candidates(SourceFileMediaProbeAdmissionScope::Source {
            source_id,
        })
        .map_err(crate::service::map_store_error)
}

fn count_primary_media_promotion_candidates(
    store: &SqliteDurableStore,
    source_id: i64,
    source_failure: Option<&protocol::SourceMaintenanceSourceFailure>,
) -> protocol::ProtocolResult<usize> {
    if source_failure.is_some() {
        return Ok(0);
    }
    store
        .count_primary_media_promotion_candidates(source_id)
        .map_err(crate::service::map_store_error)
}

fn count_track_identity_candidate_production_candidates(
    store: &SqliteDurableStore,
    source_id: i64,
    source_failure: Option<&protocol::SourceMaintenanceSourceFailure>,
) -> protocol::ProtocolResult<usize> {
    if source_failure.is_some() {
        return Ok(0);
    }
    store
        .count_track_identity_candidate_production_candidates(source_id)
        .map_err(crate::service::map_store_error)
}

fn read_attachment_link_summary(
    store: &SqliteDurableStore,
    source_id: i64,
) -> library_store_sqlite::LibrarySqliteResult<
    Option<protocol::SourceMaintenanceAttachmentLinkSummary>,
> {
    Ok(store
        .read_source_attachment_summary(source_id)?
        .map(|summary| protocol::SourceMaintenanceAttachmentLinkSummary {
            current_links_count: summary.current_links_count,
            stale_links_count: summary.stale_links_count,
            source_files_with_current_blake3_facts_count: summary
                .source_files_with_current_blake3_facts_count,
            source_files_with_attachment_links_count: summary
                .source_files_with_attachment_links_count,
            source_files_missing_attachment_links_count: summary
                .source_files_missing_attachment_links_count,
            unmaterialized_blake3_facts_count: summary.source_files_missing_attachment_links_count,
        }))
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
