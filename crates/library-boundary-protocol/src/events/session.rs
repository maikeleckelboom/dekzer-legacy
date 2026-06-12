use crate::commands::SnapshotReadCommand;

use std::fmt;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum MaintainedSnapshotScope {
    /// Invalidates navigation snapshot reads:
    /// `ReadNavigationRows`, `LoadNavigationRow`, and `LoadNavigationRowByStableKey`.
    NavigationRows,
    /// Invalidates maintained source tree, source status, contents, and
    /// track identity candidate snapshot reads.
    Contents,
}

impl MaintainedSnapshotScope {
    pub fn for_snapshot_read(command: &SnapshotReadCommand) -> Option<Self> {
        match command {
            SnapshotReadCommand::ReadNavigationRows(_)
            | SnapshotReadCommand::LoadNavigationRow(_)
            | SnapshotReadCommand::LoadNavigationRowByStableKey(_) => Some(Self::NavigationRows),
            SnapshotReadCommand::ReadLibraryTreeChildren(_)
            | SnapshotReadCommand::ReadSourceLifecycle(_)
            | SnapshotReadCommand::ReadSourceIntegrity(_)
            | SnapshotReadCommand::ReadSourceMaintenance(_)
            | SnapshotReadCommand::ReadTrackIdentityReviewCandidates(_)
            | SnapshotReadCommand::ContentsRead(_) => Some(Self::Contents),
            SnapshotReadCommand::ReadSourceFileAttachment(_)
            | SnapshotReadCommand::ReadAttachmentSourceFiles(_)
            | SnapshotReadCommand::ReadSourceAttachmentSummary(_)
            | SnapshotReadCommand::SearchFilterRead(_) => None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, schemars::JsonSchema, ts_rs::TS,
)]
#[schemars(with = "String")]
#[ts(as = "String")]
pub struct MaintainedSnapshotRevision(u64);

impl MaintainedSnapshotRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

impl serde::Serialize for MaintainedSnapshotRevision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for MaintainedSnapshotRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RevisionVisitor;

