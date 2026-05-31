use super::{SourceAccessState, SourceLifecycleIssueKind, SourceMountStatus};

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
pub struct HashSourceFilesBlake3Request {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<usize>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3Reply {
    pub effective_limit: usize,
    pub outcomes: Vec<HashSourceFilesBlake3Outcome>,
    pub hashed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub remaining_candidates: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_failure: Option<HashSourceFilesBlake3SourceFailure>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3Outcome {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub relative_path: String,
    pub status: HashSourceFilesBlake3OutcomeStatus,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum HashSourceFilesBlake3OutcomeStatus {
    Hashed(HashSourceFilesBlake3HashedOutcome),
    Skipped(HashSourceFilesBlake3SkippedOutcome),
    Failed(HashSourceFilesBlake3FailedOutcome),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3HashedOutcome {
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub accepted_artifact_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub work_item_id: i64,
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
pub struct HashSourceFilesBlake3SkippedOutcome {
    pub reason: HashSourceFilesBlake3SkipReason,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3FailedOutcome {
    pub failure: HashSourceFilesBlake3FileFailure,
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
pub enum HashSourceFilesBlake3SkipReason {
    WorkAlreadyActive,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum HashSourceFilesBlake3SourceFailure {
    SourceNotFound,
    SourceUnavailable(HashSourceFilesBlake3SourceUnavailableFailure),
    SourceRootMissing(HashSourceFilesBlake3SourceRootMissingFailure),
    SourceRootBlocked(HashSourceFilesBlake3SourceRootBlockedFailure),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3SourceUnavailableFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mount_status: Option<SourceMountStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub access_state: Option<SourceAccessState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub access_issue_kind: Option<SourceLifecycleIssueKind>,
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
pub struct HashSourceFilesBlake3SourceRootMissingFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HashSourceFilesBlake3SourceRootBlockedFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub access_issue_kind: Option<SourceLifecycleIssueKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum HashSourceFilesBlake3FileFailure {
    SourceFileNotFound,
    SourceFileUnavailable(HashSourceFilesBlake3SourceFileUnavailableFailure),
    SourceRootUnavailable(HashSourceFilesBlake3SourceUnavailableFailure),
    SourceRootMissing(HashSourceFilesBlake3SourceRootMissingFailure),
    SourceRootBlocked(HashSourceFilesBlake3SourceRootBlockedFailure),
    InvalidRelativePath(HashSourceFilesBlake3InvalidRelativePathFailure),
    SourceFilePathEscapesRoot,
    PhysicalFileMissing,
    PhysicalFileBlocked(HashSourceFilesBlake3IoFailure),
    FileOpen(HashSourceFilesBlake3IoFailure),
    FileRead(HashSourceFilesBlake3IoFailure),
    BasisChanged,
    StoreFailure,
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
pub struct HashSourceFilesBlake3SourceFileUnavailableFailure {
    pub presence_state: String,
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
pub struct HashSourceFilesBlake3InvalidRelativePathFailure {
    pub reason: String,
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
pub struct HashSourceFilesBlake3IoFailure {
    pub detail: String,
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
pub enum SourceFileHashCommand {
    HashSourceFilesBlake3(HashSourceFilesBlake3Request),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SourceFileHashReply {
    HashSourceFilesBlake3(HashSourceFilesBlake3Reply),
}

#[cfg(test)]
mod tests {
    use super::{
        HashSourceFilesBlake3FileFailure, HashSourceFilesBlake3HashedOutcome,
        HashSourceFilesBlake3Outcome, HashSourceFilesBlake3OutcomeStatus,
        HashSourceFilesBlake3Reply, HashSourceFilesBlake3Request, HashSourceFilesBlake3SkipReason,
        HashSourceFilesBlake3SkippedOutcome, HashSourceFilesBlake3SourceFailure,
        HashSourceFilesBlake3SourceUnavailableFailure, SourceFileHashCommand, SourceFileHashReply,
    };
    use crate::{SourceAccessState, SourceLifecycleIssueKind, SourceMountStatus};
    use serde_json::json;

    #[test]
    fn hash_source_files_blake3_command_and_reply_are_tagged() {
        let command = SourceFileHashCommand::HashSourceFilesBlake3(HashSourceFilesBlake3Request {
            source_id: 7,
            limit: Some(2),
        });

        assert_eq!(
            serde_json::to_value(&command).expect("serialize command"),
            json!({
                "type": "hashSourceFilesBlake3",
                "payload": {
                    "sourceId": "7",
                    "limit": 2
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SourceFileHashCommand>(
                serde_json::to_value(command.clone()).expect("serialize")
            )
            .expect("deserialize command"),
            command
        );

        let reply = SourceFileHashReply::HashSourceFilesBlake3(HashSourceFilesBlake3Reply {
            effective_limit: 2,
            outcomes: vec![
                HashSourceFilesBlake3Outcome {
                    source_file_id: 11,
                    source_id: 7,
                    relative_path: "A.flac".to_string(),
                    status: HashSourceFilesBlake3OutcomeStatus::Hashed(
                        HashSourceFilesBlake3HashedOutcome {
                            content_hash_algorithm: "blake3".to_string(),
                            content_hash_value: "abc".to_string(),
                            accepted_artifact_id: 90,
                            work_item_id: 91,
                        },
                    ),
                },
                HashSourceFilesBlake3Outcome {
                    source_file_id: 12,
                    source_id: 7,
                    relative_path: "B.flac".to_string(),
                    status: HashSourceFilesBlake3OutcomeStatus::Skipped(
                        HashSourceFilesBlake3SkippedOutcome {
                            reason: HashSourceFilesBlake3SkipReason::WorkAlreadyActive,
                        },
                    ),
                },
            ],
            hashed_count: 1,
            skipped_count: 1,
            failed_count: 0,
            remaining_candidates: 3,
            source_failure: Some(HashSourceFilesBlake3SourceFailure::SourceUnavailable(
                HashSourceFilesBlake3SourceUnavailableFailure {
                    mount_status: Some(SourceMountStatus::Unmounted),
                    access_state: Some(SourceAccessState::Blocked),
                    access_issue_kind: Some(SourceLifecycleIssueKind::UnavailableMount),
                },
            )),
        });

        let json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(json["type"], json!("hashSourceFilesBlake3"));
        assert_eq!(json["payload"]["outcomes"][0]["sourceFileId"], json!("11"));
        assert_eq!(
            json["payload"]["outcomes"][0]["status"]["type"],
            json!("hashed")
        );
        assert_eq!(
            json["payload"]["outcomes"][1]["status"]["payload"]["reason"],
            json!("workAlreadyActive")
        );
        assert_eq!(
            json["payload"]["sourceFailure"]["type"],
            json!("sourceUnavailable")
        );
        assert_eq!(
            serde_json::from_value::<SourceFileHashReply>(json).expect("deserialize reply"),
            reply
        );
    }

    #[test]
    fn hash_source_file_failures_do_not_require_paths() {
        let failure = HashSourceFilesBlake3FileFailure::PhysicalFileMissing;

        assert_eq!(
            serde_json::to_value(&failure).expect("serialize failure"),
            json!({
                "type": "physicalFileMissing"
            })
        );
    }
}
