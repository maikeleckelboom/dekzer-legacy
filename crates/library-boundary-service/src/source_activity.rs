use library_boundary_protocol as protocol;
use library_store_sqlite as store;

use crate::source_maintenance::SourceMaintenanceSnapshot;

pub(crate) fn map_read_source_activity_reply(
    source_integrity: store::StoreSourceIntegrity,
    maintenance: SourceMaintenanceSnapshot,
) -> protocol::ReadSourceActivityReply {
    let admission_state = admission_state(source_integrity.lifecycle.as_ref());
    let browse_readiness = browse_readiness(&source_integrity);
    let scan_activity = scan_activity(source_integrity.lifecycle.as_ref());
    let preparation_activity = preparation_activity(&maintenance);

    protocol::ReadSourceActivityReply {
        source_id: source_integrity.source_id,
        admission_state,
        browse_readiness,
        scan_activity,
        preparation_activity,
    }
}

fn admission_state(
    lifecycle: Option<&store::StoreSourceLifecycle>,
) -> protocol::SourceActivityAdmissionState {
    match lifecycle {
        Some(lifecycle) if lifecycle.is_user_visible => {
            protocol::SourceActivityAdmissionState::Active
        }
        Some(_) => protocol::SourceActivityAdmissionState::Restorable,
        None => protocol::SourceActivityAdmissionState::NotAdmitted,
    }
}

fn browse_readiness(
    source_integrity: &store::StoreSourceIntegrity,
) -> protocol::SourceBrowseReadiness {
    let Some(lifecycle) = source_integrity.lifecycle.as_ref() else {
        return readiness(
            protocol::SourceBrowseReadinessState::Unavailable,
            "Source is not admitted.",
        );
    };

    if !lifecycle.is_user_visible {
        return readiness(
            protocol::SourceBrowseReadinessState::Unavailable,
            "Source is restorable but not active.",
        );
    }

    if lifecycle.source_class != "internal" && lifecycle.mount_status != "mounted" {
        return readiness(
            protocol::SourceBrowseReadinessState::Unavailable,
            "Source mount is unavailable.",
        );
    }

    match lifecycle.access_state.as_str() {
        "missing" => {
            return readiness(
                protocol::SourceBrowseReadinessState::Missing,
                "Source root is missing.",
            );
        }
        "blocked" => {
            return readiness(
                if lifecycle.access_issue_kind.as_deref() == Some("unavailable_mount") {
                    protocol::SourceBrowseReadinessState::Unavailable
                } else {
                    protocol::SourceBrowseReadinessState::Blocked
                },
                "Source root is blocked.",
            );
        }
        _ => {}
    }

    match lifecycle.scan_phase.as_str() {
        "idle" if lifecycle.last_successful_scan_at.is_none() => {
            return readiness(
                protocol::SourceBrowseReadinessState::NeedsScan,
                "Source has not completed a scan.",
            );
        }
        "scanning" => {
            return readiness(
                protocol::SourceBrowseReadinessState::Indexing,
                "Source scan is running.",
            );
        }
        "blocked" => {
            return readiness(
                if lifecycle.scan_issue_kind.as_deref() == Some("unavailable_mount") {
                    protocol::SourceBrowseReadinessState::Unavailable
                } else {
                    protocol::SourceBrowseReadinessState::Blocked
                },
                "Source scan is blocked.",
            );
        }
        "failed" => {
            return readiness(
                protocol::SourceBrowseReadinessState::Blocked,
                "Source scan failed.",
            );
        }
        _ => {}
    }

    match source_integrity.coverage.state {
        store::StoreSourceIntegrityCoverageState::SourceUnavailable => readiness(
            protocol::SourceBrowseReadinessState::Unavailable,
            "Source is unavailable.",
        ),
        store::StoreSourceIntegrityCoverageState::LocationMissing => readiness(
            protocol::SourceBrowseReadinessState::Missing,
            "Source location is missing.",
        ),
        store::StoreSourceIntegrityCoverageState::Blocked
        | store::StoreSourceIntegrityCoverageState::Failed => readiness(
            protocol::SourceBrowseReadinessState::Blocked,
            "Source coverage is blocked.",
        ),
        store::StoreSourceIntegrityCoverageState::Scanning => readiness(
            protocol::SourceBrowseReadinessState::Indexing,
            "Source coverage is still indexing.",
        ),
        store::StoreSourceIntegrityCoverageState::Pending => readiness(
            protocol::SourceBrowseReadinessState::NeedsScan,
            "Source coverage is pending scan.",
        ),
        store::StoreSourceIntegrityCoverageState::Incomplete => readiness(
            protocol::SourceBrowseReadinessState::NeedsScan,
            "Source coverage is incomplete.",
        ),
        store::StoreSourceIntegrityCoverageState::Complete => {
            if source_integrity
                .inventory
                .as_ref()
                .map(|inventory| inventory.present_media_relevant_files_count == 0)
                .unwrap_or(false)
            {
                readiness(
                    protocol::SourceBrowseReadinessState::Empty,
                    "Source is indexed but no media candidates are present.",
                )
            } else {
                readiness(
                    protocol::SourceBrowseReadinessState::Ready,
                    "Source is ready to browse.",
                )
            }
        }
    }
}

