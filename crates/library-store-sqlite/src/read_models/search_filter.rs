use rusqlite::types::Value;
use rusqlite::{Connection, Row, params, params_from_iter};

use crate::{LibrarySqliteError, LibrarySqliteResult};

const SEARCH_CURSOR_VERSION: u8 = 1;
const SEARCH_INDEXER_VERSION: &str = "search_filter_v0";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SearchCursor {
    version: u8,
    identity: SearchCursorIdentity,
    position: SearchCursorPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SearchCursorIdentity {
    scope: StoreSearchScope,
    recursion: StoreSearchRecursion,
    text_query: Option<String>,
    target_kinds: Vec<String>,
    filters: StoreSearchFilters,
    sort: StoreSearchSort,
    page_size: usize,
    index_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SearchCursorPosition {
    sort_key: String,
    result_kind_order: i64,
    stable_key: String,
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
    pub fn storage_value(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::SourceLocation => "source_location",
            Self::Directory => "directory",
            Self::SourceFile => "source_file",
        }
    }

    fn from_storage_value(value: &str) -> LibrarySqliteResult<Self> {
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
    fn storage_value(self) -> &'static str {
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
    fn storage_value(self) -> &'static str {
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
    fn storage_value(self) -> &'static str {
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
    fn storage_value(self) -> &'static str {
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
}

impl StoreSearchAttachmentLinkState {
    fn storage_value(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Stale => "stale",
            Self::Missing => "missing",
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
    fn storage_value(self) -> &'static str {
        match self {
            Self::AudioWorkflow => "audio_workflow",
            Self::PlayableMedia => "playable_media",
            Self::ExplicitInventory => "explicit_inventory",
            Self::CompanionFile => "companion_file",
            Self::NotMediaRelevant => "not_media_relevant",
        }
    }
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
    pub index_state: String,
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
    pub authority_layer: String,
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
    pub attachment_link_state: String,
    pub attachment_id: Option<i64>,
    pub content_hash_algorithm: Option<String>,
    pub content_hash_value: Option<String>,
    pub evidence_coverage_state: String,
    pub match_reason: String,
    pub updated_at: i64,
    sort_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildSearchFilterIndexForSourceResult {
    pub source_id: i64,
    pub generation: i64,
    pub rows_indexed: usize,
}

pub(crate) fn rebuild_search_filter_index_for_source(
    connection: &Connection,
    source_id: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<RebuildSearchFilterIndexForSourceResult> {
    let source_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sources WHERE source_id = ?1)",
            [source_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .map_err(LibrarySqliteError::from)?;
    if !source_exists {
        return Err(LibrarySqliteError::Canonical(crate::CanonicalError::new(
            crate::CanonicalErrorCode::NotFound,
            format!("source {source_id} does not exist"),
        )));
    }

    connection.execute(
        "UPDATE search_filter_index_metadata
         SET state = 'rebuilding',
             updated_at = ?1
         WHERE search_filter_index_id = 1",
        [rebuilt_at_ms],
    )?;

    let old_row_ids = connection
        .prepare(
            "SELECT row_id
             FROM search_filter_index_rows
             WHERE source_id = ?1",
        )?
        .query_map([source_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for row_id in old_row_ids {
        connection.execute(
            "DELETE FROM search_filter_index_fts WHERE rowid = ?1",
            [row_id],
        )?;
    }
    connection.execute(
        "DELETE FROM search_filter_index_rows WHERE source_id = ?1",
        [source_id],
    )?;

    let generation = connection.query_row(
        "UPDATE search_filter_index_metadata
         SET generation = generation + 1,
             state = 'ready',
             updated_at = ?1
         WHERE search_filter_index_id = 1
         RETURNING generation",
        [rebuilt_at_ms],
        |row| row.get::<_, i64>(0),
    )?;

    insert_source_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_source_location_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_directory_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_source_file_rows(connection, source_id, generation, rebuilt_at_ms)?;

    connection.execute(
        "INSERT INTO search_filter_index_fts(rowid, display_label, display_path)
         SELECT row_id, display_label, COALESCE(display_path, '')
         FROM search_filter_index_rows
         WHERE source_id = ?1",
        [source_id],
    )?;

    let rows_indexed = connection.query_row(
        "SELECT COUNT(*)
         FROM search_filter_index_rows
         WHERE source_id = ?1",
        [source_id],
        |row| read_usize(row, 0),
    )?;

    Ok(RebuildSearchFilterIndexForSourceResult {
        source_id,
        generation,
        rows_indexed,
    })
}

pub(crate) fn read_search_filter(
    connection: &Connection,
    request: StoreSearchRequest,
) -> LibrarySqliteResult<StoreSearchResult> {
    let (index_generation, index_state) = read_index_metadata(connection)?;
    let identity = canonical_identity(&request, index_generation);
    if identity.target_kinds.is_empty() {
        return Ok(StoreSearchResult {
            state: StoreSearchState::Unsupported,
            query_identity: public_identity(&identity),
            index_generation,
            index_state,
            rows: Vec::new(),
            next_cursor: None,
            detail: Some("Search/filter requires at least one target kind.".to_string()),
        });
    }

    let cursor_position = if let Some(cursor) = request.cursor.as_deref() {
        let decoded = match decode_cursor(cursor) {
            Ok(decoded) => decoded,
            Err(_) => return cursor_invalid(identity, index_generation, index_state),
        };
        if decoded.version != SEARCH_CURSOR_VERSION || decoded.identity != identity {
            return cursor_invalid(identity, index_generation, index_state);
        }
        Some(decoded.position)
    } else {
        None
    };

    let mut sql = String::from(SEARCH_SELECT_SQL);
    let mut predicates = Vec::<String>::new();
    let mut values = Vec::<Value>::new();

    append_scope_predicate(
        &mut predicates,
        &mut values,
        &identity.scope,
        identity.recursion,
    )?;
    push_in_values(
        &mut predicates,
        &mut values,
        "result_kind",
        identity.target_kinds.iter().map(String::as_str),
    );
    append_filters(&mut predicates, &mut values, &identity.filters);
    append_text_predicate(&mut predicates, &mut values, identity.text_query.as_deref());
    if let Some(position) = &cursor_position {
        predicates.push(
            "(sort_key > ? OR (sort_key = ? AND result_kind_order > ?) OR (sort_key = ? AND result_kind_order = ? AND stable_key > ?))"
                .to_string(),
        );
        values.push(Value::Text(position.sort_key.clone()));
        values.push(Value::Text(position.sort_key.clone()));
        values.push(Value::Integer(position.result_kind_order));
        values.push(Value::Text(position.sort_key.clone()));
        values.push(Value::Integer(position.result_kind_order));
        values.push(Value::Text(position.stable_key.clone()));
    }

    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    sql.push_str(match identity.sort {
        StoreSearchSort::PathName | StoreSearchSort::Relevance => {
            " ORDER BY sort_key ASC, result_kind_order ASC, stable_key ASC"
        }
    });
    sql.push_str(" LIMIT ?");
    values.push(Value::Integer(
        i64::try_from(identity.page_size.saturating_add(1)).map_err(|_| {
            LibrarySqliteError::MalformedSchemaState("search/filter limit exceeds i64".to_string())
        })?,
    ));

    let mut rows = connection
        .prepare(&sql)?
        .query_map(params_from_iter(values.iter()), |row| {
            map_search_row(row, identity.text_query.as_deref())
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let has_more = rows.len() > identity.page_size;
    if has_more {
        rows.truncate(identity.page_size);
    }
    let next_cursor = if has_more {
        rows.last()
            .map(|row| build_next_cursor(&identity, row))
            .transpose()?
    } else {
        None
    };
    let state = if rows.is_empty() {
        StoreSearchState::Empty
    } else {
        StoreSearchState::Ready
    };

    Ok(StoreSearchResult {
        state,
        query_identity: public_identity(&identity),
        index_generation,
        index_state,
        rows,
        next_cursor,
        detail: None,
    })
}

fn insert_source_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             display_label, display_path, sort_key, source_access_state,
             source_scan_phase, evidence_coverage_state, updated_at
         )
         SELECT ?2, 'source', 'source', 'source:' || source.source_id, source.source_id,
                source.display_name,
                COALESCE(state.effective_path, locator.absolute_path),
                lower(source.display_name),
                state.access_state,
                scan.scan_phase,
                'not_applicable',
                ?3
         FROM sources source
         LEFT JOIN source_state state ON state.source_id = source.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = source.source_id
         LEFT JOIN source_locators locator ON locator.source_id = source.source_id
         WHERE source.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_source_location_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             source_location_id, display_label, display_path, relative_path, sort_key,
             source_access_state, source_scan_phase, presence_state, evidence_coverage_state,
             updated_at
         )
         SELECT ?2, 'source_location', 'source_location',
                'sourceLocation:' || location.source_location_id,
                location.source_id,
                location.source_location_id,
                COALESCE(location.display_name, location.relative_path),
                location.relative_path,
                location.relative_path,
                lower(location.relative_path),
                state.access_state,
                scan.scan_phase,
                CASE WHEN directory.source_directory_id IS NULL THEN 'missing' ELSE directory.presence_state END,
                'not_applicable',
                ?3
         FROM source_locations location
         LEFT JOIN source_directories directory
           ON directory.source_id = location.source_id
          AND directory.relative_path = location.relative_path
         LEFT JOIN source_state state ON state.source_id = location.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = location.source_id
         WHERE location.source_id = ?1
           AND location.is_user_visible = 1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_directory_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             source_directory_id, parent_source_directory_id, display_label, display_path,
             relative_path, sort_key, presence_state, source_access_state, source_scan_phase,
             evidence_coverage_state, updated_at
         )
         SELECT ?2, 'directory', 'source_hierarchy',
                'directory:' || directory.source_directory_id,
                directory.source_id,
                directory.source_directory_id,
                directory.parent_source_directory_id,
                directory.name,
                directory.relative_path,
                directory.relative_path,
                directory.name_browse_sort_key || '/' || directory.relative_path,
                directory.presence_state,
                state.access_state,
                scan.scan_phase,
                'not_applicable',
                ?3
         FROM source_directories directory
         LEFT JOIN source_state state ON state.source_id = directory.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = directory.source_id
         WHERE directory.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_source_file_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             parent_source_directory_id, source_file_id, display_label, display_path,
             relative_path, sort_key, file_class, file_kind, media_relevance,
             presence_state, source_access_state, source_scan_phase, has_current_blake3,
             has_current_probe, attachment_link_state, attachment_id, content_hash_algorithm,
             content_hash_value, evidence_coverage_state, updated_at
         )
         SELECT ?2,
                'source_file',
                'source_file_inventory',
                'sourceFile:' || file.source_file_id,
                file.source_id,
                file.parent_source_directory_id,
                file.source_file_id,
                file.name,
                file.relative_path,
                file.relative_path,
                file.relative_path_browse_sort_key || '/' || file.relative_path,
                file.file_class,
                file.file_kind,
                CASE
                    WHEN file.file_class IN ('audio', 'video') THEN 'playable_media'
                    WHEN file.file_kind = 'cue_sheet' THEN 'audio_workflow'
                    WHEN file.file_class IN ('image', 'unsupported') THEN 'explicit_inventory'
                    WHEN file.file_kind IN ('log_doc', 'text_doc') THEN 'companion_file'
                    ELSE 'not_media_relevant'
                END,
                file.presence_state,
                state.access_state,
                scan.scan_phase,
                CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND facts.content_hash_algorithm = 'blake3'
                     AND facts.content_hash_value IS NOT NULL
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                    THEN 1 ELSE 0
                END,
                CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                     AND (
                         facts.mime_type IS NOT NULL
                         OR facts.duration_ms IS NOT NULL
                         OR facts.sample_rate_hz IS NOT NULL
                         OR facts.channels IS NOT NULL
                         OR facts.bit_depth IS NOT NULL
                         OR facts.codec IS NOT NULL
                     )
                    THEN 1 ELSE 0
                END,
                CASE
                    WHEN link.source_file_attachment_link_id IS NULL THEN 'missing'
                    WHEN facts.source_file_id IS NOT NULL
                     AND facts.content_hash_algorithm = attachment.content_hash_algorithm
                     AND facts.content_hash_value = attachment.content_hash_value
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                    THEN 'current'
                    ELSE 'stale'
                END,
                attachment.attachment_id,
                attachment.content_hash_algorithm,
                attachment.content_hash_value,
                'indexed',
                ?3
         FROM source_files file
         LEFT JOIN source_state state ON state.source_id = file.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = file.source_id
         LEFT JOIN SourceFacts facts ON facts.source_file_id = file.source_file_id
         LEFT JOIN source_file_attachment_links link ON link.source_file_id = file.source_file_id
         LEFT JOIN content_attachments attachment ON attachment.attachment_id = link.attachment_id
         WHERE file.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn read_index_metadata(connection: &Connection) -> LibrarySqliteResult<(i64, String)> {
    connection
        .query_row(
            "SELECT generation, state
             FROM search_filter_index_metadata
             WHERE search_filter_index_id = 1
               AND indexer_version = ?1",
            [SEARCH_INDEXER_VERSION],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(Into::into)
}

fn canonical_identity(request: &StoreSearchRequest, index_generation: i64) -> SearchCursorIdentity {
    let mut target_kinds = if request.target_kinds.is_empty() {
        vec![
            "source".to_string(),
            "source_location".to_string(),
            "directory".to_string(),
            "source_file".to_string(),
        ]
    } else {
        request
            .target_kinds
            .iter()
            .map(|kind| kind.storage_value().to_string())
            .collect()
    };
    target_kinds.sort();
    target_kinds.dedup();

    SearchCursorIdentity {
        scope: request.scope.clone(),
        recursion: request.recursion,
        text_query: normalize_text_query(request.text_query.as_deref()),
        target_kinds,
        filters: canonical_filters(&request.filters),
        sort: request.sort,
        page_size: request.limit,
        index_generation,
    }
}

fn public_identity(identity: &SearchCursorIdentity) -> StoreSearchQueryIdentity {
    StoreSearchQueryIdentity {
        scope: identity.scope.clone(),
        recursion: identity.recursion,
        text_query: identity.text_query.clone(),
        target_kinds: identity
            .target_kinds
            .iter()
            .filter_map(|value| StoreSearchResultKind::from_storage_value(value).ok())
            .collect(),
        filters: identity.filters.clone(),
        sort: identity.sort,
        page_size: identity.page_size,
        index_generation: identity.index_generation,
    }
}

fn canonical_filters(filters: &StoreSearchFilters) -> StoreSearchFilters {
    let mut canonical = filters.clone();
    canonical
        .file_classes
        .sort_by_key(|value| value.storage_value());
    canonical.file_classes.dedup();
    canonical
        .file_kinds
        .sort_by_key(|value| value.storage_value());
    canonical.file_kinds.dedup();
    canonical
        .media_relevance
        .sort_by_key(|value| value.storage_value());
    canonical.media_relevance.dedup();
    canonical
        .presence_states
        .sort_by_key(|value| value.storage_value());
    canonical.presence_states.dedup();
    canonical
        .source_access_states
        .sort_by_key(|value| value.storage_value());
    canonical.source_access_states.dedup();
    canonical
        .attachment_link_states
        .sort_by_key(|value| value.storage_value());
    canonical.attachment_link_states.dedup();
    canonical
}

fn normalize_text_query(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| query.to_lowercase())
}

fn append_scope_predicate(
    predicates: &mut Vec<String>,
    values: &mut Vec<Value>,
    scope: &StoreSearchScope,
    recursion: StoreSearchRecursion,
) -> LibrarySqliteResult<()> {
    match scope {
        StoreSearchScope::Library => {}
        StoreSearchScope::Source { source_id } => {
            predicates.push("source_id = ?".to_string());
            values.push(Value::Integer(*source_id));
        }
        StoreSearchScope::SourceLocation { source_location_id } => {
            if recursion == StoreSearchRecursion::Recursive {
                predicates.push(
                    "(source_location_id = ? OR EXISTS (
                        SELECT 1 FROM source_locations location
                        WHERE location.source_location_id = ?
                          AND search_rows.source_id = location.source_id
                          AND (
                              search_rows.relative_path = location.relative_path
                              OR search_rows.relative_path LIKE location.relative_path || '/%'
                          )
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_location_id));
                values.push(Value::Integer(*source_location_id));
            } else {
                predicates.push(
                    "(source_location_id = ? OR parent_source_directory_id = (
                        SELECT directory.source_directory_id
                        FROM source_locations location
                        JOIN source_directories directory
                          ON directory.source_id = location.source_id
                         AND directory.relative_path = location.relative_path
                        WHERE location.source_location_id = ?
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_location_id));
                values.push(Value::Integer(*source_location_id));
            }
        }
        StoreSearchScope::Directory {
            source_id,
            source_directory_id,
        } => {
            if recursion == StoreSearchRecursion::Recursive {
                predicates.push(
                    "(source_id = ? AND (
                        source_directory_id = ?
                        OR EXISTS (
                            SELECT 1 FROM source_directories scoped
                            WHERE scoped.source_directory_id = ?
                              AND search_rows.relative_path IS NOT NULL
                              AND (
                                  search_rows.relative_path = scoped.relative_path
                                  OR search_rows.relative_path LIKE scoped.relative_path || '/%'
                              )
                        )
                    ))"
                    .to_string(),
                );
                values.push(Value::Integer(*source_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_directory_id));
            } else {
                predicates.push(
                    "(source_id = ? AND (source_directory_id = ? OR parent_source_directory_id = ?))"
                        .to_string(),
                );
                values.push(Value::Integer(*source_id));
                values.push(Value::Integer(*source_directory_id));
                values.push(Value::Integer(*source_directory_id));
            }
        }
    }
    Ok(())
}

