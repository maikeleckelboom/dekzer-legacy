use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};

use super::coverage::{read_index_metadata, resolve_scope_coverage};
use super::cursor::{
    build_next_cursor, canonical_identity, cursor_invalid, decode_cursor, public_identity,
    validate_cursor,
};
use super::filters::{append_filters, push_in_values};
use super::fts::append_text_predicate;
use super::row::{cursor_predicate, map_search_row, order_by_sql, search_select_sql};
use super::scope::append_scope_predicate;
use super::types::{
    StoreSearchIndexState, StoreSearchRequest, StoreSearchResult, StoreSearchSort, StoreSearchState,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

pub(crate) fn read_search_filter(
    connection: &Connection,
    request: StoreSearchRequest,
) -> LibrarySqliteResult<StoreSearchResult> {
    let metadata = read_index_metadata(connection)?;
    let coverage = resolve_scope_coverage(connection, &metadata, &request.scope)?;
    let identity = canonical_identity(&request, metadata.generation);
    if identity.target_kinds.is_empty() {
        return Ok(StoreSearchResult {
            state: StoreSearchState::Unsupported,
            query_identity: public_identity(&identity),
            index_generation: metadata.generation,
            index_state: coverage.index_state,
            rows: Vec::new(),
            next_cursor: None,
            detail: Some("Search/filter requires at least one target kind.".to_string()),
        });
    }

    let cursor_position = if let Some(cursor) = request.cursor.as_deref() {
        let decoded = match decode_cursor(cursor) {
            Ok(decoded) => decoded,
            Err(_) => {
                return cursor_invalid(identity, metadata.generation, coverage.index_state);
            }
        };
        match validate_cursor(decoded, &identity) {
            Some(position) => Some(position),
            None => return cursor_invalid(identity, metadata.generation, coverage.index_state),
        }
    } else {
        None
    };

    let mut values = Vec::<Value>::new();
    let mut sql = search_select_sql(identity.sort, identity.text_query.as_deref(), &mut values);
    let mut predicates = Vec::<String>::new();

    append_scope_predicate(
        &mut predicates,
        &mut values,
        &identity.scope,
        identity.recursion,
    )?;
    predicates.push(
        "EXISTS (
             SELECT 1
             FROM sources visible_source
             WHERE visible_source.source_id = search_rows.source_id
               AND visible_source.is_user_visible = 1
         )"
        .to_string(),
    );
    predicates.push(
        "(source_location_id IS NULL OR EXISTS (
             SELECT 1
             FROM source_locations visible_location
             WHERE visible_location.source_location_id = search_rows.source_location_id
               AND visible_location.is_user_visible = 1
         ))"
        .to_string(),
    );
    push_in_values(
        &mut predicates,
        &mut values,
        "result_kind",
        identity.target_kinds.iter().map(String::as_str),
    );
    append_filters(&mut predicates, &mut values, &identity.filters);
    append_text_predicate(&mut predicates, &mut values, identity.text_query.as_deref());
    if let Some(position) = &cursor_position {
        predicates.push(cursor_predicate(identity.sort).to_string());
        match identity.sort {
            StoreSearchSort::PathName => {
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Integer(position.result_kind_order));
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Integer(position.result_kind_order));
                values.push(Value::Text(position.stable_key.clone()));
            }
            StoreSearchSort::Relevance => {
                values.push(Value::Integer(position.relevance_rank));
                values.push(Value::Integer(position.relevance_rank));
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Integer(position.relevance_rank));
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Integer(position.result_kind_order));
                values.push(Value::Integer(position.relevance_rank));
                values.push(Value::Text(position.sort_key.clone()));
                values.push(Value::Integer(position.result_kind_order));
                values.push(Value::Text(position.stable_key.clone()));
            }
        }
    }

    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    sql.push_str(order_by_sql(identity.sort));
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
    let state = search_state_for_rows(rows.is_empty(), coverage.fully_ready, coverage.index_state);
    let detail = if state == StoreSearchState::Partial {
        coverage.detail
    } else {
        None
    };

    Ok(StoreSearchResult {
        state,
        query_identity: public_identity(&identity),
        index_generation: coverage.index_generation,
        index_state: coverage.index_state,
        rows,
        next_cursor,
        detail,
    })
}

fn search_state_for_rows(
    rows_empty: bool,
    coverage_ready: bool,
    index_state: StoreSearchIndexState,
) -> StoreSearchState {
    if !coverage_ready || index_state != StoreSearchIndexState::Ready {
        StoreSearchState::Partial
    } else if rows_empty {
        StoreSearchState::Empty
    } else {
        StoreSearchState::Ready
    }
}
