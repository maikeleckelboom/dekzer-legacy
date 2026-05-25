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
pub struct ReadSelectedContentsRequest {
    pub scope: SelectedContentsScope,
    pub limit: usize,
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
pub enum SelectedContentsScope {
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
pub struct ReadSelectedContentsReply {
    pub result: SelectedContentsResult,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SelectedContentsResult {
    pub state: SelectedContentsState,
    pub scope: SelectedContentsScope,
    pub rows: Vec<SelectedContentsRow>,
    pub coverage: SelectedContentsCoverage,
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
pub enum SelectedContentsState {
    Ready,
    Empty,
    Partial,
    SourceUnavailable,
    LocationMissing,
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
pub enum SelectedContentsCoverageState {
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
pub struct SelectedContentsCoverage {
    pub state: SelectedContentsCoverageState,
    pub recursive_scope_complete: bool,
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
pub enum SelectedContentsRowOrigin {
    LibraryAsset,
    SourceFile,
}

impl SelectedContentsRowOrigin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryAsset => "libraryAsset",
            Self::SourceFile => "sourceFile",
        }
    }

    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"libraryAsset" => Some(Self::LibraryAsset),
            b"sourceFile" => Some(Self::SourceFile),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SelectedContentsRow {
    pub stable_id: String,
    pub label: String,
    pub origin: SelectedContentsRowOrigin,
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
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub scoped_source_file_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub relative_path: String,
    pub file_name: String,
    pub media_class: SelectedContentsMediaClass,
    pub availability_state: LibraryAssetAvailabilityState,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub musical_key: Option<String>,
    pub tempo_bpm: Option<f64>,
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
pub enum SelectedContentsMediaClass {
    Audio,
    Video,
}

impl SelectedContentsMediaClass {
    pub fn from_projection_value(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"audio" => Some(Self::Audio),
            b"video" => Some(Self::Video),
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
pub enum LiteralHierarchyEntryPoint {
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
pub struct ReadLiteralHierarchyChildrenRequest {
    pub entry_point: LiteralHierarchyEntryPoint,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    pub offset: usize,
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_file_visibility: Option<SourceFileVisibility>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
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
pub enum SourceFileVisibility {
    #[default]
    Performance,
    PerformanceAndImages,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLiteralHierarchyChildrenReply {
    pub window: Option<LiteralHierarchyWindow>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LiteralHierarchyWindow {
    pub entry_point: LiteralHierarchyEntryPoint,
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub parent_source_directory_id: Option<i64>,
    pub offset: usize,
    pub limit: usize,
    pub total_rows: usize,
    pub rows: Vec<LiteralHierarchyNode>,
    pub coverage: LiteralHierarchyCoverage,
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
pub enum LiteralHierarchyCoverageState {
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
pub struct LiteralHierarchyCoverage {
    pub state: LiteralHierarchyCoverageState,
    pub recursive_scope_complete: bool,
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
pub enum LiteralHierarchyNodeKind {
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
pub enum LiteralHierarchyPresenceState {
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
pub enum LiteralHierarchyFileMediaClass {
    Audio,
    Video,
    Image,
    Unsupported,
    None,
}

impl LiteralHierarchyFileMediaClass {
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
pub struct LiteralHierarchyNode {
    pub node_kind: LiteralHierarchyNodeKind,
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
    pub media_class: Option<LiteralHierarchyFileMediaClass>,
    pub presence_state: LiteralHierarchyPresenceState,
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
    ReadLiteralHierarchyChildren(ReadLiteralHierarchyChildrenRequest),
    ReadNavigationNodeLibraryBrowserWindow(ReadNavigationNodeLibraryBrowserWindowRequest),
    SearchNavigationNodeLibraryBrowserWindow(SearchNavigationNodeLibraryBrowserWindowRequest),
    ReadSelectedContents(ReadSelectedContentsRequest),
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
    LiteralHierarchyChildren(ReadLiteralHierarchyChildrenReply),
    NavigationNodeLibraryBrowserWindow(ReadNavigationNodeLibraryBrowserWindowReply),
    NavigationNodeLibraryBrowserSearch(SearchNavigationNodeLibraryBrowserWindowReply),
    SelectedContents(ReadSelectedContentsReply),
    LibraryAssetWaveformOverview(ReadLibraryAssetWaveformOverviewReply),
    LibraryAssetPreparationDetail(ReadLibraryAssetPreparationDetailReply),
}

#[cfg(test)]
mod tests {
    use super::{
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
        LiteralHierarchyCoverage, LiteralHierarchyCoverageState, LiteralHierarchyEntryPoint,
        LiteralHierarchyFileMediaClass, LiteralHierarchyNode, LiteralHierarchyNodeKind,
        LiteralHierarchyPresenceState, LiteralHierarchyWindow, LoadNavigationRowByStableKeyRequest,
        LoadNavigationRowRequest, NavigationRow, NavigationRowFamily, NavigationRowKind,
        NavigationRowSelectorKind, ReadLibraryAssetPreparationDetailRequest,
        ReadLibraryAssetWaveformOverviewRequest, ReadLiteralHierarchyChildrenReply,
        ReadLiteralHierarchyChildrenRequest, ReadNavigationNodeLibraryBrowserWindowReply,
        ReadNavigationNodeLibraryBrowserWindowRequest, ReadNavigationRowsRequest,
        SearchNavigationNodeLibraryBrowserWindowReply,
        SearchNavigationNodeLibraryBrowserWindowRequest, SelectedContentsMediaClass,
        SnapshotReadCommand, SnapshotReadReply, SourceFileVisibility,
    };
    use serde_json::json;

    #[test]
    fn snapshot_read_commands_carry_windowed_read_inputs() {
        let navigation = SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
            parent_navigation_row_id: Some(7),
        });
        let literal_hierarchy = SnapshotReadCommand::ReadLiteralHierarchyChildren(
            ReadLiteralHierarchyChildrenRequest {
                entry_point: LiteralHierarchyEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                offset: 0,
                limit: 50,
                source_file_visibility: Some(SourceFileVisibility::Performance),
            },
        );
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
            literal_hierarchy,
            SnapshotReadCommand::ReadLiteralHierarchyChildren(
                ReadLiteralHierarchyChildrenRequest {
                    entry_point: LiteralHierarchyEntryPoint::Source { source_id: 7 },
                    parent_source_directory_id: None,
                    offset: 0,
                    limit: 50,
                    source_file_visibility: Some(SourceFileVisibility::Performance),
                },
            )
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
    fn selected_contents_media_class_is_primary_media_only() {
        assert_eq!(
            SelectedContentsMediaClass::from_projection_value("audio"),
            Some(SelectedContentsMediaClass::Audio)
        );
        assert_eq!(
            SelectedContentsMediaClass::from_projection_value("video"),
            Some(SelectedContentsMediaClass::Video)
        );
        assert_eq!(
            SelectedContentsMediaClass::from_projection_value("image"),
            None
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
    fn literal_hierarchy_children_are_not_asset_browser_rows() {
        let reply =
            SnapshotReadReply::LiteralHierarchyChildren(ReadLiteralHierarchyChildrenReply {
                window: Some(LiteralHierarchyWindow {
                    entry_point: LiteralHierarchyEntryPoint::Source { source_id: 7 },
                    parent_source_directory_id: None,
                    offset: 0,
                    limit: 25,
                    total_rows: 1,
                    coverage: LiteralHierarchyCoverage {
                        state: LiteralHierarchyCoverageState::Scanning,
                        recursive_scope_complete: false,
                        empty_result_authoritative: false,
                        detail: Some("Still indexing.".to_string()),
                    },
                    rows: vec![LiteralHierarchyNode {
                        node_kind: LiteralHierarchyNodeKind::Directory,
                        source_id: 7,
                        source_directory_id: Some(11),
                        source_file_id: None,
                        parent_source_directory_id: None,
                        relative_path: "Albums".to_string(),
                        display_name: "Albums".to_string(),
                        media_class: None,
                        presence_state: LiteralHierarchyPresenceState::Present,
                        size_bytes: None,
                        modified_at_ns: None,
                        updated_at_ms: 100,
                        has_child_directories: Some(true),
                        directory_primary_media_state: Some(
                            DirectoryPrimaryMediaState::HasPrimaryMediaDescendants,
                        ),
                        directory_image_media_state: Some(DirectoryImageMediaState::Unknown),
                        directory_scan_state: Some(DirectoryScanState::Scanning),
                    }],
                }),
            });

        let json = serde_json::to_value(&reply).expect("serialize literal hierarchy reply");
        assert_eq!(
            json,
            json!({
                "type": "literalHierarchyChildren",
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
                            "recursiveScopeComplete": false,
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
                            "directoryScanState": "scanning"
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
    fn literal_hierarchy_file_media_class_serializes_as_media_class() {
        let reply =
            SnapshotReadReply::LiteralHierarchyChildren(ReadLiteralHierarchyChildrenReply {
                window: Some(LiteralHierarchyWindow {
                    entry_point: LiteralHierarchyEntryPoint::Source { source_id: 7 },
                    parent_source_directory_id: None,
                    offset: 0,
                    limit: 25,
                    total_rows: 1,
                    coverage: LiteralHierarchyCoverage {
                        state: LiteralHierarchyCoverageState::Complete,
                        recursive_scope_complete: true,
                        empty_result_authoritative: false,
                        detail: Some("Complete.".to_string()),
                    },
                    rows: vec![LiteralHierarchyNode {
                        node_kind: LiteralHierarchyNodeKind::File,
                        source_id: 7,
                        source_directory_id: None,
                        source_file_id: Some(31),
                        parent_source_directory_id: None,
                        relative_path: "cover.mp3".to_string(),
                        display_name: "cover.mp3".to_string(),
                        media_class: Some(LiteralHierarchyFileMediaClass::Image),
                        presence_state: LiteralHierarchyPresenceState::Present,
                        size_bytes: Some(10),
                        modified_at_ns: Some(20),
                        updated_at_ms: 100,
                        has_child_directories: None,
                        directory_primary_media_state: None,
                        directory_image_media_state: None,
                        directory_scan_state: None,
                    }],
                }),
            });

        let json = serde_json::to_value(&reply).expect("serialize literal hierarchy reply");
        assert_eq!(
            json["payload"]["window"]["rows"][0]["mediaClass"],
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
            SnapshotReadCommand::ReadLiteralHierarchyChildren(
                ReadLiteralHierarchyChildrenRequest {
                    entry_point: LiteralHierarchyEntryPoint::Source { source_id: 1 },
                    parent_source_directory_id: None,
                    offset: 0,
                    limit: 50,
                    source_file_visibility: Some(SourceFileVisibility::Performance),
                },
            ),
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
