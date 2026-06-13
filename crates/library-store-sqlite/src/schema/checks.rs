use rusqlite::Connection;

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::introspection::{
    self, SchemaObjectKind, read_auto_vacuum_mode, read_normalized_schema_sql,
    run_foreign_key_check,
};
use super::seeds::validate_seed_rows;

pub(crate) fn run_non_structural_checks(connection: &Connection) -> LibrarySqliteResult<()> {
    validate_incremental_auto_vacuum(connection)?;
    validate_seed_rows(connection)?;
    validate_residual_semantic_checks(connection)?;
    foreign_key_integrity_check(connection)
}

pub(crate) fn validate_incremental_auto_vacuum(connection: &Connection) -> LibrarySqliteResult<()> {
    let auto_vacuum_mode = read_auto_vacuum_mode(connection)?;
    if auto_vacuum_mode == 2 {
        Ok(())
    } else {
        Err(LibrarySqliteError::MalformedSchemaState(format!(
            "baseline schema must install PRAGMA auto_vacuum = INCREMENTAL, found mode {auto_vacuum_mode}"
        )))
    }
}

pub(crate) fn foreign_key_integrity_check(connection: &Connection) -> LibrarySqliteResult<()> {
    if let Some(violation) = run_foreign_key_check(connection)? {
        Err(LibrarySqliteError::MalformedSchemaState(format!(
            "foreign_key_check failed for table={} rowid={} parent={} fk_index={}",
            violation.table_name,
            violation.row_id,
            violation.parent_table,
            violation.foreign_key_index
        )))
    } else {
        Ok(())
    }
}

fn validate_residual_semantic_checks(connection: &Connection) -> LibrarySqliteResult<()> {
    validate_source_locator_constraints(connection)?;
    validate_source_state_constraints(connection)?;
    validate_source_scan_state_constraints(connection)?;
    validate_source_directories_constraints(connection)?;
    validate_source_location_constraints(connection)?;
    validate_source_navigation_user_order_constraints(connection)?;
    validate_navigation_rows_constraints(connection)?;
    validate_work_item_constraints(connection)?;
    validate_artifact_constraints(connection)?;
    validate_search_filter_index_fts_configuration(connection)?;
    validate_projection_subscribers_constraints(connection)?;
    Ok(())
}

