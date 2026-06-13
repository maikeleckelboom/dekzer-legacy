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
pub struct CancelRootScanRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub scan_run_id: i64,
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
/// Discrimination of the cancel request outcome.
/// Every variant describes a distinct renderer state
/// and must not be collapsed into a generic not-active result.
pub enum CancelRootScanStatus {
    /// The cancellation token was set on the active job.
    Accepted,
    /// The scanRunId is unknown or unregistered.
    NotFound,
    /// The job has already reached a terminal state
    /// (completed, cancelled, failed, or blocked).
    AlreadyTerminal,
    /// The job exists but cooperative cancellation has
    /// not been implemented yet for the current scan phase.
    NotCancelable,
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
pub struct CancelRootScanReply {
    pub status: CancelRootScanStatus,
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
pub struct RegisterLocalRootRequest {
    pub requested_path: String,
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
pub struct RegisteredLocalRoot {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub root_id: i64,
    pub admitted_root_path: String,
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
pub enum SourceRegistrationRootClass {
    NormalMusicRoot,
    BroadDriveRoot,
    SystemVolumeRoot,
    UserProfileRoot,
    CloudBackedRoot,
    NetworkRoot,
    ProtectedRoot,
    IndirectionRoot,
    UnknownRoot,
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
pub struct SourceRegistrationProposalRequired {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub proposal_id: i64,
    pub root_class: SourceRegistrationRootClass,
    pub requested_path: String,
    pub resolved_path: Option<String>,
    pub confirmation_required_reason: String,
    pub suggested_root_paths: Vec<String>,
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
pub struct SourceRegistrationRejected {
    pub root_class: SourceRegistrationRootClass,
    pub requested_path: String,
    pub resolved_path: Option<String>,
    pub rejection_reason: String,
    pub suggested_root_paths: Vec<String>,
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
pub enum RegisterLocalRootReply {
    Registered(RegisteredLocalRoot),
    ProposalRequired(SourceRegistrationProposalRequired),
    Rejected(SourceRegistrationRejected),
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
    pub admitted_root_path: String,
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
    CancelRootScan(CancelRootScanRequest),
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
    CancelRootScan(CancelRootScanReply),
}

#[cfg(test)]
mod tests {
    use super::{
        CancelRootScanReply, CancelRootScanRequest, CancelRootScanStatus, LibraryRootCommand,
        LibraryRootReply, LocalRoot, LocalRootAvailability, ReadLocalRootsReply,
        ReadLocalRootsRequest, RegisterLocalRootReply, RegisterLocalRootRequest,
        RegisteredLocalRoot, SourceRegistrationProposalRequired, SourceRegistrationRejected,
        SourceRegistrationRootClass, StartRootScanReply, StartRootScanRequest,
        UnregisterLocalRootReply, UnregisterLocalRootRequest,
    };
    use serde_json::json;

