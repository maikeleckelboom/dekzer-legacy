pub mod library_roots;
pub mod playlist_writes;
pub mod session_events;
pub mod snapshot_reads;
pub mod source_file_hash;
pub mod source_maintenance;
pub mod track_identity_decisions;

pub use library_roots::*;
pub use playlist_writes::*;
pub use session_events::*;
pub use snapshot_reads::*;
pub use source_file_hash::*;
pub use source_maintenance::*;
pub use track_identity_decisions::*;

use crate::ProtocolError;

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum CommandRequest {
    LibraryBoundaryEvents(LibraryBoundaryEventStreamCommand),
    LibraryRoots(LibraryRootCommand),
    PlaylistWrite(PlaylistWriteCommand),
    SourceFileHash(SourceFileHashCommand),
    SourceMaintenance(SourceMaintenanceCommand),
    TrackIdentityDecisions(TrackIdentityDecisionCommand),
    SnapshotRead(SnapshotReadCommand),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum CommandReply {
    LibraryBoundaryEvents(LibraryBoundaryEventStreamReply),
    LibraryRoots(LibraryRootReply),
    PlaylistWrite(PlaylistWriteReply),
    SourceFileHash(SourceFileHashReply),
    SourceMaintenance(Box<SourceMaintenanceReply>),
    TrackIdentityDecisions(TrackIdentityDecisionReply),
    SnapshotRead(SnapshotReadReply),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum CommandOutcome {
    Success(CommandSuccessEnvelope),
    Error(CommandErrorEnvelope),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CommandSuccessEnvelope {
    pub reply: CommandReply,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CommandErrorEnvelope {
    pub error: ProtocolError,
}

#[cfg(test)]
mod tests {
    use super::{
        CommandErrorEnvelope, CommandOutcome, CommandReply, CommandRequest, CommandSuccessEnvelope,
        CreatePlaylistReply, CreatePlaylistRequest, LibraryBoundaryEventStreamCommand,
        LibraryBoundaryEventStreamReply, LibraryRootCommand, LibraryRootReply,
        PlaylistWriteCommand, PlaylistWriteReply, ProtocolError,
        ReadLibraryBoundaryEventsAfterRequest, ReadNavigationNodeLibraryBrowserWindowRequest,
        ReadNavigationRowsReply, SnapshotReadCommand, SnapshotReadReply, SourceFileHashCommand,
        SourceFileHashReply, SourceMaintenanceCommand, SourceMaintenanceReply, StartRootScanReply,
        StartRootScanRequest, TrackIdentityDecisionCommand, TrackIdentityDecisionCommandResult,
        TrackIdentityDecisionCommandSuccess, TrackIdentityDecisionReply,
        TrackIdentityDecisionState, TrackIdentityEffectiveDecisionCurrentStatus,
        TrackIdentityEffectiveDecisionPrecedence, TrackIdentityEffectiveDecisionSummary,
        TrackIdentityUserBlockingDecisionState,
    };
    use serde_json::json;

    #[test]
    fn command_center_routes_maintained_and_playlist_command_families() {
        let session_events = CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: None,
                max_events: 32,
            }),
        );
        let playlist_write = CommandRequest::PlaylistWrite(PlaylistWriteCommand::CreatePlaylist(
            CreatePlaylistRequest {
                display_name: "Set".to_string(),
            },
        ));
        let library_roots =
            CommandRequest::LibraryRoots(LibraryRootCommand::StartRootScan(StartRootScanRequest {
                root_id: 9,
            }));
        let snapshot = CommandRequest::SnapshotRead(
            SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
                ReadNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 7,
                    offset: 0,
                    limit: 100,
                },
            ),
        );
        let hash = CommandRequest::SourceFileHash(SourceFileHashCommand::HashSourceFilesBlake3(
            super::HashSourceFilesBlake3Request {
                source_id: 7,
                limit: Some(16),
            },
        ));
        let maintenance = CommandRequest::SourceMaintenance(
            SourceMaintenanceCommand::RunSourceMaintenance(super::RunSourceMaintenanceRequest {
                source_id: 7,
                hash_limit: Some(8),
                attachment_limit: Some(4),
                probe_limit: Some(4),
                promotion_limit: Some(4),
                identity_candidate_limit: Some(4),
                identity_decision_limit: Some(4),
            }),
        );
        let identity_decision_command = CommandRequest::TrackIdentityDecisions(
            TrackIdentityDecisionCommand::AcceptTrackIdentityCandidate(
                super::AcceptTrackIdentityCandidateRequest {
                    candidate_id: 7,
                    reason: None,
                },
            ),
        );

        for command in [
            session_events,
            playlist_write,
            library_roots,
            snapshot,
            hash,
            maintenance,
            identity_decision_command,
        ] {
            match command {
                CommandRequest::LibraryBoundaryEvents(_) => {}
                CommandRequest::LibraryRoots(_) => {}
                CommandRequest::PlaylistWrite(_) => {}
                CommandRequest::SourceFileHash(_) => {}
                CommandRequest::SourceMaintenance(_) => {}
                CommandRequest::TrackIdentityDecisions(_) => {}
                CommandRequest::SnapshotRead(_) => {}
            }
        }
    }

    #[test]
    fn command_request_uses_stable_tagged_camel_case_shape() {
        let command = CommandRequest::SnapshotRead(SnapshotReadCommand::ReadNavigationRows(
            super::ReadNavigationRowsRequest {
                parent_navigation_row_id: Some(7),
            },
        ));

        let json = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(
            json,
            json!({
                "type": "snapshotRead",
                "payload": {
                    "type": "readNavigationRows",
                    "payload": {
                        "parentNavigationRowId": "7"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<CommandRequest>(json).expect("deserialize command"),
            command
        );

        let hash_command = CommandRequest::SourceFileHash(
            SourceFileHashCommand::HashSourceFilesBlake3(super::HashSourceFilesBlake3Request {
                source_id: 7,
                limit: None,
            }),
        );
        assert_eq!(
            serde_json::to_value(&hash_command).expect("serialize hash command"),
            json!({
                "type": "sourceFileHash",
                "payload": {
                    "type": "hashSourceFilesBlake3",
                    "payload": {
                        "sourceId": "7"
                    }
                }
            })
        );

        let maintenance_command = CommandRequest::SourceMaintenance(
            SourceMaintenanceCommand::RunSourceMaintenance(super::RunSourceMaintenanceRequest {
                source_id: 7,
                hash_limit: Some(8),
                attachment_limit: Some(4),
                probe_limit: None,
                promotion_limit: None,
                identity_candidate_limit: None,
                identity_decision_limit: None,
            }),
        );
        assert_eq!(
            serde_json::to_value(&maintenance_command).expect("serialize maintenance command"),
            json!({
                "type": "sourceMaintenance",
                "payload": {
                    "type": "runSourceMaintenance",
                    "payload": {
                        "sourceId": "7",
                        "hashLimit": 8,
                        "attachmentLimit": 4
                    }
                }
            })
        );

        let decision_command = CommandRequest::TrackIdentityDecisions(
            TrackIdentityDecisionCommand::RejectTrackIdentityCandidate(
                super::RejectTrackIdentityCandidateRequest {
                    candidate_id: 7,
                    reason: Some("not the same item".to_string()),
                },
            ),
        );
        assert_eq!(
            serde_json::to_value(&decision_command).expect("serialize decision command"),
            json!({
                "type": "trackIdentityDecisions",
                "payload": {
                    "type": "rejectTrackIdentityCandidate",
                    "payload": {
                        "candidateId": "7",
                        "reason": "not the same item"
                    }
                }
            })
        );
    }

    #[test]
    fn command_reply_uses_same_tagged_family_shape() {
        let reply =
            CommandReply::LibraryRoots(LibraryRootReply::StartRootScan(StartRootScanReply {
                scan_run_id: 9,
            }));

        let json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(
            json,
            json!({
                "type": "libraryRoots",
                "payload": {
                    "type": "startRootScan",
                    "payload": {
                        "scanRunId": "9",
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<CommandReply>(json).expect("deserialize reply"),
            reply
        );

        let hash_reply = CommandReply::SourceFileHash(SourceFileHashReply::HashSourceFilesBlake3(
            super::HashSourceFilesBlake3Reply {
                effective_limit: 32,
                outcomes: Vec::new(),
                hashed_count: 0,
                skipped_count: 0,
                failed_count: 0,
                remaining_candidates: 0,
                source_failure: None,
            },
        ));
        assert_eq!(
            serde_json::to_value(&hash_reply).expect("serialize hash reply"),
            json!({
                "type": "sourceFileHash",
                "payload": {
                    "type": "hashSourceFilesBlake3",
                    "payload": {
                        "effectiveLimit": 32,
                        "outcomes": [],
                        "hashedCount": 0,
                        "skippedCount": 0,
                        "failedCount": 0,
                        "remainingCandidates": 0
                    }
                }
            })
        );

        let maintenance_reply = CommandReply::SourceMaintenance(Box::new(
            SourceMaintenanceReply::RunSourceMaintenance(super::RunSourceMaintenanceReply {
                source_id: 7,
                status: super::SourceMaintenanceRunStatus::Completed,
                effective_limits: super::SourceMaintenanceEffectiveLimits {
                    hash_limit: 8,
                    attachment_limit: 4,
                    probe_limit: 4,
                    promotion_limit: 4,
                    identity_candidate_limit: 4,
                    identity_decision_limit: 4,
                },
                hash: super::SourceMaintenanceHashSummary {
                    effective_limit: 8,
                    hashed_count: 1,
                    skipped_count: 0,
                    failed_count: 0,
                    remaining_candidates: 0,
                },
                attachment_materialization:
                    super::SourceMaintenanceAttachmentMaterializationSummary {
                        effective_limit: 4,
                        attachments_created: 1,
                        attachments_refreshed: 0,
                        links_created: 1,
                        links_replaced: 0,
                        links_refreshed: 0,
                        skipped_stale_facts: 0,
                        skipped_no_blake3: 0,
                        skipped_no_facts: 0,
                        remaining_candidates: 0,
                    },
                probe: super::SourceMaintenanceProbeSummary {
                    effective_limit: 4,
                    probed_count: 1,
                    skipped_count: 0,
                    failed_count: 0,
                    remaining_candidates: 0,
                },
                primary_media_promotion: super::SourceMaintenancePrimaryMediaPromotionSummary {
                    effective_limit: 4,
                    promoted_count: 1,
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
                track_identity_candidates: super::SourceMaintenanceTrackIdentityCandidateSummary {
                    effective_limit: 4,
                    candidates_created: 1,
                    candidates_refreshed: 0,
                    members_created: 1,
                    members_refreshed: 0,
                    evidence_created: 1,
                    evidence_refreshed: 0,
                    candidates_marked_stale: 0,
                    skipped_stale_primary_media_candidates: 0,
                    remaining_candidates: 0,
                },
                track_identity_decisions: super::SourceMaintenanceTrackIdentityDecisionSummary {
                    effective_limit: 4,
                    decisions_created: 1,
                    decision_evidence_created: 1,
                    skipped_stale_candidates: 0,
                    skipped_existing_current_decisions: 0,
                    skipped_user_blocked_candidates: 0,
                    remaining_candidates: 0,
                },
                remaining_hash_candidates: 0,
                remaining_probe_candidates: 0,
                remaining_primary_media_promotion_candidates: 0,
                remaining_track_identity_candidate_production_candidates: 0,
                remaining_track_identity_decision_production_candidates: 0,
                attachment_links: None,
                source_failure: None,
            }),
        ));
        let json = serde_json::to_value(&maintenance_reply).expect("serialize maintenance reply");
        assert_eq!(json["type"], json!("sourceMaintenance"));
        assert_eq!(json["payload"]["type"], json!("runSourceMaintenance"));
        assert_eq!(json["payload"]["payload"]["sourceId"], json!("7"));
        assert_eq!(
            serde_json::from_value::<CommandReply>(json).expect("deserialize maintenance reply"),
            maintenance_reply
        );

        let decision_command_reply = CommandReply::TrackIdentityDecisions(
            TrackIdentityDecisionReply::AcceptTrackIdentityCandidate(
                TrackIdentityDecisionCommandResult::Written(TrackIdentityDecisionCommandSuccess {
                    decision_id: 9,
                    candidate_id: 7,
                    decision_state: TrackIdentityDecisionState::Accepted,
                    decision_source: "user_local_v0".to_string(),
                    evidence_snapshot_count: 1,
                    decision_created: true,
                    effective_decision: TrackIdentityEffectiveDecisionSummary {
                        effective_decision_id: Some(9),
                        effective_decision_state: Some(TrackIdentityDecisionState::Accepted),
                        effective_decision_source: Some("user_local_v0".to_string()),
                        effective_decision_current_status:
                            TrackIdentityEffectiveDecisionCurrentStatus::Current,
                        effective_decision_precedence:
                            TrackIdentityEffectiveDecisionPrecedence::User,
                        user_blocking_decision_state: TrackIdentityUserBlockingDecisionState::None,
                        masked_system_decision_id: None,
                    },
                }),
            ),
        );
        let json = serde_json::to_value(&decision_command_reply)
            .expect("serialize decision command reply");
        assert_eq!(json["type"], json!("trackIdentityDecisions"));
        assert_eq!(
            json["payload"]["type"],
            json!("acceptTrackIdentityCandidate")
        );
        assert_eq!(json["payload"]["payload"]["type"], json!("written"));
        assert_eq!(
            serde_json::from_value::<CommandReply>(json)
                .expect("deserialize decision command reply"),
            decision_command_reply
        );
    }

    #[test]
    fn event_stream_reply_round_trips_through_outer_reply_family() {
        let reply =
            CommandReply::LibraryBoundaryEvents(LibraryBoundaryEventStreamReply::ReadAfter(
                super::ReadLibraryBoundaryEventsAfterReply {
                    events: Vec::new(),
                    latest_event_sequence: Some(0),
                    earliest_retained_sequence: Some(0),
                    gap_detected: false,
                },
            ));

        let json = serde_json::to_value(&reply).expect("serialize event reply");
        assert_eq!(
            json,
            json!({
                "type": "libraryBoundaryEvents",
                "payload": {
                    "type": "readAfter",
                    "payload": {
                        "events": [],
                        "latestEventSequence": 0,
                        "earliestRetainedSequence": 0,
                        "gapDetected": false
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<CommandReply>(json).expect("deserialize event reply"),
            reply
        );
    }

    #[test]
    fn command_outcome_uses_one_explicit_success_error_envelope() {
        let success = CommandOutcome::Success(CommandSuccessEnvelope {
            reply: CommandReply::PlaylistWrite(PlaylistWriteReply::CreatePlaylist(
                CreatePlaylistReply { playlist_id: 9 },
            )),
        });
        let error = CommandOutcome::Error(CommandErrorEnvelope {
            error: ProtocolError::InvalidRequest {
                detail: "missing command".to_string(),
            },
        });

        assert_eq!(
            serde_json::to_value(&success).expect("serialize success outcome"),
            json!({
                "type": "success",
                "payload": {
                    "reply": {
                        "type": "playlistWrite",
                        "payload": {
                            "type": "createPlaylist",
                            "payload": {
                                "playlistId": "9"
                            }
                        }
                    }
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&error).expect("serialize error outcome"),
            json!({
                "type": "error",
                "payload": {
                    "error": {
                        "type": "invalidRequest",
                        "payload": {
                            "detail": "missing command"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn snapshot_reply_family_round_trips() {
        let reply = SnapshotReadReply::NavigationRows(ReadNavigationRowsReply { rows: Vec::new() });

        let json = serde_json::to_value(&reply).expect("serialize snapshot reply");
        assert_eq!(
            json,
            json!({
                "type": "navigationRows",
                "payload": {
                    "rows": []
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize snapshot reply"),
            reply
        );
    }
}
