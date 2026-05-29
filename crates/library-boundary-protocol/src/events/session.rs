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
    /// Invalidates library browser snapshot reads, including search,
    /// node-scoped rereads, waveform overview, and selected-asset
    /// preparation detail reads.
    LibraryBrowser,
}

impl MaintainedSnapshotScope {
    pub fn for_snapshot_read(command: &SnapshotReadCommand) -> Self {
        match command {
            SnapshotReadCommand::ReadNavigationRows(_)
            | SnapshotReadCommand::LoadNavigationRow(_)
            | SnapshotReadCommand::LoadNavigationRowByStableKey(_) => Self::NavigationRows,
            SnapshotReadCommand::ReadLibraryTreeChildren(_)
            | SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(_)
            | SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(_)
            | SnapshotReadCommand::ContentsRead(_)
            | SnapshotReadCommand::ReadLibraryAssetWaveformOverview(_)
            | SnapshotReadCommand::ReadLibraryAssetPreparationDetail(_) => Self::LibraryBrowser,
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
    MaintainedSnapshotInvalidated(MaintainedSnapshotInvalidation),
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryBoundaryEvent, MaintainedSnapshotInvalidation, MaintainedSnapshotRevision,
        MaintainedSnapshotScope,
    };
    use crate::{
        ContentsMediaClass, ContentsReadPolicy, ContentsReadRequest, ContentsRecursion,
        ContentsRowProfile, ContentsScope, LibraryTreeEntryPoint,
        LoadNavigationRowByStableKeyRequest, LoadNavigationRowRequest,
        ReadLibraryAssetPreparationDetailRequest, ReadLibraryAssetWaveformOverviewRequest,
        ReadLibraryTreeChildrenRequest, ReadNavigationNodeLibraryBrowserWindowRequest,
        ReadNavigationRowsRequest, SearchNavigationNodeLibraryBrowserWindowRequest,
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
                MaintainedSnapshotScope::NavigationRows
            );
        }

        let library_asset_reads = [
            SnapshotReadCommand::ReadLibraryTreeChildren(
                ReadLibraryTreeChildrenRequest {
                    entry_point: LibraryTreeEntryPoint::Source { source_id: 8 },
                    parent_source_directory_id: None,
                    offset: 0,
                    limit: 100,
                },
            ),
            SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
                ReadNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 8,
                    offset: 0,
                    limit: 100,
                },
            ),
            SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(
                SearchNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 8,
                    query: "breakbeat".to_string(),
                    offset: 0,
                    limit: 100,
                },
            ),
            SnapshotReadCommand::ContentsRead(ContentsReadRequest {
                scope: ContentsScope::Directory {
                    source_id: 8,
                    source_directory_id: 9,
                },
                policy: ContentsReadPolicy {
                    media_classes: vec![ContentsMediaClass::Audio, ContentsMediaClass::Video],
                    row_profile: ContentsRowProfile::PrimaryMedia,
                },
                recursion: ContentsRecursion::Recursive,
                limit: Some(100),
                cursor: None,
            }),
            SnapshotReadCommand::ReadLibraryAssetWaveformOverview(
                ReadLibraryAssetWaveformOverviewRequest {
                    library_asset_id: 42,
                },
            ),
            SnapshotReadCommand::ReadLibraryAssetPreparationDetail(
                ReadLibraryAssetPreparationDetailRequest {
                    library_asset_id: 42,
                },
            ),
        ];

        for command in &library_asset_reads {
            assert_eq!(
                MaintainedSnapshotScope::for_snapshot_read(command),
                MaintainedSnapshotScope::LibraryBrowser
            );
        }
    }

    #[test]
    fn maintained_invalidation_events_are_scope_based_and_payload_lightweight() {
        let event =
            LibraryBoundaryEvent::MaintainedSnapshotInvalidated(MaintainedSnapshotInvalidation {
                scope: MaintainedSnapshotScope::LibraryBrowser,
                revision: Some(MaintainedSnapshotRevision::new(42)),
            });

        let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(invalidation) = event;

        assert_eq!(invalidation.scope, MaintainedSnapshotScope::LibraryBrowser);
        assert_eq!(
            invalidation.revision.map(|revision| revision.value()),
            Some(42)
        );
    }

    #[test]
    fn maintained_invalidation_event_serializes_with_tagged_shape() {
        let event =
            LibraryBoundaryEvent::MaintainedSnapshotInvalidated(MaintainedSnapshotInvalidation {
                scope: MaintainedSnapshotScope::LibraryBrowser,
                revision: Some(MaintainedSnapshotRevision::new(42)),
            });

        let json = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(
            json,
            json!({
                "type": "maintainedSnapshotInvalidated",
                "payload": {
                    "scope": "libraryBrowser",
                    "revision": "42"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryBoundaryEvent>(json).expect("deserialize event"),
            event
        );
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
