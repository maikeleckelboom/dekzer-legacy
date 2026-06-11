use rusqlite::Row;
use rusqlite::types::Value;

use super::fts::escape_like;
use super::types::{
    StoreSearchAttachmentLinkState, StoreSearchAuthorityLayer, StoreSearchEvidenceCoverageState,
    StoreSearchMatchReason, StoreSearchResultKind, StoreSearchResultRow, StoreSearchSort,
};

pub(crate) fn search_select_sql(
    sort: StoreSearchSort,
    text_query: Option<&str>,
    values: &mut Vec<Value>,
) -> String {
    let relevance_expr = relevance_rank_sql(sort, text_query, values);
    format!(
        "SELECT result_kind,
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
       {relevance_expr},
       CASE result_kind
           WHEN 'source' THEN 0
           WHEN 'source_location' THEN 1
           WHEN 'directory' THEN 2
           ELSE 3
       END AS result_kind_order
FROM search_filter_index_rows search_rows"
    )
}

pub(crate) fn cursor_predicate(sort: StoreSearchSort) -> &'static str {
    match sort {
        StoreSearchSort::PathName => {
            "(sort_key > ? OR (sort_key = ? AND result_kind_order > ?) OR (sort_key = ? AND result_kind_order = ? AND stable_key > ?))"
        }
        StoreSearchSort::Relevance => {
            "(relevance_rank > ? OR (relevance_rank = ? AND sort_key > ?) OR (relevance_rank = ? AND sort_key = ? AND result_kind_order > ?) OR (relevance_rank = ? AND sort_key = ? AND result_kind_order = ? AND stable_key > ?))"
        }
    }
}

pub(crate) fn order_by_sql(sort: StoreSearchSort) -> &'static str {
    match sort {
        StoreSearchSort::PathName => {
            " ORDER BY sort_key ASC, result_kind_order ASC, stable_key ASC"
        }
        StoreSearchSort::Relevance => {
            " ORDER BY relevance_rank ASC, sort_key ASC, result_kind_order ASC, stable_key ASC"
        }
    }
}

pub(crate) fn map_search_row(
    row: &Row<'_>,
    text_query: Option<&str>,
) -> rusqlite::Result<StoreSearchResultRow> {
    let result_kind_value = row.get::<_, String>(0)?;
    let authority_layer_value = row.get::<_, String>(1)?;
    let display_label = row.get::<_, String>(9)?;
    let display_path = row.get::<_, Option<String>>(10)?;
    let attachment_link_state_value = row.get::<_, String>(20)?;
    let evidence_coverage_state_value = row.get::<_, String>(24)?;
    let result_kind = map_storage_value(
        0,
        &result_kind_value,
        StoreSearchResultKind::from_storage_value,
    )?;
    let authority_layer = map_storage_value(
        1,
        &authority_layer_value,
        StoreSearchAuthorityLayer::from_storage_value,
    )?;
    let attachment_link_state = map_storage_value(
        20,
        &attachment_link_state_value,
        StoreSearchAttachmentLinkState::from_storage_value,
    )?;
    let evidence_coverage_state = map_storage_value(
        24,
        &evidence_coverage_state_value,
        StoreSearchEvidenceCoverageState::from_storage_value,
    )?;

    Ok(StoreSearchResultRow {
        result_kind,
        authority_layer,
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
        attachment_link_state,
        attachment_id: row.get(21)?,
        content_hash_algorithm: row.get(22)?,
        content_hash_value: row.get(23)?,
        evidence_coverage_state,
        match_reason: match_reason(text_query, &display_label, display_path.as_deref()),
        updated_at: row.get(25)?,
        sort_key: row.get(26)?,
        relevance_rank: row.get(27)?,
    })
}

fn relevance_rank_sql(
    sort: StoreSearchSort,
    text_query: Option<&str>,
    values: &mut Vec<Value>,
) -> &'static str {
    if sort != StoreSearchSort::Relevance {
        return "0 AS relevance_rank";
    }
    let Some(text_query) = text_query else {
        return "0 AS relevance_rank";
    };

    let prefix = format!("{}%", escape_like(text_query));
    let contains = format!("%{}%", escape_like(text_query));
    values.push(Value::Text(text_query.to_string()));
    values.push(Value::Text(prefix));
    values.push(Value::Text(contains.clone()));
    values.push(Value::Text(contains));
    "CASE
           WHEN lower(display_label) = ? THEN 0
           WHEN lower(display_label) LIKE ? ESCAPE '\\' THEN 1
           WHEN lower(display_label) LIKE ? ESCAPE '\\' THEN 2
           WHEN lower(COALESCE(display_path, '')) LIKE ? ESCAPE '\\' THEN 3
           ELSE 4
       END AS relevance_rank"
}

fn match_reason(
    text_query: Option<&str>,
    label: &str,
    path: Option<&str>,
) -> StoreSearchMatchReason {
    let Some(query) = text_query else {
        return StoreSearchMatchReason::Filter;
    };
    let label_lower = label.to_lowercase();
    if label_lower == query {
        return StoreSearchMatchReason::ExactLabel;
    }
    if label_lower.starts_with(query) {
        return StoreSearchMatchReason::LabelPrefix;
    }
    if label_lower.contains(query) {
        return StoreSearchMatchReason::Label;
    }
    if path
        .map(|path| path.to_lowercase().contains(query))
        .unwrap_or(false)
    {
        return StoreSearchMatchReason::Path;
    }
    StoreSearchMatchReason::Text
}

fn map_storage_value<T>(
    index: usize,
    value: &str,
    map: impl FnOnce(&str) -> crate::LibrarySqliteResult<T>,
) -> rusqlite::Result<T> {
    map(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
