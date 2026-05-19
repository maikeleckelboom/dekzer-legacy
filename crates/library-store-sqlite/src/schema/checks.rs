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
    validate_source_location_constraints(connection)?;
    validate_browser_user_order_constraints(connection)?;
    validate_navigation_rows_constraints(connection)?;
    validate_library_browser_rows_constraints(connection)?;
    validate_library_asset_metadata_corrections_constraints(connection)?;
    validate_work_item_constraints(connection)?;
    validate_artifact_constraints(connection)?;
    validate_library_browser_fts_configuration(connection)?;
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
        "resolution_status IN ('resolved', 'missing', 'inaccessible', 'unknown')",
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
        "scan_phase IN ('idle', 'scanning', 'blocked', 'failed')",
        "last_scan_finished_at >= last_scan_started_at",
        "scan_phase <> 'blocked' OR blocked_reason IS NOT NULL",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("source_scan_state must retain scan-state constraint fragment {fragment:?}"),
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

fn validate_browser_user_order_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "browser_user_order")?;
    for fragment in [
        "node_domain IN ('source', 'source_location')",
        "node_domain = 'source' AND parent_scope IS NULL",
        "node_domain = 'source_location'",
        "parent_scope IS NOT NULL",
        "length(trim(parent_scope)) > 0",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "browser_user_order must retain ordering-shape constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_navigation_rows_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "navigation_rows")?;
    for fragment in [
        "family IN ('Views', 'Collections', 'Preparation', 'Sources')",
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

fn validate_library_browser_rows_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "LibraryBrowserRows")?;
    for fragment in [
        "availability_state IN ('available', 'unavailable', 'degraded')",
        "prep_readiness_summary IN ( 'not_required', 'ready', 'preparing', 'underprepared', 'blocked', 'failed' )",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "LibraryBrowserRows must retain browse-summary constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_library_asset_metadata_corrections_constraints(
    connection: &Connection,
) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "LibraryAssetMetadataCorrections")?;
    for fragment in [
        "is_null_correction IN (0, 1)",
        "is_null_correction = 1",
        "value_text IS NULL",
        "value_int IS NULL",
        "is_null_correction = 0",
        "value_text IS NOT NULL AND value_int IS NULL",
        "value_text IS NULL AND value_int IS NOT NULL",
        "retracted_at IS NULL OR retracted_at >= applied_at",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "LibraryAssetMetadataCorrections must retain correction constraint fragment {fragment:?}"
            ),
        )?;
    }
    Ok(())
}

fn validate_work_item_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "WorkItems")?;
    for fragment in [
        "subject_kind IN ('source_file', 'library_asset', 'projection_domain')",
        "work_kind IN ( 'inspect_source', 'accept_segmentation', 'resolve_library_asset', 'compute_capability', 'rebind_source', 'rebuild_projection' )",
        "priority_class IN ('urgent', 'interactive', 'background')",
        "state IN ('queued', 'leased', 'completed', 'blocked', 'failed', 'canceled')",
        "work_kind = 'compute_capability'",
        "capability_kind IS NOT NULL",
        "target_profile_key IS NOT NULL",
        "target_quality IS NOT NULL",
        "work_kind <> 'compute_capability'",
        "capability_kind IS NULL",
        "target_profile_key IS NULL",
        "target_quality IS NULL",
        "state = 'leased' AND leased_until IS NOT NULL",
        "subject_kind <> 'projection_domain' OR subject_id IN ('library_browser', 'navigation')",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("WorkItems must retain work-model constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_artifact_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "Artifacts")?;
    for fragment in [
        "subject_kind IN ('source_file', 'library_asset', 'projection_domain')",
        "artifact_kind IN ( 'inspection_result', 'segmentation_result', 'capability_result', 'projection_snapshot', 'diagnostic_result' )",
        "artifact_role IN ( 'primary_result', 'preview_summary', 'manifest', 'diagnostic_payload', 'intermediate_output' )",
        "storage_kind IN ('inline_payload', 'file_store')",
        "artifact_kind <> 'capability_result'",
        "subject_kind = 'library_asset'",
        "artifact_kind NOT IN ('inspection_result', 'segmentation_result')",
        "subject_kind = 'source_file'",
        "artifact_kind <> 'projection_snapshot'",
        "subject_kind = 'projection_domain'",
        "subject_kind <> 'projection_domain' OR subject_id IN ('library_browser', 'navigation')",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!("Artifacts must retain artifact constraint fragment {fragment:?}"),
        )?;
    }
    Ok(())
}

fn validate_projection_subscribers_constraints(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "ProjectionSubscribers")?;
    require_sql_fragment(
        &sql,
        "expires_at >= last_seen_at",
        "ProjectionSubscribers must retain the liveness ordering constraint".to_string(),
    )
}

fn validate_library_browser_fts_configuration(connection: &Connection) -> LibrarySqliteResult<()> {
    let sql = read_required_normalized_table_sql(connection, "LibraryBrowserRows_fts")?;
    for fragment in [
        "USING fts5",
        "title",
        "artist",
        "album",
        "unicode61 remove_diacritics 1",
    ] {
        require_sql_fragment(
            &sql,
            fragment,
            format!(
                "LibraryBrowserRows_fts must retain searchable library-browser configuration fragment {fragment:?}"
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
