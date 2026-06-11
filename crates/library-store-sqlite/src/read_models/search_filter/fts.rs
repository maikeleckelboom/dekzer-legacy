use rusqlite::types::Value;

pub(crate) fn append_text_predicate(
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

pub(crate) fn fts_query_for_text(text: &str) -> Option<String> {
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

pub(crate) fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
