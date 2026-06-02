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
pub struct RunSourceMaintenanceRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hash_limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment_limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub probe_limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub promotion_limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub identity_candidate_limit: Option<usize>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RunSourceMaintenanceReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub status: SourceMaintenanceRunStatus,
    pub effective_limits: SourceMaintenanceEffectiveLimits,
    pub hash: SourceMaintenanceHashSummary,
    pub attachment_materialization: SourceMaintenanceAttachmentMaterializationSummary,
    pub probe: SourceMaintenanceProbeSummary,
    pub primary_media_promotion: SourceMaintenancePrimaryMediaPromotionSummary,
    pub track_identity_candidates: SourceMaintenanceTrackIdentityCandidateSummary,
    pub remaining_hash_candidates: usize,
    pub remaining_probe_candidates: usize,
    pub remaining_primary_media_promotion_candidates: usize,
    pub remaining_track_identity_candidate_production_candidates: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment_links: Option<SourceMaintenanceAttachmentLinkSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_failure: Option<SourceMaintenanceSourceFailure>,
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
pub struct ReadSourceMaintenanceRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadSourceMaintenanceReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub status: SourceMaintenanceSnapshotStatus,
    pub remaining_hash_candidates: usize,
    pub remaining_probe_candidates: usize,
    pub remaining_primary_media_promotion_candidates: usize,
    pub remaining_track_identity_candidate_production_candidates: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment_links: Option<SourceMaintenanceAttachmentLinkSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_failure: Option<SourceMaintenanceSourceFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_run: Option<SourceMaintenanceLastRunSummary>,
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
pub struct SourceMaintenanceEffectiveLimits {
    pub hash_limit: usize,
    pub attachment_limit: usize,
    pub probe_limit: usize,
    pub promotion_limit: usize,
    pub identity_candidate_limit: usize,
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
pub struct SourceMaintenanceHashSummary {
    pub effective_limit: usize,
    pub hashed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub remaining_candidates: usize,
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
pub struct SourceMaintenanceAttachmentMaterializationSummary {
    pub effective_limit: usize,
    pub attachments_created: usize,
    pub attachments_refreshed: usize,
    pub links_created: usize,
    pub links_replaced: usize,
    pub links_refreshed: usize,
    pub skipped_stale_facts: usize,
    pub skipped_no_blake3: usize,
    pub skipped_no_facts: usize,
    pub remaining_candidates: usize,
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
pub struct SourceMaintenanceProbeSummary {
    pub effective_limit: usize,
    pub probed_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub remaining_candidates: usize,
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
pub struct SourceMaintenancePrimaryMediaPromotionSummary {
    pub effective_limit: usize,
    pub promoted_count: usize,
    pub refreshed_count: usize,
    pub skipped_unusable_source: usize,
    pub skipped_unsupported_media_kind: usize,
    pub skipped_no_facts: usize,
    pub skipped_stale_facts: usize,
    pub skipped_no_blake3: usize,
    pub skipped_no_probe_facts: usize,
    pub skipped_missing_attachment_link: usize,
    pub skipped_stale_attachment_link: usize,
    pub remaining_candidates: usize,
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
pub struct SourceMaintenanceTrackIdentityCandidateSummary {
    pub effective_limit: usize,
    pub candidates_created: usize,
    pub candidates_refreshed: usize,
    pub members_created: usize,
    pub members_refreshed: usize,
    pub evidence_created: usize,
    pub evidence_refreshed: usize,
    pub candidates_marked_stale: usize,
    pub skipped_stale_primary_media_candidates: usize,
    pub remaining_candidates: usize,
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
pub struct SourceMaintenanceAttachmentLinkSummary {
    pub current_links_count: usize,
    pub stale_links_count: usize,
    pub source_files_with_current_blake3_facts_count: usize,
    pub source_files_with_attachment_links_count: usize,
    pub source_files_missing_attachment_links_count: usize,
    pub unmaterialized_blake3_facts_count: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceMaintenanceLastRunSummary {
    pub status: SourceMaintenanceRunStatus,
    pub hash: SourceMaintenanceHashSummary,
    pub attachment_materialization: SourceMaintenanceAttachmentMaterializationSummary,
    pub probe: SourceMaintenanceProbeSummary,
    pub primary_media_promotion: SourceMaintenancePrimaryMediaPromotionSummary,
    pub track_identity_candidates: SourceMaintenanceTrackIdentityCandidateSummary,
    pub remaining_hash_candidates: usize,
    pub remaining_probe_candidates: usize,
    pub remaining_primary_media_promotion_candidates: usize,
    pub remaining_track_identity_candidate_production_candidates: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_failure: Option<SourceMaintenanceSourceFailure>,
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
pub enum SourceMaintenanceRunStatus {
    Completed,
    Partial,
    Skipped,
    Failed,
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
pub enum SourceMaintenanceSnapshotStatus {
    Idle,
    Running,
    Unavailable,
    Blocked,
    Failed,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SourceMaintenanceSourceFailure {
    SourceNotFound,
    SourceUnavailable(SourceMaintenanceSourceUnavailableFailure),
    SourceRootMissing(SourceMaintenanceSourceRootMissingFailure),
    SourceRootBlocked(SourceMaintenanceSourceRootBlockedFailure),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceMaintenanceSourceUnavailableFailure {
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
pub struct SourceMaintenanceSourceRootMissingFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceMaintenanceSourceRootBlockedFailure {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub access_issue_kind: Option<SourceLifecycleIssueKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
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
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SourceMaintenanceCommand {
    RunSourceMaintenance(RunSourceMaintenanceRequest),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SourceMaintenanceReply {
    RunSourceMaintenance(RunSourceMaintenanceReply),
}