        impl serde::de::Visitor<'_> for RevisionVisitor {
            type Value = MaintainedSnapshotRevision;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a maintained snapshot revision encoded as a JSON string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                crate::wire::parse_u64_string(value).map(MaintainedSnapshotRevision)
            }
        }

        deserializer.deserialize_str(RevisionVisitor)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MaintainedSnapshotScopeRevision {
    pub scope: MaintainedSnapshotScope,
    pub revision: MaintainedSnapshotRevision,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MaintainedSnapshotInvalidation {
    pub scope: MaintainedSnapshotScope,
    pub revision: Option<MaintainedSnapshotRevision>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ScanRunPhase {
    /// Ordered: the scan has been admitted and traversal is beginning.
    Scanning,
    /// The scan could not finish because the source root became unavailable
    /// or the mount changed during traversal.
    Blocked,
    /// The scan was interrupted by an explicit cancellation.
    Interrupted,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SourceScanEventKind {
    SourceScanStarted,
    SourceScanProgressed,
    SourceScanCompleted,
    SourceScanFailed,
    SourceScanBlocked,
    SourceScanCancelled,
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
pub struct SourceScanEvent {
    pub event_sequence: i64,
    pub occurred_at_ms: i64,
    pub kind: SourceScanEventKind,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub scan_run_id: i64,
    pub phase: ScanRunPhase,
    pub directories_visited: usize,
    pub files_visited: usize,
    pub files_discovered: usize,
    pub media_candidates: usize,
    pub queued_work_items: usize,
    pub detail: Option<String>,
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
pub struct MaintainedSnapshotEvent {
    pub event_sequence: i64,
    pub occurred_at_ms: i64,
    pub invalidation: MaintainedSnapshotInvalidation,
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
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum LibraryBoundaryEvent {
    SourceScanEvent(SourceScanEvent),
    MaintainedSnapshotInvalidated(MaintainedSnapshotEvent),
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryBoundaryEvent, MaintainedSnapshotEvent, MaintainedSnapshotInvalidation,
        MaintainedSnapshotRevision, MaintainedSnapshotScope,
    };
    use crate::{
        ContentsReadPolicy, ContentsReadRequest, ContentsScope, ContentsScopeDepth,
        LibraryTreeEntryPoint, LoadNavigationRowByStableKeyRequest, LoadNavigationRowRequest,
        PrimaryMediaKind, ReadAttachmentSourceFilesRequest, ReadLibraryTreeChildrenRequest,
        ReadNavigationRowsRequest, ReadSourceAttachmentSummaryRequest,
        ReadSourceFileAttachmentRequest, ReadSourceIntegrityRequest, ReadSourceLifecycleRequest,
        SnapshotReadCommand,
    };
    use serde_json::json;

    #[test]
    fn maintained_snapshot_scopes_cover_snapshot_read_commands_without_query_subscriptions() {
        let navigation_reads = [
            SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
                parent_navigation_row_id: Some(7),
            }),
            SnapshotReadCommand::LoadNavigationRow(LoadNavigationRowRequest {
                navigation_row_id: 7,
            }),
            SnapshotReadCommand::LoadNavigationRowByStableKey(
                LoadNavigationRowByStableKeyRequest {
                    stable_key: "source:7".to_string(),
                },
            ),
        ];

        for command in &navigation_reads {
            assert_eq!(
                MaintainedSnapshotScope::for_snapshot_read(command),
                Some(MaintainedSnapshotScope::NavigationRows)
            );
        }

        let contents_reads = [
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 8 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 100,
            }),
            SnapshotReadCommand::ReadSourceLifecycle(ReadSourceLifecycleRequest { source_id: 8 }),
            SnapshotReadCommand::ReadSourceIntegrity(ReadSourceIntegrityRequest { source_id: 8 }),
            SnapshotReadCommand::ContentsRead(ContentsReadRequest {
                scope: ContentsScope::Directory {
                    source_id: 8,
                    source_directory_id: 9,
                },
                policy: ContentsReadPolicy::PrimaryMedia {
                    media_kinds: vec![PrimaryMediaKind::Audio, PrimaryMediaKind::Video],
                },
                scope_depth: ContentsScopeDepth::Recursive,
                limit: Some(100),
                cursor: None,
            }),
        ];

        for command in &contents_reads {
            assert_eq!(
                MaintainedSnapshotScope::for_snapshot_read(command),
                Some(MaintainedSnapshotScope::Contents)
            );
        }

        let explicit_attachment_reads = [
            SnapshotReadCommand::ReadSourceFileAttachment(ReadSourceFileAttachmentRequest {
                source_file_id: 8,
            }),
            SnapshotReadCommand::ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest {
                attachment_id: 9,
                limit: Some(100),
            }),
            SnapshotReadCommand::ReadSourceAttachmentSummary(ReadSourceAttachmentSummaryRequest {
                source_id: 8,
            }),
        ];

        for command in &explicit_attachment_reads {
            assert_eq!(
                MaintainedSnapshotScope::for_snapshot_read(command),
                None,
                "attachment identity reads are explicit until a precise maintained scope exists"
            );
        }
    }

    #[test]
    fn maintained_invalidation_events_are_scope_based_and_payload_lightweight() {
        let event = LibraryBoundaryEvent::MaintainedSnapshotInvalidated(MaintainedSnapshotEvent {
            event_sequence: 1,
            occurred_at_ms: 1700000000000,
            invalidation: MaintainedSnapshotInvalidation {
                scope: MaintainedSnapshotScope::Contents,
                revision: Some(MaintainedSnapshotRevision::new(42)),
            },
        });

        let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(payload) = event else {
            panic!("expected maintained snapshot invalidated event");
        };

        assert_eq!(payload.event_sequence, 1);
        assert_eq!(payload.occurred_at_ms, 1700000000000);
        assert_eq!(
            payload.invalidation.scope,
            MaintainedSnapshotScope::Contents
        );
        assert_eq!(
            payload
                .invalidation
                .revision
                .map(|revision| revision.value()),
            Some(42)
        );
    }

    #[test]
    fn maintained_invalidation_event_serializes_with_tagged_shape() {
        let event = LibraryBoundaryEvent::MaintainedSnapshotInvalidated(MaintainedSnapshotEvent {
            event_sequence: 1,
            occurred_at_ms: 1700000000000,
            invalidation: MaintainedSnapshotInvalidation {
                scope: MaintainedSnapshotScope::Contents,
                revision: Some(MaintainedSnapshotRevision::new(42)),
            },
        });

        let json = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(
            json,
            json!({
                "type": "maintainedSnapshotInvalidated",
                "payload": {
                    "eventSequence": 1,
                    "occurredAtMs": 1700000000000i64,
                    "invalidation": {
                        "scope": "contents",
                        "revision": "42"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryBoundaryEvent>(json).expect("deserialize event"),
            event
        );
    }

    #[test]
    fn source_scan_event_has_all_required_bounded_counters() {
        use super::{ScanRunPhase, SourceScanEvent, SourceScanEventKind};

        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: 3,
            occurred_at_ms: 1700000000000,
            kind: SourceScanEventKind::SourceScanProgressed,
            root_id: 7,
            scan_run_id: 14,
            phase: ScanRunPhase::Scanning,
            directories_visited: 42,
            files_visited: 100,
            files_discovered: 95,
            media_candidates: 12,
            queued_work_items: 8,
            detail: None,
        });

        let LibraryBoundaryEvent::SourceScanEvent(payload) = event else {
            panic!("expected source scan event");
        };
        assert_eq!(payload.event_sequence, 3);
        assert_eq!(payload.root_id, 7);
        assert_eq!(payload.scan_run_id, 14);
        assert_eq!(payload.directories_visited, 42);
        assert_eq!(payload.files_visited, 100);
        assert_eq!(payload.files_discovered, 95);
        assert_eq!(payload.media_candidates, 12);
        assert_eq!(payload.queued_work_items, 8);
        assert_eq!(payload.detail, None);
    }

    #[test]
    fn source_scan_event_serializes_with_tagged_shape() {
        use super::{ScanRunPhase, SourceScanEvent, SourceScanEventKind};

        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: 3,
            occurred_at_ms: 1700000000000,
            kind: SourceScanEventKind::SourceScanCompleted,
            root_id: 7,
            scan_run_id: 14,
            phase: ScanRunPhase::Scanning,
            directories_visited: 42,
            files_visited: 100,
            files_discovered: 95,
            media_candidates: 12,
            queued_work_items: 8,
            detail: None,
        });

        let json = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(
            json,
            json!({
                "type": "sourceScanEvent",
                "payload": {
                    "eventSequence": 3,
                    "occurredAtMs": 1700000000000i64,
                    "kind": "sourceScanCompleted",
                    "rootId": "7",
                    "scanRunId": "14",
                    "phase": "scanning",
                    "directoriesVisited": 42,
                    "filesVisited": 100,
                    "filesDiscovered": 95,
                    "mediaCandidates": 12,
                    "queuedWorkItems": 8,
                    "detail": null
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryBoundaryEvent>(json).expect("deserialize event"),
            event
        );
    }

    #[test]
    fn source_scan_blocked_event_carries_detail() {
        use super::{ScanRunPhase, SourceScanEvent, SourceScanEventKind};

        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: 5,
            occurred_at_ms: 1700000000000,
            kind: SourceScanEventKind::SourceScanBlocked,
            root_id: 7,
            scan_run_id: 14,
            phase: ScanRunPhase::Blocked,
            directories_visited: 10,
            files_visited: 30,
            files_discovered: 25,
            media_candidates: 3,
            queued_work_items: 0,
            detail: Some("source root is not accessible".to_string()),
        });

        let LibraryBoundaryEvent::SourceScanEvent(payload) = event else {
            panic!("expected source scan event");
        };
        assert_eq!(payload.phase, ScanRunPhase::Blocked);
        assert_eq!(
            payload.detail.as_deref(),
            Some("source root is not accessible")
        );
    }

    #[test]
    fn source_scan_cancelled_event_serializes_and_deserializes() {
        use super::{ScanRunPhase, SourceScanEvent, SourceScanEventKind};

        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: 7,
            occurred_at_ms: 1700000000000,
            kind: SourceScanEventKind::SourceScanCancelled,
            root_id: 7,
            scan_run_id: 14,
            phase: ScanRunPhase::Interrupted,
            directories_visited: 50,
            files_visited: 120,
            files_discovered: 100,
            media_candidates: 20,
            queued_work_items: 3,
            detail: Some("cancelled by user".to_string()),
        });

        let json = serde_json::to_value(&event).expect("serialize cancelled event");
        assert_eq!(
            json,
            serde_json::json!({
                "type": "sourceScanEvent",
                "payload": {
                    "eventSequence": 7,
                    "occurredAtMs": 1700000000000i64,
                    "kind": "sourceScanCancelled",
                    "rootId": "7",
                    "scanRunId": "14",
                    "phase": "interrupted",
                    "directoriesVisited": 50,
                    "filesVisited": 120,
                    "filesDiscovered": 100,
                    "mediaCandidates": 20,
                    "queuedWorkItems": 3,
                    "detail": "cancelled by user"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryBoundaryEvent>(json)
                .expect("deserialize cancelled event"),
            event
        );

        let LibraryBoundaryEvent::SourceScanEvent(payload) = event else {
            panic!("expected source scan event");
        };
        assert_eq!(payload.kind, SourceScanEventKind::SourceScanCancelled);
        assert_eq!(payload.phase, ScanRunPhase::Interrupted);
    }

    #[test]
    fn maintained_snapshot_revision_serializes_as_string_marker() {
        let revision = MaintainedSnapshotRevision::new(99);

        let json = serde_json::to_value(revision).expect("serialize revision");
        assert_eq!(json, json!("99"));
        assert_eq!(
            serde_json::from_value::<MaintainedSnapshotRevision>(json)
                .expect("deserialize revision"),
            revision
        );
    }

    #[test]
    fn maintained_snapshot_revision_rejects_json_numbers() {
        assert!(
            serde_json::from_value::<MaintainedSnapshotRevision>(json!(99)).is_err(),
            "snapshot revisions must cross the wire as strings"
        );
    }
}
