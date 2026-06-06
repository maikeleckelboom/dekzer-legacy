use rusqlite::{Connection, OptionalExtension, params};

use crate::read_models::source_location_coverage::{
    AcceptedSourceLocationCoverage, SourceLocationCoverage, aggregate_source_location_coverages,
    classify_source_location_coverage,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

const CONTENTS_CURSOR_VERSION: u8 = 3;
const CONTENTS_CURSOR_FILE_CLASS_ORDER: [StoreContentsFileClass; 4] = [
    StoreContentsFileClass::Audio,
    StoreContentsFileClass::Video,
    StoreContentsFileClass::Image,
    StoreContentsFileClass::Unsupported,
];
const PLAYABLE_MEDIA_BROWSE_FILE_CLASSES: [StoreContentsFileClass; 2] =
    [StoreContentsFileClass::Audio, StoreContentsFileClass::Video];
const AUDIO_BROWSE_FILE_CLASSES: [StoreContentsFileClass; 1] = [StoreContentsFileClass::Audio];
const BROWSE_RELEVANT_FILE_CLASSES: [StoreContentsFileClass; 3] = [
    StoreContentsFileClass::Audio,
    StoreContentsFileClass::Video,
    StoreContentsFileClass::Image,
];
const CONTENTS_CURSOR_PRIMARY_MEDIA_KIND_ORDER: [StorePrimaryMediaKind; 2] =
    [StorePrimaryMediaKind::Audio, StorePrimaryMediaKind::Video];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ContentsCursor {
    version: u8,
    scope: ContentsCursorScope,
    policy: ContentsCursorPolicy,
    scope_depth: String,
    position: ContentsCursorPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ContentsCursorPolicy {
    PlayableMediaBrowse,
    AudioBrowse,
    SourceFileInventory { file_classes: Vec<String> },
    PrimaryMedia { media_kinds: Vec<String> },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum ContentsCursorScope {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum ContentsCursorPosition {
    SourceFile {
        relative_path_browse_sort_key: String,
        relative_path: String,
        source_file_id: i64,
    },
    PrimaryMedia {
        availability_priority: i64,
        title_key: String,
        artist_key: String,
        album_key: String,
        relative_path_key: String,
        source_file_id: i64,
    },
}

fn encode_cursor(cursor: &ContentsCursor) -> Result<String, LibrarySqliteError> {
    let json = serde_json::to_string(cursor).map_err(|e| {
        LibrarySqliteError::MalformedSchemaState(format!("cursor encode failed: {e}"))
    })?;
    Ok(base64url_encode(json.as_bytes()))
}

fn decode_cursor(cursor: &str) -> Result<ContentsCursor, LibrarySqliteError> {
    let bytes = base64url_decode(cursor).map_err(|e| {
        LibrarySqliteError::MalformedSchemaState(format!("cursor decode failed: {e}"))
    })?;
    let json = String::from_utf8(bytes).map_err(|e| {
        LibrarySqliteError::MalformedSchemaState(format!("cursor utf8 failed: {e}"))
    })?;
    serde_json::from_str(&json)
        .map_err(|e| LibrarySqliteError::MalformedSchemaState(format!("cursor parse failed: {e}")))
}

fn base64url_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 2 < input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(n & 0x3F) as usize] as char);
        i += 3;
    }
    if input.len() - i == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
    } else if input.len() - i == 1 {
        let n = (input[i] as u32) << 16;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
    }
    out
}

