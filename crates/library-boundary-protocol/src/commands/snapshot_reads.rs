use super::{
    ReadSourceMaintenanceReply, ReadSourceMaintenanceRequest, TrackIdentityDecisionState,
    TrackIdentityEffectiveDecisionCurrentStatus, TrackIdentityUserBlockingDecisionState,
};

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
pub struct ReadNavigationRowsRequest {
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_navigation_row_id: Option<i64>,
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
pub struct ReadNavigationRowsReply {
    pub rows: Vec<NavigationRow>,
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
pub struct LoadNavigationRowRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub navigation_row_id: i64,
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
pub struct LoadNavigationRowReply {
    pub row: Option<NavigationRow>,
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
pub struct LoadNavigationRowByStableKeyRequest {
    pub stable_key: String,
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
pub struct LoadNavigationRowByStableKeyReply {
    pub row: Option<NavigationRow>,
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
pub struct NavigationRow {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub navigation_row_id: i64,
    pub stable_key: String,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_navigation_row_id: Option<i64>,
    pub family: Option<NavigationRowFamily>,
    pub row_kind: NavigationRowKind,
    pub display_name: String,
    pub sibling_position: i64,
    pub selectable: bool,
    pub selector_kind: Option<NavigationRowSelectorKind>,
    pub selector_payload: Option<String>,
    pub updated_at_ms: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub row_version: i64,
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
pub enum NavigationRowFamily {
    Views,
    Collections,
    Preparation,
    Sources,
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
pub enum NavigationRowKind {
    View,
    CollectionGroup,
    Playlist,
    PrepPolicyGroup,
    PrepPolicyScope,
    Source,
    LocationGroup,
    Location,
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
pub enum NavigationRowSelectorKind {
    AllMedia,
    AllAudio,
    AllVideos,
    RecentlyAdded,
    NeedsPreparation,
    PlaylistGroup,
    Source,
    SourceLocation,
    Playlist,
    PrepPolicyScope,
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
pub struct ReadNavigationNodeLibraryBrowserWindowRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub navigation_row_id: i64,
    pub offset: usize,
    pub limit: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadNavigationNodeLibraryBrowserWindowReply {
    pub window: Option<LibraryBrowserWindow>,
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
pub struct SearchNavigationNodeLibraryBrowserWindowRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub navigation_row_id: i64,
    pub query: String,
    pub offset: usize,
    pub limit: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SearchNavigationNodeLibraryBrowserWindowReply {
    pub window: Option<LibraryBrowserWindow>,
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
pub struct ContentsReadRequest {
    pub scope: ContentsScope,
    pub policy: ContentsReadPolicy,
    pub scope_depth: ContentsScopeDepth,
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
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum ContentsReadPolicy {
    PlayableMediaBrowse,
    AudioBrowse,
    SourceFileInventory {
        #[serde(rename = "fileClasses")]
        #[ts(rename = "fileClasses")]
        file_classes: Vec<ContentsFileClass>,
    },
    PrimaryMedia {
        #[serde(rename = "mediaKinds")]
        #[ts(rename = "mediaKinds")]
        media_kinds: Vec<PrimaryMediaKind>,
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
pub enum ContentsFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
}

impl ContentsFileClass {
    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"audio" => Some(Self::Audio),
            b"video" => Some(Self::Video),
            b"image" => Some(Self::Image),
            b"unsupported" => Some(Self::Unsupported),
            _ => None,
        }
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
pub enum PrimaryMediaKind {
    Audio,
    Video,
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
pub enum ContentsScopeDepth {
    Immediate,
    Recursive,
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
pub enum ContentsScope {
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
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ContentsReadReply {
    pub result: ContentsResult,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ContentsResult {
    pub state: ContentsState,
    pub scope: ContentsScope,
    pub policy: ContentsReadPolicy,
    pub scope_depth: ContentsScopeDepth,
    pub rows: Vec<ContentsFileRow>,
    pub scope_coverage: ContentsScopeCoverage,
    pub has_policy_omitted_rows: bool,
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
pub enum ContentsState {
    Ready,
    Empty,
    Partial,
    SourceUnavailable,
    LocationMissing,
    Blocked,
    Failed,
    PolicyConflict,
    CursorInvalid,
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
pub enum ContentsScopeCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
    Incomplete,
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
pub struct ContentsScopeCoverage {
    pub state: ContentsScopeCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
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
pub enum ContentsRowOrigin {
    LibraryAsset,
    SourceFile,
    PrimaryMediaCandidate,
}

impl ContentsRowOrigin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryAsset => "libraryAsset",
            Self::SourceFile => "sourceFile",
            Self::PrimaryMediaCandidate => "primaryMediaCandidate",
        }
    }

    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"libraryAsset" => Some(Self::LibraryAsset),
            b"sourceFile" => Some(Self::SourceFile),
            b"primaryMediaCandidate" => Some(Self::PrimaryMediaCandidate),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ContentsFileRow {
    pub id: String,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
    #[serde(rename = "parentDirectoryId")]
    #[ts(rename = "parentDirectoryId")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_directory_id: Option<i64>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub relative_path: Option<String>,
    pub file_name: String,
    pub file_class: ContentsFileClass,
    pub file_kind: ContentsFileKind,
    pub presence: ContentsPresenceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub availability_state: Option<LibraryAssetAvailabilityState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub primary_media: Option<PrimaryMediaSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub updated_at_ms: Option<i64>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PrimaryMediaSummary {
    pub origin: ContentsRowOrigin,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub primary_media_candidate_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub attachment_id: Option<i64>,
    pub content_hash_algorithm: Option<String>,
    pub content_hash_value: Option<String>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub evidence_source_file_id: Option<i64>,
    pub media_kind: Option<String>,
    pub mime_type: Option<String>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub library_asset_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub row_version: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub primary_source_file_id: Option<i64>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
    pub musical_key: Option<String>,
    pub tempo_bpm: Option<f64>,
    pub waveform_quality_current: Option<i64>,
    pub waveform_quality_target: Option<i64>,
    pub stems_state_summary: Option<LibraryAssetStemsStateSummary>,
    pub prep_readiness_summary: Option<LibraryAssetPrepReadinessSummary>,
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
pub enum ContentsPresenceState {
    Present,
    Missing,
    Removed,
}

impl ContentsPresenceState {
    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"present" => Some(Self::Present),
            b"missing" => Some(Self::Missing),
            b"removed" => Some(Self::Removed),
            _ => None,
        }
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
pub enum ContentsFileKind {
    Audio,
    Video,
    Image,
    CueSheet,
    LogDoc,
    TextDoc,
    Archive,
    Other,
    Unknown,
}

impl ContentsFileKind {
    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"audio" => Some(Self::Audio),
            b"video" => Some(Self::Video),
            b"image" => Some(Self::Image),
            b"cue_sheet" => Some(Self::CueSheet),
            b"log_doc" => Some(Self::LogDoc),
            b"text_doc" => Some(Self::TextDoc),
            b"archive" => Some(Self::Archive),
            b"other" => Some(Self::Other),
            b"unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
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
pub struct ReadLibraryAssetWaveformOverviewRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryAssetWaveformOverviewReply {
    pub overview: Option<LibraryAssetWaveformOverview>,
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
pub struct ReadLibraryAssetPreparationDetailRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryAssetPreparationDetailReply {
    pub detail: Option<LibraryAssetPreparationDetail>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryAssetPreparationDetail {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
    pub aggregate_readiness_summary: LibraryAssetPrepReadinessSummary,
    pub groups: Vec<LibraryAssetPreparationDetailGroup>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryAssetPreparationDetailGroup {
    pub group_key: LibraryAssetPreparationDetailGroupKey,
    pub rows: Vec<LibraryAssetPreparationDetailRow>,
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
pub enum LibraryAssetPreparationDetailGroupKey {
    RequiredFacts,
    RequiredStructures,
    RequiredArtifacts,
    OnDemandCapabilities,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryAssetPreparationDetailRow {
    pub capability_key: LibraryAssetPreparationCapabilityKey,
    pub label: String,
    pub requirement_class: LibraryAssetPreparationRequirementClass,
    pub outcome_kind: LibraryAssetPreparationOutcomeKind,
    pub work_state: LibraryAssetPreparationWorkState,
    pub outcome_state: LibraryAssetPreparationOutcomeState,
    pub satisfaction_state: LibraryAssetPreparationSatisfactionState,
    pub display_value_summary: Option<String>,
    pub target_summary: Option<String>,
    pub explanation_summary: Option<String>,
    pub artifact_coverage_state: Option<LibraryAssetPreparationArtifactCoverageState>,
    pub progress: Option<LibraryAssetPreparationProgress>,
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
pub enum LibraryAssetPreparationCapabilityKey {
    Bpm,
    MusicalKey,
    Beatgrid,
    Waveform,
    Stems,
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
pub enum LibraryAssetPreparationRequirementClass {
    Required,
    OnDemand,
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
pub enum LibraryAssetPreparationOutcomeKind {
    Fact,
    Structure,
    Artifact,
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
pub enum LibraryAssetPreparationWorkState {
    None,
    NotRequested,
    Queued,
    Active,
    Blocked,
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
pub enum LibraryAssetPreparationOutcomeState {
    Missing,
    Provisional,
    Ready,
    Stale,
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
pub enum LibraryAssetPreparationSatisfactionState {
    Satisfied,
    Unsatisfied,
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
pub enum LibraryAssetPreparationArtifactCoverageState {
    None,
    PreviewReady,
    OverviewReady,
    RefinementAvailable,
    Stale,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type", rename_all = "camelCase")]
pub enum LibraryAssetPreparationProgress {
    Indeterminate {
        #[serde(rename = "activeStage")]
        #[ts(rename = "activeStage")]
        active_stage: Option<String>,
    },
    BoundedStage {
        #[serde(rename = "activeStage")]
        #[ts(rename = "activeStage")]
        active_stage: Option<String>,
        #[serde(rename = "completedUnits")]
        #[ts(rename = "completedUnits")]
        completed_units: i64,
        #[serde(rename = "totalUnits")]
        #[ts(rename = "totalUnits")]
        total_units: i64,
        fraction: f64,
    },
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryBrowserWindow {
    pub offset: usize,
    pub limit: usize,
    pub total_rows: usize,
    pub rows: Vec<LibraryAssetBrowserRow>,
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
pub enum LibraryAssetAvailabilityState {
    Available,
    Unavailable,
    Degraded,
}

impl LibraryAssetAvailabilityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Degraded => "degraded",
        }
    }

    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"available" => Some(Self::Available),
            b"unavailable" => Some(Self::Unavailable),
            b"degraded" => Some(Self::Degraded),
            _ => None,
        }
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
pub enum LibraryAssetStemsStateSummary {
    Missing,
    Queued,
    Leased,
    Ready,
    Stale,
    Blocked,
    Failed,
}

impl LibraryAssetStemsStateSummary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Ready => "ready",
            Self::Stale => "stale",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"missing" => Some(Self::Missing),
            b"queued" => Some(Self::Queued),
            b"leased" => Some(Self::Leased),
            b"ready" => Some(Self::Ready),
            b"stale" => Some(Self::Stale),
            b"blocked" => Some(Self::Blocked),
            b"failed" => Some(Self::Failed),
            _ => None,
        }
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
pub enum LibraryAssetPrepReadinessSummary {
    NotRequired,
    Ready,
    Preparing,
    Underprepared,
    Blocked,
    Failed,
}

impl LibraryAssetPrepReadinessSummary {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "notRequired",
            Self::Ready => "ready",
            Self::Preparing => "preparing",
            Self::Underprepared => "underprepared",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"not_required" => Some(Self::NotRequired),
            b"ready" => Some(Self::Ready),
            b"preparing" => Some(Self::Preparing),
            b"underprepared" => Some(Self::Underprepared),
            b"blocked" => Some(Self::Blocked),
            b"failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryAssetBrowserRow {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub library_asset_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub row_version: i64,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub primary_source_file_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub scoped_source_file_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_id: Option<i64>,
    pub relative_path: Option<String>,
    pub file_name: Option<String>,
    pub availability_state: LibraryAssetAvailabilityState,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub musical_key: Option<String>,
    pub tempo_bpm: Option<f64>,
    /// Browse-facing summaries derived by projection rebuild logic from
    /// capability/artifact state; they are not canonical authority.
    pub waveform_quality_current: Option<i64>,
    pub waveform_quality_target: Option<i64>,
    pub stems_state_summary: Option<LibraryAssetStemsStateSummary>,
    pub prep_readiness_summary: LibraryAssetPrepReadinessSummary,
    pub updated_at_ms: i64,
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
pub enum LibraryAssetWaveformOverviewCapabilityState {
    Ready,
    Stale,
}

impl LibraryAssetWaveformOverviewCapabilityState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Stale => "stale",
        }
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
pub enum LibraryAssetWaveformOverviewAmplitudeScale {
    SignedI16,
}

impl LibraryAssetWaveformOverviewAmplitudeScale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SignedI16 => "signedI16",
        }
    }
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
pub struct LibraryAssetWaveformOverviewBucket {
    pub min_amplitude_i16: i16,
    pub max_amplitude_i16: i16,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryAssetWaveformOverview {
    pub source_profile_key: String,
    pub source_quality_current: Option<i64>,
    pub capability_state: LibraryAssetWaveformOverviewCapabilityState,
    pub amplitude_scale: LibraryAssetWaveformOverviewAmplitudeScale,
    pub bucket_count: usize,
    pub duration_ms: Option<i64>,
    pub source_sample_count: Option<i64>,
    pub samples_per_bucket: Option<i64>,
    pub buckets: Vec<LibraryAssetWaveformOverviewBucket>,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub accepted_artifact_id: i64,
    pub basis_fingerprint: String,
    pub capability_updated_at_ms: i64,
    pub artifact_created_at_ms: i64,
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
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum LibraryTreeEntryPoint {
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
pub struct ReadLibraryTreeChildrenRequest {
    pub entry_point: LibraryTreeEntryPoint,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryTreeChildrenReply {
    pub window: Option<LibraryTreeWindow>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryTreeWindow {
    pub entry_point: LibraryTreeEntryPoint,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    pub offset: usize,
    pub limit: usize,
    pub total_rows: usize,
    pub rows: Vec<LibraryTreeNode>,
    pub coverage: LibraryTreeCoverage,
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
pub enum LibraryTreeCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryTreeCoverage {
    pub state: LibraryTreeCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
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
pub enum LibraryTreeNodeKind {
    Directory,
    File,
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
pub enum LibraryTreePresenceState {
    Present,
    Missing,
    Removed,
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
pub enum DirectoryScanState {
    Pending,
    Scanning,
    Complete,
    Failed,
    Blocked,
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
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum DirectoryPrimaryMediaState {
    Unknown,
    HasPrimaryMediaDescendants,
    NoPrimaryMediaDescendants,
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
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum DirectoryImageMediaState {
    Unknown,
    HasImageMediaDescendants,
    NoImageMediaDescendants,
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
pub enum LibraryTreeFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
    None,
}

impl LibraryTreeFileClass {
    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"audio" => Some(Self::Audio),
            b"video" => Some(Self::Video),
            b"image" => Some(Self::Image),
            b"unsupported" => Some(Self::Unsupported),
            b"none" => Some(Self::None),
            _ => None,
        }
    }
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
pub struct LibraryTreeNode {
    pub node_kind: LibraryTreeNodeKind,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_directory_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub source_file_id: Option<i64>,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    pub relative_path: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub file_class: Option<LibraryTreeFileClass>,
    pub presence_state: LibraryTreePresenceState,
    pub size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub updated_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub has_child_directories: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub directory_primary_media_state: Option<DirectoryPrimaryMediaState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub directory_image_media_state: Option<DirectoryImageMediaState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub directory_scan_state: Option<DirectoryScanState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub navigable_child_scope_state: Option<NavigableChildScopeState>,
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
pub enum NavigableChildScopeState {
    Unknown,
    HasNavigableChildScopes,
    NoNavigableChildScopes,
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
pub enum SourceClass {
    Internal,
    ExternalMounted,
    RemovableMounted,
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
pub enum SourceMountStatus {
    Unknown,
    Mounted,
    Unmounted,
    EjectRequested,
    EjectPending,
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
pub enum SourceAccessState {
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
pub enum SourceLifecycleIssueKind {
    Missing,
    NotDirectory,
    PermissionDenied,
    PrivacyPermissionRequired,
    UnavailableMount,
    ResourceBusy,
    StaleNetworkHandle,
    SymlinkLoop,
    SymlinkEscapeBlocked,
    UnsupportedPath,
    InvalidPath,
    IoInterrupted,
    TimedOut,
    UnknownIo,
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
pub enum SourceScanPhase {
    Idle,
    Scanning,
    Complete,
    Partial,
    Blocked,
    Failed,
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
pub struct ReadSourceLifecycleRequest {
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
pub struct ReadSourceLifecycleReply {
    pub lifecycle: Option<SourceLifecycle>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceLifecycle {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub source_class: SourceClass,
    pub is_user_visible: bool,
    pub mount_status: SourceMountStatus,
    pub access_state: SourceAccessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub access_issue_kind: Option<SourceLifecycleIssueKind>,
    pub scan_phase: SourceScanPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub scan_issue_kind: Option<SourceLifecycleIssueKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_scan_started_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_scan_finished_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_successful_scan_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_seen_at_ms: Option<i64>,
    pub updated_at_ms: i64,
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
pub enum AttachmentIdentityReadStatus {
    Ok,
    NotFound,
    InvalidRequest,
    ReadFailed,
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
pub enum SourceFileAttachmentLinkStatus {
    Current,
    Stale,
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
pub struct AttachmentIdentity {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
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
pub struct SourceFileAttachmentLink {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub file_kind: ContentsFileKind,
    pub link_status: SourceFileAttachmentLinkStatus,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
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
pub struct SourceAttachmentSummary {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub current_links_count: usize,
    pub stale_links_count: usize,
    pub source_files_with_current_blake3_facts_count: usize,
    pub source_files_with_attachment_links_count: usize,
    pub source_files_missing_attachment_links_count: usize,
    pub unmaterialized_blake3_facts_count: usize,
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
pub struct ReadSourceFileAttachmentRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_file_id: i64,
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
pub struct ReadSourceFileAttachmentReply {
    pub status: AttachmentIdentityReadStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment_link: Option<SourceFileAttachmentLink>,
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
pub struct ReadAttachmentSourceFilesRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub limit: Option<usize>,
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
pub struct ReadAttachmentSourceFilesReply {
    pub status: AttachmentIdentityReadStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment: Option<AttachmentIdentity>,
    pub source_file_links: Vec<SourceFileAttachmentLink>,
    pub effective_limit: usize,
    pub remaining_source_file_links: usize,
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
pub struct ReadSourceAttachmentSummaryRequest {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
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
pub struct ReadSourceAttachmentSummaryReply {
    pub status: AttachmentIdentityReadStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub summary: Option<SourceAttachmentSummary>,
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
pub struct ReadTrackIdentityReviewCandidatesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    #[ts(optional)]
    pub source_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub review_state: Option<TrackIdentityReviewState>,
    pub limit: usize,
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
pub struct ReadTrackIdentityReviewCandidatesReply {
    pub status: TrackIdentityReviewReadStatus,
    pub candidates: Vec<TrackIdentityReviewCandidate>,
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
pub enum TrackIdentityReviewReadStatus {
    Ok,
    SourceNotFound,
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
pub struct TrackIdentityReviewCandidate {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub candidate_id: i64,
    pub candidate_kind: String,
    pub candidate_evidence_basis: String,
    pub candidate_status: TrackIdentityReviewCandidateStatus,
    pub evidence_key_algorithm: String,
    pub evidence_key_value: String,
    pub evidence_summary: TrackIdentityReviewEvidenceSummary,
    pub source_summary: TrackIdentityReviewSourceSummary,
    pub review_state: TrackIdentityReviewState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub effective_decision: Option<TrackIdentityReviewDecision>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
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
pub enum TrackIdentityReviewCandidateStatus {
    Active,
    Stale,
    Superseded,
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
pub struct TrackIdentityReviewDecision {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub decision_id: i64,
    pub decision_state: TrackIdentityDecisionState,
    pub decision_source: String,
    pub decision_basis: String,
    pub current_status: TrackIdentityEffectiveDecisionCurrentStatus,
    pub created_at_ms: i64,
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
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrackIdentityReviewEvidenceSummary {
    pub member_count: usize,
    pub evidence_count: usize,
    pub current_evidence_count: usize,
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
pub struct TrackIdentityReviewSourceSummary {
    pub source_count: usize,
    pub source_samples: Vec<TrackIdentityReviewSourceSample>,
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
pub struct TrackIdentityReviewSourceSample {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub display_name: String,
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
pub enum TrackIdentityReviewState {
    NeedsUserDecision,
    SystemAccepted,
    UserAccepted,
    UserRejected,
    UserDeferred,
    StaleDecision,
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
pub enum SnapshotReadCommand {
    ReadNavigationRows(ReadNavigationRowsRequest),
    LoadNavigationRow(LoadNavigationRowRequest),
    LoadNavigationRowByStableKey(LoadNavigationRowByStableKeyRequest),
    ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest),
    ReadSourceLifecycle(ReadSourceLifecycleRequest),
    ReadSourceMaintenance(ReadSourceMaintenanceRequest),
    ReadSourceFileAttachment(ReadSourceFileAttachmentRequest),
    ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest),
    ReadSourceAttachmentSummary(ReadSourceAttachmentSummaryRequest),
    ReadTrackIdentityReviewCandidates(ReadTrackIdentityReviewCandidatesRequest),
    ReadNavigationNodeLibraryBrowserWindow(ReadNavigationNodeLibraryBrowserWindowRequest),
    SearchNavigationNodeLibraryBrowserWindow(SearchNavigationNodeLibraryBrowserWindowRequest),
    ContentsRead(ContentsReadRequest),
    ReadLibraryAssetWaveformOverview(ReadLibraryAssetWaveformOverviewRequest),
    ReadLibraryAssetPreparationDetail(ReadLibraryAssetPreparationDetailRequest),
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum SnapshotReadReply {
    NavigationRows(ReadNavigationRowsReply),
    NavigationRow(LoadNavigationRowReply),
    NavigationRowByStableKey(LoadNavigationRowByStableKeyReply),
    LibraryTreeChildren(ReadLibraryTreeChildrenReply),
    SourceLifecycle(ReadSourceLifecycleReply),
    SourceMaintenance(Box<ReadSourceMaintenanceReply>),
    SourceFileAttachment(ReadSourceFileAttachmentReply),
    AttachmentSourceFiles(ReadAttachmentSourceFilesReply),
    SourceAttachmentSummary(ReadSourceAttachmentSummaryReply),
    TrackIdentityReviewCandidates(ReadTrackIdentityReviewCandidatesReply),
    NavigationNodeLibraryBrowserWindow(ReadNavigationNodeLibraryBrowserWindowReply),
    NavigationNodeLibraryBrowserSearch(SearchNavigationNodeLibraryBrowserWindowReply),
    Contents(ContentsReadReply),
    LibraryAssetWaveformOverview(ReadLibraryAssetWaveformOverviewReply),
    LibraryAssetPreparationDetail(ReadLibraryAssetPreparationDetailReply),
}

#[cfg(test)]
mod tests {
    use super::{
        AttachmentIdentity, AttachmentIdentityReadStatus, ContentsFileClass, ContentsFileKind,
        ContentsReadPolicy, ContentsReadRequest, ContentsScope, ContentsScopeDepth,
        DirectoryImageMediaState, DirectoryPrimaryMediaState, DirectoryScanState,
        LibraryAssetAvailabilityState, LibraryAssetBrowserRow, LibraryAssetPrepReadinessSummary,
        LibraryAssetPreparationArtifactCoverageState, LibraryAssetPreparationCapabilityKey,
        LibraryAssetPreparationDetail, LibraryAssetPreparationDetailGroup,
        LibraryAssetPreparationDetailGroupKey, LibraryAssetPreparationDetailRow,
        LibraryAssetPreparationOutcomeKind, LibraryAssetPreparationOutcomeState,
        LibraryAssetPreparationRequirementClass, LibraryAssetPreparationSatisfactionState,
        LibraryAssetPreparationWorkState, LibraryAssetStemsStateSummary,
        LibraryAssetWaveformOverview, LibraryAssetWaveformOverviewAmplitudeScale,
        LibraryAssetWaveformOverviewBucket, LibraryAssetWaveformOverviewCapabilityState,
        LibraryTreeCoverage, LibraryTreeCoverageState, LibraryTreeEntryPoint, LibraryTreeFileClass,
        LibraryTreeNode, LibraryTreeNodeKind, LibraryTreePresenceState, LibraryTreeWindow,
        LoadNavigationRowByStableKeyRequest, LoadNavigationRowRequest, NavigableChildScopeState,
        NavigationRow, NavigationRowFamily, NavigationRowKind, NavigationRowSelectorKind,
        PrimaryMediaKind, ReadAttachmentSourceFilesReply, ReadAttachmentSourceFilesRequest,
        ReadLibraryAssetPreparationDetailRequest, ReadLibraryAssetWaveformOverviewRequest,
        ReadLibraryTreeChildrenReply, ReadLibraryTreeChildrenRequest,
        ReadNavigationNodeLibraryBrowserWindowReply, ReadNavigationNodeLibraryBrowserWindowRequest,
        ReadNavigationRowsRequest, ReadSourceAttachmentSummaryReply,
        ReadSourceAttachmentSummaryRequest, ReadSourceFileAttachmentReply,
        ReadSourceFileAttachmentRequest, ReadSourceLifecycleReply, ReadSourceLifecycleRequest,
        ReadTrackIdentityReviewCandidatesReply, ReadTrackIdentityReviewCandidatesRequest,
        SearchNavigationNodeLibraryBrowserWindowReply,
        SearchNavigationNodeLibraryBrowserWindowRequest, SnapshotReadCommand, SnapshotReadReply,
        SourceAccessState, SourceAttachmentSummary, SourceClass, SourceFileAttachmentLink,
        SourceFileAttachmentLinkStatus, SourceLifecycle, SourceLifecycleIssueKind,
        SourceMountStatus, SourceScanPhase, TrackIdentityDecisionState,
        TrackIdentityEffectiveDecisionCurrentStatus, TrackIdentityReviewCandidate,
        TrackIdentityReviewCandidateStatus, TrackIdentityReviewDecision,
        TrackIdentityReviewEvidenceSummary, TrackIdentityReviewReadStatus,
        TrackIdentityReviewSourceSample, TrackIdentityReviewSourceSummary,
        TrackIdentityReviewState, TrackIdentityUserBlockingDecisionState,
    };
    use serde_json::json;

    #[test]
    fn snapshot_read_commands_carry_windowed_read_inputs() {
        let navigation = SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
            parent_navigation_row_id: Some(7),
        });
        let library_tree =
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 50,
            });
        let navigation_node = SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
            ReadNavigationNodeLibraryBrowserWindowRequest {
                navigation_row_id: 4,
                offset: 0,
                limit: 20,
            },
        );
        let navigation_node_search = SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(
            SearchNavigationNodeLibraryBrowserWindowRequest {
                navigation_row_id: 4,
                query: "amen".to_string(),
                offset: 0,
                limit: 20,
            },
        );
        let waveform = SnapshotReadCommand::ReadLibraryAssetWaveformOverview(
            ReadLibraryAssetWaveformOverviewRequest {
                library_asset_id: 42,
            },
        );
        let preparation_detail = SnapshotReadCommand::ReadLibraryAssetPreparationDetail(
            ReadLibraryAssetPreparationDetailRequest {
                library_asset_id: 42,
            },
        );

        assert!(matches!(
            navigation,
            SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
                parent_navigation_row_id: Some(7),
            })
        ));
        assert!(matches!(
            library_tree,
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 50,
            },)
        ));
        assert!(matches!(
            navigation_node,
            SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
                ReadNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 4,
                    offset: 0,
                    limit: 20,
                },
            )
        ));
        assert!(matches!(
            navigation_node_search,
            SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(
                SearchNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 4,
                    query,
                    offset: 0,
                    limit: 20,
                },
            ) if query == "amen"
        ));
        assert!(matches!(
            waveform,
            SnapshotReadCommand::ReadLibraryAssetWaveformOverview(
                ReadLibraryAssetWaveformOverviewRequest {
                    library_asset_id: 42,
                },
            )
        ));
        assert!(matches!(
            preparation_detail,
            SnapshotReadCommand::ReadLibraryAssetPreparationDetail(
                ReadLibraryAssetPreparationDetailRequest {
                    library_asset_id: 42,
                },
            )
        ));
    }

    #[test]
    fn contents_file_class_includes_inventory_class_values() {
        assert_eq!(
            ContentsFileClass::from_projection_value("audio"),
            Some(ContentsFileClass::Audio)
        );
        assert_eq!(
            ContentsFileClass::from_projection_value("video"),
            Some(ContentsFileClass::Video)
        );
        assert_eq!(
            ContentsFileClass::from_projection_value("image"),
            Some(ContentsFileClass::Image)
        );
        assert_eq!(
            ContentsFileClass::from_projection_value("unsupported"),
            Some(ContentsFileClass::Unsupported)
        );
    }

    #[test]
    fn contents_file_kind_includes_fine_inventory_kinds() {
        assert_eq!(
            ContentsFileKind::from_projection_value("audio"),
            Some(ContentsFileKind::Audio)
        );
        assert_eq!(
            ContentsFileKind::from_projection_value("cue_sheet"),
            Some(ContentsFileKind::CueSheet)
        );
        assert_eq!(
            ContentsFileKind::from_projection_value("text_doc"),
            Some(ContentsFileKind::TextDoc)
        );
        assert_eq!(
            ContentsFileKind::from_projection_value("unknown"),
            Some(ContentsFileKind::Unknown)
        );
    }

    #[test]
    fn contents_policy_serializes_audio_browse_boundary_shape() {
        let policy = ContentsReadPolicy::AudioBrowse;
        let json = serde_json::to_value(&policy).expect("serialize policy");
        assert_eq!(json, json!({ "kind": "audioBrowse" }));
        assert_eq!(
            serde_json::from_value::<ContentsReadPolicy>(json).expect("deserialize policy"),
            policy
        );
    }

    #[test]
    fn contents_policy_serializes_playable_media_browse_boundary_shape() {
        let policy = ContentsReadPolicy::PlayableMediaBrowse;
        let json = serde_json::to_value(&policy).expect("serialize policy");
        assert_eq!(json, json!({ "kind": "playableMediaBrowse" }));
        assert_eq!(
            serde_json::from_value::<ContentsReadPolicy>(json).expect("deserialize policy"),
            policy
        );
    }

    #[test]
    fn contents_policy_variants_remain_distinct_in_read_requests() {
        let policies = [
            (
                ContentsReadPolicy::PlayableMediaBrowse,
                "playableMediaBrowse",
            ),
            (ContentsReadPolicy::AudioBrowse, "audioBrowse"),
            (
                ContentsReadPolicy::SourceFileInventory {
                    file_classes: vec![ContentsFileClass::Audio],
                },
                "sourceFileInventory",
            ),
            (
                ContentsReadPolicy::PrimaryMedia {
                    media_kinds: vec![PrimaryMediaKind::Audio],
                },
                "primaryMedia",
            ),
        ];

        for (policy, expected_kind) in policies {
            let request = ContentsReadRequest {
                scope: ContentsScope::Directory {
                    source_id: 7,
                    source_directory_id: 11,
                },
                policy,
                scope_depth: ContentsScopeDepth::Recursive,
                limit: Some(25),
                cursor: Some("opaque-cursor".to_string()),
            };

            let command = SnapshotReadCommand::ContentsRead(request.clone());
            let json = serde_json::to_value(&command).expect("serialize contents read");
            assert_eq!(json["type"], json!("contentsRead"));
            assert_eq!(json["payload"]["policy"]["kind"], json!(expected_kind));
            assert_eq!(json["payload"]["cursor"], json!("opaque-cursor"));
            assert_eq!(
                serde_json::from_value::<SnapshotReadCommand>(json)
                    .expect("deserialize contents read"),
                command
            );
        }
    }

    #[test]
    fn source_lifecycle_serializes_semantic_source_facts() {
        let reply = SnapshotReadReply::SourceLifecycle(ReadSourceLifecycleReply {
            lifecycle: Some(SourceLifecycle {
                source_id: 7,
                source_class: SourceClass::ExternalMounted,
                is_user_visible: true,
                mount_status: SourceMountStatus::Unmounted,
                access_state: SourceAccessState::Blocked,
                access_issue_kind: Some(SourceLifecycleIssueKind::UnavailableMount),
                scan_phase: SourceScanPhase::Blocked,
                scan_issue_kind: Some(SourceLifecycleIssueKind::PermissionDenied),
                last_scan_started_at_ms: Some(10),
                last_scan_finished_at_ms: Some(20),
                last_successful_scan_at_ms: None,
                last_seen_at_ms: Some(9),
                updated_at_ms: 30,
            }),
        });

        let json = serde_json::to_value(&reply).expect("serialize lifecycle reply");
        assert_eq!(
            json,
            json!({
                "type": "sourceLifecycle",
                "payload": {
                    "lifecycle": {
                        "sourceId": "7",
                        "sourceClass": "externalMounted",
                        "isUserVisible": true,
                        "mountStatus": "unmounted",
                        "accessState": "blocked",
                        "accessIssueKind": "unavailableMount",
                        "scanPhase": "blocked",
                        "scanIssueKind": "permissionDenied",
                        "lastScanStartedAtMs": 10,
                        "lastScanFinishedAtMs": 20,
                        "lastSeenAtMs": 9,
                        "updatedAtMs": 30
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize lifecycle reply"),
            reply
        );

        let command =
            SnapshotReadCommand::ReadSourceLifecycle(ReadSourceLifecycleRequest { source_id: 7 });
        let json = serde_json::to_value(&command).expect("serialize lifecycle command");
        assert_eq!(
            json,
            json!({
                "type": "readSourceLifecycle",
                "payload": {
                    "sourceId": "7"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(json).expect("deserialize command"),
            command
        );
    }

    #[test]
    fn library_browser_status_fields_are_finite_protocol_values() {
        assert_eq!(
            LibraryAssetAvailabilityState::from_projection_value("available"),
            Some(LibraryAssetAvailabilityState::Available)
        );
        assert_eq!(
            LibraryAssetAvailabilityState::from_projection_value("missing"),
            None
        );
        assert_eq!(
            LibraryAssetStemsStateSummary::from_projection_value("leased"),
            Some(LibraryAssetStemsStateSummary::Leased)
        );
        assert_eq!(
            LibraryAssetStemsStateSummary::from_projection_value("adapter-text"),
            None
        );
        assert_eq!(
            LibraryAssetPrepReadinessSummary::from_projection_value("underprepared"),
            Some(LibraryAssetPrepReadinessSummary::Underprepared)
        );
        assert_eq!(
            LibraryAssetPrepReadinessSummary::from_projection_value("adapter-text"),
            None
        );
        assert_eq!(
            serde_json::to_value(LibraryAssetPrepReadinessSummary::NotRequired)
                .expect("serialize prep readiness"),
            json!("notRequired")
        );
        assert!(
            serde_json::from_value::<LibraryAssetPrepReadinessSummary>(json!("not_required"))
                .is_err(),
            "storage-shaped readiness values must not deserialize at the JS contract boundary"
        );
    }

    #[test]
    fn library_asset_browser_rows_expose_library_asset_identity() {
        let row = LibraryAssetBrowserRow {
            library_asset_id: 42,
            row_version: 1,
            primary_source_file_id: None,
            scoped_source_file_id: None,
            source_id: None,
            relative_path: None,
            file_name: None,
            availability_state: LibraryAssetAvailabilityState::Available,
            title: None,
            artist: None,
            album: None,
            duration_ms: None,
            musical_key: None,
            tempo_bpm: None,
            waveform_quality_current: None,
            waveform_quality_target: None,
            stems_state_summary: None,
            prep_readiness_summary: LibraryAssetPrepReadinessSummary::NotRequired,
            updated_at_ms: 100,
        };

        assert_eq!(row.library_asset_id, 42);
    }

    #[test]
    fn library_tree_children_are_not_asset_browser_rows() {
        let reply = SnapshotReadReply::LibraryTreeChildren(ReadLibraryTreeChildrenReply {
            window: Some(LibraryTreeWindow {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 25,
                total_rows: 1,
                coverage: LibraryTreeCoverage {
                    state: LibraryTreeCoverageState::Scanning,
                    subtree_coverage_complete: false,
                    empty_result_authoritative: false,
                    detail: Some("Still indexing.".to_string()),
                },
                rows: vec![LibraryTreeNode {
                    node_kind: LibraryTreeNodeKind::Directory,
                    source_id: 7,
                    source_directory_id: Some(11),
                    source_file_id: None,
                    parent_source_directory_id: None,
                    relative_path: "Albums".to_string(),
                    display_name: "Albums".to_string(),
                    file_class: None,
                    presence_state: LibraryTreePresenceState::Present,
                    size_bytes: None,
                    modified_at_ns: None,
                    updated_at_ms: 100,
                    has_child_directories: Some(true),
                    directory_primary_media_state: Some(
                        DirectoryPrimaryMediaState::HasPrimaryMediaDescendants,
                    ),
                    directory_image_media_state: Some(DirectoryImageMediaState::Unknown),
                    directory_scan_state: Some(DirectoryScanState::Scanning),
                    navigable_child_scope_state: Some(NavigableChildScopeState::Unknown),
                }],
            }),
        });

        let json = serde_json::to_value(&reply).expect("serialize library tree reply");
        assert_eq!(
            json,
            json!({
                "type": "libraryTreeChildren",
                "payload": {
                    "window": {
                        "entryPoint": {
                            "type": "source",
                            "payload": {
                                "sourceId": "7"
                            }
                        },
                        "parentSourceDirectoryId": null,
                        "offset": 0,
                        "limit": 25,
                        "totalRows": 1,
                        "coverage": {
                            "state": "scanning",
                            "subtreeCoverageComplete": false,
                            "emptyResultAuthoritative": false,
                            "detail": "Still indexing."
                        },
                        "rows": [{
                            "nodeKind": "directory",
                            "sourceId": "7",
                            "sourceDirectoryId": "11",
                            "sourceFileId": null,
                            "parentSourceDirectoryId": null,
                            "relativePath": "Albums",
                            "displayName": "Albums",
                            "presenceState": "present",
                            "sizeBytes": null,
                            "modifiedAtNs": null,
                            "updatedAtMs": 100,
                            "hasChildDirectories": true,
                            "directoryPrimaryMediaState": {
                                "kind": "hasPrimaryMediaDescendants"
                            },
                            "directoryImageMediaState": {
                                "kind": "unknown"
                            },
                            "directoryScanState": "scanning",
                            "navigableChildScopeState": "unknown"
                        }]
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize reply"),
            reply
        );
    }

    #[test]
    fn library_tree_file_class_serializes_as_file_class() {
        let reply = SnapshotReadReply::LibraryTreeChildren(ReadLibraryTreeChildrenReply {
            window: Some(LibraryTreeWindow {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 25,
                total_rows: 1,
                coverage: LibraryTreeCoverage {
                    state: LibraryTreeCoverageState::Complete,
                    subtree_coverage_complete: true,
                    empty_result_authoritative: false,
                    detail: Some("Complete.".to_string()),
                },
                rows: vec![LibraryTreeNode {
                    node_kind: LibraryTreeNodeKind::File,
                    source_id: 7,
                    source_directory_id: None,
                    source_file_id: Some(31),
                    parent_source_directory_id: None,
                    relative_path: "cover.mp3".to_string(),
                    display_name: "cover.mp3".to_string(),
                    file_class: Some(LibraryTreeFileClass::Image),
                    presence_state: LibraryTreePresenceState::Present,
                    size_bytes: Some(10),
                    modified_at_ns: Some(20),
                    updated_at_ms: 100,
                    has_child_directories: None,
                    directory_primary_media_state: None,
                    directory_image_media_state: None,
                    directory_scan_state: None,
                    navigable_child_scope_state: None,
                }],
            }),
        });

        let json = serde_json::to_value(&reply).expect("serialize literal hierarchy reply");
        assert_eq!(
            json["payload"]["window"]["rows"][0]["fileClass"],
            json!("image")
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize reply"),
            reply
        );
    }

    #[test]
    fn library_asset_waveform_overview_payload_is_explicit_and_bucketed() {
        let overview = LibraryAssetWaveformOverview {
            source_profile_key: "default".to_string(),
            source_quality_current: Some(90),
            capability_state: LibraryAssetWaveformOverviewCapabilityState::Ready,
            amplitude_scale: LibraryAssetWaveformOverviewAmplitudeScale::SignedI16,
            bucket_count: 2,
            duration_ms: Some(1_000),
            source_sample_count: Some(44_100),
            samples_per_bucket: Some(22_050),
            buckets: vec![
                LibraryAssetWaveformOverviewBucket {
                    min_amplitude_i16: -120,
                    max_amplitude_i16: 180,
                },
                LibraryAssetWaveformOverviewBucket {
                    min_amplitude_i16: -80,
                    max_amplitude_i16: 140,
                },
            ],
            accepted_artifact_id: 7,
            basis_fingerprint: "basis:waveform:7".to_string(),
            capability_updated_at_ms: 2_000,
            artifact_created_at_ms: 1_900,
        };

        assert_eq!(overview.bucket_count, overview.buckets.len());
        assert_eq!(
            overview.amplitude_scale.as_str(),
            LibraryAssetWaveformOverviewAmplitudeScale::SignedI16.as_str()
        );
        assert_eq!(
            overview.capability_state.as_str(),
            LibraryAssetWaveformOverviewCapabilityState::Ready.as_str()
        );
    }

    #[test]
    fn attachment_identity_snapshot_reads_are_read_only_tagged_commands() {
        let source_file_command =
            SnapshotReadCommand::ReadSourceFileAttachment(ReadSourceFileAttachmentRequest {
                source_file_id: 11,
            });
        assert_eq!(
            serde_json::to_value(&source_file_command).expect("serialize source-file read"),
            json!({
                "type": "readSourceFileAttachment",
                "payload": {
                    "sourceFileId": "11"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(
                serde_json::to_value(source_file_command.clone()).expect("serialize")
            )
            .expect("deserialize source-file read"),
            source_file_command
        );

        let attachment_command =
            SnapshotReadCommand::ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest {
                attachment_id: 7,
                limit: Some(25),
            });
        assert_eq!(
            serde_json::to_value(&attachment_command).expect("serialize attachment read"),
            json!({
                "type": "readAttachmentSourceFiles",
                "payload": {
                    "attachmentId": "7",
                    "limit": 25
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(
                serde_json::to_value(attachment_command.clone()).expect("serialize")
            )
            .expect("deserialize attachment read"),
            attachment_command
        );

        let summary_command =
            SnapshotReadCommand::ReadSourceAttachmentSummary(ReadSourceAttachmentSummaryRequest {
                source_id: 3,
            });
        assert_eq!(
            serde_json::to_value(&summary_command).expect("serialize summary read"),
            json!({
                "type": "readSourceAttachmentSummary",
                "payload": {
                    "sourceId": "3"
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(
                serde_json::to_value(summary_command.clone()).expect("serialize")
            )
            .expect("deserialize summary read"),
            summary_command
        );
    }

    #[test]
    fn track_identity_review_candidates_read_is_limit_only_and_candidate_scoped() {
        let command = SnapshotReadCommand::ReadTrackIdentityReviewCandidates(
            ReadTrackIdentityReviewCandidatesRequest {
                source_id: Some(7),
                review_state: Some(TrackIdentityReviewState::NeedsUserDecision),
                limit: 25,
            },
        );

        let json = serde_json::to_value(&command).expect("serialize review read");
        assert_eq!(
            json,
            json!({
                "type": "readTrackIdentityReviewCandidates",
                "payload": {
                    "sourceId": "7",
                    "reviewState": "needsUserDecision",
                    "limit": 25
                }
            })
        );
        for forbidden in [
            "/payload/cursor",
            "/payload/sourcePath",
            "/payload/filePath",
            "/payload/title",
            "/payload/artist",
            "/payload/album",
            "/payload/metadata",
        ] {
            assert!(
                json.pointer(forbidden).is_none(),
                "{forbidden} must be absent"
            );
        }
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(json).expect("deserialize review read"),
            command
        );

        let reply = SnapshotReadReply::TrackIdentityReviewCandidates(
            ReadTrackIdentityReviewCandidatesReply {
                status: TrackIdentityReviewReadStatus::Ok,
                candidates: vec![TrackIdentityReviewCandidate {
                    candidate_id: 11,
                    candidate_kind: "exact_primary_media_content".to_string(),
                    candidate_evidence_basis: "current_primary_media_exact_blake3".to_string(),
                    candidate_status: TrackIdentityReviewCandidateStatus::Active,
                    evidence_key_algorithm: "blake3".to_string(),
                    evidence_key_value:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    evidence_summary: TrackIdentityReviewEvidenceSummary {
                        member_count: 1,
                        evidence_count: 2,
                        current_evidence_count: 2,
                    },
                    source_summary: TrackIdentityReviewSourceSummary {
                        source_count: 1,
                        source_samples: vec![TrackIdentityReviewSourceSample {
                            source_id: 7,
                            display_name: "Local".to_string(),
                        }],
                    },
                    review_state: TrackIdentityReviewState::UserRejected,
                    effective_decision: Some(TrackIdentityReviewDecision {
                        decision_id: 12,
                        decision_state: TrackIdentityDecisionState::Rejected,
                        decision_source: "user_local_v0".to_string(),
                        decision_basis: "explicit_user_local_decision_v0".to_string(),
                        current_status: TrackIdentityEffectiveDecisionCurrentStatus::Current,
                        created_at_ms: 100,
                        user_blocking_decision_state:
                            TrackIdentityUserBlockingDecisionState::Rejected,
                        masked_system_decision_id: Some(9),
                    }),
                    created_at_ms: 80,
                    updated_at_ms: 90,
                }],
            },
        );

        let json = serde_json::to_value(&reply).expect("serialize review reply");
        assert_eq!(json["type"], json!("trackIdentityReviewCandidates"));
        assert_eq!(json["payload"]["candidates"][0]["candidateId"], json!("11"));
        assert_eq!(
            json["payload"]["candidates"][0]["reviewState"],
            json!("userRejected")
        );
        assert!(json.pointer("/payload/cursor").is_none());
        assert!(json.pointer("/payload/nextCursor").is_none());
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize review reply"),
            reply
        );
    }

    #[test]
    fn attachment_identity_snapshot_replies_expose_identity_state_only() {
        let link = SourceFileAttachmentLink {
            attachment_id: 7,
            source_file_id: 11,
            source_id: 3,
            content_hash_algorithm: "blake3".to_string(),
            content_hash_value: "abc".to_string(),
            file_kind: ContentsFileKind::Audio,
            link_status: SourceFileAttachmentLinkStatus::Current,
            created_at_ms: 100,
            updated_at_ms: 200,
        };
        let source_file_reply =
            SnapshotReadReply::SourceFileAttachment(ReadSourceFileAttachmentReply {
                status: AttachmentIdentityReadStatus::Ok,
                attachment_link: Some(link.clone()),
            });
        let json = serde_json::to_value(&source_file_reply).expect("serialize source-file reply");
        assert_eq!(json["type"], json!("sourceFileAttachment"));
        assert_eq!(json["payload"]["status"], json!("ok"));
        assert_eq!(
            json["payload"]["attachmentLink"]["attachmentId"],
            json!("7")
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize source reply"),
            source_file_reply
        );

        let attachment_reply =
            SnapshotReadReply::AttachmentSourceFiles(ReadAttachmentSourceFilesReply {
                status: AttachmentIdentityReadStatus::Ok,
                attachment: Some(AttachmentIdentity {
                    attachment_id: 7,
                    content_hash_algorithm: "blake3".to_string(),
                    content_hash_value: "abc".to_string(),
                }),
                source_file_links: vec![link],
                effective_limit: 25,
                remaining_source_file_links: 1,
            });
        let json = serde_json::to_value(&attachment_reply).expect("serialize attachment reply");
        assert_eq!(json["type"], json!("attachmentSourceFiles"));
        assert_eq!(json["payload"]["attachment"]["attachmentId"], json!("7"));
        assert_eq!(
            json["payload"]["sourceFileLinks"][0]["linkStatus"],
            json!("current")
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json)
                .expect("deserialize attachment reply"),
            attachment_reply
        );

        let summary_reply =
            SnapshotReadReply::SourceAttachmentSummary(ReadSourceAttachmentSummaryReply {
                status: AttachmentIdentityReadStatus::Ok,
                summary: Some(SourceAttachmentSummary {
                    source_id: 3,
                    current_links_count: 2,
                    stale_links_count: 1,
                    source_files_with_current_blake3_facts_count: 4,
                    source_files_with_attachment_links_count: 3,
                    source_files_missing_attachment_links_count: 2,
                    unmaterialized_blake3_facts_count: 2,
                }),
            });
        let json = serde_json::to_value(&summary_reply).expect("serialize summary reply");
        assert_eq!(json["type"], json!("sourceAttachmentSummary"));
        assert_eq!(json["payload"]["summary"]["sourceId"], json!("3"));
        assert_eq!(
            json["payload"]["summary"]["unmaterializedBlake3FactsCount"],
            json!(2)
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize summary reply"),
            summary_reply
        );
    }

    #[test]
    fn snapshot_read_command_family_keeps_tree_first_node_scoped_boundary() {
        let commands = [
            SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
                parent_navigation_row_id: None,
            }),
            SnapshotReadCommand::LoadNavigationRow(LoadNavigationRowRequest {
                navigation_row_id: 1,
            }),
            SnapshotReadCommand::LoadNavigationRowByStableKey(
                LoadNavigationRowByStableKeyRequest {
                    stable_key: "view:all_media".to_string(),
                },
            ),
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 1 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 50,
            }),
            SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
                ReadNavigationNodeLibraryBrowserWindowRequest {
                    navigation_row_id: 2,
                    offset: 0,
                    limit: 50,
                },
            ),
        ];

        for command in commands {
            let json = serde_json::to_value(&command).expect("serialize snapshot read command");
            let round_trip =
                serde_json::from_value::<SnapshotReadCommand>(json).expect("deserialize command");
            assert_eq!(round_trip, command);
        }
    }

    #[test]
    fn snapshot_read_browser_transport_round_trips_only_node_scoped_paths() {
        let read = SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(
            ReadNavigationNodeLibraryBrowserWindowRequest {
                navigation_row_id: 4,
                offset: 0,
                limit: 20,
            },
        );
        let search = SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(
            SearchNavigationNodeLibraryBrowserWindowRequest {
                navigation_row_id: 4,
                query: "amen".to_string(),
                offset: 0,
                limit: 20,
            },
        );
        let read_reply = SnapshotReadReply::NavigationNodeLibraryBrowserWindow(
            ReadNavigationNodeLibraryBrowserWindowReply { window: None },
        );
        let search_reply = SnapshotReadReply::NavigationNodeLibraryBrowserSearch(
            SearchNavigationNodeLibraryBrowserWindowReply { window: None },
        );

        for command in [read, search] {
            let json = serde_json::to_value(&command).expect("serialize node-scoped command");
            assert_eq!(
                serde_json::from_value::<SnapshotReadCommand>(json).expect("deserialize command"),
                command
            );
        }

        for reply in [read_reply, search_reply] {
            let json = serde_json::to_value(&reply).expect("serialize node-scoped reply");
            assert_eq!(
                serde_json::from_value::<SnapshotReadReply>(json).expect("deserialize reply"),
                reply
            );
        }
    }

    #[test]
    fn retired_direct_browser_transport_tags_are_not_accepted() {
        for tag in [
            "readLibraryBrowserWindow",
            "searchLibraryBrowserWindow",
            "readLocationRootLibraryBrowserWindow",
            "readPrepPolicyLibraryBrowserWindow",
            "searchPrepPolicyLibraryBrowserWindow",
        ] {
            let json = json!({
                "type": tag,
                "payload": {
                    "navigationRowId": "1",
                    "query": "amen",
                    "offset": 0,
                    "limit": 20
                }
            });

            assert!(
                serde_json::from_value::<SnapshotReadCommand>(json).is_err(),
                "retired tag {tag} must not deserialize"
            );
        }
    }

    #[test]
    fn snapshot_read_command_serializes_camel_case_fields() {
        let command = SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(
            SearchNavigationNodeLibraryBrowserWindowRequest {
                navigation_row_id: 4,
                query: "amen".to_string(),
                offset: 0,
                limit: 20,
            },
        );

        let json = serde_json::to_value(&command).expect("serialize snapshot read command");
        assert_eq!(
            json,
            json!({
                "type": "searchNavigationNodeLibraryBrowserWindow",
                "payload": {
                    "navigationRowId": "4",
                    "query": "amen",
                    "offset": 0,
                    "limit": 20
                }
            })
        );
    }

    #[test]
    fn snapshot_read_id_fields_reject_json_numbers() {
        let json = json!({
            "type": "loadNavigationRow",
            "payload": {
                "navigationRowId": 4
            }
        });

        assert!(
            serde_json::from_value::<SnapshotReadCommand>(json).is_err(),
            "durable row ids must cross the wire as strings"
        );
    }

    #[test]
    fn navigation_row_serializes_finite_camel_case_boundary_vocabulary() {
        let row = NavigationRow {
            navigation_row_id: -5,
            stable_key: "view:needs_preparation".to_string(),
            parent_navigation_row_id: None,
            family: Some(NavigationRowFamily::Views),
            row_kind: NavigationRowKind::View,
            display_name: "Needs Preparation".to_string(),
            sibling_position: 4,
            selectable: true,
            selector_kind: Some(NavigationRowSelectorKind::NeedsPreparation),
            selector_payload: Some(String::new()),
            updated_at_ms: 100,
            row_version: 1,
        };

        let json = serde_json::to_value(&row).expect("serialize navigation row");
        assert_eq!(json["navigationRowId"], json!("-5"));
        assert_eq!(json["stableKey"], json!("view:needs_preparation"));
        assert_eq!(json["family"], json!("views"));
        assert_eq!(json["rowKind"], json!("view"));
        assert_eq!(json["selectorKind"], json!("needsPreparation"));
        assert_eq!(
            serde_json::from_value::<NavigationRow>(json).expect("deserialize navigation row"),
            row
        );
        assert!(
            serde_json::from_value::<NavigationRowSelectorKind>(json!("needs_preparation"))
                .is_err(),
            "storage-shaped selector values must not deserialize at the JS contract boundary"
        );
    }

    #[test]
    fn library_browser_row_serializes_transport_value_enums_and_fields() {
        let row = LibraryAssetBrowserRow {
            library_asset_id: 42,
            row_version: 1,
            primary_source_file_id: Some(11),
            scoped_source_file_id: None,
            source_id: Some(3),
            relative_path: Some("Music/track.wav".to_string()),
            file_name: Some("track.wav".to_string()),
            availability_state: LibraryAssetAvailabilityState::Available,
            title: Some("Track".to_string()),
            artist: None,
            album: None,
            duration_ms: Some(1234),
            musical_key: None,
            tempo_bpm: Some(128.0),
            waveform_quality_current: Some(90),
            waveform_quality_target: Some(100),
            stems_state_summary: Some(LibraryAssetStemsStateSummary::Ready),
            prep_readiness_summary: LibraryAssetPrepReadinessSummary::Underprepared,
            updated_at_ms: 1000,
        };

        let json = serde_json::to_value(&row).expect("serialize browser row");
        assert_eq!(json["libraryAssetId"], json!("42"));
        assert_eq!(json["primarySourceFileId"], json!("11"));
        assert_eq!(json["rowVersion"], json!("1"));
        assert_eq!(json["sourceId"], json!("3"));
        assert_eq!(json["availabilityState"], json!("available"));
        assert_eq!(json["stemsStateSummary"], json!("ready"));
        assert_eq!(json["prepReadinessSummary"], json!("underprepared"));
        assert_eq!(
            serde_json::from_value::<LibraryAssetBrowserRow>(json).expect("deserialize row"),
            row
        );
    }

    #[test]
    fn waveform_overview_serializes_camel_case_fields_and_values() {
        let overview = LibraryAssetWaveformOverview {
            source_profile_key: "default".to_string(),
            source_quality_current: Some(90),
            capability_state: LibraryAssetWaveformOverviewCapabilityState::Ready,
            amplitude_scale: LibraryAssetWaveformOverviewAmplitudeScale::SignedI16,
            bucket_count: 1,
            duration_ms: Some(1_000),
            source_sample_count: Some(44_100),
            samples_per_bucket: Some(44_100),
            buckets: vec![LibraryAssetWaveformOverviewBucket {
                min_amplitude_i16: -120,
                max_amplitude_i16: 180,
            }],
            accepted_artifact_id: 7,
            basis_fingerprint: "basis:waveform:7".to_string(),
            capability_updated_at_ms: 2_000,
            artifact_created_at_ms: 1_900,
        };

        let json = serde_json::to_value(&overview).expect("serialize waveform overview");
        assert_eq!(json["sourceProfileKey"], json!("default"));
        assert_eq!(json["capabilityState"], json!("ready"));
        assert_eq!(json["amplitudeScale"], json!("signedI16"));
        assert_eq!(json["acceptedArtifactId"], json!("7"));
        assert_eq!(json["buckets"][0]["minAmplitudeI16"], json!(-120));
        assert_eq!(
            serde_json::from_value::<LibraryAssetWaveformOverview>(json)
                .expect("deserialize waveform overview"),
            overview
        );
    }

    #[test]
    fn preparation_detail_serializes_grouped_js_native_contract_values() {
        let detail = LibraryAssetPreparationDetail {
            library_asset_id: 42,
            aggregate_readiness_summary: LibraryAssetPrepReadinessSummary::Underprepared,
            groups: vec![
                LibraryAssetPreparationDetailGroup {
                    group_key: LibraryAssetPreparationDetailGroupKey::RequiredFacts,
                    rows: vec![LibraryAssetPreparationDetailRow {
                        capability_key: LibraryAssetPreparationCapabilityKey::Bpm,
                        label: "BPM".to_string(),
                        requirement_class: LibraryAssetPreparationRequirementClass::Required,
                        outcome_kind: LibraryAssetPreparationOutcomeKind::Fact,
                        work_state: LibraryAssetPreparationWorkState::None,
                        outcome_state: LibraryAssetPreparationOutcomeState::Missing,
                        satisfaction_state: LibraryAssetPreparationSatisfactionState::Unsatisfied,
                        display_value_summary: None,
                        target_summary: Some("profile default, quality 0, stable".to_string()),
                        explanation_summary: Some("missing required BPM".to_string()),
                        artifact_coverage_state: None,
                        progress: None,
                    }],
                },
                LibraryAssetPreparationDetailGroup {
                    group_key: LibraryAssetPreparationDetailGroupKey::RequiredStructures,
                    rows: vec![LibraryAssetPreparationDetailRow {
                        capability_key: LibraryAssetPreparationCapabilityKey::Beatgrid,
                        label: "Beatgrid".to_string(),
                        requirement_class: LibraryAssetPreparationRequirementClass::Required,
                        outcome_kind: LibraryAssetPreparationOutcomeKind::Structure,
                        work_state: LibraryAssetPreparationWorkState::Queued,
                        outcome_state: LibraryAssetPreparationOutcomeState::Missing,
                        satisfaction_state: LibraryAssetPreparationSatisfactionState::Unsatisfied,
                        display_value_summary: None,
                        target_summary: None,
                        explanation_summary: Some("queued in background".to_string()),
                        artifact_coverage_state: None,
                        progress: None,
                    }],
                },
                LibraryAssetPreparationDetailGroup {
                    group_key: LibraryAssetPreparationDetailGroupKey::RequiredArtifacts,
                    rows: vec![LibraryAssetPreparationDetailRow {
                        capability_key: LibraryAssetPreparationCapabilityKey::Waveform,
                        label: "Waveform".to_string(),
                        requirement_class: LibraryAssetPreparationRequirementClass::Required,
                        outcome_kind: LibraryAssetPreparationOutcomeKind::Artifact,
                        work_state: LibraryAssetPreparationWorkState::None,
                        outcome_state: LibraryAssetPreparationOutcomeState::Ready,
                        satisfaction_state: LibraryAssetPreparationSatisfactionState::Satisfied,
                        display_value_summary: Some("Overview ready".to_string()),
                        target_summary: None,
                        explanation_summary: None,
                        artifact_coverage_state: Some(
                            LibraryAssetPreparationArtifactCoverageState::OverviewReady,
                        ),
                        progress: None,
                    }],
                },
                LibraryAssetPreparationDetailGroup {
                    group_key: LibraryAssetPreparationDetailGroupKey::OnDemandCapabilities,
                    rows: vec![LibraryAssetPreparationDetailRow {
                        capability_key: LibraryAssetPreparationCapabilityKey::MusicalKey,
                        label: "Musical Key".to_string(),
                        requirement_class: LibraryAssetPreparationRequirementClass::OnDemand,
                        outcome_kind: LibraryAssetPreparationOutcomeKind::Fact,
                        work_state: LibraryAssetPreparationWorkState::NotRequested,
                        outcome_state: LibraryAssetPreparationOutcomeState::Missing,
                        satisfaction_state: LibraryAssetPreparationSatisfactionState::Unsatisfied,
                        display_value_summary: None,
                        target_summary: None,
                        explanation_summary: Some(
                            "available on demand; no current obligation".to_string(),
                        ),
                        artifact_coverage_state: None,
                        progress: None,
                    }],
                },
            ],
        };

        let json = serde_json::to_value(&detail).expect("serialize detail");
        assert_eq!(json["libraryAssetId"], json!("42"));
        assert_eq!(json["aggregateReadinessSummary"], json!("underprepared"));
        assert_eq!(json["groups"][0]["groupKey"], json!("requiredFacts"));
        assert_eq!(json["groups"][0]["rows"][0]["capabilityKey"], json!("bpm"));
        assert_eq!(
            json["groups"][1]["rows"][0]["outcomeKind"],
            json!("structure")
        );
        assert_eq!(
            json["groups"][2]["rows"][0]["artifactCoverageState"],
            json!("overviewReady")
        );
        assert!(json["groups"][2]["rows"][0]["progress"].is_null());
        assert_eq!(
            json["groups"][3]["rows"][0]["requirementClass"],
            json!("onDemand")
        );
        assert_eq!(
            serde_json::from_value::<LibraryAssetPreparationDetail>(json)
                .expect("deserialize detail"),
            detail
        );
    }
}