fn readiness(
    state: protocol::SourceBrowseReadinessState,
    detail: &str,
) -> protocol::SourceBrowseReadiness {
    protocol::SourceBrowseReadiness {
        state,
        detail: Some(detail.to_string()),
    }
}

fn scan_activity(lifecycle: Option<&store::StoreSourceLifecycle>) -> protocol::SourceScanActivity {
    let Some(lifecycle) = lifecycle else {
        return protocol::SourceScanActivity {
            state: protocol::SourceScanActivityState::Blocked,
            counters: protocol::SourceScanActivityCounters::default(),
            detail: Some("Source is not admitted.".to_string()),
            scan_run_id: None,
            last_started_at_ms: None,
            last_finished_at_ms: None,
        };
    };

    let state = match lifecycle.scan_phase.as_str() {
        "scanning" => protocol::SourceScanActivityState::Running,
        "complete" => protocol::SourceScanActivityState::Completed,
        "failed" => protocol::SourceScanActivityState::Failed,
        "blocked" => protocol::SourceScanActivityState::Blocked,
        "partial" => protocol::SourceScanActivityState::Failed,
        _ => protocol::SourceScanActivityState::Idle,
    };

    protocol::SourceScanActivity {
        state,
        counters: protocol::SourceScanActivityCounters::default(),
        detail: scan_activity_detail(state, lifecycle).map(str::to_string),
        scan_run_id: None,
        last_started_at_ms: lifecycle.last_scan_started_at,
        last_finished_at_ms: lifecycle.last_scan_finished_at,
    }
}

fn scan_activity_detail(
    state: protocol::SourceScanActivityState,
    lifecycle: &store::StoreSourceLifecycle,
) -> Option<&'static str> {
    match state {
        protocol::SourceScanActivityState::Idle
            if lifecycle.last_successful_scan_at.is_none()
                && lifecycle.last_scan_started_at.is_none() =>
        {
            Some("Scan has not started.")
        }
        protocol::SourceScanActivityState::Idle => Some("Scan is idle."),
        protocol::SourceScanActivityState::Running => Some("Scan is running."),
        protocol::SourceScanActivityState::Completed => Some("Scan completed."),
        protocol::SourceScanActivityState::Failed => Some("Scan failed or ended partially."),
        protocol::SourceScanActivityState::Blocked => Some("Scan is blocked."),
        protocol::SourceScanActivityState::Cancelled => Some("Scan was cancelled."),
    }
}