fn append_filters(
    predicates: &mut Vec<String>,
    values: &mut Vec<Value>,
    filters: &StoreSearchFilters,
) {
    push_in_values(
        predicates,
        values,
        "file_class",
        filters
            .file_classes
            .iter()
            .map(|value| value.storage_value()),
    );
    push_in_values(
        predicates,
        values,
        "file_kind",
        filters.file_kinds.iter().map(|value| value.storage_value()),
    );
    push_in_values(
        predicates,
        values,
        "media_relevance",
        filters
            .media_relevance
            .iter()
            .map(|value| value.storage_value()),
    );
    push_in_values(
        predicates,
        values,
        "presence_state",
        filters
            .presence_states
            .iter()
            .map(|value| value.storage_value()),
    );
    push_in_values(
        predicates,
        values,
        "source_access_state",
        filters
            .source_access_states
            .iter()
            .map(|value| value.storage_value()),
    );
    match filters.blake3 {
        Some(StoreSearchEvidenceAvailability::HasCurrent) => {
            predicates.push("has_current_blake3 = 1".to_string())
        }
        Some(StoreSearchEvidenceAvailability::MissingCurrent) => {
            predicates.push("(result_kind = 'source_file' AND has_current_blake3 = 0)".to_string())
        }
        None => {}
    }
    match filters.probe {
        Some(StoreSearchEvidenceAvailability::HasCurrent) => {
            predicates.push("has_current_probe = 1".to_string())
        }
        Some(StoreSearchEvidenceAvailability::MissingCurrent) => {
            predicates.push("(result_kind = 'source_file' AND has_current_probe = 0)".to_string())
        }
        None => {}
    }
    push_in_values(
        predicates,
        values,
        "attachment_link_state",
        filters
            .attachment_link_states
            .iter()
            .map(|value| value.storage_value()),
    );
}

