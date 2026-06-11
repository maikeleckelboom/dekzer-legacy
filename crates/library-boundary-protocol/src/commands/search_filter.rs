use super::snapshot_reads::{ContentsFileKind, ContentsPresenceState, SourceScanPhase};

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
pub struct SearchFilterReadRequest {
    pub scope: SearchFilterScope,
    pub recursion: SearchFilterRecursion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub text_query: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_kinds: Vec<SearchFilterResultKind>,
    #[serde(default)]
    pub filters: SearchFilterSet,
    pub sort: SearchFilterSort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cursor: Option<String>,
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
pub enum SearchFilterScope {
    Library,
    Source {
        #[serde(rename = "sourceId")]
        #[ts(rename = "sourceId")]
        #[serde(with = "crate::wire::i64_string")]
        #[schemars(with = "String")]
        #[ts(as = "String")]
        source_id: i64,
    },
    SourceLocation {
        #[serde(rename = "sourceLocationId")]
        #[ts(rename = "sourceLocationId")]
        #[serde(with = "crate::wire::i64_string")]
        #[schemars(with = "String")]
        #[ts(as = "String")]
        source_location_id: i64,
    },
    Directory {
        #[serde(rename = "sourceId")]
        #[ts(rename = "sourceId")]
        #[serde(with = "crate::wire::i64_string")]
        #[schemars(with = "String")]
        #[ts(as = "String")]
        source_id: i64,
        #[serde(rename = "sourceDirectoryId")]
        #[ts(rename = "sourceDirectoryId")]
        #[serde(with = "crate::wire::i64_string")]
        #[schemars(with = "String")]
        #[ts(as = "String")]
        source_directory_id: i64,
    },
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
pub enum SearchFilterRecursion {
    Immediate,
    Recursive,
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
pub enum SearchFilterSort {
    PathName,
    Relevance,
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
pub enum SearchFilterResultKind {
    Source,
    SourceLocation,
    Directory,
    SourceFile,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SearchFilterSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_classes: Vec<SearchFilterFileClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_kinds: Vec<ContentsFileKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media_relevance: Vec<SearchFilterMediaRelevance>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presence_states: Vec<ContentsPresenceState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_access_states: Vec<SearchFilterSourceAccessState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub blake3: Option<SearchFilterEvidenceAvailability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub probe: Option<SearchFilterEvidenceAvailability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_link_states: Vec<SearchFilterAttachmentLinkState>,
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
pub enum SearchFilterFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
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
pub enum SearchFilterSourceAccessState {
    Accessible,
    Missing,
    Blocked,
    Unknown,
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
pub enum SearchFilterEvidenceAvailability {
    HasCurrent,
    MissingCurrent,
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
pub enum SearchFilterAttachmentLinkState {
    Current,
    Stale,
    Missing,
    NotApplicable,
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
pub enum SearchFilterMediaRelevance {
    AudioWorkflow,
    PlayableMedia,
    ExplicitInventory,
    CompanionFile,
    NotMediaRelevant,
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
pub enum SearchFilterIndexState {
    Ready,
    Rebuilding,
    Partial,
    Failed,
    Missing,
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
pub enum SearchFilterAuthorityLayer {
    Source,
    SourceLocation,
    SourceHierarchy,
    SourceFileInventory,
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
pub enum SearchFilterEvidenceCoverageState {
    Indexed,
    NotApplicable,
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
pub enum SearchFilterMatchReason {
    Filter,
    ExactLabel,
    LabelPrefix,
    Label,
    Path,
    Text,
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
pub struct SearchFilterReadReply {
    pub result: SearchFilterResult,
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
pub struct SearchFilterResult {
    pub state: SearchFilterState,
    pub query_identity: SearchFilterQueryIdentity,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub index_generation: i64,
    pub index_state: SearchFilterIndexState,
    pub rows: Vec<SearchFilterResultRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub next_cursor: Option<String>,
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
pub enum SearchFilterState {
    Ready,
    Empty,
    Partial,
    CursorInvalid,
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
pub struct SearchFilterQueryIdentity {
    pub scope: SearchFilterScope,
    pub recursion: SearchFilterRecursion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub text_query: Option<String>,
    pub target_kinds: Vec<SearchFilterResultKind>,
    pub filters: SearchFilterSet,
    pub sort: SearchFilterSort,
    pub page_size: usize,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub index_generation: i64,
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
pub struct SearchFilterResultRow {
    pub result_kind: SearchFilterResultKind,
    pub authority_layer: SearchFilterAuthorityLayer,
    pub stable_key: String,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_location_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_directory_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_file_id: Option<i64>,
    pub display_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub display_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub file_class: Option<SearchFilterFileClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub file_kind: Option<ContentsFileKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub media_relevance: Option<SearchFilterMediaRelevance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub presence_state: Option<ContentsPresenceState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_access_state: Option<SearchFilterSourceAccessState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_scan_phase: Option<SourceScanPhase>,
    pub has_current_blake3: bool,
    pub has_current_probe: bool,
    pub attachment_link_state: SearchFilterAttachmentLinkState,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub attachment_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub content_hash_algorithm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub content_hash_value: Option<String>,
    pub evidence_coverage_state: SearchFilterEvidenceCoverageState,
    pub match_reason: SearchFilterMatchReason,
    pub updated_at_ms: i64,
}