fn preparation_activity(
    maintenance: &SourceMaintenanceSnapshot,
) -> protocol::SourcePreparationActivity {
    let backlog = preparation_backlog(maintenance);
    let remaining = preparation_backlog_total(backlog);
    let last_run_status = maintenance.last_run.as_ref().map(|run| run.status);
    let last_run_processed = maintenance
        .last_run
        .as_ref()
        .map(last_run_preparation_processed_counts);
    let state = match maintenance.status {
        protocol::SourceMaintenanceSnapshotStatus::Running => {
            protocol::SourcePreparationActivityState::Running
        }
        protocol::SourceMaintenanceSnapshotStatus::Unavailable
        | protocol::SourceMaintenanceSnapshotStatus::Blocked => {
            protocol::SourcePreparationActivityState::Unavailable
        }
        protocol::SourceMaintenanceSnapshotStatus::Failed => {
            protocol::SourcePreparationActivityState::Failed
        }
        protocol::SourceMaintenanceSnapshotStatus::Idle
            if remaining > 0 && maintenance.last_run.is_some() =>
        {
            protocol::SourcePreparationActivityState::CompletedWithRemainingWork
        }
        protocol::SourceMaintenanceSnapshotStatus::Idle if remaining > 0 => {
            protocol::SourcePreparationActivityState::Idle
        }
        protocol::SourceMaintenanceSnapshotStatus::Idle => {
            protocol::SourcePreparationActivityState::Complete
        }
    };

    protocol::SourcePreparationActivity {
        state,
        backlog,
        provenance: protocol::SourcePreparationProvenance::MaintenanceSnapshot,
        bounded_batch: true,
        last_run_status,
        last_run_processed,
        source_failure: maintenance.source_failure.clone(),
    }
}

fn preparation_backlog(
    maintenance: &SourceMaintenanceSnapshot,
) -> protocol::SourcePreparationBacklogCounts {
    let attachment = maintenance
        .attachment_links
        .as_ref()
        .map(|links| links.stale_links_count + links.source_files_missing_attachment_links_count)
        .unwrap_or(0);
    protocol::SourcePreparationBacklogCounts {
        hash: maintenance.remaining_hash_candidates,
        probe: maintenance.remaining_probe_candidates,
        attachment,
        promotion: maintenance.remaining_playable_media_promotion_candidates,
        identity: maintenance.remaining_track_identity_candidate_production_candidates
            + maintenance.remaining_track_identity_decision_production_candidates,
    }
}

fn preparation_backlog_total(backlog: protocol::SourcePreparationBacklogCounts) -> usize {
    backlog.hash + backlog.probe + backlog.attachment + backlog.promotion + backlog.identity
}

fn last_run_preparation_processed_counts(
    last_run: &protocol::SourceMaintenanceLastRunSummary,
) -> protocol::SourcePreparationProcessedCounts {
    protocol::SourcePreparationProcessedCounts {
        hash: last_run.hash.hashed_count + last_run.hash.skipped_count + last_run.hash.failed_count,
        probe: last_run.probe.probed_count
            + last_run.probe.skipped_count
            + last_run.probe.failed_count,
        attachment: last_run.attachment_materialization.attachments_created
            + last_run.attachment_materialization.attachments_refreshed
            + last_run.attachment_materialization.links_created
            + last_run.attachment_materialization.links_replaced
            + last_run.attachment_materialization.links_refreshed,
        promotion: last_run.playable_media_promotion.promoted_count
            + last_run.playable_media_promotion.refreshed_count,
        identity: last_run.track_identity_candidates.candidates_created
            + last_run.track_identity_candidates.candidates_refreshed
            + last_run.track_identity_decisions.decisions_created,
    }
}

#[cfg(test)]
mod tests {
    use super::map_read_source_activity_reply;
    use crate::source_maintenance::SourceMaintenanceSnapshot;
    use library_boundary_protocol as protocol;
    use library_store_sqlite as store;

    #[test]
    fn scan_terminal_and_running_states_map_from_lifecycle_without_percentage() {
        let running = map_read_source_activity_reply(
            source_integrity(lifecycle("scanning"), coverage_complete_with_media()),
            maintenance_snapshot(),
        );
        assert_eq!(
            running.scan_activity.state,
            protocol::SourceScanActivityState::Running
        );
        assert_eq!(
            running.browse_readiness.state,
            protocol::SourceBrowseReadinessState::Indexing
        );

        let failed = map_read_source_activity_reply(
            source_integrity(lifecycle("failed"), coverage_complete_with_media()),
            maintenance_snapshot(),
        );
        assert_eq!(
            failed.scan_activity.state,
            protocol::SourceScanActivityState::Failed
        );
        assert_eq!(
            failed.browse_readiness.state,
            protocol::SourceBrowseReadinessState::Blocked
        );

        let json = serde_json::to_value(running).expect("serialize activity");
        assert!(json.pointer("/scanActivity/percentage").is_none());
    }