fn base64url_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: &[u8; 256] = &{
        let mut table = [255u8; 256];
        let mut i = 0;
        while i < 26 {
            table[(b'A' + i) as usize] = i;
            table[(b'a' + i) as usize] = i + 26;
            i += 1;
        }
        table[b'0' as usize] = 52;
        table[b'1' as usize] = 53;
        table[b'2' as usize] = 54;
        table[b'3' as usize] = 55;
        table[b'4' as usize] = 56;
        table[b'5' as usize] = 57;
        table[b'6' as usize] = 58;
        table[b'7' as usize] = 59;
        table[b'8' as usize] = 60;
        table[b'9' as usize] = 61;
        table[b'-' as usize] = 62;
        table[b'_' as usize] = 63;
        table
    };
    let input_bytes = input.as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for &b in input_bytes {
        let v = TABLE[b as usize];
        if v == 255 {
            return Err(format!("invalid base64url character: {}", b as char));
        }
        buf = (buf << 6) | (v as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn validate_cursor_identity(
    cursor: &ContentsCursor,
    scope: &StoreContentsScope,
    policy: &StoreContentsReadPolicy,
    scope_depth: StoreContentsScopeDepth,
) -> bool {
    if cursor.version != CONTENTS_CURSOR_VERSION {
        return false;
    }
    let scope_matches = match (&cursor.scope, scope) {
        (
            ContentsCursorScope::Source { source_id: a },
            StoreContentsScope::Source { source_id: b },
        ) => a == b,
        (
            ContentsCursorScope::SourceLocation {
                source_location_id: a,
            },
            StoreContentsScope::SourceLocation {
                source_location_id: b,
            },
        ) => a == b,
        (
            ContentsCursorScope::Directory {
                source_id: a,
                source_directory_id: b_dir,
            },
            StoreContentsScope::Directory {
                source_id: c,
                source_directory_id: d,
            },
        ) => a == c && b_dir == d,
        _ => false,
    };
    if !scope_matches {
        return false;
    }
    if cursor.policy != canonical_cursor_policy(policy) {
        return false;
    }
    let expected_scope_depth = match scope_depth {
        StoreContentsScopeDepth::Immediate => "immediate",
        StoreContentsScopeDepth::Recursive => "recursive",
    };
    cursor.scope_depth == expected_scope_depth
}

fn compute_cursor_position(
    row: &StoreContentsFileRow,
    policy: &StoreContentsReadPolicy,
) -> ContentsCursorPosition {
    match policy {
        StoreContentsReadPolicy::PlayableMediaBrowse
        | StoreContentsReadPolicy::SourceFileInventory { .. }
        | StoreContentsReadPolicy::AudioBrowse => ContentsCursorPosition::SourceFile {
            relative_path_browse_sort_key: row
                .relative_path_browse_sort_key
                .clone()
                .expect("source-file contents rows select relative_path_browse_sort_key"),
            relative_path: row.relative_path.clone(),
            source_file_id: row.source_file_id,
        },
        StoreContentsReadPolicy::PrimaryMedia { .. } => ContentsCursorPosition::PrimaryMedia {
            availability_priority: availability_priority(&row.availability_state),
            title_key: compute_title_key(row),
            artist_key: compute_artist_key(row),
            album_key: compute_album_key(row),
            relative_path_key: row.relative_path.to_lowercase(),
            source_file_id: row.source_file_id,
        },
    }
}

fn availability_priority(availability_state: &Option<String>) -> i64 {
    match availability_state.as_deref() {
        Some("available") => 0,
        Some("degraded") => 1,
        Some("unavailable") => 2,
        _ => 3,
    }
}

fn compute_title_key(row: &StoreContentsFileRow) -> String {
    row.primary_media
        .as_ref()
        .and_then(|pm| pm.title.as_ref())
        .map(|t| t.to_lowercase())
        .unwrap_or_else(|| row.relative_path.to_lowercase())
}

fn compute_artist_key(row: &StoreContentsFileRow) -> String {
    row.primary_media
        .as_ref()
        .and_then(|pm| pm.artist.as_ref())
        .map(|a| a.to_lowercase())
        .unwrap_or_default()
}

fn compute_album_key(row: &StoreContentsFileRow) -> String {
    row.primary_media
        .as_ref()
        .and_then(|pm| pm.album.as_ref())
        .map(|a| a.to_lowercase())
        .unwrap_or_default()
}

fn canonical_cursor_policy(policy: &StoreContentsReadPolicy) -> ContentsCursorPolicy {
    match policy {
        StoreContentsReadPolicy::PlayableMediaBrowse => ContentsCursorPolicy::PlayableMediaBrowse,
        StoreContentsReadPolicy::AudioBrowse => ContentsCursorPolicy::AudioBrowse,
        StoreContentsReadPolicy::SourceFileInventory { file_classes } => {
            ContentsCursorPolicy::SourceFileInventory {
                file_classes: CONTENTS_CURSOR_FILE_CLASS_ORDER
                    .iter()
                    .filter(|file_class| file_classes.contains(file_class))
                    .map(|file_class| file_class.as_str().to_string())
                    .collect(),
            }
        }
        StoreContentsReadPolicy::PrimaryMedia { media_kinds } => {
            ContentsCursorPolicy::PrimaryMedia {
                media_kinds: CONTENTS_CURSOR_PRIMARY_MEDIA_KIND_ORDER
                    .iter()
                    .filter(|media_kind| media_kinds.contains(media_kind))
                    .map(|media_kind| media_kind.as_str().to_string())
                    .collect(),
            }
        }
    }
}

fn build_next_cursor(
    rows: &[StoreContentsFileRow],
    scope: &StoreContentsScope,
    policy: &StoreContentsReadPolicy,
    scope_depth: StoreContentsScopeDepth,
) -> Option<String> {
    let last_row = rows.last()?;
    let position = compute_cursor_position(last_row, policy);
    let cursor_scope = match scope {
        StoreContentsScope::Source { source_id } => ContentsCursorScope::Source {
            source_id: *source_id,
        },
        StoreContentsScope::SourceLocation { source_location_id } => {
            ContentsCursorScope::SourceLocation {
                source_location_id: *source_location_id,
            }
        }
        StoreContentsScope::Directory {
            source_id,
            source_directory_id,
        } => ContentsCursorScope::Directory {
            source_id: *source_id,
            source_directory_id: *source_directory_id,
        },
    };
    let scope_depth_str = match scope_depth {
        StoreContentsScopeDepth::Immediate => "immediate".to_string(),
        StoreContentsScopeDepth::Recursive => "recursive".to_string(),
    };
    let cursor = ContentsCursor {
        version: CONTENTS_CURSOR_VERSION,
        scope: cursor_scope,
        policy: canonical_cursor_policy(policy),
        scope_depth: scope_depth_str,
        position,
    };
    encode_cursor(&cursor).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreContentsScope {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreContentsState {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreContentsScopeCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreContentsScopeCoverage {
    pub state: StoreContentsScopeCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreContentsResult {
    pub state: StoreContentsState,
    pub scope: StoreContentsScope,
    pub policy: StoreContentsReadPolicy,
    pub scope_depth: StoreContentsScopeDepth,
    pub rows: Vec<StoreContentsFileRow>,
    pub scope_coverage: StoreContentsScopeCoverage,
    pub has_policy_omitted_rows: bool,
    pub next_cursor: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreContentsFileClass {
    Audio,
    Video,
    Image,
    Unsupported,
}

impl StoreContentsFileClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Image => "image",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorePrimaryMediaKind {
    Audio,
    Video,
}

impl StorePrimaryMediaKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreContentsScopeDepth {
    Immediate,
    Recursive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreContentsReadPolicy {
    PlayableMediaBrowse,
    AudioBrowse,
    SourceFileInventory {
        file_classes: Vec<StoreContentsFileClass>,
    },
    PrimaryMedia {
        media_kinds: Vec<StorePrimaryMediaKind>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreContentsRowOrigin {
    LibraryAsset,
    SourceFile,
    PrimaryMediaCandidate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorePrimaryMediaSummary {
    pub origin: StoreContentsRowOrigin,
    pub primary_media_candidate_id: Option<i64>,
    pub attachment_id: Option<i64>,
    pub content_hash_algorithm: Option<String>,
    pub content_hash_value: Option<String>,
    pub evidence_source_file_id: Option<i64>,
    pub media_kind: Option<String>,
    pub mime_type: Option<String>,
    pub library_asset_id: Option<i64>,
    pub row_version: Option<i64>,
    pub primary_source_file_id: Option<i64>,
    pub availability_state: String,
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
    pub stems_state_summary: Option<String>,
    pub prep_readiness_summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreContentsFileRow {
    pub id: String,
    pub source_id: i64,
    pub source_file_id: i64,
    pub parent_directory_id: Option<i64>,
    pub label: String,
    pub relative_path: String,
    pub file_name: String,
    pub file_class: String,
    pub file_kind: String,
    pub presence: String,
    pub availability_state: Option<String>,
    pub primary_media: Option<StorePrimaryMediaSummary>,
    pub updated_at: i64,
    pub(crate) relative_path_browse_sort_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedContentsScope {
    WholeSource {
        source_id: i64,
    },
    AcceptedSourceLocations {
        source_id: i64,
    },
    Prefix {
        source_id: i64,
        relative_path: String,
    },
    SourceLocationPrefix {
        source_id: i64,
        relative_path: String,
    },
    MissingLocation {
        source_id: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceReadiness {
    source_class: String,
    mount_status: Option<String>,
    access_state: Option<String>,
    access_issue_kind: Option<String>,
    scan_phase: Option<String>,
    scan_issue_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopeCoverageCounts {
    total_directories: i64,
    pending_directories: i64,
    scanning_directories: i64,
    blocked_directories: i64,
    failed_directories: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectoryPresence {
    Present,
    Missing,
    Unknown,
}

const RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL: &str = "char(48)";
const PRIMARY_MEDIA_CONTENTS_ORDER_SQL: &str = "CASE availability_state
        WHEN 'available' THEN 0
        WHEN 'degraded' THEN 1
        WHEN 'unavailable' THEN 2
        ELSE 3
    END ASC,
    lower(COALESCE(title, relative_path, '')) ASC,
    lower(COALESCE(artist, '')) ASC,
    lower(COALESCE(album, '')) ASC,
    lower(COALESCE(relative_path, '')) ASC,
    source_file_id ASC";

const SOURCE_FILE_CONTENTS_ORDER_SQL: &str = "sf.relative_path_browse_sort_key ASC,
    sf.relative_path ASC,
    sf.source_file_id ASC";

pub(crate) fn read_contents(
    connection: &Connection,
    scope: StoreContentsScope,
    policy: StoreContentsReadPolicy,
    scope_depth: StoreContentsScopeDepth,
    limit: usize,
    cursor: Option<&str>,
) -> LibrarySqliteResult<StoreContentsResult> {
    let policy = canonicalize_policy(policy)?;

    let cursor_position = if let Some(cursor_str) = cursor {
        let decoded = match decode_cursor(cursor_str) {
            Ok(cursor) => cursor,
            Err(_) => {
                return Ok(StoreContentsResult {
                    state: StoreContentsState::CursorInvalid,
                    scope,
                    policy,
                    scope_depth,
                    rows: Vec::new(),
                    scope_coverage: StoreContentsScopeCoverage {
                        state: StoreContentsScopeCoverageState::Failed,
                        subtree_coverage_complete: false,
                        empty_result_authoritative: false,
                        detail: Some("The contents cursor could not be decoded.".to_string()),
                    },
                    has_policy_omitted_rows: false,
                    next_cursor: None,
                    detail: Some("The contents cursor could not be decoded.".to_string()),
                });
            }
        };
        if !validate_cursor_identity(&decoded, &scope, &policy, scope_depth) {
            return Ok(StoreContentsResult {
                state: StoreContentsState::CursorInvalid,
                scope,
                policy,
                scope_depth,
                rows: Vec::new(),
                scope_coverage: StoreContentsScopeCoverage {
                    state: StoreContentsScopeCoverageState::Failed,
                    subtree_coverage_complete: false,
                    empty_result_authoritative: false,
                    detail: Some(
                        "The contents cursor does not match the current request.".to_string(),
                    ),
                },
                has_policy_omitted_rows: false,
                next_cursor: None,
                detail: Some("The contents cursor does not match the current request.".to_string()),
            });
        }
        let position_matches_policy = matches!(
            (&decoded.position, &policy),
            (
                ContentsCursorPosition::SourceFile { .. },
                StoreContentsReadPolicy::PlayableMediaBrowse
            ) | (
                ContentsCursorPosition::SourceFile { .. },
                StoreContentsReadPolicy::SourceFileInventory { .. }
            ) | (
                ContentsCursorPosition::SourceFile { .. },
                StoreContentsReadPolicy::AudioBrowse
            ) | (
                ContentsCursorPosition::PrimaryMedia { .. },
                StoreContentsReadPolicy::PrimaryMedia { .. },
            )
        );
        if !position_matches_policy {
            return Ok(StoreContentsResult {
                state: StoreContentsState::CursorInvalid,
                scope,
                policy,
                scope_depth,
                rows: Vec::new(),
                scope_coverage: StoreContentsScopeCoverage {
                    state: StoreContentsScopeCoverageState::Failed,
                    subtree_coverage_complete: false,
                    empty_result_authoritative: false,
                    detail: Some("The contents cursor does not match the policy.".to_string()),
                },
                has_policy_omitted_rows: false,
                next_cursor: None,
                detail: Some("The contents cursor does not match the policy.".to_string()),
            });
        }
        Some(decoded.position)
    } else {
        None
    };

    let Some((source_readiness, resolved_scope)) = resolve_scope(connection, &scope)? else {
        return Ok(StoreContentsResult {
            state: StoreContentsState::LocationMissing,
            scope,
            policy,
            scope_depth,
            rows: Vec::new(),
            scope_coverage: StoreContentsScopeCoverage {
                state: StoreContentsScopeCoverageState::LocationMissing,
                subtree_coverage_complete: false,
                empty_result_authoritative: false,
                detail: Some("The library contents target is not available.".to_string()),
            },
            has_policy_omitted_rows: false,
            next_cursor: None,
            detail: Some("The library contents target is not available.".to_string()),
        });
    };

    if let Some((state, coverage_state, detail)) = source_unavailable_state(&source_readiness) {
        let rows = read_rows(
            connection,
            &resolved_scope,
            &policy,
            scope_depth,
            limit,
            cursor_position.as_ref(),
        )?;
        let next_cursor = if rows.len() > limit {
            build_next_cursor(&rows[..limit], &scope, &policy, scope_depth)
        } else {
            None
        };
        let rows = if rows.len() > limit {
            rows[..limit].to_vec()
        } else {
            rows
        };
        return Ok(StoreContentsResult {
            state,
            scope,
            policy,
            scope_depth,
            rows,
            scope_coverage: StoreContentsScopeCoverage {
                state: coverage_state,
                subtree_coverage_complete: false,
                empty_result_authoritative: false,
                detail: Some(detail.to_string()),
            },
            has_policy_omitted_rows: false,
            next_cursor,
            detail: Some(detail.to_string()),
        });
    }

    let scope_coverage = read_scope_coverage(connection, &source_readiness, &resolved_scope)?;
    let rows = read_rows(
        connection,
        &resolved_scope,
        &policy,
        scope_depth,
        limit,
        cursor_position.as_ref(),
    )?;
    let next_cursor = if rows.len() > limit {
        build_next_cursor(&rows[..limit], &scope, &policy, scope_depth)
    } else {
        None
    };
    let rows = if rows.len() > limit {
        rows[..limit].to_vec()
    } else {
        rows
    };
    let has_policy_omitted_rows = if matches!(
        scope_coverage.state,
        StoreContentsScopeCoverageState::Blocked
            | StoreContentsScopeCoverageState::Failed
            | StoreContentsScopeCoverageState::SourceUnavailable
            | StoreContentsScopeCoverageState::LocationMissing
    ) {
        false
    } else {
        read_has_policy_omitted_rows(connection, &resolved_scope, &policy, scope_depth)?
    };
    let state = contents_state(&scope_coverage, rows.is_empty());
    let detail = contents_detail(state, scope_coverage.state);

    let empty_result_authoritative = state == StoreContentsState::Empty
        && scope_coverage.subtree_coverage_complete
        && !has_policy_omitted_rows;

    Ok(StoreContentsResult {
        state,
        scope,
        policy,
        scope_depth,
        rows,
        scope_coverage: StoreContentsScopeCoverage {
            empty_result_authoritative,
            ..scope_coverage
        },
        has_policy_omitted_rows,
        next_cursor,
        detail: detail.map(str::to_string),
    })
}

pub(crate) fn canonicalize_policy(
    policy: StoreContentsReadPolicy,
) -> LibrarySqliteResult<StoreContentsReadPolicy> {
    match policy {
        StoreContentsReadPolicy::PlayableMediaBrowse => {
            Ok(StoreContentsReadPolicy::PlayableMediaBrowse)
        }
        StoreContentsReadPolicy::AudioBrowse => Ok(StoreContentsReadPolicy::AudioBrowse),
        StoreContentsReadPolicy::SourceFileInventory { file_classes } => {
            let file_classes = canonicalize_file_classes(file_classes);
            if file_classes.is_empty() {
                return Err(LibrarySqliteError::MalformedSchemaState(
                    "contents sourceFileInventory fileClasses must not be empty".to_string(),
                ));
            }
            Ok(StoreContentsReadPolicy::SourceFileInventory { file_classes })
        }
        StoreContentsReadPolicy::PrimaryMedia { media_kinds } => {
            let media_kinds = canonicalize_primary_media_kinds(media_kinds);
            if media_kinds.is_empty() {
                return Err(LibrarySqliteError::MalformedSchemaState(
                    "contents primaryMedia mediaKinds must not be empty".to_string(),
                ));
            }
            Ok(StoreContentsReadPolicy::PrimaryMedia { media_kinds })
        }
    }
}

fn canonicalize_file_classes(
    file_classes: Vec<StoreContentsFileClass>,
) -> Vec<StoreContentsFileClass> {
    CONTENTS_CURSOR_FILE_CLASS_ORDER
        .into_iter()
        .filter(|file_class| file_classes.contains(file_class))
        .collect()
}

fn canonicalize_primary_media_kinds(
    media_kinds: Vec<StorePrimaryMediaKind>,
) -> Vec<StorePrimaryMediaKind> {
    CONTENTS_CURSOR_PRIMARY_MEDIA_KIND_ORDER
        .into_iter()
        .filter(|media_kind| media_kinds.contains(media_kind))
        .collect()
}

fn file_classes_predicate_sql(
    file_class_column_sql: &str,
    file_kind_column_sql: &str,
    file_classes: &[StoreContentsFileClass],
) -> String {
    let predicates = file_classes
        .iter()
        .map(|file_class| match file_class {
            StoreContentsFileClass::Unsupported => format!(
                "({file_class_column_sql} = 'unsupported' AND {file_kind_column_sql} = 'cue_sheet')"
            ),
            _ => format!("{file_class_column_sql} = '{}'", file_class.as_str()),
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    format!("({predicates})")
}

fn policy_file_classes(policy: &StoreContentsReadPolicy) -> Vec<StoreContentsFileClass> {
    match policy {
        StoreContentsReadPolicy::PlayableMediaBrowse => PLAYABLE_MEDIA_BROWSE_FILE_CLASSES.to_vec(),
        StoreContentsReadPolicy::AudioBrowse => AUDIO_BROWSE_FILE_CLASSES.to_vec(),
        StoreContentsReadPolicy::SourceFileInventory { file_classes } => file_classes.clone(),
        StoreContentsReadPolicy::PrimaryMedia { media_kinds } => media_kinds
            .iter()
            .map(|media_kind| match media_kind {
                StorePrimaryMediaKind::Audio => StoreContentsFileClass::Audio,
                StorePrimaryMediaKind::Video => StoreContentsFileClass::Video,
            })
            .collect(),
    }
}

fn policy_omitted_file_classes(
    policy: &StoreContentsReadPolicy,
) -> Option<Vec<StoreContentsFileClass>> {
    if matches!(policy, StoreContentsReadPolicy::PrimaryMedia { .. }) {
        return None;
    }

    let included_file_classes = policy_file_classes(policy);
    let browse_relevant_file_classes = match policy {
        StoreContentsReadPolicy::SourceFileInventory { .. } => {
            CONTENTS_CURSOR_FILE_CLASS_ORDER.as_slice()
        }
        StoreContentsReadPolicy::PlayableMediaBrowse | StoreContentsReadPolicy::AudioBrowse => {
            BROWSE_RELEVANT_FILE_CLASSES.as_slice()
        }
        StoreContentsReadPolicy::PrimaryMedia { .. } => unreachable!(),
    };

    Some(
        browse_relevant_file_classes
            .iter()
            .copied()
            .filter(|file_class| !included_file_classes.contains(file_class))
            .collect(),
    )
}

fn resolve_scope(
    connection: &Connection,
    scope: &StoreContentsScope,
) -> LibrarySqliteResult<Option<(SourceReadiness, ResolvedContentsScope)>> {
    match scope {
        StoreContentsScope::Source { source_id } => {
            let Some(source_readiness) = load_source_readiness(connection, *source_id)? else {
                return Ok(None);
            };

            let accepted_count = accepted_source_location_count(connection, *source_id)?;
            let resolved_scope = if accepted_count == 0 {
                ResolvedContentsScope::WholeSource {
                    source_id: *source_id,
                }
            } else {
                ResolvedContentsScope::AcceptedSourceLocations {
                    source_id: *source_id,
                }
            };

            Ok(Some((source_readiness, resolved_scope)))
        }
        StoreContentsScope::SourceLocation { source_location_id } => {
            let Some((source_id, relative_path)) =
                load_accepted_source_location(connection, *source_location_id)?
            else {
                return Ok(None);
            };
            let Some(source_readiness) = load_source_readiness(connection, source_id)? else {
                return Ok(None);
            };

            Ok(Some((
                source_readiness,
                ResolvedContentsScope::SourceLocationPrefix {
                    source_id,
                    relative_path,
                },
            )))
        }
        StoreContentsScope::Directory {
            source_id,
            source_directory_id,
        } => {
            let Some(source_readiness) = load_source_readiness(connection, *source_id)? else {
                return Ok(None);
            };
            let Some(relative_path) =
                load_present_directory_path(connection, *source_id, *source_directory_id)?
            else {
                return Ok(Some((
                    source_readiness,
                    ResolvedContentsScope::MissingLocation {
                        source_id: *source_id,
                    },
                )));
            };

            Ok(Some((
                source_readiness,
                ResolvedContentsScope::Prefix {
                    source_id: *source_id,
                    relative_path,
                },
            )))
        }
    }
}

fn load_source_readiness(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<SourceReadiness>> {
    connection
        .query_row(
            "SELECT s.source_class,
                    ss.mount_status,
                    ss.access_state,
                    ss.access_issue_kind,
                    sss.scan_phase,
                    sss.scan_issue_kind
             FROM sources s
             LEFT JOIN source_state ss
               ON ss.source_id = s.source_id
             LEFT JOIN source_scan_state sss
               ON sss.source_id = s.source_id
             WHERE s.source_id = ?1",
            [source_id],
            |row| {
                Ok(SourceReadiness {
                    source_class: row.get(0)?,
                    mount_status: row.get(1)?,
                    access_state: row.get(2)?,
                    access_issue_kind: row.get(3)?,
                    scan_phase: row.get(4)?,
                    scan_issue_kind: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_accepted_source_location(
    connection: &Connection,
    source_location_id: i64,
) -> LibrarySqliteResult<Option<(i64, String)>> {
    connection
        .query_row(
            "SELECT source_id, relative_path
             FROM source_locations
             WHERE source_location_id = ?1
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1",
            [source_location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn accepted_source_location_count(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<i64> {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM source_locations
             WHERE source_id = ?1
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1",
            [source_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn load_accepted_source_location_paths(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Vec<String>> {
    let mut stmt = connection.prepare(
        "SELECT relative_path
         FROM source_locations
         WHERE source_id = ?1
           AND authority = 'user'
           AND location_kind = 'registered_subpath'
           AND is_user_visible = 1",
    )?;
    let paths: Vec<String> = stmt
        .query_map([source_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(paths)
}

fn load_directory_presence(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<DirectoryPresence> {
    let presence_state = connection
        .query_row(
            "SELECT presence_state
             FROM source_directories
             WHERE source_id = ?1
               AND relative_path COLLATE BINARY = ?2 COLLATE BINARY",
            params![source_id, relative_path],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(presence_state
        .map(|state| match state.as_str() {
            "present" => DirectoryPresence::Present,
            "missing" => DirectoryPresence::Missing,
            _ => DirectoryPresence::Unknown,
        })
        .unwrap_or(DirectoryPresence::Unknown))
}

fn load_present_directory_path(
    connection: &Connection,
    source_id: i64,
    source_directory_id: i64,
) -> LibrarySqliteResult<Option<String>> {
    connection
        .query_row(
            "SELECT relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND source_directory_id = ?2
               AND presence_state = 'present'",
            params![source_id, source_directory_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn source_unavailable_state(
    source: &SourceReadiness,
) -> Option<(
    StoreContentsState,
    StoreContentsScopeCoverageState,
    &'static str,
)> {
    if source.source_class != "internal"
        && !matches!(source.mount_status.as_deref(), Some("mounted"))
    {
        return Some((
            StoreContentsState::SourceUnavailable,
            StoreContentsScopeCoverageState::SourceUnavailable,
            "The selected source is unavailable.",
        ));
    }

    match source.access_state.as_deref() {
        Some("missing") => {
            return Some((
                StoreContentsState::LocationMissing,
                StoreContentsScopeCoverageState::LocationMissing,
                "The selected source root is missing.",
            ));
        }
        Some("blocked") => {
            let coverage_state = if source.access_issue_kind.as_deref() == Some("unavailable_mount")
            {
                StoreContentsScopeCoverageState::SourceUnavailable
            } else {
                StoreContentsScopeCoverageState::Blocked
            };
            let state = match coverage_state {
                StoreContentsScopeCoverageState::SourceUnavailable => {
                    StoreContentsState::SourceUnavailable
                }
                _ => StoreContentsState::Blocked,
            };
            return Some((
                state,
                coverage_state,
                "The selected source root is blocked.",
            ));
        }
        _ => {}
    }

    match source.scan_phase.as_deref() {
        Some("blocked") => Some((
            if source.scan_issue_kind.as_deref() == Some("unavailable_mount") {
                StoreContentsState::SourceUnavailable
            } else {
                StoreContentsState::Blocked
            },
            if source.scan_issue_kind.as_deref() == Some("unavailable_mount") {
                StoreContentsScopeCoverageState::SourceUnavailable
            } else {
                StoreContentsScopeCoverageState::Blocked
            },
            "The selected source scan is blocked.",
        )),
        Some("failed") => Some((
            StoreContentsState::Failed,
            StoreContentsScopeCoverageState::Failed,
            "The selected source scan failed.",
        )),
        Some("partial") => None,
        _ => None,
    }
}

fn read_scope_coverage(
    connection: &Connection,
    source: &SourceReadiness,
    scope: &ResolvedContentsScope,
) -> LibrarySqliteResult<StoreContentsScopeCoverage> {
    let counts = match scope {
        ResolvedContentsScope::WholeSource { source_id } => {
            read_whole_source_scope_coverage_counts(connection, *source_id)?
        }
        ResolvedContentsScope::AcceptedSourceLocations { source_id } => {
            let paths = load_accepted_source_location_paths(connection, *source_id)?;
            if paths.is_empty() {
                return Ok(scope_coverage(
                    StoreContentsScopeCoverageState::LocationMissing,
                    false,
                    "No accepted source locations found.",
                ));
            }
            let mut coverages = Vec::with_capacity(paths.len());
            for path in &paths {
                coverages.push(classify_source_location_coverage(
                    connection,
                    *source_id,
                    path,
                    source.scan_phase.as_deref(),
                )?);
            }
            let aggregate = aggregate_source_location_coverages(&coverages);
            match aggregate {
                AcceptedSourceLocationCoverage::AllPresent => {}
                AcceptedSourceLocationCoverage::AllMissing => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::LocationMissing,
                        false,
                        "All accepted source locations are missing.",
                    ));
                }
                AcceptedSourceLocationCoverage::MixedMissing => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Incomplete,
                        false,
                        "One or more accepted source locations are missing. Results may be incomplete.",
                    ));
                }
                AcceptedSourceLocationCoverage::Blocked => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Blocked,
                        false,
                        "One or more accepted source locations is under a blocked subtree.",
                    ));
                }
                AcceptedSourceLocationCoverage::Failed => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Failed,
                        false,
                        "One or more accepted source locations is under a failed subtree.",
                    ));
                }
                AcceptedSourceLocationCoverage::Scanning => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Scanning,
                        false,
                        "One or more accepted source locations is being scanned.",
                    ));
                }
                AcceptedSourceLocationCoverage::Pending => {
                    return Ok(scope_coverage(
                        pending_or_scanning_coverage_state(source),
                        false,
                        "One or more accepted source locations has incomplete coverage.",
                    ));
                }
            }
            read_accepted_source_locations_scope_coverage_counts(connection, *source_id)?
        }
        ResolvedContentsScope::MissingLocation { .. } => {
            return Ok(scope_coverage(
                StoreContentsScopeCoverageState::LocationMissing,
                false,
                "The selected folder is missing.",
            ));
        }
        ResolvedContentsScope::SourceLocationPrefix {
            source_id,
            relative_path,
        } => {
            let location_coverage = classify_source_location_coverage(
                connection,
                *source_id,
                relative_path,
                source.scan_phase.as_deref(),
            )?;
            match location_coverage {
                SourceLocationCoverage::Present => {}
                SourceLocationCoverage::Missing => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::LocationMissing,
                        false,
                        "The selected source location is missing.",
                    ));
                }
                SourceLocationCoverage::Blocked => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Blocked,
                        false,
                        "The selected source location is under a blocked subtree.",
                    ));
                }
                SourceLocationCoverage::Failed => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Failed,
                        false,
                        "The selected source location is under a failed subtree.",
                    ));
                }
                SourceLocationCoverage::Scanning => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Scanning,
                        false,
                        "The selected source location is being scanned.",
                    ));
                }
                SourceLocationCoverage::Pending => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::Pending,
                        false,
                        "The selected source location has pending coverage.",
                    ));
                }
                SourceLocationCoverage::Unknown => {
                    return Ok(scope_coverage(
                        pending_or_scanning_coverage_state(source),
                        false,
                        "The selected source location has unproven coverage.",
                    ));
                }
            }
            read_prefix_scope_coverage_counts(connection, *source_id, relative_path)?
        }
        ResolvedContentsScope::Prefix {
            source_id,
            relative_path,
        } => {
            match load_directory_presence(connection, *source_id, relative_path)? {
                DirectoryPresence::Present => {}
                DirectoryPresence::Missing => {
                    return Ok(scope_coverage(
                        StoreContentsScopeCoverageState::LocationMissing,
                        false,
                        "The selected folder is missing.",
                    ));
                }
                DirectoryPresence::Unknown => {
                    return Ok(scope_coverage(
                        pending_or_scanning_coverage_state(source),
                        false,
                        "The selected folder has not been proven present or missing yet.",
                    ));
                }
            }
            read_prefix_scope_coverage_counts(connection, *source_id, relative_path)?
        }
    };

    Ok(scope_coverage_from_counts(source, scope, counts))
}

fn read_whole_source_scope_coverage_counts(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<ScopeCoverageCounts> {
    connection
        .query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'failed' THEN 1 ELSE 0 END)
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'",
            [source_id],
            scope_coverage_counts_from_row,
        )
        .map_err(Into::into)
}

fn read_accepted_source_locations_scope_coverage_counts(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<ScopeCoverageCounts> {
    connection
        .query_row(
            &format!(
                "WITH accepted_locations(relative_path) AS (
                     SELECT relative_path
                     FROM source_locations
                     WHERE source_id = ?1
                       AND authority = 'user'
                       AND location_kind = 'registered_subpath'
                       AND is_user_visible = 1
                 )
                 SELECT COUNT(*),
                        SUM(CASE WHEN sd.dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'failed' THEN 1 ELSE 0 END)
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.presence_state = 'present'
                   AND EXISTS (
                       SELECT 1
                       FROM accepted_locations al
                       WHERE {}
                   )",
                relative_path_scope_predicate("sd", "al.relative_path")
            ),
            [source_id],
            scope_coverage_counts_from_row,
        )
        .map_err(Into::into)
}

fn read_prefix_scope_coverage_counts(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<ScopeCoverageCounts> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        SUM(CASE WHEN dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'failed' THEN 1 ELSE 0 END)
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.presence_state = 'present'
                   AND {}",
                relative_path_scope_predicate("sd", "?2")
            ),
            params![source_id, relative_path],
            scope_coverage_counts_from_row,
        )
        .map_err(Into::into)
}

fn scope_coverage_counts_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ScopeCoverageCounts> {
    Ok(ScopeCoverageCounts {
        total_directories: row.get(0)?,
        pending_directories: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
        scanning_directories: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
        blocked_directories: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
        failed_directories: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
    })
}

fn scope_coverage_from_counts(
    source: &SourceReadiness,
    scope: &ResolvedContentsScope,
    counts: ScopeCoverageCounts,
) -> StoreContentsScopeCoverage {
    if counts.total_directories == 0 {
        if matches!(scope, ResolvedContentsScope::WholeSource { .. })
            && source.scan_phase.as_deref() == Some("complete")
        {
            return scope_coverage(
                StoreContentsScopeCoverageState::Complete,
                true,
                "The selected source has complete scan coverage.",
            );
        }

        if matches!(scope, ResolvedContentsScope::WholeSource { .. }) {
            return scope_coverage(
                StoreContentsScopeCoverageState::Pending,
                false,
                "The selected source has no completed directory coverage yet.",
            );
        }

        if matches!(
            scope,
            ResolvedContentsScope::AcceptedSourceLocations { .. }
                | ResolvedContentsScope::SourceLocationPrefix { .. }
        ) {
            return scope_coverage(
                StoreContentsScopeCoverageState::Complete,
                true,
                "The contents scope has complete scan coverage.",
            );
        }

        return scope_coverage(
            StoreContentsScopeCoverageState::LocationMissing,
            false,
            "The contents scope has no present directory coverage.",
        );
    }

    if counts.blocked_directories > 0 {
        return scope_coverage(
            StoreContentsScopeCoverageState::Blocked,
            false,
            "Part of the contents scope is blocked.",
        );
    }

    if counts.failed_directories > 0 {
        return scope_coverage(
            StoreContentsScopeCoverageState::Failed,
            false,
            "Part of the contents scope failed to scan.",
        );
    }

    if counts.scanning_directories > 0 {
        return scope_coverage(
            StoreContentsScopeCoverageState::Scanning,
            false,
            "The contents scope is still scanning.",
        );
    }

    if counts.pending_directories > 0 {
        return scope_coverage(
            StoreContentsScopeCoverageState::Pending,
            false,
            "The contents scope has pending scan coverage.",
        );
    }

    if matches!(scope, ResolvedContentsScope::WholeSource { .. })
        && !matches!(
            source.scan_phase.as_deref(),
            Some("complete") | Some("partial")
        )
    {
        return scope_coverage(
            StoreContentsScopeCoverageState::Pending,
            false,
            "The selected source has not completed a full subtree scan.",
        );
    }

    if matches!(scope, ResolvedContentsScope::WholeSource { .. })
        && source.scan_phase.as_deref() == Some("partial")
    {
        return scope_coverage(
            StoreContentsScopeCoverageState::Pending,
            false,
            "The selected source scan has incomplete descendant coverage.",
        );
    }

    scope_coverage(
        StoreContentsScopeCoverageState::Complete,
        true,
        "The contents scope has complete scan coverage.",
    )
}

fn pending_or_scanning_coverage_state(source: &SourceReadiness) -> StoreContentsScopeCoverageState {
    if source.scan_phase.as_deref() == Some("scanning") {
        StoreContentsScopeCoverageState::Scanning
    } else {
        StoreContentsScopeCoverageState::Pending
    }
}

fn scope_coverage(
    state: StoreContentsScopeCoverageState,
    subtree_coverage_complete: bool,
    detail: &str,
) -> StoreContentsScopeCoverage {
    StoreContentsScopeCoverage {
        state,
        subtree_coverage_complete,
        empty_result_authoritative: false,
        detail: Some(detail.to_string()),
    }
}

fn contents_state(coverage: &StoreContentsScopeCoverage, rows_empty: bool) -> StoreContentsState {
    match coverage.state {
        StoreContentsScopeCoverageState::Complete => {
            if rows_empty {
                StoreContentsState::Empty
            } else {
                StoreContentsState::Ready
            }
        }
        StoreContentsScopeCoverageState::Pending
        | StoreContentsScopeCoverageState::Scanning
        | StoreContentsScopeCoverageState::Incomplete => StoreContentsState::Partial,
        StoreContentsScopeCoverageState::Blocked => StoreContentsState::Blocked,
        StoreContentsScopeCoverageState::Failed => StoreContentsState::Failed,
        StoreContentsScopeCoverageState::SourceUnavailable => StoreContentsState::SourceUnavailable,
        StoreContentsScopeCoverageState::LocationMissing => StoreContentsState::LocationMissing,
    }
}

fn contents_detail(
    state: StoreContentsState,
    coverage_state: StoreContentsScopeCoverageState,
) -> Option<&'static str> {
    match state {
        StoreContentsState::Ready => None,
        StoreContentsState::Empty => None,
        StoreContentsState::Partial => Some(match coverage_state {
            StoreContentsScopeCoverageState::Scanning => {
                "Still indexing. Results may be incomplete."
            }
            StoreContentsScopeCoverageState::Incomplete => {
                "One or more accepted source locations are missing. Results may be incomplete."
            }
            _ => "Indexing is incomplete. Results may be incomplete.",
        }),
        StoreContentsState::SourceUnavailable => Some("The selected source is unavailable."),
        StoreContentsState::LocationMissing => Some("The selected folder is missing."),
        StoreContentsState::Blocked => Some("The contents scope is blocked."),
        StoreContentsState::Failed => Some("The contents scope failed to scan."),
        StoreContentsState::PolicyConflict => Some("The contents policy cannot be read."),
        StoreContentsState::CursorInvalid => Some("The contents cursor is invalid."),
    }
}

fn read_has_policy_omitted_rows(
    connection: &Connection,
    scope: &ResolvedContentsScope,
    policy: &StoreContentsReadPolicy,
    scope_depth: StoreContentsScopeDepth,
) -> LibrarySqliteResult<bool> {
    let Some(omitted_file_classes) = policy_omitted_file_classes(policy) else {
        // primaryMedia omission metadata is not surfaced until its own row universe is queryable.
        return Ok(false);
    };
    if omitted_file_classes.is_empty() {
        return Ok(false);
    }

    let omitted_file_class_predicate =
        file_classes_predicate_sql("sf.file_class", "sf.file_kind", &omitted_file_classes);
    match scope {
        ResolvedContentsScope::WholeSource { source_id } => source_file_exists(
            connection,
            None,
            &whole_source_file_predicate(scope_depth),
            &omitted_file_class_predicate,
            *source_id,
            None,
        ),
        ResolvedContentsScope::AcceptedSourceLocations { source_id } => source_file_exists(
            connection,
            Some(accepted_locations_cte()),
            &accepted_locations_source_file_predicate(scope_depth),
            &omitted_file_class_predicate,
            *source_id,
            None,
        ),
        ResolvedContentsScope::Prefix {
            source_id,
            relative_path,
        }
        | ResolvedContentsScope::SourceLocationPrefix {
            source_id,
            relative_path,
        } => source_file_exists(
            connection,
            None,
            &scoped_path_predicate(scope_depth),
            &omitted_file_class_predicate,
            *source_id,
            Some(relative_path),
        ),
        ResolvedContentsScope::MissingLocation { .. } => Ok(false),
    }
}

fn source_file_exists(
    connection: &Connection,
    prefix_cte: Option<&str>,
    source_predicate: &str,
    file_class_predicate: &str,
    source_id: i64,
    relative_path: Option<&str>,
) -> LibrarySqliteResult<bool> {
    let cte_prefix = prefix_cte
        .map(|cte| format!("WITH {cte} "))
        .unwrap_or_default();
    let sql = format!(
        "{cte_prefix}SELECT EXISTS (
             SELECT 1
             FROM source_files sf
             WHERE {file_class_predicate}
               AND {source_predicate}
             LIMIT 1
         )"
    );
    let exists = if let Some(relative_path) = relative_path {
        connection.query_row(&sql, params![source_id, relative_path], |row| {
            row.get::<_, i64>(0)
        })?
    } else {
        connection.query_row(&sql, [source_id], |row| row.get::<_, i64>(0))?
    };
    Ok(exists != 0)
}

fn read_rows(
    connection: &Connection,
    scope: &ResolvedContentsScope,
    policy: &StoreContentsReadPolicy,
    scope_depth: StoreContentsScopeDepth,
    limit: usize,
    cursor_position: Option<&ContentsCursorPosition>,
) -> LibrarySqliteResult<Vec<StoreContentsFileRow>> {
    let limit_plus_one = i64::try_from(limit.saturating_add(1)).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "contents limit {limit} exceeds i64 range"
        ))
    })?;
    let file_classes = policy_file_classes(policy);
    let file_class_predicate =
        file_classes_predicate_sql("sf.file_class", "sf.file_kind", &file_classes);
    let primary_media_rows = matches!(policy, StoreContentsReadPolicy::PrimaryMedia { .. });

    match scope {
        ResolvedContentsScope::WholeSource { source_id } => {
            let source_predicate = whole_source_file_predicate(scope_depth);
            read_rows_with_source_predicate(
                connection,
                SourcePredicateReadInput {
                    source_predicate: &source_predicate,
                    file_class_predicate: &file_class_predicate,
                    primary_media_rows,
                    source_id: *source_id,
                    relative_path: None,
                    limit_plus_one,
                    cursor_position,
                },
            )
        }
        ResolvedContentsScope::AcceptedSourceLocations { source_id } => {
            let predicate = accepted_locations_source_file_predicate(scope_depth);
            read_rows_for_accepted_locations(
                connection,
                &predicate,
                &file_class_predicate,
                primary_media_rows,
                *source_id,
                limit_plus_one,
                cursor_position,
            )
        }
        ResolvedContentsScope::Prefix {
            source_id,
            relative_path,
        } => {
            let predicate = scoped_path_predicate(scope_depth);
            read_rows_with_source_predicate(
                connection,
                SourcePredicateReadInput {
                    source_predicate: &predicate,
                    file_class_predicate: &file_class_predicate,
                    primary_media_rows,
                    source_id: *source_id,
                    relative_path: Some(relative_path),
                    limit_plus_one,
                    cursor_position,
                },
            )
        }
        ResolvedContentsScope::SourceLocationPrefix {
            source_id,
            relative_path,
        } => {
            let predicate = scoped_path_predicate(scope_depth);
            read_rows_with_source_predicate(
                connection,
                SourcePredicateReadInput {
                    source_predicate: &predicate,
                    file_class_predicate: &file_class_predicate,
                    primary_media_rows,
                    source_id: *source_id,
                    relative_path: Some(relative_path),
                    limit_plus_one,
                    cursor_position,
                },
            )
        }
        ResolvedContentsScope::MissingLocation { .. } => Ok(Vec::new()),
    }
}

