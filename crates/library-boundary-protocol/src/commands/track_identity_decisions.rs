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
pub struct AcceptTrackIdentityCandidateRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub candidate_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
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
pub struct RejectTrackIdentityCandidateRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub candidate_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
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
pub struct DeferTrackIdentityCandidateRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub candidate_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reason: Option<String>,
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
pub enum TrackIdentityDecisionCommandResult {
    Written(TrackIdentityDecisionCommandSuccess),
    Failed(TrackIdentityDecisionCommandFailure),
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
pub struct TrackIdentityDecisionCommandSuccess {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub decision_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub candidate_id: i64,
    pub decision_state: TrackIdentityDecisionState,
    pub decision_source: String,
    pub evidence_snapshot_count: usize,
    pub decision_created: bool,
    pub effective_decision: TrackIdentityEffectiveDecisionSummary,
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
pub enum TrackIdentityDecisionState {
    Accepted,
    Rejected,
    Deferred,
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
pub struct TrackIdentityEffectiveDecisionSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub effective_decision_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub effective_decision_state: Option<TrackIdentityDecisionState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub effective_decision_source: Option<String>,
    pub effective_decision_current_status: TrackIdentityEffectiveDecisionCurrentStatus,
    pub effective_decision_precedence: TrackIdentityEffectiveDecisionPrecedence,
    pub user_blocking_decision_state: TrackIdentityUserBlockingDecisionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub masked_system_decision_id: Option<i64>,
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
pub enum TrackIdentityEffectiveDecisionCurrentStatus {
    Current,
    Stale,
    NoCurrentDecision,
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
pub enum TrackIdentityEffectiveDecisionPrecedence {
    User,
    System,
    None,
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
pub enum TrackIdentityUserBlockingDecisionState {
    None,
    Rejected,
    Deferred,
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
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum TrackIdentityDecisionCommandFailure {
    CandidateNotFound,
    CandidateStaleForAccept,
    NoCurrentEvidenceForAccept,
    NoSourceScopeForDecision,
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
pub enum TrackIdentityDecisionCommand {
    AcceptTrackIdentityCandidate(AcceptTrackIdentityCandidateRequest),
    RejectTrackIdentityCandidate(RejectTrackIdentityCandidateRequest),
    DeferTrackIdentityCandidate(DeferTrackIdentityCandidateRequest),
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
pub enum TrackIdentityDecisionReply {
    AcceptTrackIdentityCandidate(TrackIdentityDecisionCommandResult),
    RejectTrackIdentityCandidate(TrackIdentityDecisionCommandResult),
    DeferTrackIdentityCandidate(TrackIdentityDecisionCommandResult),
}

#[cfg(test)]
mod tests {
    use super::{
        AcceptTrackIdentityCandidateRequest, TrackIdentityDecisionCommand,
        TrackIdentityDecisionCommandFailure, TrackIdentityDecisionCommandResult,
        TrackIdentityDecisionCommandSuccess, TrackIdentityDecisionReply,
        TrackIdentityDecisionState, TrackIdentityEffectiveDecisionCurrentStatus,
        TrackIdentityEffectiveDecisionPrecedence, TrackIdentityEffectiveDecisionSummary,
        TrackIdentityUserBlockingDecisionState,
    };
    use serde_json::json;

    #[test]
    fn track_identity_decisions_commands_are_candidate_scoped() {
        let command = TrackIdentityDecisionCommand::AcceptTrackIdentityCandidate(
            AcceptTrackIdentityCandidateRequest {
                candidate_id: 7,
                reason: Some("same recording".to_string()),
            },
        );

        let json = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(
            json,
            json!({
                "type": "acceptTrackIdentityCandidate",
                "payload": {
                    "candidateId": "7",
                    "reason": "same recording"
                }
            })
        );
        assert!(json.pointer("/payload/sourcePath").is_none());
        assert!(json.pointer("/payload/absolutePath").is_none());
        assert!(json.pointer("/payload/title").is_none());
        assert!(json.pointer("/payload/artist").is_none());
        assert!(json.pointer("/payload/album").is_none());

        assert_eq!(
            serde_json::from_value::<TrackIdentityDecisionCommand>(json)
                .expect("deserialize command"),
            command
        );
    }

    #[test]
    fn track_identity_decisions_reply_is_typed() {
        let reply = TrackIdentityDecisionReply::RejectTrackIdentityCandidate(
            TrackIdentityDecisionCommandResult::Written(TrackIdentityDecisionCommandSuccess {
                decision_id: 11,
                candidate_id: 7,
                decision_state: TrackIdentityDecisionState::Rejected,
                decision_source: "user_local_v0".to_string(),
                evidence_snapshot_count: 1,
                decision_created: true,
                effective_decision: TrackIdentityEffectiveDecisionSummary {
                    effective_decision_id: Some(11),
                    effective_decision_state: Some(TrackIdentityDecisionState::Rejected),
                    effective_decision_source: Some("user_local_v0".to_string()),
                    effective_decision_current_status:
                        TrackIdentityEffectiveDecisionCurrentStatus::Current,
                    effective_decision_precedence: TrackIdentityEffectiveDecisionPrecedence::User,
                    user_blocking_decision_state: TrackIdentityUserBlockingDecisionState::Rejected,
                    masked_system_decision_id: Some(10),
                },
            }),
        );

        let json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(json["type"], json!("rejectTrackIdentityCandidate"));
        assert_eq!(json["payload"]["type"], json!("written"));
        assert_eq!(json["payload"]["payload"]["decisionId"], json!("11"));
        assert_eq!(
            serde_json::from_value::<TrackIdentityDecisionReply>(json).expect("deserialize reply"),
            reply
        );

        let failure = TrackIdentityDecisionCommandResult::Failed(
            TrackIdentityDecisionCommandFailure::NoCurrentEvidenceForAccept,
        );
        assert_eq!(
            serde_json::to_value(&failure).expect("serialize failure"),
            json!({
                "type": "failed",
                "payload": {
                    "type": "noCurrentEvidenceForAccept"
                }
            })
        );
    }
}
