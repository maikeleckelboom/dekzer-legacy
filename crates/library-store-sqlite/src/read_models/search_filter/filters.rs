use rusqlite::types::Value;

use super::types::{StoreSearchEvidenceAvailability, StoreSearchFilters};

pub(crate) fn canonical_filters(filters: &StoreSearchFilters) -> StoreSearchFilters {
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

pub(crate) fn append_filters(
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

pub(crate) fn push_in_values<'a>(
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
