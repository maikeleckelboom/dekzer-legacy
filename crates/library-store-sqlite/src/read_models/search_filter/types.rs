use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchIndexState {
    Ready,
    Rebuilding,
    Partial,
    Failed,
    Missing,
}

impl StoreSearchIndexState {
    pub(crate) fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "ready" => Ok(Self::Ready),
            "rebuilding" => Ok(Self::Rebuilding),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "search/filter index state contains unsupported value {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StoreSearchScope {
    Library,
    Source {
        source_id: i64,
    },
    SourceLocation {
        source_location_id: i64,
    },
    Directory {
        source_id: i64,
        source_directory_id: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchRecursion {
    Immediate,
    Recursive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSearchState {
    Ready,
    Empty,
    Partial,
    CursorInvalid,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchSort {
    PathName,
    Relevance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchResultKind {
    Source,
    SourceLocation,
    Directory,
    SourceFile,
}

impl StoreSearchResultKind {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::SourceLocation => "source_location",
            Self::Directory => "directory",
            Self::SourceFile => "source_file",
        }
    }

    pub(crate) fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "source" => Ok(Self::Source),
            "source_location" => Ok(Self::SourceLocation),
            "directory" => Ok(Self::Directory),
            "source_file" => Ok(Self::SourceFile),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "search_filter_index_rows contains unsupported result_kind {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSearchAuthorityLayer {
    Source,
    SourceLocation,
    SourceHierarchy,
    SourceFileInventory,
}

impl StoreSearchAuthorityLayer {
    pub(crate) fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "source" => Ok(Self::Source),
            "source_location" => Ok(Self::SourceLocation),
            "source_hierarchy" => Ok(Self::SourceHierarchy),
            "source_file_inventory" => Ok(Self::SourceFileInventory),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "search_filter_index_rows contains unsupported authority_layer {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
    None,
}

impl StoreSearchFileClass {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Image => "image",
            Self::Unsupported => "unsupported",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchFileKind {
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

impl StoreSearchFileKind {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Image => "image",
            Self::CueSheet => "cue_sheet",
            Self::LogDoc => "log_doc",
            Self::TextDoc => "text_doc",
            Self::Archive => "archive",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchPresenceState {
    Present,
    Missing,
    Removed,
}

impl StoreSearchPresenceState {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Missing => "missing",
            Self::Removed => "removed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchAccessState {
    Accessible,
    Missing,
    Blocked,
    Unknown,
}

impl StoreSearchAccessState {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Accessible => "accessible",
            Self::Missing => "missing",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchEvidenceAvailability {
    HasCurrent,
    MissingCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchAttachmentLinkState {
    Current,
    Stale,
    Missing,
    NotApplicable,
}

impl StoreSearchAttachmentLinkState {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Missing => "missing",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub(crate) fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "current" => Ok(Self::Current),
            "stale" => Ok(Self::Stale),
            "missing" => Ok(Self::Missing),
            "not_applicable" => Ok(Self::NotApplicable),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "search_filter_index_rows contains unsupported attachment_link_state {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoreSearchMediaRelevance {
    AudioWorkflow,
    PlayableMedia,
    ExplicitInventory,
    CompanionFile,
    NotMediaRelevant,
}

impl StoreSearchMediaRelevance {
    pub(crate) fn storage_value(self) -> &'static str {
        match self {
            Self::AudioWorkflow => "audio_workflow",
            Self::PlayableMedia => "playable_media",
            Self::ExplicitInventory => "explicit_inventory",
            Self::CompanionFile => "companion_file",
            Self::NotMediaRelevant => "not_media_relevant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSearchEvidenceCoverageState {
    Indexed,
    NotApplicable,
}

impl StoreSearchEvidenceCoverageState {
    pub(crate) fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
        match value {
            "indexed" => Ok(Self::Indexed),
            "not_applicable" => Ok(Self::NotApplicable),
            other => Err(LibrarySqliteError::MalformedSchemaState(format!(
                "search_filter_index_rows contains unsupported evidence_coverage_state {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSearchMatchReason {
    Filter,
    ExactLabel,
    LabelPrefix,
    Label,
    Path,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct StoreSearchFilters {
    pub file_classes: Vec<StoreSearchFileClass>,
    pub file_kinds: Vec<StoreSearchFileKind>,
    pub media_relevance: Vec<StoreSearchMediaRelevance>,
    pub presence_states: Vec<StoreSearchPresenceState>,
    pub source_access_states: Vec<StoreSearchAccessState>,
    pub blake3: Option<StoreSearchEvidenceAvailability>,
    pub probe: Option<StoreSearchEvidenceAvailability>,
    pub attachment_link_states: Vec<StoreSearchAttachmentLinkState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSearchRequest {
    pub scope: StoreSearchScope,
    pub recursion: StoreSearchRecursion,
    pub text_query: Option<String>,
    pub target_kinds: Vec<StoreSearchResultKind>,
    pub filters: StoreSearchFilters,
    pub sort: StoreSearchSort,
    pub limit: usize,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSearchResult {
    pub state: StoreSearchState,
    pub query_identity: StoreSearchQueryIdentity,
    pub index_generation: i64,
    pub index_state: StoreSearchIndexState,
    pub rows: Vec<StoreSearchResultRow>,
    pub next_cursor: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSearchQueryIdentity {
    pub scope: StoreSearchScope,
    pub recursion: StoreSearchRecursion,
    pub text_query: Option<String>,
    pub target_kinds: Vec<StoreSearchResultKind>,
    pub filters: StoreSearchFilters,
    pub sort: StoreSearchSort,
    pub page_size: usize,
    pub index_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSearchResultRow {
    pub result_kind: StoreSearchResultKind,
    pub authority_layer: StoreSearchAuthorityLayer,
    pub stable_key: String,
    pub source_id: Option<i64>,
    pub source_location_id: Option<i64>,
    pub source_directory_id: Option<i64>,
    pub parent_source_directory_id: Option<i64>,
    pub source_file_id: Option<i64>,
    pub display_label: String,
    pub display_path: Option<String>,
    pub relative_path: Option<String>,
    pub file_class: Option<String>,
    pub file_kind: Option<String>,
    pub media_relevance: Option<String>,
    pub presence_state: Option<String>,
    pub source_access_state: Option<String>,
    pub source_scan_phase: Option<String>,
    pub has_current_blake3: bool,
    pub has_current_probe: bool,
    pub attachment_link_state: StoreSearchAttachmentLinkState,
    pub attachment_id: Option<i64>,
    pub content_hash_algorithm: Option<String>,
    pub content_hash_value: Option<String>,
    pub evidence_coverage_state: StoreSearchEvidenceCoverageState,
    pub match_reason: StoreSearchMatchReason,
    pub updated_at: i64,
    pub(crate) relevance_rank: i64,
    pub(crate) sort_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildSearchFilterIndexForSourceResult {
    pub source_id: i64,
    pub generation: i64,
    pub rows_indexed: usize,
}