    #[test]
    fn completed_maintenance_with_remaining_work_is_not_complete_preparation() {
        let activity = map_read_source_activity_reply(
            source_integrity(lifecycle("complete"), coverage_complete_with_media()),
            SourceMaintenanceSnapshot {
                remaining_hash_candidates: 2,
                last_run: Some(last_run_summary()),
                ..maintenance_snapshot()
            },
        );

        assert_eq!(
            activity.preparation_activity.state,
            protocol::SourcePreparationActivityState::CompletedWithRemainingWork
        );
        assert_eq!(activity.preparation_activity.backlog.hash, 2);
        assert_eq!(
            activity.preparation_activity.provenance,
            protocol::SourcePreparationProvenance::MaintenanceSnapshot
        );
        assert_eq!(
            activity.preparation_activity.last_run_status,
            Some(protocol::SourceMaintenanceRunStatus::Completed)
        );
    }

    #[test]
    fn zero_backlog_yields_complete_preparation() {
        let activity = map_read_source_activity_reply(
            source_integrity(lifecycle("complete"), coverage_complete_with_media()),
            SourceMaintenanceSnapshot {
                last_run: Some(last_run_summary()),
                ..maintenance_snapshot()
            },
        );

        assert_eq!(
            activity.preparation_activity.state,
            protocol::SourcePreparationActivityState::Complete
        );
    }

    #[test]
    fn admission_state_distinguishes_active_restorable_and_not_admitted() {
        let active = map_read_source_activity_reply(
            source_integrity(lifecycle("complete"), coverage_complete_with_media()),
            maintenance_snapshot(),
        );
        assert_eq!(
            active.admission_state,
            protocol::SourceActivityAdmissionState::Active
        );

        let mut restorable_lifecycle = lifecycle("complete");
        restorable_lifecycle.is_user_visible = false;
        let restorable = map_read_source_activity_reply(
            source_integrity(restorable_lifecycle, coverage_complete_with_media()),
            maintenance_snapshot(),
        );
        assert_eq!(
            restorable.admission_state,
            protocol::SourceActivityAdmissionState::Restorable
        );

        let not_admitted = map_read_source_activity_reply(
            store::StoreSourceIntegrity {
                source_id: 7,
                lifecycle: None,
                coverage: store::StoreSourceIntegrityCoverage {
                    state: store::StoreSourceIntegrityCoverageState::SourceUnavailable,
                    subtree_coverage_complete: false,
                    empty_result_authoritative: false,
                    total_directories_count: 0,
                    missing_directories_count: 0,
                    pending_directories_count: 0,
                    scanning_directories_count: 0,
                    blocked_directories_count: 0,
                    failed_directories_count: 0,
                },
                inventory: None,
            },
            maintenance_snapshot(),
        );
        assert_eq!(
            not_admitted.admission_state,
            protocol::SourceActivityAdmissionState::NotAdmitted
        );
    }

    fn source_integrity(
        lifecycle: store::StoreSourceLifecycle,
        coverage: store::StoreSourceIntegrityCoverage,
    ) -> store::StoreSourceIntegrity {
        store::StoreSourceIntegrity {
            source_id: lifecycle.source_id,
            lifecycle: Some(lifecycle),
            coverage,
            inventory: Some(store::StoreSourceIntegrityInventory {
                counts_by_presence_state: Vec::new(),
                counts_by_file_class: Vec::new(),
                counts_by_file_kind: Vec::new(),
                media_relevant_files_count: 1,
                present_media_relevant_files_count: 1,
            }),
        }
    }

