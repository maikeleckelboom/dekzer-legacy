use super::SEARCH_CURSOR_VERSION;
use super::filters::canonical_filters;
use super::types::{
    StoreSearchFilters, StoreSearchIndexState, StoreSearchQueryIdentity, StoreSearchRecursion,
    StoreSearchRequest, StoreSearchResult, StoreSearchResultKind, StoreSearchResultRow,
    StoreSearchScope, StoreSearchSort, StoreSearchState,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SearchCursor {
    version: u8,
    identity: SearchCursorIdentity,
    position: SearchCursorPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SearchCursorIdentity {
    pub(crate) scope: StoreSearchScope,
    pub(crate) recursion: StoreSearchRecursion,
    pub(crate) text_query: Option<String>,
    pub(crate) target_kinds: Vec<String>,
    pub(crate) filters: StoreSearchFilters,
    pub(crate) sort: StoreSearchSort,
    pub(crate) page_size: usize,
    pub(crate) index_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SearchCursorPosition {
    pub(crate) relevance_rank: i64,
    pub(crate) sort_key: String,
    pub(crate) result_kind_order: i64,
    pub(crate) stable_key: String,
}

pub(crate) fn decode_cursor(cursor: &str) -> LibrarySqliteResult<SearchCursor> {
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

pub(crate) fn validate_cursor(
    cursor: SearchCursor,
    identity: &SearchCursorIdentity,
) -> Option<SearchCursorPosition> {
    if cursor.version == SEARCH_CURSOR_VERSION && &cursor.identity == identity {
        Some(cursor.position)
    } else {
        None
    }
}

pub(crate) fn canonical_identity(
    request: &StoreSearchRequest,
    index_generation: i64,
) -> SearchCursorIdentity {
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

pub(crate) fn public_identity(identity: &SearchCursorIdentity) -> StoreSearchQueryIdentity {
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

pub(crate) fn build_next_cursor(
    identity: &SearchCursorIdentity,
    row: &StoreSearchResultRow,
) -> LibrarySqliteResult<String> {
    let cursor = SearchCursor {
        version: SEARCH_CURSOR_VERSION,
        identity: identity.clone(),
        position: SearchCursorPosition {
            relevance_rank: row.relevance_rank,
            sort_key: row.sort_key.clone(),
            result_kind_order: result_kind_order(row.result_kind),
            stable_key: row.stable_key.clone(),
        },
    };
    encode_cursor(&cursor)
}

pub(crate) fn cursor_invalid(
    identity: SearchCursorIdentity,
    index_generation: i64,
    index_state: StoreSearchIndexState,
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

pub(crate) fn result_kind_order(kind: StoreSearchResultKind) -> i64 {
    match kind {
        StoreSearchResultKind::Source => 0,
        StoreSearchResultKind::SourceLocation => 1,
        StoreSearchResultKind::Directory => 2,
        StoreSearchResultKind::SourceFile => 3,
    }
}

fn normalize_text_query(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| query.to_lowercase())
}

fn encode_cursor(cursor: &SearchCursor) -> LibrarySqliteResult<String> {
    let json = serde_json::to_string(cursor).map_err(|error| {
        LibrarySqliteError::MalformedSchemaState(format!("search cursor encode failed: {error}"))
    })?;
    Ok(base64url_encode(json.as_bytes()))
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