fn append_text_predicate(
    predicates: &mut Vec<String>,
    values: &mut Vec<Value>,
    text_query: Option<&str>,
) {
    let Some(text_query) = text_query else {
        return;
    };
    let like = format!("%{}%", escape_like(text_query));
    let fts_query = fts_query_for_text(text_query);
    if let Some(fts_query) = fts_query {
        predicates.push(
            "(row_id IN (
                SELECT rowid FROM search_filter_index_fts WHERE search_filter_index_fts MATCH ?
             )
             OR lower(display_label) LIKE ? ESCAPE '\\'
             OR lower(COALESCE(display_path, '')) LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        values.push(Value::Text(fts_query));
        values.push(Value::Text(like.clone()));
        values.push(Value::Text(like));
    } else {
        predicates.push(
            "(lower(display_label) LIKE ? ESCAPE '\\'
              OR lower(COALESCE(display_path, '')) LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        values.push(Value::Text(like.clone()));
        values.push(Value::Text(like));
    }
}

fn push_in_values<'a>(
    predicates: &mut Vec<String>,
    values: &mut Vec<Value>,
    column_name: &str,
    items: impl Iterator<Item = &'a str>,
) {
    let items = items.collect::<Vec<_>>();
    if items.is_empty() {
        return;
    }
    let placeholders = std::iter::repeat_n("?", items.len())
        .collect::<Vec<_>>()
        .join(", ");
    predicates.push(format!("{column_name} IN ({placeholders})"));
    values.extend(items.into_iter().map(|item| Value::Text(item.to_string())));
}

