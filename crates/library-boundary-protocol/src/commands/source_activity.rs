use super::{SourceMaintenanceRunStatus, SourceMaintenanceSourceFailure};

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
pub struct ReadSourceActivityRequest {
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
pub struct ReadSourceActivityReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub admission_state: SourceActivityAdmissionState,
    pub browse_readiness: SourceBrowseReadiness,
    pub scan_activity: SourceScanActivity,
    pub preparation_activity: SourcePreparationActivity,
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
pub enum SourceActivityAdmissionState {
    Active,
    Restorable,
    NotAdmitted,
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
pub struct SourceBrowseReadiness {
    pub state: SourceBrowseReadinessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<String>,
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
pub enum SourceBrowseReadinessState {
    NeedsScan,
    Indexing,
    Ready,
    Empty,
    Missing,
    Blocked,
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
pub struct SourceScanActivity {
    pub state: SourceScanActivityState,
    pub counters: SourceScanActivityCounters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub scan_run_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_started_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_finished_at_ms: Option<i64>,
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
pub enum SourceScanActivityState {
    Idle,
    Running,
    Completed,
    Failed,
    Blocked,
    Cancelled,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceScanActivityCounters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub directories_visited: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub files_visited: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub files_discovered: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub media_candidates: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub queued_work_items: Option<usize>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourcePreparationActivity {
    pub state: SourcePreparationActivityState,
    pub backlog: SourcePreparationBacklogCounts,
    pub provenance: SourcePreparationProvenance,
    pub bounded_batch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_run_status: Option<SourceMaintenanceRunStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_run_processed: Option<SourcePreparationProcessedCounts>,
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
pub enum SourcePreparationActivityState {
    Idle,
    Running,
    CompletedWithRemainingWork,
    Complete,
    Failed,
    Unavailable,
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
pub enum SourcePreparationProvenance {
    MaintenanceSnapshot,
    RunResult,
    IntegrityFallback,
    Unavailable,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourcePreparationBacklogCounts {
    pub hash: usize,
    pub probe: usize,
    pub attachment: usize,
    pub promotion: usize,
    pub identity: usize,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourcePreparationProcessedCounts {
    pub hash: usize,
    pub probe: usize,
    pub attachment: usize,
    pub promotion: usize,
    pub identity: usize,
}

#[cfg(test)]
mod tests {
    use super::{
        ReadSourceActivityReply, SourceActivityAdmissionState, SourceBrowseReadiness,
        SourceBrowseReadinessState, SourcePreparationActivity, SourcePreparationActivityState,
        SourcePreparationBacklogCounts, SourcePreparationProvenance, SourceScanActivity,
        SourceScanActivityCounters, SourceScanActivityState,
    };
    use serde_json::json;

    #[test]
    fn source_activity_contract_does_not_serialize_fake_percentage() {
        let reply = ReadSourceActivityReply {
            source_id: 7,
            admission_state: SourceActivityAdmissionState::Active,
            browse_readiness: SourceBrowseReadiness {
                state: SourceBrowseReadinessState::Indexing,
                detail: Some("Scan is running.".to_string()),
            },
            scan_activity: SourceScanActivity {
                state: SourceScanActivityState::Running,
                counters: SourceScanActivityCounters {
                    files_discovered: Some(5),
                    ..SourceScanActivityCounters::default()
                },
                detail: None,
                scan_run_id: None,
                last_started_at_ms: Some(100),
                last_finished_at_ms: None,
            },
            preparation_activity: SourcePreparationActivity {
                state: SourcePreparationActivityState::Idle,
                backlog: SourcePreparationBacklogCounts::default(),
                provenance: SourcePreparationProvenance::MaintenanceSnapshot,
                bounded_batch: true,
                last_run_status: None,
                last_run_processed: None,
                source_failure: None,
            },
        };

        let json = serde_json::to_value(reply).expect("serialize source activity");
        assert_eq!(json["sourceId"], json!("7"));
        assert_eq!(
            json["scanActivity"]["counters"]["filesDiscovered"],
            json!(5)
        );
        assert!(json.pointer("/scanActivity/percentage").is_none());
        assert!(json.pointer("/preparationActivity/percentage").is_none());
    }

    #[test]
    fn source_activity_provenance_serializes_distinct_sources() {
        assert_eq!(
            serde_json::to_value(SourcePreparationProvenance::MaintenanceSnapshot)
                .expect("serialize maintenance provenance"),
            json!("maintenanceSnapshot")
        );
        assert_eq!(
            serde_json::to_value(SourcePreparationProvenance::RunResult)
                .expect("serialize run-result provenance"),
            json!("runResult")
        );
        assert_eq!(
            serde_json::to_value(SourcePreparationProvenance::IntegrityFallback)
                .expect("serialize integrity fallback provenance"),
            json!("integrityFallback")
        );
        assert_eq!(
            serde_json::to_value(SourcePreparationProvenance::Unavailable)
                .expect("serialize unavailable provenance"),
            json!("unavailable")
        );
    }
}