fn whole_source_file_predicate(scope_depth: StoreContentsScopeDepth) -> String {
    match scope_depth {
        StoreContentsScopeDepth::Recursive => "sf.source_id = ?1".to_string(),
        StoreContentsScopeDepth::Immediate => {
            "sf.source_id = ?1 AND sf.parent_source_directory_id IS NULL".to_string()
        }
    }
}

fn accepted_locations_source_file_predicate(scope_depth: StoreContentsScopeDepth) -> String {
    match scope_depth {
        StoreContentsScopeDepth::Recursive => format!(
            "sf.source_id = ?1
             AND EXISTS (
                 SELECT 1
                 FROM accepted_locations al
                 WHERE {}
             )",
            relative_path_scope_predicate("sf", "al.relative_path")
        ),
        StoreContentsScopeDepth::Immediate => "sf.source_id = ?1
             AND EXISTS (
                 SELECT 1
                 FROM accepted_locations al
                 JOIN source_directories sd
                   ON sd.source_id = sf.source_id
                  AND sd.relative_path COLLATE BINARY = al.relative_path COLLATE BINARY
                   AND sd.presence_state = 'present'
                 WHERE sf.parent_source_directory_id = sd.source_directory_id
             )"
        .to_string(),
    }
}

fn read_rows_for_accepted_locations(
    connection: &Connection,
    source_predicate: &str,
    file_class_predicate: &str,
    primary_media_rows: bool,
    source_id: i64,
    limit_plus_one: i64,
    cursor_position: Option<&ContentsCursorPosition>,
) -> LibrarySqliteResult<Vec<StoreContentsFileRow>> {
    let scope_param_count: usize = 1;
    let cursor_param_count: usize = match cursor_position {
        Some(ContentsCursorPosition::SourceFile { .. }) => 3,
        Some(ContentsCursorPosition::PrimaryMedia { .. }) => 6,
        None => 0,
    };
    let cursor_start: Option<usize> = if cursor_param_count > 0 {
        Some(scope_param_count + 1)
    } else {
        None
    };
    let limit_param: usize = scope_param_count + cursor_param_count + 1;

    let sql = contents_rows_sql(
        Some(accepted_locations_cte()),
        source_predicate,
        file_class_predicate,
        primary_media_rows,
        cursor_start,
        limit_param,
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = if let Some(cursor) = cursor_position {
        let mut params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Integer(source_id)];
        push_cursor_params(cursor, &mut params);
        params.push(rusqlite::types::Value::Integer(limit_plus_one));
        statement
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                contents_row_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    } else {
        statement
            .query_map(params![source_id, limit_plus_one], contents_row_from_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    };
    Ok(rows)
}

struct SourcePredicateReadInput<'a> {
    source_predicate: &'a str,
    file_class_predicate: &'a str,
    primary_media_rows: bool,
    source_id: i64,
    relative_path: Option<&'a str>,
    limit_plus_one: i64,
    cursor_position: Option<&'a ContentsCursorPosition>,
}

