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
pub struct RegisterLocalRootRequest {
    pub absolute_path: String,
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
pub struct RegisterLocalRootReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
    pub canonical_path: String,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StartRootScanRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
/// Returned immediately after a root scan job is admitted.
/// The scan continues in the background and publishes progress
/// through `SourceScanEvent` lifecycle events.
pub struct StartRootScanReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub scan_run_id: i64,
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
pub struct ReadLocalRootsRequest;

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
pub struct ReadLocalRootsReply {
    pub roots: Vec<LocalRoot>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UnregisterLocalRootRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UnregisterLocalRootReply {
    pub unregistered: bool,
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
pub enum LocalRootAvailability {
    Available,
    Unavailable,
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
pub struct LocalRoot {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
    pub canonical_path: String,
    pub availability: LocalRootAvailability,
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
pub enum LibraryRootCommand {
    RegisterLocalRoot(RegisterLocalRootRequest),
    StartRootScan(StartRootScanRequest),
    ReadLocalRoots(ReadLocalRootsRequest),
    UnregisterLocalRoot(UnregisterLocalRootRequest),
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
pub enum LibraryRootReply {
    RegisterLocalRoot(RegisterLocalRootReply),
    StartRootScan(StartRootScanReply),
    ReadLocalRoots(ReadLocalRootsReply),
    UnregisterLocalRoot(UnregisterLocalRootReply),
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryRootCommand, LibraryRootReply, LocalRoot, LocalRootAvailability,
        ReadLocalRootsReply, ReadLocalRootsRequest, RegisterLocalRootReply,
        RegisterLocalRootRequest, StartRootScanReply, StartRootScanRequest, UnregisterLocalRootReply,
        UnregisterLocalRootRequest,
    };
    use serde_json::json;

    #[test]
    fn library_root_commands_are_explicit_tagged_boundary_operations() {
        let register = LibraryRootCommand::RegisterLocalRoot(RegisterLocalRootRequest {
            absolute_path: "C:/Music".to_string(),
        });
        let scan = LibraryRootCommand::StartRootScan(StartRootScanRequest { root_id: 7 });
        let read_local = LibraryRootCommand::ReadLocalRoots(ReadLocalRootsRequest);
        let unregister =
            LibraryRootCommand::UnregisterLocalRoot(UnregisterLocalRootRequest { root_id: 42 });

        assert_eq!(
            serde_json::to_value(&register).expect("serialize register command"),
            json!({
                "type": "registerLocalRoot",
                "payload": {
                    "absolutePath": "C:/Music"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&scan).expect("serialize scan command"),
            json!({
                "type": "startRootScan",
                "payload": {
                    "rootId": "7"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&read_local).expect("serialize read local roots command"),
            json!({
                "type": "readLocalRoots",
                "payload": null
            })
        );
        assert_eq!(
            serde_json::to_value(&unregister).expect("serialize unregister command"),
            json!({
                "type": "unregisterLocalRoot",
                "payload": {
                    "rootId": "42"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryRootCommand>(
                serde_json::to_value(register.clone()).expect("serialize register")
            )
            .expect("deserialize register"),
            register
        );
        assert_eq!(
            serde_json::from_value::<LibraryRootCommand>(
                serde_json::to_value(unregister.clone()).expect("serialize unregister")
            )
            .expect("deserialize unregister"),
            unregister
        );
    }

    #[test]
    fn library_root_replies_keep_durable_ids_as_strings() {
        let registered = LibraryRootReply::RegisterLocalRoot(RegisterLocalRootReply {
            root_id: 7,
            canonical_path: "C:/Music".to_string(),
        });
        let scanned = LibraryRootReply::StartRootScan(StartRootScanReply {
            scan_run_id: 1000,
        });
        let read_local = LibraryRootReply::ReadLocalRoots(ReadLocalRootsReply {
            roots: vec![LocalRoot {
                root_id: 3,
                canonical_path: "C:/Music".to_string(),
                availability: LocalRootAvailability::Available,
            }],
        });
        let unregistered =
            LibraryRootReply::UnregisterLocalRoot(UnregisterLocalRootReply { unregistered: true });

        assert_eq!(
            serde_json::to_value(&registered).expect("serialize register reply"),
            json!({
                "type": "registerLocalRoot",
                "payload": {
                    "rootId": "7",
                    "canonicalPath": "C:/Music"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&scanned).expect("serialize scan reply"),
            json!({
                "type": "startRootScan",
                "payload": {
                    "scanRunId": "1000"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&read_local).expect("serialize read local roots reply"),
            json!({
                "type": "readLocalRoots",
                "payload": {
                    "roots": [{
                        "rootId": "3",
                        "canonicalPath": "C:/Music",
                        "availability": "available"
                    }]
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&unregistered).expect("serialize unregister reply"),
            json!({
                "type": "unregisterLocalRoot",
                "payload": {
                    "unregistered": true
                }
            })
        );
    }
}