fn fts_query_for_text(text: &str) -> Option<String> {
    let terms = text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("{}*", term.to_lowercase()))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn map_search_row(
    row: &Row<'_>,
    text_query: Option<&str>,
) -> rusqlite::Result<StoreSearchResultRow> {
    let result_kind_value = row.get::<_, String>(0)?;
    let display_label = row.get::<_, String>(9)?;
    let display_path = row.get::<_, Option<String>>(10)?;
    let result_kind =
        StoreSearchResultKind::from_storage_value(&result_kind_value).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
    Ok(StoreSearchResultRow {
        result_kind,
        authority_layer: row.get(1)?,
        stable_key: row.get(2)?,
        source_id: row.get(3)?,
        source_location_id: row.get(4)?,
        source_directory_id: row.get(5)?,
        parent_source_directory_id: row.get(6)?,
        source_file_id: row.get(7)?,
        display_label: display_label.clone(),
        display_path: display_path.clone(),
        relative_path: row.get(11)?,
        file_class: row.get(12)?,
        file_kind: row.get(13)?,
        media_relevance: row.get(14)?,
        presence_state: row.get(15)?,
        source_access_state: row.get(16)?,
        source_scan_phase: row.get(17)?,
        has_current_blake3: row.get::<_, i64>(18)? != 0,
        has_current_probe: row.get::<_, i64>(19)? != 0,
        attachment_link_state: row.get(20)?,
        attachment_id: row.get(21)?,
        content_hash_algorithm: row.get(22)?,
        content_hash_value: row.get(23)?,
        evidence_coverage_state: row.get(24)?,
        match_reason: match_reason(text_query, &display_label, display_path.as_deref()),
        updated_at: row.get(25)?,
        sort_key: row.get(26)?,
    })
}