fn read_rows_with_source_predicate(
    connection: &Connection,
    input: SourcePredicateReadInput<'_>,
) -> LibrarySqliteResult<Vec<StoreContentsFileRow>> {
    let scope_param_count: usize = if input.relative_path.is_some() { 2 } else { 1 };
    let cursor_param_count: usize = match input.cursor_position {
        Some(ContentsCursorPosition::SourceFile { .. }) => 3,
        Some(ContentsCursorPosition::PrimaryMedia { .. }) => 6,
        None => 0,
    };
    let cursor_start: Option<usize> = if cursor_param_count > 0 {
        Some(scope_param_count + 1)
    } else {
        None
    };
    let limit_param: usize = scope_param_count + cursor_param_count + 1;

    let sql = contents_rows_sql(
        None,
        input.source_predicate,
        input.file_class_predicate,
        input.primary_media_rows,
        cursor_start,
        limit_param,
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = if let Some(cursor) = input.cursor_position {
        let mut params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Integer(input.source_id)];
        if let Some(relative_path) = input.relative_path {
            params.push(rusqlite::types::Value::Text(relative_path.to_string()));
        }
        push_cursor_params(cursor, &mut params);
        params.push(rusqlite::types::Value::Integer(input.limit_plus_one));
        statement
            .query_map(
                rusqlite::params_from_iter(params.iter()),
                contents_row_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    } else if let Some(relative_path) = input.relative_path {
        statement
            .query_map(
                params![input.source_id, relative_path, input.limit_plus_one],
                contents_row_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    } else {
        statement
            .query_map(
                params![input.source_id, input.limit_plus_one],
                contents_row_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    };
    Ok(rows)
}

fn scoped_path_predicate(scope_depth: StoreContentsScopeDepth) -> String {
    match scope_depth {
        StoreContentsScopeDepth::Recursive => format!(
            "sf.source_id = ?1
             AND {}",
            source_file_descendant_predicate("sf", "?2")
        ),
        StoreContentsScopeDepth::Immediate => "sf.source_id = ?1
             AND sf.parent_source_directory_id = (
                 SELECT sd.source_directory_id
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.relative_path COLLATE BINARY = ?2 COLLATE BINARY
                   AND sd.presence_state = 'present'
             )"
        .to_string(),
    }
}

fn contents_rows_sql(
    prefix_cte: Option<&str>,
    source_predicate: &str,
    file_class_predicate: &str,
    primary_media_rows: bool,
    cursor_start: Option<usize>,
    limit_param: usize,
) -> String {
    if primary_media_rows {
        primary_media_rows_sql(
            prefix_cte,
            source_predicate,
            file_class_predicate,
            cursor_start,
            limit_param,
        )
    } else {
        source_file_rows_sql(
            prefix_cte,
            source_predicate,
            file_class_predicate,
            cursor_start,
            limit_param,
        )
    }
}

fn source_file_rows_sql(
    prefix_cte: Option<&str>,
    source_predicate: &str,
    file_class_predicate: &str,
    cursor_start: Option<usize>,
    limit_param: usize,
) -> String {
    let cte_prefix = prefix_cte
        .map(|cte| format!("WITH {cte} "))
        .unwrap_or_default();
    let cursor_clause = match cursor_start {
        Some(base) => {
            let bsk_idx = base;
            let rp_idx = base + 1;
            let sid_idx = base + 2;
            format!(
                "\n  AND (\n      sf.relative_path_browse_sort_key > ?{bsk_idx}\n      OR (sf.relative_path_browse_sort_key = ?{bsk_idx} AND sf.relative_path > ?{rp_idx})\n      OR (sf.relative_path_browse_sort_key = ?{bsk_idx} AND sf.relative_path = ?{rp_idx} AND sf.source_file_id > ?{sid_idx})\n  )"
            )
        }
        None => String::new(),
    };
    format!(
        "{cte_prefix} \
SELECT sf.source_file_id, \
       sf.source_id, \
       sf.parent_source_directory_id, \
       sf.relative_path, \
       sf.name AS file_name, \
       sf.file_class, \
       sf.file_kind, \
       sf.presence_state, \
       NULL AS library_asset_id, \
       NULL AS row_version, \
       NULL AS primary_source_file_id, \
       NULL AS availability_state, \
       NULL AS title, \
       NULL AS artist, \
       NULL AS album, \
       NULL AS duration_ms, \
       NULL AS musical_key, \
       NULL AS tempo_bpm, \
       NULL AS waveform_quality_current, \
       NULL AS waveform_quality_target, \
       NULL AS stems_state_summary, \
       NULL AS prep_readiness_summary, \
       sf.updated_at, \
       NULL AS primary_media_candidate_id, \
       NULL AS attachment_id, \
       NULL AS content_hash_algorithm, \
       NULL AS content_hash_value, \
       NULL AS evidence_source_file_id, \
       NULL AS candidate_media_kind, \
       NULL AS mime_type, \
       NULL AS sample_rate_hz, \
       NULL AS channels, \
       NULL AS bit_depth, \
       NULL AS codec, \
       sf.relative_path_browse_sort_key \
   FROM source_files sf \
WHERE {file_class_predicate} \
  AND {source_predicate}{cursor_clause} \
 ORDER BY {SOURCE_FILE_CONTENTS_ORDER_SQL} \
 LIMIT ?{limit_param}"
    )
}

fn primary_media_rows_sql(
    prefix_cte: Option<&str>,
    source_predicate: &str,
    file_class_predicate: &str,
    cursor_start: Option<usize>,
    limit_param: usize,
) -> String {
    let cte_prefix = prefix_cte
        .map(|cte| format!("WITH {cte},"))
        .unwrap_or_else(|| "WITH".to_string());

    let cursor_clause = match cursor_start {
        Some(base) => {
            let ap_idx = base;
            let tk_idx = base + 1;
            let ak_idx = base + 2;
            let bk_idx = base + 3;
            let rpk_idx = base + 4;
            let sid_idx = base + 5;
            format!(
                "\n WHERE (\
                    \n     CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END > ?{ap_idx}\
                    \n     OR (CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END = ?{ap_idx}\
                    \n         AND lower(COALESCE(title, relative_path, '')) > ?{tk_idx})\
                    \n     OR (CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END = ?{ap_idx}\
                    \n         AND lower(COALESCE(title, relative_path, '')) = ?{tk_idx}\
                    \n         AND lower(COALESCE(artist, '')) > ?{ak_idx})\
                    \n     OR (CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END = ?{ap_idx}\
                    \n         AND lower(COALESCE(title, relative_path, '')) = ?{tk_idx}\
                    \n         AND lower(COALESCE(artist, '')) = ?{ak_idx}\
                    \n         AND lower(COALESCE(album, '')) > ?{bk_idx})\
                    \n     OR (CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END = ?{ap_idx}\
                    \n         AND lower(COALESCE(title, relative_path, '')) = ?{tk_idx}\
                    \n         AND lower(COALESCE(artist, '')) = ?{ak_idx}\
                    \n         AND lower(COALESCE(album, '')) = ?{bk_idx}\
                    \n         AND lower(COALESCE(relative_path, '')) > ?{rpk_idx})\
                    \n     OR (CASE availability_state\
                    \n         WHEN 'available' THEN 0\
                    \n         WHEN 'degraded' THEN 1\
                    \n         WHEN 'unavailable' THEN 2\
                    \n         ELSE 3\
                    \n     END = ?{ap_idx}\
                    \n         AND lower(COALESCE(title, relative_path, '')) = ?{tk_idx}\
                    \n         AND lower(COALESCE(artist, '')) = ?{ak_idx}\
                    \n         AND lower(COALESCE(album, '')) = ?{bk_idx}\
                    \n         AND lower(COALESCE(relative_path, '')) = ?{rpk_idx}\
                    \n         AND source_file_id > ?{sid_idx})\
                    \n )"
            )
        }
        None => String::new(),
    };

    format!(
        "{cte_prefix} \
         scope_files AS ( \
              SELECT sf.source_file_id, \
                     sf.source_id, \
                      sf.parent_source_directory_id, \
                      sf.relative_path, \
                      sf.name, \
                      sf.size_bytes, \
                      sf.mtime_ns, \
                      sf.file_class, \
                      sf.file_kind, \
                     sf.presence_state, \
                      sf.updated_at \
               FROM source_files sf \
               WHERE sf.presence_state = 'present' \
                 AND {file_class_predicate} \
                 AND {source_predicate} \
           ), \
           candidate_scope AS ( \
               SELECT pmc.primary_media_candidate_id, \
                      pmc.attachment_id, \
                      attachment.content_hash_algorithm, \
                      attachment.content_hash_value, \
                      pmc.evidence_source_file_id, \
                      pmc.media_kind AS candidate_media_kind, \
                      pmc.mime_type, \
                      pmc.duration_ms, \
                      pmc.sample_rate_hz, \
                      pmc.channels, \
                      pmc.bit_depth, \
                      pmc.codec, \
                      MAX( \
                          pmc.updated_at, \
                          attachment.updated_at, \
                          link.updated_at, \
                          facts.updated_at, \
                          sf.updated_at \
                      ) AS updated_at, \
                      sf.source_file_id, \
                      sf.source_id, \
                      sf.parent_source_directory_id, \
                      sf.relative_path, \
                      sf.name, \
                      sf.size_bytes, \
                      sf.mtime_ns, \
                      sf.file_class, \
                      sf.file_kind, \
                      sf.presence_state, \
                      ROW_NUMBER() OVER ( \
                          PARTITION BY pmc.primary_media_candidate_id \
                          ORDER BY CASE \
                                       WHEN sf.source_file_id = pmc.evidence_source_file_id THEN 0 \
                                       ELSE 1 \
                                   END ASC, \
                                   lower(sf.relative_path) ASC, \
                                   sf.source_file_id ASC \
                      ) AS scoped_occurrence_rank \
               FROM scope_files sf \
               JOIN source_file_attachment_links link \
                 ON link.source_file_id = sf.source_file_id \
                AND link.source_id = sf.source_id \
               JOIN content_attachments attachment \
                 ON attachment.attachment_id = link.attachment_id \
               JOIN primary_media_candidates pmc \
                 ON pmc.attachment_id = attachment.attachment_id \
               JOIN SourceFacts facts \
                 ON facts.source_file_id = sf.source_file_id \
              WHERE sf.source_id = facts.basis_source_id \
                AND sf.relative_path = facts.basis_relative_path \
                AND sf.size_bytes IS facts.basis_size_bytes \
                AND sf.mtime_ns IS facts.basis_mtime_ns \
                AND sf.presence_state = facts.basis_presence_state \
                AND facts.content_hash_algorithm = attachment.content_hash_algorithm \
                AND facts.content_hash_value = attachment.content_hash_value \
                AND facts.media_kind = 'audio' \
                AND ( \
                    facts.mime_type IS NOT NULL \
                    OR facts.duration_ms IS NOT NULL \
                    OR facts.sample_rate_hz IS NOT NULL \
                    OR facts.channels IS NOT NULL \
                    OR facts.bit_depth IS NOT NULL \
                    OR facts.codec IS NOT NULL \
                ) \
           ), \
           promoted AS ( \
               SELECT source_file_id, \
                      source_id, \
                      parent_source_directory_id, \
                     relative_path, \
                      name AS file_name, \
                      file_class, \
                      file_kind, \
                      presence_state, \
                      NULL AS library_asset_id, \
                      row_version, \
                      primary_source_file_id, \
                      availability_state, \
                      title, \
                     artist, \
                     album, \
                     duration_ms, \
                     musical_key, \
                     tempo_bpm, \
                     waveform_quality_current, \
                     waveform_quality_target, \
                      stems_state_summary, \
                      prep_readiness_summary, \
                      updated_at, \
                      primary_media_candidate_id, \
                      attachment_id, \
                      content_hash_algorithm, \
                      content_hash_value, \
                      evidence_source_file_id, \
                      candidate_media_kind, \
                      mime_type, \
                      sample_rate_hz, \
                      channels, \
                      bit_depth, \
                      codec \
               FROM ( \
                   SELECT source_file_id, \
                          source_id, \
                          parent_source_directory_id, \
                          relative_path, \
                          name, \
                          file_class, \
                          file_kind, \
                          presence_state, \
                          NULL AS row_version, \
                          evidence_source_file_id AS primary_source_file_id, \
                          'available' AS availability_state, \
                          NULL AS title, \
                          NULL AS artist, \
                          NULL AS album, \
                          duration_ms, \
                          NULL AS musical_key, \
                          NULL AS tempo_bpm, \
                          NULL AS waveform_quality_current, \
                          NULL AS waveform_quality_target, \
                          NULL AS stems_state_summary, \
                          'not_required' AS prep_readiness_summary, \
                          updated_at, \
                          primary_media_candidate_id, \
                          attachment_id, \
                          content_hash_algorithm, \
                          content_hash_value, \
                          evidence_source_file_id, \
                          candidate_media_kind, \
                          mime_type, \
                          sample_rate_hz, \
                          channels, \
                          bit_depth, \
                          codec \
                   FROM candidate_scope \
                   WHERE scoped_occurrence_rank = 1 \
               ) \
           ) \
           SELECT source_file_id, \
                  source_id, \
                 parent_source_directory_id, \
                 relative_path, \
                 file_name, \
                 file_class, \
                 file_kind, \
                 presence_state, \
                 library_asset_id, \
                 row_version, \
                 primary_source_file_id, \
                 availability_state, \
                 title, \
                 artist, \
                 album, \
                 duration_ms, \
                 musical_key, \
                 tempo_bpm, \
                 waveform_quality_current, \
                  waveform_quality_target, \
                  stems_state_summary, \
                  prep_readiness_summary, \
                  updated_at, \
                  primary_media_candidate_id, \
                  attachment_id, \
                  content_hash_algorithm, \
                  content_hash_value, \
                  evidence_source_file_id, \
                  candidate_media_kind, \
                  mime_type, \
                  sample_rate_hz, \
                  channels, \
                  bit_depth, \
                  codec, \
                  NULL AS relative_path_browse_sort_key \
           FROM promoted{cursor_clause} \
           ORDER BY {PRIMARY_MEDIA_CONTENTS_ORDER_SQL} \
           LIMIT ?{limit_param}"
    )
}

fn push_cursor_params(cursor: &ContentsCursorPosition, params: &mut Vec<rusqlite::types::Value>) {
    match cursor {
        ContentsCursorPosition::SourceFile {
            relative_path_browse_sort_key,
            relative_path,
            source_file_id,
        } => {
            params.push(rusqlite::types::Value::Text(
                relative_path_browse_sort_key.clone(),
            ));
            params.push(rusqlite::types::Value::Text(relative_path.clone()));
            params.push(rusqlite::types::Value::Integer(*source_file_id));
        }
        ContentsCursorPosition::PrimaryMedia {
            availability_priority,
            title_key,
            artist_key,
            album_key,
            relative_path_key,
            source_file_id,
        } => {
            params.push(rusqlite::types::Value::Integer(*availability_priority));
            params.push(rusqlite::types::Value::Text(title_key.clone()));
            params.push(rusqlite::types::Value::Text(artist_key.clone()));
            params.push(rusqlite::types::Value::Text(album_key.clone()));
            params.push(rusqlite::types::Value::Text(relative_path_key.clone()));
            params.push(rusqlite::types::Value::Integer(*source_file_id));
        }
    }
}

fn accepted_locations_cte() -> &'static str {
    "accepted_locations(relative_path) AS (
         SELECT relative_path
         FROM source_locations
         WHERE source_id = ?1
           AND authority = 'user'
           AND location_kind = 'registered_subpath'
           AND is_user_visible = 1
     )"
}

fn relative_path_scope_predicate(alias: &str, prefix_sql: &str) -> String {
    format!(
        "{alias}.relative_path COLLATE BINARY = {prefix_sql} COLLATE BINARY
         OR {}",
        source_file_descendant_predicate(alias, prefix_sql)
    )
}

fn source_file_descendant_predicate(alias: &str, prefix_sql: &str) -> String {
    format!(
        "({alias}.relative_path COLLATE BINARY >= {prefix_sql} || '/'
          AND {alias}.relative_path COLLATE BINARY < {prefix_sql} || {RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL})"
    )
}

