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
pub struct AnalyzePlayableMediaRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playable_media_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AnalyzePlayableMediaReply {
    pub result: TrackMusicalAnalysisResult,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackMusicalAnalysisResult {
    pub target: TrackMusicalAnalysisTarget,
    pub status: TrackMusicalAnalysisStatus,
    pub status_detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bpm: Option<TrackBpmEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub key: Option<TrackKeyEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub beatgrid: Option<TrackBeatgridEvidence>,
    pub warnings: Vec<TrackMusicalAnalysisWarning>,
    pub basis: TrackMusicalAnalysisBasis,
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
pub enum TrackMusicalAnalysisStatus {
    Advisory,
    Inconclusive,
    Blocked,
    Unsupported,
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
pub struct TrackMusicalAnalysisTarget {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playable_media_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub media_kind: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackBpmEvidence {
    pub bpm: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub confidence: Option<f64>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackKeyEvidence {
    pub notation: String,
    pub mode: TrackKeyMode,
    pub tonic_index: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub confidence: Option<f64>,
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
pub enum TrackKeyMode {
    Major,
    Minor,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackBeatgridEvidence {
    pub beat_count: usize,
    pub preview_seconds: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub grid_stability: Option<f64>,
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
pub struct TrackMusicalAnalysisWarning {
    pub severity: TrackMusicalAnalysisWarningSeverity,
    pub code: String,
    pub message: String,
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
pub enum TrackMusicalAnalysisWarningSeverity {
    Info,
    Warning,
    Error,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackMusicalAnalysisBasis {
    pub adapter_key: String,
    pub adapter_version: String,
    pub upstream_crate_name: String,
    pub upstream_crate_version: String,
    pub upstream_feature_flags: Vec<String>,
    pub decoder_policy: String,
    pub input_policy: String,
    pub channel_mixdown_policy: String,
    pub normalization_policy: String,
    pub upstream_analysis_config_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sample_rate_hz: Option<u32>,
    pub ml_enabled: bool,
    pub persistence_authorized: bool,
    pub authority: String,
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
pub enum MusicalAnalysisCommand {
    AnalyzePlayableMedia(AnalyzePlayableMediaRequest),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum MusicalAnalysisReply {
    AnalyzePlayableMedia(AnalyzePlayableMediaReply),
}

#[cfg(test)]
mod tests {
    use super::{
        AnalyzePlayableMediaReply, AnalyzePlayableMediaRequest, MusicalAnalysisCommand,
        MusicalAnalysisReply, TrackMusicalAnalysisBasis, TrackMusicalAnalysisResult,
        TrackMusicalAnalysisStatus, TrackMusicalAnalysisTarget, TrackMusicalAnalysisWarning,
        TrackMusicalAnalysisWarningSeverity,
    };
    use serde_json::json;

    #[test]
    fn analyze_playable_media_command_and_reply_are_tagged() {
        let command = MusicalAnalysisCommand::AnalyzePlayableMedia(AnalyzePlayableMediaRequest {
            playable_media_id: 40,
            source_id: 7,
            source_file_id: 11,
            attachment_id: 30,
        });

        assert_eq!(
            serde_json::to_value(&command).expect("serialize command"),
            json!({
                "type": "analyzePlayableMedia",
                "payload": {
                    "playableMediaId": "40",
                    "sourceId": "7",
                    "sourceFileId": "11",
                    "attachmentId": "30"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<MusicalAnalysisCommand>(
                serde_json::to_value(command.clone()).expect("serialize")
            )
            .expect("deserialize command"),
            command
        );

        let reply = MusicalAnalysisReply::AnalyzePlayableMedia(AnalyzePlayableMediaReply {
            result: TrackMusicalAnalysisResult {
                target: TrackMusicalAnalysisTarget {
                    playable_media_id: 40,
                    source_id: 7,
                    source_file_id: 11,
                    attachment_id: 30,
                    relative_path: Some("Album/track.wav".to_string()),
                    media_kind: Some("audio".to_string()),
                },
                status: TrackMusicalAnalysisStatus::Unsupported,
                status_detail: "WAV-only analyzer path did not run for this input.".to_string(),
                bpm: None,
                key: None,
                beatgrid: None,
                warnings: vec![TrackMusicalAnalysisWarning {
                    severity: TrackMusicalAnalysisWarningSeverity::Warning,
                    code: "unsupported_input".to_string(),
                    message: "Only 16-bit PCM WAV is supported in this slice.".to_string(),
                }],
                basis: TrackMusicalAnalysisBasis {
                    adapter_key: "dekzer_analyzer_musical_stratum_spike".to_string(),
                    adapter_version: "0.0.0".to_string(),
                    upstream_crate_name: "stratum-dsp".to_string(),
                    upstream_crate_version: "1.0.0".to_string(),
                    upstream_feature_flags: vec!["ort=disabled".to_string()],
                    decoder_policy: "hound_16_bit_integer_pcm_wav_to_mono_f32_v1".to_string(),
                    input_policy: "blocked_before_adapter_invocation_v1".to_string(),
                    channel_mixdown_policy: "not_applicable_adapter_not_invoked_v1".to_string(),
                    normalization_policy: "not_applicable_adapter_not_invoked_v1".to_string(),
                    upstream_analysis_config_policy: "not_applicable_adapter_not_invoked_v1"
                        .to_string(),
                    sample_rate_hz: None,
                    ml_enabled: false,
                    persistence_authorized: false,
                    authority: "non_authoritative_advisory_v1".to_string(),
                },
            },
        });

        let json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(json["type"], json!("analyzePlayableMedia"));
        assert_eq!(
            json["payload"]["result"]["target"]["sourceFileId"],
            json!("11")
        );
        assert_eq!(json["payload"]["result"]["status"], json!("unsupported"));
        assert_eq!(
            serde_json::from_value::<MusicalAnalysisReply>(json).expect("deserialize reply"),
            reply
        );
    }
}