fn match_reason(text_query: Option<&str>, label: &str, path: Option<&str>) -> String {
    let Some(query) = text_query else {
        return "filter".to_string();
    };
    let label_lower = label.to_lowercase();
    if label_lower.contains(query) {
        return "label".to_string();
    }
    if path
        .map(|path| path.to_lowercase().contains(query))
        .unwrap_or(false)
    {
        return "path".to_string();
    }
    "text".to_string()
}

fn build_next_cursor(
    identity: &SearchCursorIdentity,
    row: &StoreSearchResultRow,
) -> LibrarySqliteResult<String> {
    let cursor = SearchCursor {
        version: SEARCH_CURSOR_VERSION,
        identity: identity.clone(),
        position: SearchCursorPosition {
            sort_key: row.sort_key.clone(),
            result_kind_order: result_kind_order(row.result_kind),
            stable_key: row.stable_key.clone(),
        },
    };
    encode_cursor(&cursor)
}

fn cursor_invalid(
    identity: SearchCursorIdentity,
    index_generation: i64,
    index_state: String,
) -> LibrarySqliteResult<StoreSearchResult> {
    Ok(StoreSearchResult {
        state: StoreSearchState::CursorInvalid,
        query_identity: public_identity(&identity),
        index_generation,
        index_state,
        rows: Vec::new(),
        next_cursor: None,
        detail: Some("The search/filter cursor does not match the current request.".to_string()),
    })
}

