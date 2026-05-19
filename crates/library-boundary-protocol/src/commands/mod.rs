pub mod library_roots;
pub mod playlist_writes;
pub mod session_events;
pub mod snapshot_reads;

pub use library_roots::*;
pub use playlist_writes::*;
pub use session_events::*;
pub use snapshot_reads::*;

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
    SnapshotRead(SnapshotReadReply),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
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
        PlaylistWriteCommand, PlaylistWriteReply, ProtocolError, ReadLibraryBoundaryEventsRequest,
        ReadNavigationNodeLibraryBrowserWindowRequest, ReadNavigationRowsReply, RunRootScanReply,
        RunRootScanRequest, SnapshotReadCommand, SnapshotReadReply,
    };
    use serde_json::json;

    #[test]
    fn command_center_routes_maintained_and_playlist_command_families() {
        let session_events =
            CommandRequest::LibraryBoundaryEvents(LibraryBoundaryEventStreamCommand::ReadPending(
                ReadLibraryBoundaryEventsRequest { max_events: 32 },
            ));
        let playlist_write = CommandRequest::PlaylistWrite(PlaylistWriteCommand::CreatePlaylist(
            CreatePlaylistRequest {
                display_name: "Set".to_string(),
            },
        ));
        let library_roots =
            CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(RunRootScanRequest {
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

        match session_events {
            CommandRequest::LibraryBoundaryEvents(_) => {}
            CommandRequest::LibraryRoots(_) => {}
            CommandRequest::PlaylistWrite(_) => {}
            CommandRequest::SnapshotRead(_) => {}
        }

        match playlist_write {
            CommandRequest::LibraryBoundaryEvents(_) => {}
            CommandRequest::LibraryRoots(_) => {}
            CommandRequest::PlaylistWrite(_) => {}
            CommandRequest::SnapshotRead(_) => {}
        }

        match library_roots {
            CommandRequest::LibraryBoundaryEvents(_) => {}
            CommandRequest::LibraryRoots(_) => {}
            CommandRequest::PlaylistWrite(_) => {}
            CommandRequest::SnapshotRead(_) => {}
        }

        match snapshot {
            CommandRequest::LibraryBoundaryEvents(_) => {}
            CommandRequest::LibraryRoots(_) => {}
            CommandRequest::PlaylistWrite(_) => {}
            CommandRequest::SnapshotRead(_) => {}
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
    }

    #[test]
    fn command_reply_uses_same_tagged_family_shape() {
        let reply = CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(RunRootScanReply {
            root_id: 7,
            scan_run_id: 9,
            discovered_file_count: 3,
            queued_source_work_items: 2,
        }));

        let json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(
            json,
            json!({
                "type": "libraryRoots",
                "payload": {
                    "type": "runRootScan",
                    "payload": {
                        "rootId": "7",
                        "scanRunId": "9",
                        "discoveredFileCount": 3,
                        "queuedSourceWorkItems": 2
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<CommandReply>(json).expect("deserialize reply"),
            reply
        );
    }

    #[test]
    fn event_stream_reply_round_trips_through_outer_reply_family() {
        let reply =
            CommandReply::LibraryBoundaryEvents(LibraryBoundaryEventStreamReply::ReadPending(
                super::ReadLibraryBoundaryEventsReply { events: Vec::new() },
            ));

        let json = serde_json::to_value(&reply).expect("serialize event reply");
        assert_eq!(
            json,
            json!({
                "type": "libraryBoundaryEvents",
                "payload": {
                    "type": "readPending",
                    "payload": {
                        "events": []
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