    #[test]
    fn library_root_commands_are_explicit_tagged_boundary_operations() {
        let register = LibraryRootCommand::RegisterLocalRoot(RegisterLocalRootRequest {
            requested_path: "C:/Music".to_string(),
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
                    "requestedPath": "C:/Music"
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
        let registered = LibraryRootReply::RegisterLocalRoot(RegisterLocalRootReply::Registered(
            RegisteredLocalRoot {
                root_id: 7,
                admitted_root_path: "C:/Music".to_string(),
            },
        ));
        let proposal = LibraryRootReply::RegisterLocalRoot(
            RegisterLocalRootReply::ProposalRequired(SourceRegistrationProposalRequired {
                proposal_id: 11,
                root_class: SourceRegistrationRootClass::SystemVolumeRoot,
                requested_path: "C:/".to_string(),
                resolved_path: Some("C:/".to_string()),
                confirmation_required_reason: "system volume roots require scan-plan confirmation"
                    .to_string(),
                suggested_root_paths: vec!["C:/Users/Maikel/Music".to_string()],
            }),
        );
        let rejected = LibraryRootReply::RegisterLocalRoot(RegisterLocalRootReply::Rejected(
            SourceRegistrationRejected {
                root_class: SourceRegistrationRootClass::ProtectedRoot,
                requested_path: "C:/Windows".to_string(),
                resolved_path: Some("C:/Windows".to_string()),
                rejection_reason: "protected roots cannot be registered as sources".to_string(),
                suggested_root_paths: vec!["C:/Users/Maikel/Music".to_string()],
            },
        ));
        let scanned = LibraryRootReply::StartRootScan(StartRootScanReply { scan_run_id: 1000 });
        let read_local = LibraryRootReply::ReadLocalRoots(ReadLocalRootsReply {
            roots: vec![LocalRoot {
                root_id: 3,
                admitted_root_path: "C:/Music".to_string(),
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
                    "type": "registered",
                    "payload": {
                        "rootId": "7",
                        "admittedRootPath": "C:/Music"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&proposal).expect("serialize proposal reply"),
            json!({
                "type": "registerLocalRoot",
                "payload": {
                    "type": "proposalRequired",
                    "payload": {
                        "proposalId": "11",
                        "rootClass": "systemVolumeRoot",
                        "requestedPath": "C:/",
                        "resolvedPath": "C:/",
                        "confirmationRequiredReason": "system volume roots require scan-plan confirmation",
                        "suggestedRootPaths": ["C:/Users/Maikel/Music"]
                    }
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&rejected).expect("serialize rejected reply"),
            json!({
                "type": "registerLocalRoot",
                "payload": {
                    "type": "rejected",
                    "payload": {
                        "rootClass": "protectedRoot",
                        "requestedPath": "C:/Windows",
                        "resolvedPath": "C:/Windows",
                        "rejectionReason": "protected roots cannot be registered as sources",
                        "suggestedRootPaths": ["C:/Users/Maikel/Music"]
                    }
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
                        "admittedRootPath": "C:/Music",
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

    #[test]
    fn cancel_root_scan_command_serializes_with_tagged_shape() {
        let cancel = LibraryRootCommand::CancelRootScan(CancelRootScanRequest { scan_run_id: 42 });

        assert_eq!(
            serde_json::to_value(&cancel).expect("serialize cancel command"),
            json!({
                "type": "cancelRootScan",
                "payload": {
                    "scanRunId": "42"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<LibraryRootCommand>(
                serde_json::to_value(cancel.clone()).expect("serialize")
            )
            .expect("deserialize cancel command"),
            cancel
        );
    }

    #[test]
    fn cancel_root_scan_reply_serializes_each_status_variant() {
        let accepted = LibraryRootReply::CancelRootScan(CancelRootScanReply {
            status: CancelRootScanStatus::Accepted,
        });
        let not_found = LibraryRootReply::CancelRootScan(CancelRootScanReply {
            status: CancelRootScanStatus::NotFound,
        });
        let already_terminal = LibraryRootReply::CancelRootScan(CancelRootScanReply {
            status: CancelRootScanStatus::AlreadyTerminal,
        });
        let not_cancelable = LibraryRootReply::CancelRootScan(CancelRootScanReply {
            status: CancelRootScanStatus::NotCancelable,
        });

        assert_eq!(
            serde_json::to_value(&accepted).expect("serialize accepted"),
            json!({
                "type": "cancelRootScan",
                "payload": {
                    "status": "accepted"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&not_found).expect("serialize notFound"),
            json!({
                "type": "cancelRootScan",
                "payload": {
                    "status": "notFound"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&already_terminal).expect("serialize alreadyTerminal"),
            json!({
                "type": "cancelRootScan",
                "payload": {
                    "status": "alreadyTerminal"
                }
            })
        );
        assert_eq!(
            serde_json::to_value(&not_cancelable).expect("serialize notCancelable"),
            json!({
                "type": "cancelRootScan",
                "payload": {
                    "status": "notCancelable"
                }
            })
        );

        assert_eq!(
            serde_json::from_value::<LibraryRootReply>(
                serde_json::to_value(accepted.clone()).expect("serialize")
            )
            .expect("deserialize accepted"),
            accepted
        );
        assert_eq!(
            serde_json::from_value::<LibraryRootReply>(
                serde_json::to_value(not_found.clone()).expect("serialize")
            )
            .expect("deserialize notFound"),
            not_found
        );
    }
}