fn encode_cursor(cursor: &SearchCursor) -> LibrarySqliteResult<String> {
    let json = serde_json::to_string(cursor).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!("search cursor encode failed: {error}"))
    })?;
    Ok(base64url_encode(json.as_bytes()))
}

fn decode_cursor(cursor: &str) -> LibrarySqliteResult<SearchCursor> {
    let bytes = base64url_decode(cursor).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!("search cursor decode failed: {error}"))
    })?;
    let json = String::from_utf8(bytes).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!("search cursor utf8 failed: {error}"))
    })?;
    serde_json::from_str(&json).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!("search cursor parse failed: {error}"))
    })
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
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &b in input.as_bytes() {
        let v = TABLE[b as usize];
        if v == 255 {
            return Err(format!("invalid base64url character: {}", b as char));
        }
        buf = (buf << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn result_kind_order(kind: StoreSearchResultKind) -> i64 {
    match kind {
        StoreSearchResultKind::Source => 0,
        StoreSearchResultKind::SourceLocation => 1,
        StoreSearchResultKind::Directory => 2,
        StoreSearchResultKind::SourceFile => 3,
    }
}

fn read_usize(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

const SEARCH_SELECT_SQL: &str = "SELECT result_kind,
       authority_layer,
       stable_key,
       source_id,
       source_location_id,
       source_directory_id,
       parent_source_directory_id,
       source_file_id,
       row_id,
       display_label,
       display_path,
       relative_path,
       file_class,
       file_kind,
       media_relevance,
       presence_state,
       source_access_state,
       source_scan_phase,
       has_current_blake3,
       has_current_probe,
       attachment_link_state,
       attachment_id,
       content_hash_algorithm,
       content_hash_value,
       evidence_coverage_state,
       updated_at,
       sort_key,
       CASE result_kind
           WHEN 'source' THEN 0
           WHEN 'source_location' THEN 1
           WHEN 'directory' THEN 2
           ELSE 3
       END AS result_kind_order
FROM search_filter_index_rows search_rows";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::install_baseline_schema_for_test;

    fn open_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory");
        install_baseline_schema_for_test(&mut connection).expect("install schema");
        connection
    }

    fn seed_source(connection: &Connection) {
        connection
            .execute_batch(
                "INSERT INTO sources (
                     source_id, source_class, authority, identity_key, display_name,
                     is_user_visible, created_at, updated_at
                 ) VALUES (1, 'internal', 'system', 'source:search-test', 'Search Test', 1, 1, 1);
                 INSERT INTO source_state (
                     source_id, mount_status, mount_epoch, access_state, updated_at
                 ) VALUES (1, 'mounted', 1, 'accessible', 1);
                 INSERT INTO source_scan_state (
                     source_id, scan_phase, updated_at
                 ) VALUES (1, 'complete', 1);
                 INSERT INTO source_directories (
                     source_directory_id, source_id, parent_source_directory_id, name,
                     name_browse_sort_key, relative_path, presence_state, dir_scan_state,
                     dir_scan_updated_at, created_at, updated_at
                 ) VALUES
                     (10, 1, NULL, 'Music', 'music', 'Music', 'present', 'complete', 1, 1, 1),
                     (11, 1, 10, 'Breaks', 'breaks', 'Music/Breaks', 'present', 'complete', 1, 1, 1);
                 INSERT INTO source_locations (
                     source_location_id, source_id, authority, location_kind, relative_path,
                     display_name, is_user_visible, created_at, updated_at
                 ) VALUES (100, 1, 'user', 'registered_subpath', 'Music/Breaks', 'Breaks', 1, 1, 1);
                 INSERT INTO source_files (
                     source_file_id, source_id, parent_source_directory_id, name,
                     name_browse_sort_key, relative_path_browse_sort_key, relative_path,
                     size_bytes, mtime_ns, file_kind, file_class, presence_state,
                     first_discovered_at, last_observed_at, last_presence_change_at,
                     created_at, updated_at
                 ) VALUES
                     (1000, 1, 11, 'Amen.wav', 'amen', 'music/breaks/amen', 'Music/Breaks/Amen.wav', 10, 100, 'audio', 'audio', 'present', 1, 1, 1, 1, 1),
                     (1001, 1, 11, 'Cover.jpg', 'cover', 'music/breaks/cover', 'Music/Breaks/Cover.jpg', 5, 100, 'image', 'image', 'present', 1, 1, 1, 1, 1),
                     (1002, 1, 10, 'Notes.txt', 'notes', 'music/notes', 'Music/Notes.txt', 3, 100, 'text_doc', 'none', 'present', 1, 1, 1, 1, 1);
                 INSERT INTO WorkItems (
                     work_item_id, subject_kind, subject_id, work_kind, priority_class,
                     basis_fingerprint, state, created_at, updated_at
                 ) VALUES (1, 'source_file', '1000', 'inspect_source', 'interactive', 'basis:1000', 'completed', 1, 1);
                 INSERT INTO WorkRuns (
                     work_run_id, work_item_id, adapter_key, adapter_version,
                     started_at, finished_at, outcome
                 ) VALUES (1, 1, 'test', '1', 1, 1, 'completed');
                 INSERT INTO Artifacts (
                     artifact_id, work_run_id, subject_kind, subject_id, artifact_kind,
                     artifact_role, adapter_key, adapter_version, basis_fingerprint,
                     media_type, storage_kind, payload_hash, created_at
                 ) VALUES (1, 1, 'source_file', '1000', 'inspection_result', 'primary_result', 'test', '1', 'basis:1000', 'application/json', 'inline_payload', 'hash', 1);
                 INSERT INTO SourceFacts (
                     source_file_id, fact_kind, basis_fingerprint, basis_source_id,
                     basis_relative_path, basis_size_bytes, basis_mtime_ns,
                     basis_presence_state, observed_at_ms, content_hash_algorithm,
                     content_hash_value, media_kind, mime_type, duration_ms,
                     sample_rate_hz, channels, bit_depth, codec, updated_at,
                     accepted_artifact_id
                 ) VALUES (1000, 'source_inspection', 'basis:1000', 1, 'Music/Breaks/Amen.wav', 10, 100, 'present', 1, 'blake3', 'abc', 'audio', 'audio/wav', 1000, 44100, 2, 16, 'pcm', 1, 1);
                 INSERT INTO content_attachments (
                     attachment_id, content_hash_algorithm, content_hash_value,
                     first_observed_at, updated_at
                 ) VALUES (50, 'blake3', 'abc', 1, 1);
                 INSERT INTO source_file_attachment_links (
                     source_file_attachment_link_id, attachment_id, source_file_id,
                     source_id, file_kind, created_at, updated_at
                 ) VALUES (60, 50, 1000, 1, 'audio', 1, 1);",
            )
            .expect("seed source");
    }

    fn request(query: Option<&str>) -> StoreSearchRequest {
        StoreSearchRequest {
            scope: StoreSearchScope::Library,
            recursion: StoreSearchRecursion::Recursive,
            text_query: query.map(str::to_string),
            target_kinds: Vec::new(),
            filters: StoreSearchFilters::default(),
            sort: StoreSearchSort::PathName,
            limit: 50,
            cursor: None,
        }
    }

    #[test]
    fn rebuild_indexes_source_rows_and_searches_path_names() {
        let connection = open_connection();
        seed_source(&connection);
        let rebuild = rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");
        assert_eq!(rebuild.rows_indexed, 7);

        let result = read_search_filter(&connection, request(Some("amen"))).expect("search");
        assert_eq!(result.state, StoreSearchState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0].result_kind,
            StoreSearchResultKind::SourceFile
        );
        assert_eq!(result.rows[0].display_label, "Amen.wav");
        assert!(result.rows[0].has_current_blake3);
        assert!(result.rows[0].has_current_probe);
        assert_eq!(result.rows[0].attachment_link_state, "current");
    }

    #[test]
    fn scoped_search_and_file_class_filter_use_index_rows() {
        let connection = open_connection();
        seed_source(&connection);
        rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

        let mut scoped = request(None);
        scoped.scope = StoreSearchScope::SourceLocation {
            source_location_id: 100,
        };
        scoped.filters.file_classes = vec![StoreSearchFileClass::Image];
        let result = read_search_filter(&connection, scoped).expect("search");
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].display_label, "Cover.jpg");
    }

    #[test]
    fn deterministic_pagination_and_cursor_identity_are_enforced() {
        let connection = open_connection();
        seed_source(&connection);
        let rebuild = rebuild_search_filter_index_for_source(&connection, 1, 10).expect("rebuild");

        let mut first = request(None);
        first.limit = 2;
        let page1 = read_search_filter(&connection, first.clone()).expect("page1");
        assert_eq!(page1.rows.len(), 2);
        let cursor = page1.next_cursor.expect("cursor");

        first.cursor = Some(cursor.clone());
        let page2 = read_search_filter(&connection, first.clone()).expect("page2");
        assert!(!page2.rows.is_empty());
        assert_ne!(page1.rows[0].stable_key, page2.rows[0].stable_key);

        let mut changed = first;
        changed.text_query = Some("amen".to_string());
        let invalid = read_search_filter(&connection, changed).expect("invalid cursor");
        assert_eq!(invalid.state, StoreSearchState::CursorInvalid);

        rebuild_search_filter_index_for_source(&connection, 1, 20).expect("rebuild again");
        let mut stale_generation = request(None);
        stale_generation.limit = 2;
        stale_generation.cursor = Some(cursor);
        let invalid_generation =
            read_search_filter(&connection, stale_generation).expect("invalid generation");
        assert_eq!(invalid_generation.state, StoreSearchState::CursorInvalid);
        assert!(invalid_generation.index_generation > rebuild.generation);
    }
}