    fn lifecycle(scan_phase: &str) -> store::StoreSourceLifecycle {
        store::StoreSourceLifecycle {
            source_id: 7,
            source_class: "external_mounted".to_string(),
            is_user_visible: true,
            mount_status: "mounted".to_string(),
            access_state: "accessible".to_string(),
            access_issue_kind: None,
            scan_phase: scan_phase.to_string(),
            scan_issue_kind: None,
            last_scan_started_at: Some(10),
            last_scan_finished_at: if scan_phase == "scanning" {
                None
            } else {
                Some(20)
            },
            last_successful_scan_at: if scan_phase == "complete" {
                Some(20)
            } else {
                None
            },
            last_seen_at: Some(5),
            updated_at: 30,
        }
    }

    fn coverage_complete_with_media() -> store::StoreSourceIntegrityCoverage {
        store::StoreSourceIntegrityCoverage {
            state: store::StoreSourceIntegrityCoverageState::Complete,
            subtree_coverage_complete: true,
            empty_result_authoritative: true,
            total_directories_count: 1,
            missing_directories_count: 0,
            pending_directories_count: 0,
            scanning_directories_count: 0,
            blocked_directories_count: 0,
            failed_directories_count: 0,
        }
    }

    fn maintenance_snapshot() -> SourceMaintenanceSnapshot {
        SourceMaintenanceSnapshot {
            source_id: 7,
            status: protocol::SourceMaintenanceSnapshotStatus::Idle,
            remaining_hash_candidates: 0,
            remaining_probe_candidates: 0,
            remaining_playable_media_promotion_candidates: 0,
            remaining_track_identity_candidate_production_candidates: 0,
            remaining_track_identity_decision_production_candidates: 0,
            attachment_links: None,
            source_failure: None,
            last_run: None,
        }
    }

    fn last_run_summary() -> protocol::SourceMaintenanceLastRunSummary {
        protocol::SourceMaintenanceLastRunSummary {
            status: protocol::SourceMaintenanceRunStatus::Completed,
            hash: protocol::SourceMaintenanceHashSummary {
                effective_limit: 8,
                hashed_count: 1,
                skipped_count: 0,
                failed_count: 0,
                remaining_candidates: 0,
            },
            attachment_materialization:
                protocol::SourceMaintenanceAttachmentMaterializationSummary {
                    effective_limit: 4,
                    attachments_created: 0,
                    attachments_refreshed: 0,
                    links_created: 1,
                    links_replaced: 0,
                    links_refreshed: 0,
                    skipped_stale_observations: 0,
                    skipped_no_blake3: 0,
                    skipped_no_observations: 0,
                    remaining_candidates: 0,
                },
            probe: protocol::SourceMaintenanceProbeSummary {
                effective_limit: 4,
                probed_count: 1,
                skipped_count: 0,
                failed_count: 0,
                remaining_candidates: 0,
            },
            playable_media_promotion: protocol::SourceMaintenancePlayableMediaPromotionSummary {
                effective_limit: 4,
                promoted_count: 1,
                refreshed_count: 0,
                skipped_unusable_source: 0,
                skipped_unsupported_media_kind: 0,
                skipped_no_observations: 0,
                skipped_stale_observations: 0,
                skipped_no_blake3: 0,
                skipped_no_probe_observations: 0,
                skipped_missing_attachment_link: 0,
                skipped_stale_attachment_link: 0,
                remaining_candidates: 0,
            },
            track_identity_candidates: protocol::SourceMaintenanceTrackIdentityCandidateSummary {
                effective_limit: 4,
                candidates_created: 1,
                candidates_refreshed: 0,
                members_created: 0,
                members_refreshed: 0,
                evidence_created: 0,
                evidence_refreshed: 0,
                candidates_marked_stale: 0,
                skipped_stale_playable_media: 0,
                remaining_candidates: 0,
            },
            track_identity_decisions: protocol::SourceMaintenanceTrackIdentityDecisionSummary {
                effective_limit: 4,
                decisions_created: 1,
                decision_evidence_created: 0,
                skipped_stale_candidates: 0,
                skipped_existing_current_decisions: 0,
                skipped_user_blocked_candidates: 0,
                remaining_candidates: 0,
            },
            remaining_hash_candidates: 0,
            remaining_probe_candidates: 0,
            remaining_playable_media_promotion_candidates: 0,
            remaining_track_identity_candidate_production_candidates: 0,
            remaining_track_identity_decision_production_candidates: 0,
            source_failure: None,
        }
    }
}