fn contents_row_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoreContentsFileRow> {
    let source_file_id: i64 = row.get(0)?;
    let source_id: i64 = row.get(1)?;
    let parent_directory_id: Option<i64> = row.get(2)?;
    let relative_path: String = row.get(3)?;
    let file_name: String = row.get(4)?;
    let file_class: String = row.get(5)?;
    let file_kind: String = row.get(6)?;
    let presence: String = row.get(7)?;
    let library_asset_id: Option<i64> = row.get(8)?;
    let row_version: Option<i64> = row.get(9)?;
    let primary_source_file_id: Option<i64> = row.get(10)?;
    let availability_state: Option<String> = row.get(11)?;
    let title: Option<String> = row.get(12)?;
    let artist: Option<String> = row.get(13)?;
    let album: Option<String> = row.get(14)?;
    let duration_ms: Option<i64> = row.get(15)?;
    let musical_key: Option<String> = row.get(16)?;
    let tempo_bpm: Option<f64> = row.get(17)?;
    let waveform_quality_current: Option<i64> = row.get(18)?;
    let waveform_quality_target: Option<i64> = row.get(19)?;
    let stems_state_summary: Option<String> = row.get(20)?;
    let prep_readiness_summary: Option<String> = row.get(21)?;
    let updated_at: i64 = row.get(22)?;
    let primary_media_candidate_id: Option<i64> = row.get(23)?;
    let attachment_id: Option<i64> = row.get(24)?;
    let content_hash_algorithm: Option<String> = row.get(25)?;
    let content_hash_value: Option<String> = row.get(26)?;
    let evidence_source_file_id: Option<i64> = row.get(27)?;
    let candidate_media_kind: Option<String> = row.get(28)?;
    let mime_type: Option<String> = row.get(29)?;
    let sample_rate_hz: Option<i64> = row.get(30)?;
    let channels: Option<i64> = row.get(31)?;
    let bit_depth: Option<i64> = row.get(32)?;
    let codec: Option<String> = row.get(33)?;
    let relative_path_browse_sort_key: Option<String> = row.get(34)?;

    let primary_media = availability_state.as_ref().map(|availability_state| {
        let origin = if primary_media_candidate_id.is_some() {
            StoreContentsRowOrigin::PrimaryMediaCandidate
        } else if library_asset_id.is_some() {
            StoreContentsRowOrigin::LibraryAsset
        } else {
            StoreContentsRowOrigin::SourceFile
        };

        StorePrimaryMediaSummary {
            origin,
            primary_media_candidate_id,
            attachment_id,
            content_hash_algorithm: content_hash_algorithm.clone(),
            content_hash_value: content_hash_value.clone(),
            evidence_source_file_id,
            media_kind: candidate_media_kind.clone(),
            mime_type: mime_type.clone(),
            library_asset_id,
            row_version,
            primary_source_file_id,
            availability_state: availability_state.clone(),
            title: title.clone(),
            artist: artist.clone(),
            album: album.clone(),
            duration_ms,
            sample_rate_hz,
            channels,
            bit_depth,
            codec: codec.clone(),
            musical_key: musical_key.clone(),
            tempo_bpm,
            waveform_quality_current,
            waveform_quality_target,
            stems_state_summary: stems_state_summary.clone(),
            prep_readiness_summary: prep_readiness_summary
                .clone()
                .unwrap_or_else(|| "underprepared".to_string()),
        }
    });

    let label = contents_label(title.as_deref(), &file_name, &relative_path);

    Ok(StoreContentsFileRow {
        id: primary_media
            .as_ref()
            .and_then(|summary| {
                summary
                    .primary_media_candidate_id
                    .map(|id| format!("primary-media:{id}"))
                    .or_else(|| {
                        summary
                            .library_asset_id
                            .map(|id| format!("library-asset:{id}"))
                    })
            })
            .unwrap_or_else(|| format!("source-file:{source_file_id}")),
        source_id,
        source_file_id,
        parent_directory_id,
        label,
        relative_path,
        file_name,
        file_class,
        file_kind,
        presence,
        availability_state,
        primary_media,
        updated_at,
        relative_path_browse_sort_key,
    })
}

