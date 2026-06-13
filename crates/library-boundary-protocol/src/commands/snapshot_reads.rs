use super::{
    ReadSourceMaintenanceReply, ReadSourceMaintenanceRequest, SourceMaintenanceLastRunSummary,
    SourceMaintenanceSourceFailure, TrackIdentityDecisionState,
    TrackIdentityEffectiveDecisionCurrentStatus, TrackIdentityUserBlockingDecisionState,
    search_filter::*,
};

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
pub struct ReadLocalBrowseEntryPointsRequest;

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
pub struct ReadLocalBrowseEntryPointsReply {
    pub status: LocalBrowseEntryPointsReadStatus,
    pub entries: Vec<LocalBrowseEntryPoint>,
    pub failure: Option<LocalBrowseEntryPointFailure>,
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
pub enum LocalBrowseEntryPointsReadStatus {
    Complete,
    PartialFailure,
    Failed,
    UnsupportedPlatform,
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
pub struct LocalBrowseEntryPoint {
    pub identity: LocalBrowseEntryPointIdentity,
    pub display_name: String,
    pub status: LocalBrowseEntryPointStatus,
    pub platform: LocalBrowsePlatform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub matched_source_id: Option<i64>,
    pub available_operations: Vec<LocalBrowseOperation>,
    pub failure: Option<LocalBrowseEntryPointFailure>,
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
pub struct LocalBrowseEntryPointIdentity {
    pub entry_point_kind: LocalBrowseEntryPointKind,
    pub resolved_path: Option<String>,
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
pub enum LocalBrowseEntryPointKind {
    SystemDriveRoot,
    LocalDataVolumeRoot,
    RemovableVolumeRoot,
    UserHome,
    Desktop,
    Downloads,
    Music,
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
pub enum LocalBrowseEntryPointStatus {
    Resolving,
    Available,
    Unavailable,
    PermissionBlocked,
    Missing,
    UnsupportedPlatform,
    DuplicateOfAdmittedSource,
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
pub enum LocalBrowsePlatform {
    Windows,
    Macos,
    Linux,
    Unsupported,
}

#[derive(
    Debug,
    Clone,
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
pub enum LocalBrowseOperation {
    BrowseChildren,
    ChooseDescendant,
    RequestSourceAdmission {
        #[serde(rename = "requestKind")]
        #[ts(rename = "requestKind")]
        request_kind: LocalBrowseSourceAdmissionRequestKind,
        #[serde(rename = "resolvedPath")]
        #[ts(rename = "resolvedPath")]
        resolved_path: String,
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
pub enum LocalBrowseSourceAdmissionRequestKind {
    DefaultMusicFolder,
    SelectedDirectory,
    ParentDirectory,
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
pub enum LocalBrowseProfile {
    Audio,
    Playable,
    AllFiles,
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
pub struct LocalBrowseEntryPointFailure {
    pub code: LocalBrowseEntryPointFailureCode,
    pub detail: String,
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
pub enum LocalBrowseEntryPointFailureCode {
    UnsupportedPlatform,
    KnownFolderUnavailable,
    SystemDriveUnavailable,
    VolumeEnumerationUnavailable,
    MetadataUnavailable,
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
pub struct ReadLocalBrowseItemsRequest {
    pub entry_point_kind: LocalBrowseEntryPointKind,
    pub resolved_root_path: String,
    pub resolved_parent_path: String,
    pub profile: LocalBrowseProfile,
    pub offset: usize,
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
pub struct ReadLocalBrowseItemsReply {
    pub status: LocalBrowseItemsReadStatus,
    pub window_identity: LocalBrowseWindowIdentity,
    pub offset: usize,
    pub limit: usize,
    pub total_items: usize,
    pub items: Vec<LocalBrowseItem>,
    pub failure: Option<LocalBrowseItemFailure>,
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
pub enum LocalBrowseItemsReadStatus {
    Complete,
    PartialFailure,
    Failed,
    UnsupportedPlatform,
    Missing,
    PermissionBlocked,
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
pub struct LocalBrowseWindowIdentity {
    pub entry_point_kind: LocalBrowseEntryPointKind,
    pub resolved_root_path: String,
    pub resolved_parent_path: String,
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
pub struct LocalBrowseItem {
    pub identity: LocalBrowseItemIdentity,
    pub item_kind: LocalBrowseItemKind,
    pub display_name: String,
    pub status: LocalBrowseItemStatus,
    pub platform: LocalBrowsePlatform,
    pub file_kind: Option<ContentsFileKind>,
    pub media_relevance: Option<LocalBrowseItemMediaRelevance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    pub matched_source_id: Option<i64>,
    pub available_operations: Vec<LocalBrowseOperation>,
    pub failure: Option<LocalBrowseItemFailure>,
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
pub struct LocalBrowseItemIdentity {
    pub entry_point_kind: LocalBrowseEntryPointKind,
    pub resolved_root_path: String,
    pub resolved_item_path: String,
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
pub enum LocalBrowseItemKind {
    Directory,
    MediaFile,
    UnsupportedFile,
    RejectedRoot,
    Inaccessible,
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
pub enum LocalBrowseItemStatus {
    Available,
    Unavailable,
    PermissionBlocked,
    Missing,
    UnsupportedPlatform,
    DuplicateOfAdmittedSource,
    Rejected,
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
pub enum LocalBrowseItemMediaRelevance {
    MediaRelevant,
    CompanionMetadata,
    Unsupported,
    Unknown,
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
pub struct LocalBrowseItemFailure {
    pub code: LocalBrowseItemFailureCode,
    pub detail: String,
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
pub enum LocalBrowseItemFailureCode {
    UnsupportedPlatform,
    RootIdentityMismatch,
    RootPathUnavailable,
    ParentPathUnavailable,
    ParentMissing,
    ParentNotDirectory,
    ParentOutsideRoot,
    PermissionDenied,
    MetadataUnavailable,
    EnumerationUnavailable,
    ReparsePointSkipped,
    RejectedRoot,
    UnknownFileType,
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
    Source,
    SourceLocation,
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
    PlayableMedia {
        #[serde(rename = "mediaKinds")]
        #[ts(rename = "mediaKinds")]
        media_kinds: Vec<PlayableMediaKind>,
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
pub enum PlayableMediaKind {
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
    pub playable_media: Option<PlayableMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub updated_at_ms: Option<i64>,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PlayableMedia {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub playable_media_id: i64,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub attachment_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub evidence_source_file_id: i64,
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub row_policy: Option<LibraryTreeRowPolicy>,
    pub offset: usize,
    pub limit: usize,
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
pub enum LibraryTreeRowPolicy {
    AudioBrowse,
    PlayableMediaBrowse,
    SourceFileInventory,
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
pub enum DirectoryPlayableMediaState {
    Unknown,
    HasPlayableMediaDescendants,
    NoPlayableMediaDescendants,
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
    pub directory_playable_media_state: Option<DirectoryPlayableMediaState>,
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
pub struct ReadSourceIntegrityRequest {
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
pub struct ReadSourceIntegrityReply {
    #[serde(with = "crate::wire::i64_string")]
    #[schemars(with = "String")]
    #[ts(as = "String")]
    pub source_id: i64,
    pub source_availability: SourceIntegrityAvailability,
    pub coverage_integrity: SourceIntegrityCoverage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub inventory: Option<SourceIntegrityInventory>,
    pub evidence_and_maintenance: SourceIntegrityEvidenceAndMaintenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachment_integrity: Option<SourceIntegrityAttachmentIntegrity>,
    pub runtime_maintenance: SourceIntegrityRuntimeMaintenance,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceIntegrityAvailability {
    pub state: SourceIntegrityAvailabilityState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lifecycle: Option<SourceLifecycle>,
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
pub enum SourceIntegrityAvailabilityState {
    NotFound,
    Mounted,
    Unavailable,
    Missing,
    Blocked,
    Partial,
    Unknown,
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
pub struct SourceIntegrityCoverage {
    pub state: ContentsScopeCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
    pub total_directories_count: usize,
    pub missing_directories_count: usize,
    pub pending_directories_count: usize,
    pub scanning_directories_count: usize,
    pub blocked_directories_count: usize,
    pub failed_directories_count: usize,
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
pub struct SourceIntegrityInventory {
    pub counts_by_presence_state: Vec<SourceIntegrityPresenceCount>,
    pub counts_by_file_class: Vec<SourceIntegrityFileClassCount>,
    pub counts_by_file_kind: Vec<SourceIntegrityFileKindCount>,
    pub media_relevant_files_count: usize,
    pub present_media_relevant_files_count: usize,
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
pub enum SourceIntegrityFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
    None,
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
pub struct SourceIntegrityPresenceCount {
    pub presence_state: ContentsPresenceState,
    pub count: usize,
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
pub struct SourceIntegrityFileClassCount {
    pub file_class: SourceIntegrityFileClass,
    pub count: usize,
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
pub struct SourceIntegrityFileKindCount {
    pub file_kind: ContentsFileKind,
    pub count: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceIntegrityEvidenceAndMaintenance {
    pub remaining_hash_candidates: usize,
    pub remaining_probe_candidates: usize,
    pub remaining_playable_media_promotion_candidates: usize,
    pub remaining_track_identity_candidate_production_candidates: usize,
    pub remaining_track_identity_decision_production_candidates: usize,
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
pub struct SourceIntegrityAttachmentIntegrity {
    pub current_links_count: usize,
    pub stale_links_count: usize,
    pub missing_links_count: usize,
    pub source_files_with_current_blake3_observations_count: usize,
    pub source_files_with_attachment_links_count: usize,
    pub unmaterialized_blake3_observations_count: usize,
}

#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SourceIntegrityRuntimeMaintenance {
    pub state: SourceIntegrityRuntimeMaintenanceState,
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
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SourceIntegrityRuntimeMaintenanceState {
    Idle,
    Running,
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
pub enum AttachmentSourceFileOccurrenceStatus {
    Available,
    SourceUnavailable,
    SourceMissing,
    SourceBlocked,
    FileMissing,
    FileRemoved,
    Unknown,
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
    pub source_file_attachment_link_id: i64,
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
    pub source_display_name: String,
    pub source_class: SourceClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "crate::wire::option_i64_string")]
    #[schemars(with = "Option<String>")]
    #[ts(as = "Option<String>")]
    #[ts(optional)]
    pub parent_source_directory_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mtime_ns: Option<i64>,
    pub file_kind: ContentsFileKind,
    pub file_class: SearchFilterFileClass,
    pub presence_state: ContentsPresenceState,
    pub has_current_blake3_observation: bool,
    pub link_status: SourceFileAttachmentLinkStatus,
    pub source_mount_status: SourceMountStatus,
    pub source_access_state: SourceAccessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_access_issue_kind: Option<SourceLifecycleIssueKind>,
    pub source_scan_phase: SourceScanPhase,
    pub source_availability_state: SourceIntegrityAvailabilityState,
    pub occurrence_status: AttachmentSourceFileOccurrenceStatus,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub source_file_updated_at_ms: i64,
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
pub struct AttachmentSourceFilesSummary {
    pub total_occurrence_count: usize,
    pub available_occurrence_count: usize,
    pub unavailable_occurrence_count: usize,
    pub current_link_occurrence_count: usize,
    pub stale_link_occurrence_count: usize,
    pub distinct_source_count: usize,
    pub has_multiple_occurrences: bool,
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
    pub source_files_with_current_blake3_observations_count: usize,
    pub source_files_with_attachment_links_count: usize,
    pub source_files_missing_attachment_links_count: usize,
    pub unmaterialized_blake3_observations_count: usize,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub summary: Option<AttachmentSourceFilesSummary>,
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
    ReadLocalBrowseEntryPoints(ReadLocalBrowseEntryPointsRequest),
    ReadLocalBrowseItems(ReadLocalBrowseItemsRequest),
    ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest),
    ReadSourceLifecycle(ReadSourceLifecycleRequest),
    ReadSourceIntegrity(ReadSourceIntegrityRequest),
    ReadSourceMaintenance(ReadSourceMaintenanceRequest),
    ReadSourceFileAttachment(ReadSourceFileAttachmentRequest),
    ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest),
    ReadSourceAttachmentSummary(ReadSourceAttachmentSummaryRequest),
    ReadTrackIdentityReviewCandidates(ReadTrackIdentityReviewCandidatesRequest),
    ContentsRead(ContentsReadRequest),
    SearchFilterRead(SearchFilterReadRequest),
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
    LocalBrowseEntryPoints(ReadLocalBrowseEntryPointsReply),
    LocalBrowseItems(ReadLocalBrowseItemsReply),
    LibraryTreeChildren(ReadLibraryTreeChildrenReply),
    SourceLifecycle(ReadSourceLifecycleReply),
    SourceIntegrity(Box<ReadSourceIntegrityReply>),
    SourceMaintenance(Box<ReadSourceMaintenanceReply>),
    SourceFileAttachment(ReadSourceFileAttachmentReply),
    AttachmentSourceFiles(ReadAttachmentSourceFilesReply),
    SourceAttachmentSummary(ReadSourceAttachmentSummaryReply),
    TrackIdentityReviewCandidates(ReadTrackIdentityReviewCandidatesReply),
    Contents(ContentsReadReply),
    SearchFilter(Box<SearchFilterReadReply>),
}

#[cfg(test)]
mod tests {
    use super::{
        AttachmentIdentity, AttachmentIdentityReadStatus, AttachmentSourceFileOccurrenceStatus,
        AttachmentSourceFilesSummary, ContentsFileClass, ContentsFileKind, ContentsPresenceState,
        ContentsReadPolicy, ContentsReadRequest, ContentsScope, ContentsScopeCoverageState,
        ContentsScopeDepth, DirectoryImageMediaState, DirectoryPlayableMediaState,
        DirectoryScanState, LibraryTreeCoverage, LibraryTreeCoverageState, LibraryTreeEntryPoint,
        LibraryTreeFileClass, LibraryTreeNode, LibraryTreeNodeKind, LibraryTreePresenceState,
        LibraryTreeRowPolicy, LibraryTreeWindow, LoadNavigationRowByStableKeyRequest,
        LoadNavigationRowRequest, LocalBrowseEntryPoint, LocalBrowseEntryPointFailure,
        LocalBrowseEntryPointFailureCode, LocalBrowseEntryPointIdentity, LocalBrowseEntryPointKind,
        LocalBrowseEntryPointStatus, LocalBrowseEntryPointsReadStatus, LocalBrowseItem,
        LocalBrowseItemFailure, LocalBrowseItemFailureCode, LocalBrowseItemIdentity,
        LocalBrowseItemKind, LocalBrowseItemMediaRelevance, LocalBrowseItemStatus,
        LocalBrowseItemsReadStatus, LocalBrowseOperation, LocalBrowsePlatform, LocalBrowseProfile,
        LocalBrowseSourceAdmissionRequestKind, LocalBrowseWindowIdentity, NavigableChildScopeState,
        NavigationRow, NavigationRowFamily, NavigationRowKind, NavigationRowSelectorKind,
        PlayableMedia, PlayableMediaKind, ReadAttachmentSourceFilesReply,
        ReadAttachmentSourceFilesRequest, ReadLibraryTreeChildrenReply,
        ReadLibraryTreeChildrenRequest, ReadLocalBrowseEntryPointsReply,
        ReadLocalBrowseEntryPointsRequest, ReadLocalBrowseItemsReply, ReadLocalBrowseItemsRequest,
        ReadNavigationRowsRequest, ReadSourceAttachmentSummaryReply,
        ReadSourceAttachmentSummaryRequest, ReadSourceFileAttachmentReply,
        ReadSourceFileAttachmentRequest, ReadSourceIntegrityReply, ReadSourceLifecycleReply,
        ReadSourceLifecycleRequest, ReadTrackIdentityReviewCandidatesReply,
        ReadTrackIdentityReviewCandidatesRequest, SearchFilterFileClass, SnapshotReadCommand,
        SnapshotReadReply, SourceAccessState, SourceAttachmentSummary, SourceClass,
        SourceFileAttachmentLink, SourceFileAttachmentLinkStatus, SourceIntegrityAvailability,
        SourceIntegrityAvailabilityState, SourceIntegrityCoverage,
        SourceIntegrityEvidenceAndMaintenance, SourceIntegrityRuntimeMaintenance,
        SourceIntegrityRuntimeMaintenanceState, SourceLifecycle, SourceLifecycleIssueKind,
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
        let entry_points =
            SnapshotReadCommand::ReadLocalBrowseEntryPoints(ReadLocalBrowseEntryPointsRequest);
        let local_items = SnapshotReadCommand::ReadLocalBrowseItems(ReadLocalBrowseItemsRequest {
            entry_point_kind: LocalBrowseEntryPointKind::Music,
            resolved_root_path: "C:\\Users\\DJ\\Music".to_string(),
            resolved_parent_path: "C:\\Users\\DJ\\Music\\Albums".to_string(),
            profile: LocalBrowseProfile::Audio,
            offset: 5,
            limit: 25,
        });
        let library_tree =
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                row_policy: Some(LibraryTreeRowPolicy::AudioBrowse),
                offset: 0,
                limit: 50,
            });

        assert!(matches!(
            navigation,
            SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
                parent_navigation_row_id: Some(7),
            })
        ));
        assert!(matches!(
            entry_points,
            SnapshotReadCommand::ReadLocalBrowseEntryPoints(ReadLocalBrowseEntryPointsRequest)
        ));
        assert!(matches!(
            local_items,
            SnapshotReadCommand::ReadLocalBrowseItems(ReadLocalBrowseItemsRequest {
                entry_point_kind: LocalBrowseEntryPointKind::Music,
                ref resolved_root_path,
                ref resolved_parent_path,
                profile: LocalBrowseProfile::Audio,
                offset: 5,
                limit: 25,
            }) if resolved_root_path == "C:\\Users\\DJ\\Music"
                && resolved_parent_path == "C:\\Users\\DJ\\Music\\Albums"
        ));
        assert!(matches!(
            library_tree,
            SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id: 7 },
                parent_source_directory_id: None,
                row_policy: Some(LibraryTreeRowPolicy::AudioBrowse),
                offset: 0,
                limit: 50,
            },)
        ));
    }

    #[test]
    fn local_browse_entry_points_serialize_boundary_shape() {
        let command =
            SnapshotReadCommand::ReadLocalBrowseEntryPoints(ReadLocalBrowseEntryPointsRequest);
        let command_json = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(
            command_json,
            json!({
                "type": "readLocalBrowseEntryPoints",
                "payload": null
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(command_json)
                .expect("deserialize command"),
            command
        );

        let reply = SnapshotReadReply::LocalBrowseEntryPoints(ReadLocalBrowseEntryPointsReply {
            status: LocalBrowseEntryPointsReadStatus::Complete,
            entries: vec![LocalBrowseEntryPoint {
                identity: LocalBrowseEntryPointIdentity {
                    entry_point_kind: LocalBrowseEntryPointKind::Music,
                    resolved_path: Some("C:\\Users\\DJ\\Music".to_string()),
                },
                display_name: "Music".to_string(),
                status: LocalBrowseEntryPointStatus::Available,
                matched_source_id: None,
                platform: LocalBrowsePlatform::Windows,
                available_operations: vec![
                    LocalBrowseOperation::BrowseChildren,
                    LocalBrowseOperation::ChooseDescendant,
                    LocalBrowseOperation::RequestSourceAdmission {
                        request_kind: LocalBrowseSourceAdmissionRequestKind::DefaultMusicFolder,
                        resolved_path: "C:\\Users\\DJ\\Music".to_string(),
                    },
                ],
                failure: None,
            }],
            failure: Some(LocalBrowseEntryPointFailure {
                code: LocalBrowseEntryPointFailureCode::VolumeEnumerationUnavailable,
                detail: "volume enumeration failed".to_string(),
            }),
        });
        let reply_json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(
            reply_json,
            json!({
                "type": "localBrowseEntryPoints",
                "payload": {
                    "status": "complete",
                    "entries": [{
                        "identity": {
                            "entryPointKind": "music",
                            "resolvedPath": "C:\\Users\\DJ\\Music"
                        },
                        "displayName": "Music",
                        "status": "available",
                        "platform": "windows",
                        "availableOperations": [
                            { "kind": "browseChildren" },
                            { "kind": "chooseDescendant" },
                            {
                                "kind": "requestSourceAdmission",
                                "requestKind": "defaultMusicFolder",
                                "resolvedPath": "C:\\Users\\DJ\\Music"
                            }
                        ],
                        "failure": null
                    }],
                    "failure": {
                        "code": "volumeEnumerationUnavailable",
                        "detail": "volume enumeration failed"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(reply_json).expect("deserialize reply"),
            reply
        );
    }

    #[test]
    fn local_browse_items_serialize_boundary_shape() {
        let command = SnapshotReadCommand::ReadLocalBrowseItems(ReadLocalBrowseItemsRequest {
            entry_point_kind: LocalBrowseEntryPointKind::Music,
            resolved_root_path: "C:\\Users\\DJ\\Music".to_string(),
            resolved_parent_path: "C:\\Users\\DJ\\Music".to_string(),
            profile: LocalBrowseProfile::Audio,
            offset: 0,
            limit: 50,
        });
        let command_json = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(
            command_json,
            json!({
                "type": "readLocalBrowseItems",
                    "payload": {
                        "entryPointKind": "music",
                        "resolvedRootPath": "C:\\Users\\DJ\\Music",
                        "resolvedParentPath": "C:\\Users\\DJ\\Music",
                        "profile": "audio",
                        "offset": 0,
                        "limit": 50
                    }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadCommand>(command_json)
                .expect("deserialize command"),
            command
        );

        let reply = SnapshotReadReply::LocalBrowseItems(ReadLocalBrowseItemsReply {
            status: LocalBrowseItemsReadStatus::Complete,
            window_identity: LocalBrowseWindowIdentity {
                entry_point_kind: LocalBrowseEntryPointKind::Music,
                resolved_root_path: "C:\\Users\\DJ\\Music".to_string(),
                resolved_parent_path: "C:\\Users\\DJ\\Music".to_string(),
            },
            offset: 0,
            limit: 50,
            total_items: 1,
            items: vec![LocalBrowseItem {
                identity: LocalBrowseItemIdentity {
                    entry_point_kind: LocalBrowseEntryPointKind::Music,
                    resolved_root_path: "C:\\Users\\DJ\\Music".to_string(),
                    resolved_item_path: "C:\\Users\\DJ\\Music\\Track.flac".to_string(),
                },
                item_kind: LocalBrowseItemKind::MediaFile,
                display_name: "Track.flac".to_string(),
                status: LocalBrowseItemStatus::Available,
                matched_source_id: None,
                platform: LocalBrowsePlatform::Windows,
                file_kind: Some(ContentsFileKind::Audio),
                media_relevance: Some(LocalBrowseItemMediaRelevance::MediaRelevant),
                available_operations: vec![LocalBrowseOperation::RequestSourceAdmission {
                    request_kind: LocalBrowseSourceAdmissionRequestKind::ParentDirectory,
                    resolved_path: "C:\\Users\\DJ\\Music".to_string(),
                }],
                failure: None,
            }],
            failure: Some(LocalBrowseItemFailure {
                code: LocalBrowseItemFailureCode::EnumerationUnavailable,
                detail: "one child failed".to_string(),
            }),
        });
        let reply_json = serde_json::to_value(&reply).expect("serialize reply");
        assert_eq!(
            reply_json,
            json!({
                "type": "localBrowseItems",
                "payload": {
                    "status": "complete",
                    "windowIdentity": {
                        "entryPointKind": "music",
                        "resolvedRootPath": "C:\\Users\\DJ\\Music",
                        "resolvedParentPath": "C:\\Users\\DJ\\Music"
                    },
                    "offset": 0,
                    "limit": 50,
                    "totalItems": 1,
                    "items": [{
                        "identity": {
                            "entryPointKind": "music",
                            "resolvedRootPath": "C:\\Users\\DJ\\Music",
                            "resolvedItemPath": "C:\\Users\\DJ\\Music\\Track.flac"
                        },
                        "itemKind": "mediaFile",
                        "displayName": "Track.flac",
                        "status": "available",
                        "platform": "windows",
                        "fileKind": "audio",
                        "mediaRelevance": "mediaRelevant",
                        "availableOperations": [{
                            "kind": "requestSourceAdmission",
                            "requestKind": "parentDirectory",
                            "resolvedPath": "C:\\Users\\DJ\\Music"
                        }],
                        "failure": null
                    }],
                    "failure": {
                        "code": "enumerationUnavailable",
                        "detail": "one child failed"
                    }
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(reply_json).expect("deserialize reply"),
            reply
        );
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
    fn contents_playable_media_serializes_required_durable_fields() {
        let media = PlayableMedia {
            playable_media_id: 5,
            attachment_id: 7,
            content_hash_algorithm: "blake3".to_string(),
            content_hash_value: "abc".to_string(),
            evidence_source_file_id: 11,
            media_kind: "audio".to_string(),
            mime_type: None,
            duration_ms: Some(120_000),
            sample_rate_hz: None,
            channels: None,
            bit_depth: None,
            codec: None,
        };

        let json = serde_json::to_value(&media).expect("serialize playable media");
        assert_eq!(
            json,
            json!({
                "playableMediaId": "5",
                "attachmentId": "7",
                "contentHashAlgorithm": "blake3",
                "contentHashValue": "abc",
                "evidenceSourceFileId": "11",
                "mediaKind": "audio",
                "mimeType": null,
                "durationMs": 120000,
                "sampleRateHz": null,
                "channels": null,
                "bitDepth": null,
                "codec": null
            })
        );
        assert_eq!(
            serde_json::from_value::<PlayableMedia>(json).expect("deserialize playable media"),
            media
        );
    }

    #[test]
    fn authority_docs_avoid_old_playable_media_boundary_wording() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let docs = [
            "docs/library/evidence/playable-media-promotion-contract.md",
            "docs/library/evidence/media-identity-schema-authority.md",
            "docs/library/identity/track-identity-candidate-contract.md",
            "docs/docs-authority-map.md",
            "docs/library/roadmap/product-roadmap-and-substrate-authority.md",
        ];
        let disallowed = [
            concat!("observed-file-", "observations"),
            concat!("playable-media ", "candidate"),
            concat!("playableMedia ", "candidate"),
            concat!("candidate ", "row"),
            concat!("candidate ", "rows"),
            concat!("candidate ", "projection"),
            concat!("primary", "Media"),
            concat!("Primary", "Media"),
            concat!("source_file_", "facts"),
            concat!("Source", "Facts"),
            concat!("primary_media_", "facts"),
            concat!("primary_media_", "fact"),
        ];

        for doc in docs {
            let path = repo_root.join(doc);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            for phrase in disallowed {
                assert!(
                    !text.contains(phrase),
                    "{} contains old playable-media boundary wording: {phrase}",
                    path.display()
                );
            }
        }
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
                ContentsReadPolicy::PlayableMedia {
                    media_kinds: vec![PlayableMediaKind::Audio],
                },
                "playableMedia",
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
    fn source_integrity_coverage_exposes_missing_directory_count() {
        let reply = SnapshotReadReply::SourceIntegrity(Box::new(ReadSourceIntegrityReply {
            source_id: 7,
            source_availability: SourceIntegrityAvailability {
                state: SourceIntegrityAvailabilityState::Mounted,
                lifecycle: None,
                source_failure: None,
            },
            coverage_integrity: SourceIntegrityCoverage {
                state: ContentsScopeCoverageState::Incomplete,
                subtree_coverage_complete: false,
                empty_result_authoritative: false,
                total_directories_count: 2,
                missing_directories_count: 1,
                pending_directories_count: 0,
                scanning_directories_count: 0,
                blocked_directories_count: 0,
                failed_directories_count: 0,
            },
            inventory: None,
            evidence_and_maintenance: SourceIntegrityEvidenceAndMaintenance {
                remaining_hash_candidates: 0,
                remaining_probe_candidates: 0,
                remaining_playable_media_promotion_candidates: 0,
                remaining_track_identity_candidate_production_candidates: 0,
                remaining_track_identity_decision_production_candidates: 0,
                source_failure: None,
            },
            attachment_integrity: None,
            runtime_maintenance: SourceIntegrityRuntimeMaintenance {
                state: SourceIntegrityRuntimeMaintenanceState::Idle,
                last_run: None,
            },
        }));

        let json = serde_json::to_value(&reply).expect("serialize source integrity reply");
        assert_eq!(
            json["payload"]["coverageIntegrity"]["missingDirectoriesCount"],
            json!(1)
        );
        assert_eq!(
            serde_json::from_value::<SnapshotReadReply>(json)
                .expect("deserialize source integrity reply"),
            reply
        );
    }

    #[test]
    fn source_lifecycle_serializes_semantic_source_state() {
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
    fn library_tree_children_are_source_tree_rows() {
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
                    directory_playable_media_state: Some(
                        DirectoryPlayableMediaState::HasPlayableMediaDescendants,
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
                            "directoryPlayableMediaState": {
                                "kind": "hasPlayableMediaDescendants"
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
                    directory_playable_media_state: None,
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
                    candidate_kind: "exact_playable_media_content".to_string(),
                    candidate_evidence_basis: "current_playable_media_exact_blake3".to_string(),
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
            source_file_attachment_link_id: 5,
            attachment_id: 7,
            source_file_id: 11,
            source_id: 3,
            content_hash_algorithm: "blake3".to_string(),
            content_hash_value: "abc".to_string(),
            source_display_name: "Local".to_string(),
            source_class: SourceClass::ExternalMounted,
            parent_source_directory_id: Some(2),
            name: "track.flac".to_string(),
            relative_path: "Album/track.flac".to_string(),
            size_bytes: Some(123),
            mtime_ns: Some(456),
            file_kind: ContentsFileKind::Audio,
            file_class: SearchFilterFileClass::Audio,
            presence_state: ContentsPresenceState::Present,
            has_current_blake3_observation: true,
            link_status: SourceFileAttachmentLinkStatus::Current,
            source_mount_status: SourceMountStatus::Mounted,
            source_access_state: SourceAccessState::Accessible,
            source_access_issue_kind: None,
            source_scan_phase: SourceScanPhase::Complete,
            source_availability_state: SourceIntegrityAvailabilityState::Mounted,
            occurrence_status: AttachmentSourceFileOccurrenceStatus::Available,
            created_at_ms: 100,
            updated_at_ms: 200,
            source_file_updated_at_ms: 300,
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
                summary: Some(AttachmentSourceFilesSummary {
                    total_occurrence_count: 2,
                    available_occurrence_count: 1,
                    unavailable_occurrence_count: 1,
                    current_link_occurrence_count: 1,
                    stale_link_occurrence_count: 1,
                    distinct_source_count: 2,
                    has_multiple_occurrences: true,
                }),
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
            json["payload"]["sourceFileLinks"][0]["occurrenceStatus"],
            json!("available")
        );
        assert_eq!(json["payload"]["summary"]["totalOccurrenceCount"], json!(2));
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
                    source_files_with_current_blake3_observations_count: 4,
                    source_files_with_attachment_links_count: 3,
                    source_files_missing_attachment_links_count: 2,
                    unmaterialized_blake3_observations_count: 2,
                }),
            });
        let json = serde_json::to_value(&summary_reply).expect("serialize summary reply");
        assert_eq!(json["type"], json!("sourceAttachmentSummary"));
        assert_eq!(json["payload"]["summary"]["sourceId"], json!("3"));
        assert_eq!(
            json["payload"]["summary"]["unmaterializedBlake3ObservationsCount"],
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
                row_policy: None,
                offset: 0,
                limit: 50,
            }),
        ];

        for command in commands {
            let json = serde_json::to_value(&command).expect("serialize snapshot read command");
            let round_trip =
                serde_json::from_value::<SnapshotReadCommand>(json).expect("deserialize command");
            assert_eq!(round_trip, command);
        }
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
            stable_key: "view:all_media".to_string(),
            parent_navigation_row_id: None,
            family: Some(NavigationRowFamily::Views),
            row_kind: NavigationRowKind::View,
            display_name: "All Media".to_string(),
            sibling_position: 4,
            selectable: true,
            selector_kind: Some(NavigationRowSelectorKind::AllMedia),
            selector_payload: Some(String::new()),
            updated_at_ms: 100,
            row_version: 1,
        };

        let json = serde_json::to_value(&row).expect("serialize navigation row");
        assert_eq!(json["navigationRowId"], json!("-5"));
        assert_eq!(json["stableKey"], json!("view:all_media"));
        assert_eq!(json["family"], json!("views"));
        assert_eq!(json["rowKind"], json!("view"));
        assert_eq!(json["selectorKind"], json!("allMedia"));
        assert_eq!(
            serde_json::from_value::<NavigationRow>(json).expect("deserialize navigation row"),
            row
        );
        assert!(
            serde_json::from_value::<NavigationRowSelectorKind>(json!("all_media")).is_err(),
            "storage-shaped selector values must not deserialize at the JS contract boundary"
        );
    }
}