fn validate_source_locator_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_locators")?;
    for fragment in [
        "locator_kind IN ('absolute_path', 'removable_volume')",
        "locator_kind = 'absolute_path'",
        "absolute_path IS NOT NULL",
        "device_identity_kind IS NULL",
        "device_identity_value IS NULL",
        "relative_suffix = ''",
        "locator_kind = 'removable_volume'",
        "absolute_path IS NULL",
        "device_identity_kind IS NOT NULL",
        "device_identity_value IS NOT NULL",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("source_locators must retain locator-shape constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_source_state_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_state")?;
    for fragment in [
        "mount_status IN ('unknown', 'mounted', 'unmounted', 'eject_requested', 'eject_pending')",
        "mount_epoch >= 0",
        "access_state IN ('accessible', 'missing', 'blocked', 'unknown')",
        "access_issue_kind IN ( 'missing', 'not_directory', 'permission_denied', 'privacy_permission_required', 'unavailable_mount', 'resource_busy', 'stale_network_handle', 'symlink_loop', 'symlink_escape_blocked', 'unsupported_path', 'invalid_path', 'io_interrupted', 'timed_out', 'unknown_io' )",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("source_state must retain source-state constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_source_scan_state_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_scan_state")?;
    for fragment in [
        "scan_phase IN ('idle', 'scanning', 'complete', 'partial', 'blocked', 'failed')",
        "last_scan_finished_at >= last_scan_started_at",
        "scan_phase NOT IN ('blocked', 'partial') OR scan_issue_kind IS NOT NULL",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("source_scan_state must retain scan-state constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_source_directories_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_directories")?;
    for fragment in [
        "presence_state IN ('present', 'missing', 'removed')",
        "dir_scan_state IN ('pending', 'scanning', 'complete', 'failed', 'blocked')",
        "dir_scan_state NOT IN ('blocked', 'failed') OR dir_scan_issue_kind IS NOT NULL",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("source_directories must retain directory constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_source_location_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_locations")?;
    for fragment in [
        "authority IN ('device', 'user')",
        "location_kind IN ('observed_path', 'registered_subpath')",
        "relative_path NOT LIKE '/%'",
        "relative_path NOT GLOB '[A-Za-z]:*'",
        "relative_path NOT LIKE '%://%'",
        "relative_path NOT LIKE '%\\\\%'",
        "relative_path NOT LIKE '%//%'",
        "relative_path NOT LIKE '%/'",
        "relative_path NOT IN ('.', '..')",
        "relative_path NOT LIKE './%'",
        "relative_path NOT LIKE '../%'",
        "relative_path NOT LIKE '%/./%'",
        "relative_path NOT LIKE '%/../%'",
        "relative_path NOT LIKE '%/.'",
        "relative_path NOT LIKE '%/..'",
        "location_kind = 'observed_path' AND authority = 'device'",
        "location_kind = 'registered_subpath' AND authority = 'user'",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "source_locations must retain source-location constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_source_navigation_user_order_constraints(
    connection: &Connection,
) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "source_navigation_user_order")?;
    for fragment in [
        "item_kind IN ('source', 'source_location')",
        "item_kind = 'source' AND parent_source_key IS NULL",
        "item_kind = 'source_location'",
        "parent_source_key IS NOT NULL",
        "length(trim(parent_source_key)) > 0",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "source_navigation_user_order must retain ordering-shape constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_navigation_rows_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "navigation_rows")?;
    for fragment in [
        "family IN ('views', 'sources')",
        "parent_navigation_row_id IS NULL AND family IS NOT NULL",
        "parent_navigation_row_id IS NOT NULL AND family IS NULL",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "navigation_rows must retain substrate family constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_work_item_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "work_items")?;
    for fragment in [
        "subject_kind IN ('source_file', 'projection_domain')",
        "work_kind IN ( 'inspect_source_file', 'rebuild_projection' )",
        "priority_class IN ('urgent', 'interactive', 'background')",
        "state IN ('queued', 'leased', 'completed', 'blocked', 'failed', 'canceled')",
        "state = 'leased' AND leased_until IS NOT NULL",
        "subject_kind <> 'projection_domain' OR subject_id = 'navigation'",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("work_items must retain work-model constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_artifact_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "work_artifacts")?;
    for fragment in [
        "subject_kind IN ('source_file', 'projection_domain')",
        "artifact_kind IN ( 'inspection_result', 'projection_snapshot' )",
        "storage_kind IN ('inline_payload', 'file_store')",
        "artifact_kind <> 'inspection_result' OR subject_kind = 'source_file'",
        "subject_kind = 'source_file'",
        "artifact_kind <> 'projection_snapshot'",
        "subject_kind = 'projection_domain'",
        "subject_kind <> 'projection_domain' OR subject_id = 'navigation'",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("work_artifacts must retain artifact constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_projection_subscribers_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "projection_subscribers")?;
    require_sql_fragment(
        &sql,
        "expires_at >= last_seen_at",
        "projection_subscribers must retain the liveness ordering constraint".to_string(),
    )
}

fn validate_search_filter_index_fts_configuration(
    connection: &Connection,
) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "search_filter_index_fts")?;
    for fragment in [
        "USING fts5",
        "display_label",
        "display_path",
        "unicode61 remove_diacritics 1",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "search_filter_index_fts must retain searchable search/filter configuration fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn read_required_normalized_table_sql(
    connection: &Connection,
    table_name: &str,
) -> LibrarySqliteResult<String> {
    read_normalized_schema_sql(connection, SchemaObjectKind::Table, table_name)?.ok_or_else(|| {
        introspection::missing_schema_object_message(SchemaObjectKind::Table, table_name)
    })
}

fn require_sql_fragment(
    normalized_sql: &str,
    fragment: &str,
    message: String,
) -> LibrarySqliteResult<()> {
    let normalized_fragment = fragment.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized_sql.contains(&normalized_fragment) {
        Ok(())
    } else {
        Err(LibrarySqliteError::MalformedSchemaState(message))
    }
}