fn contents_label(title: Option<&str>, file_name: &str, relative_path: &str) -> String {
    title
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if !file_name.trim().is_empty() {
                Some(file_name)
            } else {
                None
            }
        })
        .or_else(|| {
            if !relative_path.trim().is_empty() {
                Some(relative_path)
            } else {
                None
            }
        })
        .unwrap_or("Untitled")
        .to_string()
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{
        StoreContentsFileClass, StoreContentsReadPolicy, StoreContentsRowOrigin,
        StoreContentsScope, StoreContentsScopeCoverageState, StoreContentsScopeDepth,
        StoreContentsState, StorePrimaryMediaKind, canonical_cursor_policy, read_contents,
    };
    use crate::schema::install_baseline_schema_for_test;

    fn open_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open test database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        connection
    }

    fn primary_media_policy() -> StoreContentsReadPolicy {
        StoreContentsReadPolicy::PrimaryMedia {
            media_kinds: vec![StorePrimaryMediaKind::Audio, StorePrimaryMediaKind::Video],
        }
    }

    fn primary_media_file_classes() -> Vec<StoreContentsFileClass> {
        vec![StoreContentsFileClass::Audio, StoreContentsFileClass::Video]
    }

    fn source_file_policy(file_classes: Vec<StoreContentsFileClass>) -> StoreContentsReadPolicy {
        StoreContentsReadPolicy::SourceFileInventory { file_classes }
    }

    fn playable_media_browse_policy() -> StoreContentsReadPolicy {
        StoreContentsReadPolicy::PlayableMediaBrowse
    }

    fn audio_browse_policy() -> StoreContentsReadPolicy {
        StoreContentsReadPolicy::AudioBrowse
    }

    fn default_source_file_policy() -> StoreContentsReadPolicy {
        source_file_policy(vec![
            StoreContentsFileClass::Audio,
            StoreContentsFileClass::Video,
            StoreContentsFileClass::Image,
            StoreContentsFileClass::Unsupported,
        ])
    }

    #[test]
    fn canonical_cursor_file_class_identity_uses_explicit_stable_order() {
        for file_classes in [
            vec![
                StoreContentsFileClass::Unsupported,
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Image,
            ],
            vec![
                StoreContentsFileClass::Image,
                StoreContentsFileClass::Unsupported,
                StoreContentsFileClass::Audio,
            ],
            vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Image,
                StoreContentsFileClass::Unsupported,
            ],
        ] {
            assert_eq!(
                canonical_cursor_policy(&source_file_policy(file_classes)),
                super::ContentsCursorPolicy::SourceFileInventory {
                    file_classes: vec![
                        "audio".to_string(),
                        "image".to_string(),
                        "unsupported".to_string(),
                    ],
                }
            );
        }

        assert_eq!(
            canonical_cursor_policy(&default_source_file_policy()),
            super::ContentsCursorPolicy::SourceFileInventory {
                file_classes: vec![
                    "audio".to_string(),
                    "video".to_string(),
                    "image".to_string(),
                    "unsupported".to_string(),
                ],
            }
        );
        assert_eq!(
            canonical_cursor_policy(&playable_media_browse_policy()),
            super::ContentsCursorPolicy::PlayableMediaBrowse
        );
    }

    fn primary_media(row: &super::StoreContentsFileRow) -> &super::StorePrimaryMediaSummary {
        row.primary_media
            .as_ref()
            .expect("primary-media profile rows carry a summary")
    }

    fn insert_source(connection: &Connection, source_id: i64) {
        connection
            .execute(
                "INSERT INTO sources (
                     source_id,
                     source_class,
                     authority,
                     identity_key,
                     display_name,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'external_mounted', 'device', ?2, ?3, 1, 1)",
                params![
                    source_id,
                    format!("source:{source_id}"),
                    format!("Source {source_id}")
                ],
            )
            .expect("insert source");
        connection
            .execute(
                "INSERT INTO source_state (
                     source_id,
                     mount_status,
                     mount_epoch,
                     access_state,
                     access_checked_at,
                     mount_root,
                     effective_path,
                     updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 'root', 1)",
                [source_id],
            )
            .expect("insert source state");
        connection
            .execute(
                "INSERT INTO source_scan_state (
                     source_id,
                     scan_phase,
                     last_scan_started_at,
                     last_scan_finished_at,
                     last_successful_scan_at,
                     updated_at
                 )
                 VALUES (?1, 'complete', 1, 2, 2, 2)",
                [source_id],
            )
            .expect("insert source scan state");
    }

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        source_id: i64,
        relative_path: &str,
        dir_scan_state: &str,
    ) {
        let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
        let name_browse_sort_key = crate::browse_sort_key::compute_name_browse_sort_key(name);
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_browse_sort_key,
                     relative_path,
                     presence_state,
                     dir_scan_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, 'present', ?6, 1, 1, 1)",
                params![
                    source_directory_id,
                    source_id,
                    name,
                    name_browse_sort_key,
                    relative_path,
                    dir_scan_state,
                ],
            )
            .expect("insert source directory");
    }

    fn set_source_access(
        connection: &Connection,
        source_id: i64,
        access_state: &str,
        issue_kind: Option<&str>,
    ) {
        connection
            .execute(
                "UPDATE source_state
                 SET access_state = ?2,
                     access_issue_kind = ?3,
                     access_checked_at = 10,
                     updated_at = 10
                 WHERE source_id = ?1",
                params![source_id, access_state, issue_kind],
            )
            .expect("update source access");
    }

    fn set_source_scan_phase(
        connection: &Connection,
        source_id: i64,
        scan_phase: &str,
        issue_kind: Option<&str>,
    ) {
        connection
            .execute(
                "UPDATE source_scan_state
                 SET scan_phase = ?2,
                     scan_issue_kind = ?3,
                     updated_at = 10
                 WHERE source_id = ?1",
                params![source_id, scan_phase, issue_kind],
            )
            .expect("update source scan phase");
    }

    fn set_directory_scan_issue(
        connection: &Connection,
        source_directory_id: i64,
        dir_scan_state: &str,
        issue_kind: &str,
    ) {
        connection
            .execute(
                "UPDATE source_directories
                 SET dir_scan_state = ?2,
                     dir_scan_issue_kind = ?3,
                     dir_scan_updated_at = 10,
                     updated_at = 10
                 WHERE source_directory_id = ?1",
                params![source_directory_id, dir_scan_state, issue_kind],
            )
            .expect("update directory scan issue");
    }

    fn insert_location(
        connection: &Connection,
        source_location_id: i64,
        source_id: i64,
        relative_path: &str,
        authority: &str,
        location_kind: &str,
    ) {
        connection
            .execute(
                "INSERT INTO source_locations (
                     source_location_id,
                     source_id,
                     authority,
                     location_kind,
                     relative_path,
                     display_name,
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1, 1, 1)",
                params![
                    source_location_id,
                    source_id,
                    authority,
                    location_kind,
                    relative_path
                ],
            )
            .expect("insert source location");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_promoted_media_file(
        connection: &Connection,
        _legacy_asset_id: i64,
        source_file_id: i64,
        source_id: i64,
        parent_directory_id: i64,
        relative_path: &str,
        file_class: &str,
        _title: &str,
    ) {
        let file_kind = crate::browse_media::file_kind_str_from_path(relative_path);
        insert_scanned_file(
            connection,
            source_file_id,
            source_id,
            parent_directory_id,
            relative_path,
            file_class,
        );

        if file_class != "audio" || file_kind != "audio" {
            return;
        }

        connection
            .execute(
                "INSERT OR IGNORE INTO WorkItems (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     basis_fingerprint,
                     state,
                     priority_class,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'source_file', 'fixture', 'inspect_source', 'fixture', 'completed', 'interactive', 1, 1)",
                [],
            )
            .expect("insert work item");
        connection
            .execute(
                "INSERT OR IGNORE INTO WorkRuns (
                     work_run_id,
                     work_item_id,
                     adapter_key,
                     adapter_version,
                     started_at,
                     outcome
                 )
                 VALUES (1, 1, 'test.contents', '1', 1, 'ok')",
                [],
            )
            .expect("insert work run");
        let artifact_id = 10_000 + source_file_id;
        let basis_fingerprint = format!("basis:{source_file_id}");
        connection
            .execute(
                "INSERT INTO Artifacts (
                     artifact_id,
                     work_run_id,
                     subject_kind,
                     subject_id,
                     artifact_kind,
                     artifact_role,
                     adapter_key,
                     adapter_version,
                     basis_fingerprint,
                     media_type,
                     storage_kind,
                     payload_hash,
                     created_at
                 )
                 VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test.contents', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
                params![
                    artifact_id,
                    source_file_id,
                    basis_fingerprint,
                    format!("payload:{source_file_id}")
                ],
            )
            .expect("insert artifact");
        let (basis_relative_path, basis_size_bytes, basis_mtime_ns, basis_presence_state): (
            String,
            Option<i64>,
            Option<i64>,
            String,
        ) = connection
            .query_row(
                "SELECT relative_path, size_bytes, mtime_ns, presence_state
                 FROM source_files
                 WHERE source_file_id = ?1",
                [source_file_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read source file basis");
        let hash_value = format!("hash:{source_file_id}");
        connection
            .execute(
                "INSERT INTO SourceFacts (
                     source_file_id,
                     fact_kind,
                     basis_fingerprint,
                     basis_source_id,
                     basis_relative_path,
                     basis_size_bytes,
                     basis_mtime_ns,
                     basis_presence_state,
                     observed_at_ms,
                     content_hash_algorithm,
                     content_hash_value,
                     media_kind,
                     mime_type,
                     duration_ms,
                     sample_rate_hz,
                     channels,
                     bit_depth,
                     codec,
                     updated_at,
                     accepted_artifact_id
                 )
                 VALUES (?1, 'source_inspection', ?2, ?3, ?4, ?5, ?6, ?7, 1, 'blake3', ?8, 'audio', 'audio/wav', 120000, 44100, 2, 16, 'pcm', 1, ?9)",
                params![
                    source_file_id,
                    basis_fingerprint,
                    source_id,
                    basis_relative_path,
                    basis_size_bytes,
                    basis_mtime_ns,
                    basis_presence_state,
                    hash_value,
                    artifact_id
                ],
            )
            .expect("insert source facts");
        connection
            .execute(
                "INSERT OR IGNORE INTO content_attachments (
                     content_hash_algorithm,
                     content_hash_value,
                     first_observed_at,
                     updated_at
                 )
                 VALUES ('blake3', ?1, 1, 1)",
                [format!("hash:{source_file_id}")],
            )
            .expect("insert content attachment");
        let attachment_id: i64 = connection
            .query_row(
                "SELECT attachment_id
                 FROM content_attachments
                 WHERE content_hash_algorithm = 'blake3'
                   AND content_hash_value = ?1",
                [format!("hash:{source_file_id}")],
                |row| row.get(0),
            )
            .expect("read attachment id");
        connection
            .execute(
                "INSERT INTO source_file_attachment_links (
                     attachment_id,
                     source_file_id,
                     source_id,
                     file_kind,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, 1, 1)",
                params![attachment_id, source_file_id, source_id, file_kind],
            )
            .expect("insert attachment link");
        connection
            .execute(
                "INSERT INTO primary_media_candidates (
                     attachment_id,
                     evidence_source_file_id,
                     evidence_basis_fingerprint,
                     media_kind,
                     mime_type,
                     duration_ms,
                     sample_rate_hz,
                     channels,
                     bit_depth,
                     codec,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, 'audio', 'audio/wav', 120000, 44100, 2, 16, 'pcm', 1, 1)",
                params![attachment_id, source_file_id, basis_fingerprint],
            )
            .expect("insert primary media candidate");
    }

    fn seed_assets(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO WorkItems (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     basis_fingerprint,
                     state,
                     priority_class,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'projection_domain', 'library_browser', 'rebuild_projection', 'basis:test', 'completed', 'interactive', 1, 1)",
                [],
            )
            .expect("insert work item");
        connection
            .execute(
                "INSERT INTO WorkRuns (
                     work_run_id,
                     work_item_id,
                     adapter_key,
                     adapter_version,
                     started_at,
                     outcome
                 )
                 VALUES (1, 1, 'test', '1', 1, 'ok')",
                [],
            )
            .expect("insert work run");
    }

    #[test]
    fn source_scope_uses_only_accepted_locations_when_present() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Other", "complete");
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        insert_location(&connection, 101, 1, "Other", "device", "observed_path");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/track.wav",
            "audio",
            "Track",
        );
        insert_promoted_media_file(
            &connection,
            2,
            1001,
            1,
            11,
            "Other/clip.mp4",
            "video",
            "Clip",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| {
                    let summary = primary_media(row);
                    (summary.origin, summary.attachment_id.is_some())
                })
                .collect::<Vec<_>>(),
            vec![(StoreContentsRowOrigin::PrimaryMediaCandidate, true)]
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
    }

    #[test]
    fn source_location_scope_requires_accepted_user_visible_location() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Observed", "complete");
        insert_location(&connection, 100, 1, "Observed", "device", "observed_path");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Observed/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::LocationMissing);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn directory_scope_reads_descendants_media_and_excludes_prefix_siblings() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Nested", "complete");
        insert_directory(&connection, 12, 1, "Music2", "complete");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Nested/alpha.wav",
            "audio",
            "Alpha",
        );
        insert_promoted_media_file(
            &connection,
            2,
            1001,
            1,
            12,
            "Music2/beta.wav",
            "audio",
            "Beta",
        );
        insert_promoted_media_file(
            &connection,
            3,
            1002,
            1,
            10,
            "Music/readme.txt",
            "unsupported",
            "Cover",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| {
                    let summary = primary_media(row);
                    (
                        summary.origin,
                        summary.attachment_id.is_some(),
                        row.file_class.as_str(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![(StoreContentsRowOrigin::PrimaryMediaCandidate, true, "audio")]
        );
    }

    #[test]
    fn incomplete_empty_scope_is_partial_not_authoritative_empty() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Pending", "pending");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Partial);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Pending
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn source_access_block_is_blocked_not_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        set_source_access(&connection, 1, "blocked", Some("permission_denied"));

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Blocked);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked
        );
        assert!(result.rows.is_empty());
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn missing_source_root_is_location_missing_not_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        set_source_access(&connection, 1, "missing", Some("missing"));

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::LocationMissing);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing
        );
        assert!(result.rows.is_empty());
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn scanning_empty_scope_is_scanning_not_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        set_source_scan_phase(&connection, 1, "scanning", None);
        insert_directory(&connection, 10, 1, "Scanning", "scanning");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Partial);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Scanning
        );
        assert!(result.rows.is_empty());
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn blocked_descendant_prevents_descendant_scope_from_being_complete() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 11, "blocked", "permission_denied");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Blocked);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked
        );
        assert!(!result.scope_coverage.subtree_coverage_complete);
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    fn insert_scanned_file(
        connection: &Connection,
        source_file_id: i64,
        source_id: i64,
        parent_directory_id: i64,
        relative_path: &str,
        file_class: &str,
    ) {
        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
        let file_kind = crate::browse_media::file_kind_str_from_path(relative_path);
        let name_browse_sort_key = crate::browse_sort_key::compute_name_browse_sort_key(file_name);
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_browse_sort_key,
                     relative_path_browse_sort_key,
                     relative_path,
                     file_kind,
                     file_class,
                     presence_state,
                     first_discovered_at,
                     last_observed_at,
                     last_presence_change_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    source_id,
                    parent_directory_id,
                    file_name,
                    name_browse_sort_key,
                    crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path),
                    relative_path,
                    file_kind,
                    file_class,
                ],
            )
            .expect("insert scanned source file");
    }

    fn insert_audio_source_files(
        connection: &Connection,
        parent_directory_id: i64,
        files: &[(i64, &str)],
    ) {
        for (source_file_id, relative_path) in files {
            insert_scanned_file(
                connection,
                *source_file_id,
                1,
                parent_directory_id,
                relative_path,
                "audio",
            );
        }
    }

    fn assert_relative_paths(rows: &[super::StoreContentsFileRow], expected: &[&str]) {
        assert_eq!(
            rows.iter()
                .map(|row| row.relative_path.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    fn assert_audio_browse_matches_source_file_audio(
        connection: &Connection,
        scope: StoreContentsScope,
        scope_depth: StoreContentsScopeDepth,
        limit: usize,
    ) -> (super::StoreContentsResult, super::StoreContentsResult) {
        let source_file = read_contents(
            connection,
            scope.clone(),
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            scope_depth,
            limit,
            None,
        )
        .expect("read source-file audio contents");
        let audio_browse = read_contents(
            connection,
            scope,
            audio_browse_policy(),
            scope_depth,
            limit,
            None,
        )
        .expect("read audio-browse contents");

        assert_eq!(audio_browse.state, source_file.state);
        assert_eq!(audio_browse.scope_coverage, source_file.scope_coverage);
        assert_eq!(audio_browse.rows, source_file.rows);
        assert_eq!(audio_browse.scope_depth, source_file.scope_depth);
        assert_eq!(audio_browse.policy, StoreContentsReadPolicy::AudioBrowse);

        (source_file, audio_browse)
    }

    #[test]
    fn source_scope_returns_empty_primary_media_without_promotion() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Music/clip.mp4", "video");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert!(result.rows.is_empty());
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(result.scope_coverage.empty_result_authoritative);
        assert!(!result.has_policy_omitted_rows);
    }

    #[test]
    fn promoted_candidate_rows_are_returned_when_evidence_is_current() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        let summary = primary_media(&result.rows[0]);
        assert_eq!(
            summary.origin,
            StoreContentsRowOrigin::PrimaryMediaCandidate
        );
        assert!(summary.primary_media_candidate_id.is_some());
        assert!(summary.attachment_id.is_some());
        assert!(summary.library_asset_id.is_none());
        assert_eq!(summary.content_hash_value.as_deref(), Some("hash:1000"));
        assert_eq!(summary.media_kind.as_deref(), Some("audio"));
        assert!(
            result.rows[0].id.starts_with("primary-media:"),
            "promoted row id must start with 'primary-media:', got: {}",
            result.rows[0].id
        );
    }

    #[test]
    fn complete_scope_with_media_files_without_promotion_returns_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/track.wav", "audio");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert!(result.rows.is_empty());
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn directory_with_image_files_returns_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Covers", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Covers/front.jpg", "image");
        insert_scanned_file(&connection, 1001, 1, 10, "Covers/back.png", "image");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert!(result.rows.is_empty());
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn directory_with_only_unsupported_files_returns_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Documents", "complete");
        insert_scanned_file(
            &connection,
            1000,
            1,
            10,
            "Documents/readme.txt",
            "unsupported",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert!(result.rows.is_empty());
        assert!(result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn directory_with_none_files_returns_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Data", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Data/notes", "none");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert!(result.rows.is_empty());
        assert!(result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn directory_with_image_files_during_pending_scan_returns_partial_without_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Covers", "pending");
        insert_scanned_file(&connection, 1000, 1, 10, "Covers/front.jpg", "image");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Partial);
        assert!(result.rows.is_empty());
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Pending
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn primary_media_profile_omits_unpromoted_source_file_rows() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/promoted.wav",
            "audio",
            "Promoted",
        );
        insert_scanned_file(&connection, 1001, 1, 10, "Music/scanned.wav", "audio");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].source_file_id, 1000);
        assert_eq!(
            primary_media(&result.rows[0]).origin,
            StoreContentsRowOrigin::PrimaryMediaCandidate
        );
    }

    #[test]
    fn source_file_profile_returns_media_relevant_inventory_without_primary_media() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/cover.jpg", "image");
        insert_scanned_file(&connection, 1003, 1, 10, "Media/album.cue", "unsupported");
        insert_scanned_file(&connection, 1004, 1, 10, "Media/readme.txt", "unsupported");
        insert_scanned_file(&connection, 1005, 1, 10, "Media/archive.zip", "unsupported");
        insert_scanned_file(&connection, 1006, 1, 10, "Media/blob.bin", "unsupported");
        insert_scanned_file(&connection, 1007, 1, 10, "Media/mystery", "none");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
                StoreContentsFileClass::Image,
                StoreContentsFileClass::Unsupported,
            ]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read source-file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.policy, default_source_file_policy());
        let rows = result
            .rows
            .iter()
            .map(|row| {
                (
                    row.relative_path.as_str(),
                    row.file_class.as_str(),
                    row.file_kind.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![
                ("Media/album.cue", "unsupported", "cue_sheet"),
                ("Media/clip.mp4", "video", "video"),
                ("Media/cover.jpg", "image", "image"),
                ("Media/track.wav", "audio", "audio"),
            ]
        );
        assert!(result.rows.iter().all(|row| row.primary_media.is_none()));
        assert!(
            result
                .rows
                .iter()
                .all(|row| row.availability_state.is_none())
        );
    }

    #[test]
    fn source_file_profile_audio_policy_returns_only_audio_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/cover.jpg", "image");
        insert_scanned_file(&connection, 1003, 1, 10, "Media/album.cue", "unsupported");
        insert_scanned_file(&connection, 1004, 1, 10, "Media/readme.txt", "unsupported");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read audio-only source-file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(
            result.policy,
            source_file_policy(vec![StoreContentsFileClass::Audio])
        );
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].relative_path, "Media/track.wav");
        assert_eq!(result.rows[0].file_class, "audio");
        assert_eq!(result.rows[0].file_kind, "audio");
        assert!(result.rows[0].primary_media.is_none());
    }

    #[test]
    fn audio_browse_profile_reuses_source_file_audio_rows_for_scopes_and_scope_depth() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Sub", "complete");
        insert_directory(&connection, 20, 1, "Images", "complete");
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/01.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Music/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 11, "Music/Sub/02.wav", "audio");
        insert_scanned_file(&connection, 1003, 1, 20, "Images/front.jpg", "image");

        let (_source_file, source_descendants) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_relative_paths(
            &source_descendants.rows,
            &["Music/01.wav", "Music/Sub/02.wav"],
        );

        assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            StoreContentsScopeDepth::Immediate,
            10,
        );

        let (_source_file, directory_descendants) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_relative_paths(
            &directory_descendants.rows,
            &["Music/01.wav", "Music/Sub/02.wav"],
        );

        let (_source_file, directory_immediate) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            StoreContentsScopeDepth::Immediate,
            10,
        );
        assert_relative_paths(&directory_immediate.rows, &["Music/01.wav"]);

        let (_source_file, location_descendants) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_relative_paths(
            &location_descendants.rows,
            &["Music/01.wav", "Music/Sub/02.wav"],
        );

        let (_source_file, location_immediate) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            StoreContentsScopeDepth::Immediate,
            10,
        );
        assert_relative_paths(&location_immediate.rows, &["Music/01.wav"]);

        for row in source_descendants.rows {
            assert!(row.primary_media.is_none());
            assert!(row.availability_state.is_none());
            assert_eq!(row.file_class, "audio");
        }
    }

    #[test]
    fn audio_browse_policy_is_audio_only_without_weakening_source_file_inventory() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");

        let source_file = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
            ]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read source-file contents");
        assert_eq!(source_file.state, StoreContentsState::Ready);
        assert_relative_paths(&source_file.rows, &["Media/clip.mp4", "Media/track.wav"]);

        let audio_browse = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read audio-browse contents");

        assert_eq!(audio_browse.state, StoreContentsState::Ready);
        assert_relative_paths(&audio_browse.rows, &["Media/track.wav"]);
        assert!(audio_browse.has_policy_omitted_rows);
    }

    #[test]
    fn playable_media_browse_includes_audio_and_video_and_omits_browse_relevant_images() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.m4a", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/cover.jpg", "image");
        insert_scanned_file(&connection, 1003, 1, 10, "Media/album.cue", "unsupported");
        insert_scanned_file(&connection, 1004, 1, 10, "Media/readme.txt", "unsupported");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            playable_media_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read playable-media browse contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_relative_paths(&result.rows, &["Media/clip.mp4", "Media/track.m4a"]);
        assert_eq!(result.rows[0].file_class, "video");
        assert_eq!(result.rows[0].file_kind, "video");
        assert_eq!(result.rows[1].file_class, "audio");
        assert_eq!(result.rows[1].file_kind, "audio");
        assert!(result.has_policy_omitted_rows);
    }

    #[test]
    fn playable_media_browse_distinguishes_mp4_only_from_image_only_scope() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Videos", "complete");
        insert_directory(&connection, 20, 1, "Covers", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Videos/clip.mp4", "video");
        insert_scanned_file(&connection, 2000, 1, 20, "Covers/front.jpg", "image");

        let video = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            playable_media_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read video contents");
        assert_eq!(video.state, StoreContentsState::Ready);
        assert_relative_paths(&video.rows, &["Videos/clip.mp4"]);
        assert!(!video.has_policy_omitted_rows);

        let image = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 20,
            },
            playable_media_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read image-only contents");
        assert_eq!(image.state, StoreContentsState::Empty);
        assert!(image.rows.is_empty());
        assert!(image.has_policy_omitted_rows);
        assert!(!image.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn audio_browse_reports_video_omission_but_ignores_cue_and_raw_unsupported_files() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Videos", "complete");
        insert_directory(&connection, 20, 1, "Companions", "complete");
        insert_directory(&connection, 30, 1, "Empty", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Videos/clip.mp4", "video");
        insert_scanned_file(
            &connection,
            2000,
            1,
            20,
            "Companions/album.cue",
            "unsupported",
        );
        insert_scanned_file(
            &connection,
            2001,
            1,
            20,
            "Companions/readme.txt",
            "unsupported",
        );

        let video = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read video scope as audio");
        assert_eq!(video.state, StoreContentsState::Empty);
        assert!(video.rows.is_empty());
        assert!(video.has_policy_omitted_rows);
        assert!(!video.scope_coverage.empty_result_authoritative);

        for source_directory_id in [20, 30] {
            let result = read_contents(
                &connection,
                StoreContentsScope::Directory {
                    source_id: 1,
                    source_directory_id,
                },
                audio_browse_policy(),
                StoreContentsScopeDepth::Recursive,
                10,
                None,
            )
            .expect("read non-audio scope");
            assert_eq!(result.state, StoreContentsState::Empty);
            assert!(!result.has_policy_omitted_rows);
            assert!(result.scope_coverage.empty_result_authoritative);
        }
    }

    #[test]
    fn source_file_inventory_omission_uses_requestable_classes_only() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/album.cue", "unsupported");
        insert_scanned_file(&connection, 1003, 1, 10, "Media/readme.txt", "unsupported");

        let audio_only = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read audio inventory");
        assert_relative_paths(&audio_only.rows, &["Media/track.wav"]);
        assert!(audio_only.has_policy_omitted_rows);

        let all_requestable = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            default_source_file_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read all requestable inventory");
        assert!(!all_requestable.has_policy_omitted_rows);
    }

    #[test]
    fn omission_metadata_respects_scope_depth_and_is_not_page_local() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_directory(&connection, 11, 1, "Media/Sub", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/a.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/b.wav", "audio");
        insert_scanned_file(&connection, 1002, 1, 11, "Media/Sub/clip.mp4", "video");

        let immediate = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Immediate,
            1,
            None,
        )
        .expect("read immediate audio");
        assert!(!immediate.has_policy_omitted_rows);

        let descendants_page_one = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            1,
            None,
        )
        .expect("read descendants audio page one");
        assert_eq!(descendants_page_one.rows.len(), 1);
        assert!(descendants_page_one.next_cursor.is_some());
        assert!(descendants_page_one.has_policy_omitted_rows);

        let descendants_page_two = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            1,
            descendants_page_one.next_cursor.as_deref(),
        )
        .expect("read descendants audio page two");
        assert!(descendants_page_two.has_policy_omitted_rows);
    }

    #[test]
    fn primary_media_omission_does_not_use_raw_source_files_as_a_proxy() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/clip.mp4", "video");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/cover.jpg", "image");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read primary media");

        assert!(result.rows.is_empty());
        assert!(!result.has_policy_omitted_rows);
    }

    #[test]
    fn audio_browse_profile_preserves_source_file_audio_coverage_states() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Empty", "complete");
        insert_directory(&connection, 11, 1, "Scanning", "scanning");
        insert_directory(&connection, 12, 1, "Blocked", "complete");
        insert_directory(&connection, 13, 1, "Failed", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        set_directory_scan_issue(&connection, 13, "failed", "unknown_io");
        insert_location(&connection, 100, 1, "Missing", "user", "registered_subpath");

        let (_source_file, empty) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_eq!(empty.state, StoreContentsState::Empty);
        assert_eq!(
            empty.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(empty.scope_coverage.empty_result_authoritative);

        let (_source_file, scanning) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 11,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_eq!(scanning.state, StoreContentsState::Partial);
        assert_eq!(
            scanning.scope_coverage.state,
            StoreContentsScopeCoverageState::Scanning
        );
        assert!(!scanning.scope_coverage.empty_result_authoritative);

        let (_source_file, blocked) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 12,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_eq!(blocked.state, StoreContentsState::Blocked);
        assert_eq!(
            blocked.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked
        );
        assert!(!blocked.scope_coverage.empty_result_authoritative);

        let (_source_file, failed) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 13,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_eq!(failed.state, StoreContentsState::Failed);
        assert_eq!(
            failed.scope_coverage.state,
            StoreContentsScopeCoverageState::Failed
        );
        assert!(!failed.scope_coverage.empty_result_authoritative);

        let (_source_file, missing_location) = assert_audio_browse_matches_source_file_audio(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            StoreContentsScopeDepth::Recursive,
            10,
        );
        assert_eq!(missing_location.state, StoreContentsState::LocationMissing);
        assert_eq!(
            missing_location.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing
        );
        assert!(!missing_location.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn source_file_profile_unsupported_policy_admits_only_cue_sheets() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/album.cue", "unsupported");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/readme.txt", "unsupported");
        insert_scanned_file(&connection, 1002, 1, 10, "Media/log.pdf", "unsupported");
        insert_scanned_file(&connection, 1003, 1, 10, "Media/archive.zip", "unsupported");
        insert_scanned_file(&connection, 1004, 1, 10, "Media/blob.bin", "unsupported");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Unsupported]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read unsupported source-file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].relative_path, "Media/album.cue");
        assert_eq!(result.rows[0].file_class, "unsupported");
        assert_eq!(result.rows[0].file_kind, "cue_sheet");
        assert!(result.rows[0].primary_media.is_none());
    }

    #[test]
    fn source_file_profile_represents_missing_and_removed_media_inventory_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/missing.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Media/removed.cue", "unsupported");
        connection
            .execute(
                "UPDATE source_files
                 SET presence_state = 'missing',
                     updated_at = 2
                 WHERE source_file_id = 1000",
                [],
            )
            .expect("mark missing");
        connection
            .execute(
                "UPDATE source_files
                 SET presence_state = 'removed',
                     updated_at = 3
                 WHERE source_file_id = 1001",
                [],
            )
            .expect("mark removed");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Unsupported,
            ]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read source-file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| (row.relative_path.as_str(), row.presence.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Media/missing.wav", "missing"),
                ("Media/removed.cue", "removed")
            ]
        );
    }

    #[test]
    fn source_and_directory_scopes_apply_backend_scope_depth_without_renderer_fanout() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");

        let descendants_source = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read descendants source");
        let immediate_source = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Immediate,
            10,
            None,
        )
        .expect("read immediate source");
        let descendants_directory = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read descendants directory");

        assert_eq!(descendants_source.rows.len(), 1);
        assert_eq!(descendants_source.rows[0].relative_path, "Media/track.wav");
        assert_eq!(immediate_source.state, StoreContentsState::Empty);
        assert!(immediate_source.rows.is_empty());
        assert_eq!(descendants_directory.rows.len(), 1);
        assert_eq!(
            descendants_directory.rows[0].relative_path,
            "Media/track.wav"
        );
    }

    #[test]
    fn source_file_profile_image_only_directory_returns_image_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Covers", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Covers/front.jpg", "image");
        insert_scanned_file(&connection, 1001, 1, 10, "Covers/back.png", "image");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Image]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read source-file contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|row| row.file_class == "image"));
        assert!(result.rows.iter().all(|row| row.primary_media.is_none()));
    }

    #[test]
    fn duplicate_file_classes_are_canonicalized_deterministically() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Media", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Media/track.wav", "audio");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![
                StoreContentsFileClass::Image,
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
                StoreContentsFileClass::Unsupported,
                StoreContentsFileClass::Unsupported,
            ]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read source-file contents");

        assert_eq!(result.policy, default_source_file_policy());
    }

    #[test]
    fn empty_primary_media_policy_is_rejected() {
        let connection = open_connection();
        insert_source(&connection, 1);

        let error = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            StoreContentsReadPolicy::PrimaryMedia {
                media_kinds: Vec::new(),
            },
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect_err("empty primary-media kinds must be rejected");
        assert!(error.to_string().contains("mediaKinds must not be empty"));
    }

    #[test]
    fn empty_source_file_inventory_policy_is_rejected() {
        let connection = open_connection();
        insert_source(&connection, 1);

        let error = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            StoreContentsReadPolicy::SourceFileInventory {
                file_classes: Vec::new(),
            },
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect_err("empty source-file classes must be rejected");

        assert!(error.to_string().contains("fileClasses must not be empty"));
    }

    #[test]
    fn provided_cursor_returns_cursor_invalid_without_next_cursor() {
        let connection = open_connection();
        insert_source(&connection, 1);

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            Some("page-2"),
        )
        .expect("read contents with cursor");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn source_file_profile_returns_next_cursor_when_more_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 3);
        assert!(
            result.next_cursor.is_some(),
            "expected next_cursor when more rows exist"
        );
    }

    #[test]
    fn primary_media_profile_returns_next_cursor_when_more_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(result.rows.len(), 3);
        assert!(
            result.next_cursor.is_some(),
            "expected next_cursor when more rows exist"
        );
    }

    #[test]
    fn source_file_cursor_page_two_returns_next_deterministic_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let audio_browse_mismatch = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("reject primary-media cursor for audio browse");
        assert_eq!(
            audio_browse_mismatch.state,
            StoreContentsState::CursorInvalid
        );

        let media_kind_mismatch = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            StoreContentsReadPolicy::PrimaryMedia {
                media_kinds: vec![StorePrimaryMediaKind::Video],
            },
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("reject primary-media cursor with changed media kinds");
        assert_eq!(media_kind_mismatch.state, StoreContentsState::CursorInvalid);

        let page2 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read page 2");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn source_file_cursor_pages_interleaved_audio_and_cue_rows_without_gaps() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "disc1", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "disc1/track01.flac", "audio");
        insert_scanned_file(
            &connection,
            1001,
            1,
            10,
            "disc1/track01a.cue",
            "unsupported",
        );
        insert_scanned_file(&connection, 1002, 1, 10, "disc1/track02.flac", "audio");
        insert_scanned_file(&connection, 1003, 1, 10, "disc1/track03.flac", "audio");

        let mut cursor = None;
        let mut rows = Vec::new();
        loop {
            let result = read_contents(
                &connection,
                StoreContentsScope::Directory {
                    source_id: 1,
                    source_directory_id: 10,
                },
                source_file_policy(vec![
                    StoreContentsFileClass::Audio,
                    StoreContentsFileClass::Unsupported,
                ]),
                StoreContentsScopeDepth::Recursive,
                2,
                cursor.as_deref(),
            )
            .expect("read page");

            assert_eq!(result.state, StoreContentsState::Ready);
            rows.extend(result.rows);
            match result.next_cursor {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => break,
            }
        }

        let unique_ids = rows
            .iter()
            .map(|row| row.source_file_id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            unique_ids.len(),
            rows.len(),
            "paged rows must not duplicate"
        );
        assert_eq!(
            rows.iter()
                .map(|row| {
                    (
                        row.relative_path.as_str(),
                        row.file_class.as_str(),
                        row.file_kind.as_str(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("disc1/track01.flac", "audio", "audio"),
                ("disc1/track01a.cue", "unsupported", "cue_sheet"),
                ("disc1/track02.flac", "audio", "audio"),
                ("disc1/track03.flac", "audio", "audio"),
            ]
        );
    }

    #[test]
    fn primary_media_cursor_page_two_returns_next_deterministic_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let page2 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read page 2");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn source_file_inventory_cursor_with_changed_file_classes_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
            ]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read with changed file classes");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn cursor_with_unsupported_inclusion_removed_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            default_source_file_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");
        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
                StoreContentsFileClass::Image,
            ]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read with unsupported removed");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn cursor_with_unsupported_inclusion_added_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
                StoreContentsFileClass::Image,
            ]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");
        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            default_source_file_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read with unsupported added");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn cursor_file_class_identity_uses_requested_policy_not_returned_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            default_source_file_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            None,
        )
        .expect("read page 1");

        assert_eq!(page1.rows.len(), 2);
        assert!(
            page1
                .rows
                .iter()
                .all(|row| row.file_class.as_str() == "audio"),
            "fixture page should return only audio rows"
        );
        let cursor = page1.next_cursor.expect("expected cursor");

        let same_policy = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            default_source_file_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&cursor),
        )
        .expect("read with same requested policy");
        assert_ne!(same_policy.state, StoreContentsState::CursorInvalid);

        let changed_policy = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![
                StoreContentsFileClass::Audio,
                StoreContentsFileClass::Video,
                StoreContentsFileClass::Image,
            ]),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&cursor),
        )
        .expect("read with unsupported excluded");

        assert_eq!(changed_policy.state, StoreContentsState::CursorInvalid);
        assert!(changed_policy.rows.is_empty());
    }

    #[test]
    fn source_file_inventory_cursor_rejects_primary_media_policy() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read with changed policy");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn audio_browse_cursor_paginates_like_source_file_audio_with_distinct_policy_identity() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_audio_source_files(
            &connection,
            10,
            &[
                (1000, "Music/[1].wav"),
                (1001, "Music/[10].wav"),
                (1002, "Music/[2].wav"),
                (1003, "Music/[3].wav"),
            ],
        );

        let source_file_page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            2,
            None,
        )
        .expect("read source-file page 1");
        let audio_browse_page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            None,
        )
        .expect("read audio-browse page 1");

        assert_eq!(audio_browse_page1.rows, source_file_page1.rows);
        assert_relative_paths(
            &audio_browse_page1.rows,
            &["Music/[1].wav", "Music/[2].wav"],
        );
        let source_file_cursor = source_file_page1.next_cursor.expect("source-file cursor");
        let audio_browse_cursor = audio_browse_page1.next_cursor.expect("audio-browse cursor");
        assert_ne!(
            audio_browse_cursor, source_file_cursor,
            "cursor identity must include policy discriminant"
        );

        let source_file_page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&source_file_cursor),
        )
        .expect("read source-file page 2");
        let audio_browse_page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&audio_browse_cursor),
        )
        .expect("read audio-browse page 2");

        assert_eq!(audio_browse_page2.rows, source_file_page2.rows);
        assert_relative_paths(
            &audio_browse_page2.rows,
            &["Music/[3].wav", "Music/[10].wav"],
        );
        assert!(audio_browse_page2.next_cursor.is_none());

        let audio_from_source_file_cursor = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            audio_browse_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&source_file_cursor),
        )
        .expect("read audio-browse with source-file cursor");
        assert_eq!(
            audio_from_source_file_cursor.state,
            StoreContentsState::CursorInvalid
        );
        assert!(audio_from_source_file_cursor.rows.is_empty());

        let source_file_from_audio_cursor = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&audio_browse_cursor),
        )
        .expect("read source-file with audio-browse cursor");
        assert_eq!(
            source_file_from_audio_cursor.state,
            StoreContentsState::CursorInvalid
        );
        assert!(source_file_from_audio_cursor.rows.is_empty());

        let primary_media_from_audio_cursor = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&audio_browse_cursor),
        )
        .expect("read primary-media with audio-browse cursor");
        assert_eq!(
            primary_media_from_audio_cursor.state,
            StoreContentsState::CursorInvalid
        );
        assert!(primary_media_from_audio_cursor.rows.is_empty());
    }

    #[test]
    fn cursor_with_changed_scope_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_source(&connection, 2);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 20, 2, "Videos", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
            insert_scanned_file(
                &connection,
                2000 + i,
                2,
                20,
                &format!("Videos/clip_{:02}.mp4", i),
                "video",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 2 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read with changed scope");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn source_cursor_does_not_validate_for_directory_request() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");
        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read directory with source cursor");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn directory_cursor_does_not_validate_for_source_location_request() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");
        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read source location with directory cursor");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn cursor_with_changed_scope_depth_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1");

        let cursor = page1.next_cursor.expect("expected cursor");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Immediate,
            3,
            Some(&cursor),
        )
        .expect("read with changed scope depth");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn malformed_cursor_returns_cursor_invalid() {
        let connection = open_connection();
        insert_source(&connection, 1);

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            Some("!invalid-base64url!"),
        )
        .expect("read with malformed cursor");

        assert_eq!(result.state, StoreContentsState::CursorInvalid);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn source_file_last_page_emits_no_next_cursor() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..3 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.rows.len(), 3);
        assert!(
            result.next_cursor.is_none(),
            "last page should have no next_cursor when all rows fit in limit"
        );
    }

    #[test]
    fn primary_media_last_page_emits_no_next_cursor() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..3 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.rows.len(), 3);
        assert!(
            result.next_cursor.is_none(),
            "last page should have no next_cursor when all rows fit in limit"
        );
    }

    #[test]
    fn directory_source_file_cursor_returns_correct_page_two() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1 (directory scope)");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read page 2 (directory scope)");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn directory_primary_media_cursor_returns_correct_page_two() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_promoted_media_file(
                &connection,
                i + 1,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1 (directory scope, primary media)");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read page 2 (directory scope, primary media)");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn source_location_source_file_cursor_returns_correct_page_two() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            None,
        )
        .expect("read page 1 (source-location scope)");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let page2 = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            3,
            Some(&cursor),
        )
        .expect("read page 2 (source-location scope)");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn immediate_directory_source_file_cursor_returns_correct_page_two() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..5 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let page1 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Immediate,
            3,
            None,
        )
        .expect("read page 1 (immediate directory scope)");

        assert_eq!(page1.rows.len(), 3);
        let cursor = page1.next_cursor.expect("expected cursor for page 2");

        let page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Immediate,
            3,
            Some(&cursor),
        )
        .expect("read page 2 (immediate directory scope)");

        assert_eq!(page2.rows.len(), 2);
        assert!(
            page2.next_cursor.is_none(),
            "last page should have no next_cursor"
        );
        assert_eq!(
            page1.rows[0].source_file_id, 1000,
            "page 1 should start with source_file_id 1000"
        );
        assert_eq!(
            page2.rows[0].source_file_id, 1003,
            "page 2 should start with source_file_id 1003"
        );
    }

    #[test]
    fn contents_whole_source_uses_index_not_table_scan() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..30 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let file_class_predicate = super::file_classes_predicate_sql(
            "sf.file_class",
            "sf.file_kind",
            &primary_media_file_classes(),
        );
        let sql = super::primary_media_rows_sql(
            None,
            "sf.source_id = ?1",
            &file_class_predicate,
            None,
            2,
        );
        let plan = dump_query_plan(
            &connection,
            &sql,
            &[
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Integer(100),
            ],
        );
        let plan_lower = plan.to_lowercase();

        eprintln!("=== Whole source query plan ===\n{plan}");

        assert!(
            !plan_lower.contains("scan source_files"),
            "whole-source scope should use an index, not a full table scan"
        );
    }

    #[test]
    fn source_file_contents_whole_source_uses_browse_order_index() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..30 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }

        let file_class_predicate = super::file_classes_predicate_sql(
            "sf.file_class",
            "sf.file_kind",
            &[StoreContentsFileClass::Audio],
        );
        let sql =
            super::source_file_rows_sql(None, "sf.source_id = ?1", &file_class_predicate, None, 2);
        let plan = dump_query_plan(
            &connection,
            &sql,
            &[
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Integer(100),
            ],
        );

        assert!(
            plan.contains("source_files_source_browse_order"),
            "whole-source source-file contents should use browse-order index, observed:\n{plan}"
        );
    }

    #[test]
    fn contents_directory_prefix_uses_indexed_relative_path_scope() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Nested", "complete");
        insert_directory(&connection, 12, 1, "Music2", "complete");
        for i in 0..30 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                11,
                &format!("Music/Nested/track_{:02}.wav", i),
                "audio",
            );
        }
        insert_scanned_file(&connection, 2000, 1, 12, "Music2/other.wav", "audio");

        let source_predicate = format!(
            "sf.source_id = ?1 AND {}",
            super::source_file_descendant_predicate("sf", "?2")
        );
        let file_class_predicate = super::file_classes_predicate_sql(
            "sf.file_class",
            "sf.file_kind",
            &primary_media_file_classes(),
        );
        let sql =
            super::primary_media_rows_sql(None, &source_predicate, &file_class_predicate, None, 3);
        let plan = dump_query_plan(
            &connection,
            &sql,
            &[
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Text("Music/Nested".to_string()),
                rusqlite::types::Value::Integer(100),
            ],
        );
        let plan_lower = plan.to_lowercase();

        eprintln!("=== Directory prefix query plan ===\n{plan}");

        assert!(
            plan_lower.contains("source_files_source_relative_path_binary")
                || plan_lower.contains("sourcefacts_source_basis"),
            "directory-prefix scope should use an indexed relative-path range, observed:\n{plan}"
        );
    }

    #[test]
    fn contents_accepted_locations_avoids_source_files_table_scan() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Other", "complete");
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        for i in 0..30 {
            insert_scanned_file(
                &connection,
                1000 + i,
                1,
                10,
                &format!("Music/track_{:02}.wav", i),
                "audio",
            );
        }
        insert_scanned_file(&connection, 2000, 1, 11, "Other/clip.mp4", "video");

        let predicate = format!(
            "sf.source_id = ?1
             AND EXISTS (
                 SELECT 1
                 FROM accepted_locations al
                 WHERE {}
             )",
            super::relative_path_scope_predicate("sf", "al.relative_path")
        );
        let file_class_predicate = super::file_classes_predicate_sql(
            "sf.file_class",
            "sf.file_kind",
            &primary_media_file_classes(),
        );
        let sql = super::primary_media_rows_sql(
            Some(super::accepted_locations_cte()),
            &predicate,
            &file_class_predicate,
            None,
            2,
        );
        let plan = dump_query_plan(
            &connection,
            &sql,
            &[
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Integer(100),
            ],
        );
        let plan_lower = plan.to_lowercase();

        eprintln!("=== Accepted locations query plan ===\n{plan}");

        assert!(
            !plan_lower.contains("scan source_files"),
            "accepted-locations scope should not full-table-scan source_files, observed:\n{plan}"
        );
    }

    #[test]
    fn contents_mixed_promotion_uses_join_indexes() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        for i in 0..10 {
            let source_file_id = 1000 + i as i64;
            let library_asset_id = i as i64 + 1;
            insert_promoted_media_file(
                &connection,
                library_asset_id,
                source_file_id,
                1,
                10,
                &format!("Music/promoted_{:02}.wav", i),
                "audio",
                &format!("Track {:02}", i),
            );
        }
        for i in 0..10 {
            insert_scanned_file(
                &connection,
                2000 + i as i64,
                1,
                10,
                &format!("Music/scanned_{:02}.wav", i),
                "audio",
            );
        }

        let file_class_predicate = super::file_classes_predicate_sql(
            "sf.file_class",
            "sf.file_kind",
            &primary_media_file_classes(),
        );
        let sql = super::primary_media_rows_sql(
            None,
            "sf.source_id = ?1",
            &file_class_predicate,
            None,
            2,
        );
        let plan = dump_query_plan(
            &connection,
            &sql,
            &[
                rusqlite::types::Value::Integer(1),
                rusqlite::types::Value::Integer(100),
            ],
        );
        let plan_lower = plan.to_lowercase();

        eprintln!("=== Mixed promotion query plan ===\n{plan}");

        assert!(
            plan_lower.contains("source_files_source_presence"),
            "mixed-promotion plan should use indexed source-file scope lookup, observed:\n{plan}"
        );
        assert!(
            plan_lower.contains("source_file_attachment_links"),
            "mixed-promotion plan should use indexed attachment-link lookup, observed:\n{plan}"
        );
        assert!(
            plan_lower.contains("search attachment"),
            "mixed-promotion plan should use content-attachment lookup, observed:\n{plan}"
        );
        assert!(
            plan_lower.contains("primary_media_candidates"),
            "mixed-promotion plan should use promoted-candidate lookup, observed:\n{plan}"
        );
        assert!(
            !plan_lower.contains("sourcesegmentsets")
                && !plan_lower.contains("sourcesegments")
                && !plan_lower.contains("libraryassetattachments")
                && !plan_lower.contains("librarybrowserrows"),
            "mixed-promotion plan must not depend on legacy browser-row joins, observed:\n{plan}"
        );
    }

    fn dump_query_plan(
        connection: &Connection,
        sql: &str,
        params: &[rusqlite::types::Value],
    ) -> String {
        let explain_sql = format!("EXPLAIN QUERY PLAN {sql}");
        let mut stmt = connection
            .prepare(&explain_sql)
            .expect("prepare EXPLAIN QUERY PLAN");
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok(format!(
                    "{:>4}|{:>4}|{}",
                    row.get::<_, i64>(0).unwrap_or(-1),
                    row.get::<_, i64>(1).unwrap_or(-1),
                    row.get::<_, String>(3).unwrap_or_default(),
                ))
            })
            .expect("query EXPLAIN QUERY PLAN rows");

        rows.map(|r| r.expect("read plan row"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn partial_scan_phase_does_not_produce_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert!(!result.scope_coverage.empty_result_authoritative);
        assert!(!result.scope_coverage.subtree_coverage_complete);
        assert_ne!(result.state, StoreContentsState::Empty);
    }

    #[test]
    fn partial_scan_phase_returns_visible_rows() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/track.wav",
            "audio",
            "Track",
        );
        insert_directory(&connection, 11, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 11, "blocked", "permission_denied");

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert!(
            !result.rows.is_empty(),
            "partial scan must return visible rows"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked
        );
        assert!(!result.scope_coverage.subtree_coverage_complete);
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn clean_directory_under_partial_source_is_complete_and_authoritative() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );

        let dir_result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 11,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for Good");

        assert_eq!(
            dir_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "clean sibling directory under partial source must have complete coverage"
        );
        assert!(dir_result.scope_coverage.subtree_coverage_complete);
        assert_eq!(dir_result.state, StoreContentsState::Ready);
        assert!(!dir_result.rows.is_empty());

        let source_result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for source");

        assert_ne!(
            source_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "whole source must remain non-complete when partial"
        );
        assert!(!source_result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn clean_empty_directory_under_partial_source_is_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");

        let dir_result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 11,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for empty Good");

        assert_eq!(
            dir_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "clean empty sibling directory under partial source may be authoritative empty"
        );
        assert!(dir_result.scope_coverage.subtree_coverage_complete);
        assert!(dir_result.scope_coverage.empty_result_authoritative);
        assert_eq!(dir_result.state, StoreContentsState::Empty);
    }

    #[test]
    fn blocked_scope_under_partial_source_remains_blocked() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 11, "blocked", "permission_denied");

        let dir_result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 11,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for Locked");

        assert_eq!(
            dir_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked,
            "blocked scope under partial source must remain blocked"
        );
        assert!(!dir_result.scope_coverage.subtree_coverage_complete);
        assert!(!dir_result.scope_coverage.empty_result_authoritative);
        assert_eq!(dir_result.state, StoreContentsState::Blocked);
    }

    #[test]
    fn source_location_missing_proven_under_partial_scan() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 11, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(
            result.state,
            StoreContentsState::LocationMissing,
            "Music/DeletedFolder must be classified missing, not unknown or pending"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing,
            "coverage must be locationMissing for scope-proven absent path under partial scan"
        );
        assert!(!result.scope_coverage.subtree_coverage_complete);
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn source_location_under_blocked_parent_is_blocked_not_missing() {
        let connection = open_connection();
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 11, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/Locked/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_ne!(
            result.state,
            StoreContentsState::Empty,
            "blocked location must not produce authoritative empty"
        );
        assert_ne!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing,
            "blocked parent must not be classified as missing"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
        assert!(!result.scope_coverage.subtree_coverage_complete);
    }

    #[test]
    fn source_location_under_failed_parent_is_failed_not_missing() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Crashed", "complete");
        set_directory_scan_issue(&connection, 11, "failed", "unknown_io");
        insert_location(
            &connection,
            100,
            1,
            "Music/Crashed/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Failed,
            "failed parent must produce failed coverage"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn source_location_under_pending_parent_is_pending_not_missing() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Pending", "pending");
        insert_location(
            &connection,
            100,
            1,
            "Music/Pending/SubFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Pending,
            "pending parent must produce pending coverage"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
        assert!(!result.scope_coverage.subtree_coverage_complete);
    }

    #[test]
    fn accepted_locations_complete_under_partial_source() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Other", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for source with accepted location");

        assert!(
            !result.rows.is_empty(),
            "accepted location rows must be returned"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "all accepted locations complete must produce complete coverage even under partial source"
        );
        assert!(result.scope_coverage.subtree_coverage_complete);
        assert!(result.scope_coverage.empty_result_authoritative || !result.rows.is_empty());
    }

    #[test]
    fn accepted_locations_noncomplete_under_partial_source_when_any_scope_unproven() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/Locked",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_ne!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "source with a blocked accepted location must not be complete"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
        assert!(!result.scope_coverage.subtree_coverage_complete);
    }

    #[test]
    fn source_location_scope_and_source_agree_on_scope_coverage() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 1",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );

        let location_result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for SourceLocation");

        let source_result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for Source");

        assert_eq!(
            location_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "SourceLocation scope for Music/Good must be complete"
        );
        assert_eq!(
            source_result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "Source with only Music/Good accepted location must be complete"
        );

        insert_location(
            &connection,
            101,
            1,
            "Music/Locked",
            "user",
            "registered_subpath",
        );

        let location_result_blocked = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 101,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for blocked SourceLocation");

        let source_result_blocked = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents for Source with blocked location");

        assert_eq!(
            location_result_blocked.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked,
            "SourceLocation scope for Music/Locked must be blocked"
        );
        assert_eq!(
            source_result_blocked.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked,
            "Source with blocked accepted location must be blocked"
        );
    }

    #[test]
    fn accepted_locations_mixed_present_rows_plus_missing_is_not_location_missing() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_promoted_media_file(
            &connection,
            1,
            5000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert!(
            !result.rows.is_empty(),
            "rows from present accepted location must be returned"
        );
        assert_ne!(
            result.state,
            StoreContentsState::LocationMissing,
            "mixed present + missing must not be LocationMissing"
        );
        assert_ne!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing,
            "mixed present + missing coverage must not be LocationMissing"
        );
        assert_ne!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete,
            "mixed present + missing coverage must not be Complete"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Incomplete,
            "mixed present + missing coverage must be Incomplete"
        );
        assert!(!result.scope_coverage.subtree_coverage_complete);
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn accepted_locations_mixed_present_empty_plus_missing_is_not_location_missing() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Empty", "complete");
        insert_location(
            &connection,
            100,
            1,
            "Music/Empty",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_ne!(
            result.state,
            StoreContentsState::Empty,
            "mixed present + missing must not be Empty"
        );
        assert_ne!(
            result.state,
            StoreContentsState::LocationMissing,
            "mixed present + missing must not be LocationMissing"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Incomplete,
            "mixed present + missing coverage must be Incomplete"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn accepted_locations_all_missing_is_location_missing() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_location(
            &connection,
            100,
            1,
            "Music/Gone1",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/Gone2",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(
            result.state,
            StoreContentsState::LocationMissing,
            "all missing accepted locations must be LocationMissing"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing,
            "all missing coverage must be LocationMissing"
        );
        assert!(result.rows.is_empty());
        assert!(!result.scope_coverage.empty_result_authoritative);
        assert!(!result.scope_coverage.subtree_coverage_complete);
    }

    #[test]
    fn accepted_locations_present_plus_blocked_returns_rows_and_blocked_scope_coverage() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Locked", "complete");
        set_directory_scan_issue(&connection, 12, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/Locked",
            "user",
            "registered_subpath",
        );
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert!(
            !result.rows.is_empty(),
            "rows from present accepted location must be returned"
        );
        assert_ne!(
            result.state,
            StoreContentsState::LocationMissing,
            "present + blocked must not be LocationMissing"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Blocked,
            "present + blocked must be Blocked coverage"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn accepted_locations_present_plus_failed_returns_rows_and_failed_scope_coverage() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_directory(&connection, 12, 1, "Music/Crashed", "complete");
        set_directory_scan_issue(&connection, 12, "failed", "unknown_io");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_location(
            &connection,
            101,
            1,
            "Music/Crashed",
            "user",
            "registered_subpath",
        );
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert!(
            !result.rows.is_empty(),
            "rows from present accepted location must be returned"
        );
        assert_ne!(
            result.state,
            StoreContentsState::LocationMissing,
            "present + failed must not be LocationMissing"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Failed,
            "present + failed must be Failed coverage"
        );
        assert!(!result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn accepted_locations_all_present_complete_no_rows_is_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Empty", "complete");
        insert_location(
            &connection,
            100,
            1,
            "Music/Empty",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Empty);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(result.scope_coverage.subtree_coverage_complete);
        assert!(result.scope_coverage.empty_result_authoritative);
    }

    #[test]
    fn accepted_locations_all_present_complete_with_rows_is_ready() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Good", "complete");
        insert_location(
            &connection,
            100,
            1,
            "Music/Good",
            "user",
            "registered_subpath",
        );
        insert_promoted_media_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Good/track.wav",
            "audio",
            "Track",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Source { source_id: 1 },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::Complete
        );
        assert!(result.scope_coverage.subtree_coverage_complete);
    }

    #[test]
    fn direct_source_location_missing_still_returns_location_missing() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_location(
            &connection,
            100,
            1,
            "Music/Deleted",
            "user",
            "registered_subpath",
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::SourceLocation {
                source_location_id: 100,
            },
            primary_media_policy(),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(
            result.state,
            StoreContentsState::LocationMissing,
            "direct SourceLocation missing must remain LocationMissing"
        );
        assert_eq!(
            result.scope_coverage.state,
            StoreContentsScopeCoverageState::LocationMissing,
            "direct SourceLocation missing coverage must be LocationMissing"
        );
    }

    #[test]
    fn source_file_contents_natural_order_track_numbers() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_audio_source_files(
            &connection,
            10,
            &[
                (1000, "Music/Track 001.wav"),
                (1001, "Music/Track 01.wav"),
                (1002, "Music/Track 10.wav"),
                (1003, "Music/Track 2.wav"),
                (1004, "Music/Track 1.wav"),
            ],
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            10,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_relative_paths(
            &result.rows,
            &[
                "Music/Track 1.wav",
                "Music/Track 01.wav",
                "Music/Track 001.wav",
                "Music/Track 2.wav",
                "Music/Track 10.wav",
            ],
        );
    }

    #[test]
    fn source_file_contents_descendants_path_natural_order() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Folder [2]", "complete");
        insert_directory(&connection, 12, 1, "Music/Folder [1]", "complete");
        insert_directory(&connection, 13, 1, "Music/Folder [10]", "complete");
        insert_audio_source_files(
            &connection,
            12,
            &[
                (1000, "Music/Folder [1]/Track 10.wav"),
                (1001, "Music/Folder [1]/Track 2.wav"),
                (1002, "Music/Folder [1]/Track 1.wav"),
            ],
        );
        insert_audio_source_files(&connection, 11, &[(1003, "Music/Folder [2]/Track 1.wav")]);
        insert_audio_source_files(&connection, 13, &[(1004, "Music/Folder [10]/Track 1.wav")]);

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            20,
            None,
        )
        .expect("read contents");

        assert_eq!(result.state, StoreContentsState::Ready);
        assert_relative_paths(
            &result.rows,
            &[
                "Music/Folder [1]/Track 1.wav",
                "Music/Folder [1]/Track 2.wav",
                "Music/Folder [1]/Track 10.wav",
                "Music/Folder [2]/Track 1.wav",
                "Music/Folder [10]/Track 1.wav",
            ],
        );
    }

    #[test]
    fn source_file_contents_cursor_uses_selected_persisted_browse_sort_key() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/Track 1.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Music/Track 2.wav", "audio");
        connection
            .execute(
                "UPDATE source_files
                 SET relative_path_browse_sort_key = 'persisted-authority-key'
                 WHERE source_file_id = 1000",
                [],
            )
            .expect("set persisted browse sort key");

        let page = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            1,
            None,
        )
        .expect("read contents");

        assert_eq!(page.rows[0].source_file_id, 1000);
        let cursor = super::decode_cursor(page.next_cursor.as_deref().expect("next cursor"))
            .expect("decode next cursor");
        assert!(matches!(
            cursor.position,
            super::ContentsCursorPosition::SourceFile {
                relative_path_browse_sort_key,
                relative_path,
                source_file_id: 1000,
            } if relative_path_browse_sort_key == "persisted-authority-key"
                && relative_path == "Music/Track 1.wav"
        ));
    }

    #[test]
    fn source_file_contents_cursor_pagination_matches_natural_sql_order() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_audio_source_files(
            &connection,
            10,
            &[
                (1000, "Music/[1].wav"),
                (1001, "Music/[10].wav"),
                (1002, "Music/[2].wav"),
            ],
        );

        let result = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            2,
            None,
        )
        .expect("read first page");

        assert_relative_paths(&result.rows, &["Music/[1].wav", "Music/[2].wav"]);
        assert!(result.next_cursor.is_some());

        let cursor = result.next_cursor.unwrap();
        let page2 = read_contents(
            &connection,
            StoreContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            source_file_policy(vec![StoreContentsFileClass::Audio]),
            StoreContentsScopeDepth::Recursive,
            2,
            Some(&cursor),
        )
        .expect("read second page");

        assert_relative_paths(&page2.rows, &["Music/[10].wav"]);
        assert!(page2.next_cursor.is_none());
        assert_eq!(page2.state, StoreContentsState::Ready);
    }
}
