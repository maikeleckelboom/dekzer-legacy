use rusqlite::Connection;

use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};

mod baseline;
mod checks;
mod compare;
mod introspection;
mod seeds;
mod snapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaselineSchemaBootstrapStatus {
    InstalledCanonicalBaseline,
    ExistingCanonicalBaseline,
}

pub(crate) fn ensure_baseline_schema(
    connection: &mut Connection,
) -> LibrarySqliteResult<BaselineSchemaBootstrapStatus> {
    if database_is_empty(connection)? {
        baseline::install_canonical_baseline(connection, unix_time_ms()?)?;
        validate_baseline_schema(connection)?;
        return Ok(BaselineSchemaBootstrapStatus::InstalledCanonicalBaseline);
    }

    validate_existing_canonical_baseline_schema(connection)?;
    Ok(BaselineSchemaBootstrapStatus::ExistingCanonicalBaseline)
}

pub(crate) fn canonical_baseline_generation() -> &'static str {
    baseline::CANONICAL_BASELINE_GENERATION
}

#[cfg(test)]
pub(crate) fn install_baseline_schema_for_test(
    connection: &mut Connection,
) -> LibrarySqliteResult<()> {
    baseline::install_canonical_baseline(connection, 0)?;
    validate_baseline_schema(connection)
}

pub(crate) fn check_existing_canonical_baseline_schema(
    connection: &Connection,
) -> LibrarySqliteResult<()> {
    validate_baseline_schema(connection)
}

pub(crate) fn validate_existing_canonical_baseline_schema(
    connection: &Connection,
) -> LibrarySqliteResult<()> {
    check_existing_canonical_baseline_schema(connection).map_err(|error| match error {
        LibrarySqliteError::MalformedSchemaState(detail) => {
            LibrarySqliteError::MalformedSchemaState(format!(
                "database schema does not match the canonical substrate baseline: {detail}"
            ))
        }
        other => LibrarySqliteError::MalformedSchemaState(format!(
            "database schema does not match the canonical substrate baseline: {other}"
        )),
    })
}

fn validate_baseline_schema(connection: &Connection) -> LibrarySqliteResult<()> {
    let canonical = snapshot::canonical_snapshot()?;
    let live = snapshot::live_snapshot(connection)?;
    compare::compare_snapshots(&canonical, &live)?;
    checks::run_non_structural_checks(connection)
}

fn database_is_empty(connection: &Connection) -> LibrarySqliteResult<bool> {
    let object_count: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM sqlite_master
         WHERE type IN ('table', 'index', 'trigger', 'view')
           AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(object_count == 0)
}

#[cfg(test)]
mod tests {
    use super::{
        BaselineSchemaBootstrapStatus, checks, compare, ensure_baseline_schema,
        install_baseline_schema_for_test, introspection, seeds, snapshot,
    };
    use crate::{LibrarySqliteError, LibrarySqliteResult};
    use rusqlite::Connection;

    const EXPECTED_TABLES: &[&str] = &[
        "work_artifact_claims",
        "work_artifact_file_store_entries",
        "work_artifact_inline_payloads",
        "work_artifacts",
        "library_metadata",
        "source_navigation_user_order",
        "source_root_navigation_state",
        "root_admission_proposals",
        "source_state",
        "source_scan_state",
        "source_locations",
        "sources",
        "search_filter_index_fts",
        "search_filter_index_metadata",
        "search_filter_index_rows",
        "search_filter_index_source_coverage",
        "navigation_rows",
        "projection_change_log",
        "projection_cursors",
        "projection_retention_watermarks",
        "projection_subscribers",
        "content_attachments",
        "primary_media_facts",
        "source_directories",
        "source_file_attachment_links",
        "source_file_facts",
        "source_files",
        "source_locators",
        "track_identity_candidate_evidence",
        "track_identity_candidate_members",
        "track_identity_candidates",
        "track_identity_decision_evidence",
        "track_identity_decision_source_scope",
        "track_identity_decisions",
        "work_items",
        "work_runs",
    ];

    fn install_test_baseline() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install canonical baseline");
        connection
    }

    fn malformed_detail(error: LibrarySqliteError) -> String {
        match error {
            LibrarySqliteError::MalformedSchemaState(detail) => detail,
            other => panic!("expected malformed schema error, found {other:?}"),
        }
    }

    fn sorted_user_table_names(connection: &Connection) -> Vec<String> {
        let mut stmt = connection
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'table'
                   AND name NOT LIKE 'sqlite_%'
                   AND name NOT LIKE 'search_filter_index_fts_%'
                 ORDER BY name",
            )
            .expect("prepare table-name query");
        stmt.query_map([], |row| row.get::<_, String>(0))
            .expect("query user table names")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect user table names")
    }

    fn table_column_names(connection: &Connection, table_name: &str) -> Vec<String> {
        let mut stmt = connection
            .prepare(&format!("PRAGMA table_info({table_name})"))
            .expect("prepare table-info query");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("query table columns")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect table columns")
    }

    fn table_foreign_keys(
        connection: &Connection,
        table_name: &str,
    ) -> Vec<(String, String, String)> {
        let mut stmt = connection
            .prepare(&format!("PRAGMA foreign_key_list({table_name})"))
            .expect("prepare foreign-key-list query");
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(6)?,
            ))
        })
        .expect("query table foreign keys")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect table foreign keys")
    }

    fn table_index_names(connection: &Connection, table_name: &str) -> Vec<String> {
        let mut stmt = connection
            .prepare(&format!("PRAGMA index_list({table_name})"))
            .expect("prepare index-list query");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("query table indexes")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect table indexes")
    }

    fn schema_object_names(connection: &Connection, object_type: &str) -> Vec<String> {
        let mut stmt = connection
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = ?1
                   AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .expect("prepare schema object query");
        stmt.query_map([object_type], |row| row.get::<_, String>(0))
            .expect("query schema objects")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect schema objects")
    }

    fn table_trigger_names(connection: &Connection, table_name: &str) -> Vec<String> {
        let mut stmt = connection
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'trigger'
                   AND tbl_name = ?1
                 ORDER BY name",
            )
            .expect("prepare trigger-list query");
        stmt.query_map([table_name], |row| row.get::<_, String>(0))
            .expect("query table triggers")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect table triggers")
    }

    fn insert_schema_test_source(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO sources (
                     source_id,
                     source_class,
                     authority,
                     identity_key,
                     display_name,
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'internal', 'system', 'schema:test', 'Schema Test', 1, 1, 1)",
                [],
            )
            .expect("insert schema test source");
    }

    #[test]
    fn ensure_baseline_schema_installs_the_canonical_schema_into_an_empty_database() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        let status = ensure_baseline_schema(&mut connection).expect("install canonical baseline");

        assert_eq!(
            status,
            BaselineSchemaBootstrapStatus::InstalledCanonicalBaseline
        );

        let mut expected = EXPECTED_TABLES
            .iter()
            .map(|table_name| (*table_name).to_string())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(sorted_user_table_names(&connection), expected);

        let metadata_rows = connection
            .prepare(
                "SELECT library_id, schema_generation
                 FROM library_metadata
                 ORDER BY library_id",
            )
            .expect("prepare metadata query")
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query metadata rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect metadata rows");
        assert_eq!(
            metadata_rows,
            vec![(1, super::canonical_baseline_generation().to_string())]
        );
    }

    #[test]
    fn canonical_baseline_uses_snake_case_schema_object_names() {
        let connection = install_test_baseline();

        for table_name in sorted_user_table_names(&connection) {
            assert!(
                table_name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
                "canonical table name must be snake_case: {table_name}"
            );
        }

        for index_name in schema_object_names(&connection, "index") {
            assert!(
                !index_name.bytes().any(|byte| byte.is_ascii_uppercase()),
                "canonical index name must not contain PascalCase: {index_name}"
            );
        }
    }

    #[test]
    fn canonical_baseline_has_renamed_schema_objects_and_no_old_names() {
        let connection = install_test_baseline();
        let tables = sorted_user_table_names(&connection);
        let indexes = schema_object_names(&connection, "index");

        for table_name in [
            "library_metadata",
            "work_items",
            "work_runs",
            "work_artifacts",
            "work_artifact_inline_payloads",
            "work_artifact_file_store_entries",
            "work_artifact_claims",
            "source_file_facts",
            "projection_change_log",
            "projection_subscribers",
            "projection_cursors",
            "projection_retention_watermarks",
            "root_admission_proposals",
            "primary_media_facts",
        ] {
            assert!(
                tables.iter().any(|table| table == table_name),
                "renamed table {table_name} is missing"
            );
        }

        for index_name in [
            "work_items_subject_state",
            "work_items_active_work",
            "work_runs_work_item_started_at",
            "work_artifacts_work_run",
            "work_artifacts_subject_created_at",
            "work_artifacts_kind_role_lookup",
            "work_artifact_claims_active_claim",
            "source_file_facts_source_basis",
            "source_file_facts_media_kind",
            "projection_change_log_domain_sequence",
            "projection_subscribers_expires_at",
            "root_admission_proposals_active_canonical_path",
            "source_directories_parent_order",
            "source_files_path_order",
            "source_files_parent_order",
            "source_navigation_user_order_source_item",
            "source_navigation_user_order_source_location_item",
            "source_navigation_user_order_parent_order",
        ] {
            assert!(
                indexes.iter().any(|index| index == index_name),
                "renamed index {index_name} is missing"
            );
        }

        for old_name in [
            concat!("Library", "Metadata"),
            concat!("Work", "Items"),
            concat!("Work", "Runs"),
            concat!("Arti", "facts"),
            concat!("Artifact", "InlinePayloads"),
            concat!("Artifact", "FileStoreEntries"),
            concat!("Artifact", "Claims"),
            concat!("Source", "Facts"),
            concat!("Projection", "ChangeLog"),
            concat!("Projection", "Subscribers"),
            concat!("Projection", "Cursors"),
            concat!("Projection", "RetentionWatermarks"),
            concat!("source_registration", "_proposals"),
            concat!("primary_media_", "candidates"),
            concat!("Work", "Items_subject_state"),
            concat!("Work", "Items_active_work"),
            concat!("Work", "Runs_work_item_started_at"),
            concat!("Arti", "facts_work_run"),
            concat!("Arti", "facts_subject_created_at"),
            concat!("Arti", "facts_kind_role_lookup"),
            concat!("Artifact", "Claims_active_claim"),
            concat!("Source", "Facts_source_basis"),
            concat!("Source", "Facts_media_kind"),
            concat!("Projection", "ChangeLog_domain_sequence"),
            concat!("Projection", "Subscribers_expires_at"),
            concat!("source_registration", "_proposals_active_canonical_path"),
            concat!("source_directories_parent_", "browse"),
            concat!("source_files_source_", "browse_order"),
            concat!("source_files_parent_", "browse"),
            concat!("source_navigation_user_order_source_", "node"),
            concat!("source_navigation_user_order_source_location_", "node"),
            concat!("source_navigation_user_order_domain_", "parent_ordinal"),
        ] {
            assert!(
                !tables.iter().any(|table| table == old_name)
                    && !indexes.iter().any(|index| index == old_name),
                "old schema object name remains active: {old_name}"
            );
        }

        for (table_name, old_column) in [
            ("source_directories", concat!("name_browse", "_sort_key")),
            ("source_files", concat!("name_browse", "_sort_key")),
            ("source_files", concat!("relative_path_browse", "_sort_key")),
            (
                "source_root_navigation_state",
                concat!("root_window", "_state"),
            ),
            ("source_navigation_user_order", concat!("node", "_domain")),
            ("source_navigation_user_order", concat!("node", "_id")),
            ("source_navigation_user_order", concat!("parent", "_scope")),
            ("source_file_facts", concat!("fact", "_kind")),
        ] {
            assert!(
                !table_column_names(&connection, table_name)
                    .iter()
                    .any(|column| column == old_column),
                "old column {table_name}.{old_column} remains active"
            );
        }
    }

    #[test]
    fn canonical_baseline_defines_source_location_ownership_and_projection_tables() {
        let connection = install_test_baseline();
        let tables = sorted_user_table_names(&connection);

        for table_name in [
            "sources",
            "source_locators",
            "source_state",
            "source_scan_state",
            "root_admission_proposals",
            "source_locations",
            "source_directories",
            "source_files",
            "source_navigation_user_order",
            "navigation_rows",
        ] {
            assert!(
                tables.iter().any(|table| table == table_name),
                "canonical source/location table {table_name} is missing"
            );
        }
        assert_eq!(
            table_column_names(&connection, "source_locations"),
            vec![
                "source_location_id",
                "source_id",
                "authority",
                "location_kind",
                "relative_path",
                "display_name",
                "is_user_visible",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "source_directories"),
            vec![
                "source_directory_id",
                "source_id",
                "parent_source_directory_id",
                "name",
                "name_sort_key",
                "relative_path",
                "presence_state",
                "has_child_directories",
                "has_primary_media_descendant",
                "has_image_media_descendant",
                "dir_scan_state",
                "dir_scan_issue_kind",
                "dir_scan_error_detail",
                "dir_scan_updated_at",
                "scanned_at",
                "mtime_ns",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "source_directories")
                .contains(&"source_directories_source_relative_path_binary".to_string())
        );
        assert!(
            table_index_names(&connection, "source_files")
                .contains(&"source_files_source_relative_path_binary".to_string())
        );
        assert!(
            table_index_names(&connection, "source_files")
                .contains(&"source_files_path_order".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "source_root_navigation_state"),
            vec![
                "source_id",
                "root_reach_state",
                "immediate_child_directory_count",
                "issue_kind",
                "detail",
                "checked_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "root_admission_proposals"),
            vec![
                "root_admission_proposal_id",
                "proposal_status",
                "root_class",
                "requested_path",
                "canonical_path",
                "confirmation_required_reason",
                "suggested_roots_json",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "root_admission_proposals")
                .contains(&"root_admission_proposals_active_canonical_path".to_string())
        );
        assert!(
            table_foreign_keys(&connection, "source_root_navigation_state").contains(&(
                "sources".to_string(),
                "source_id".to_string(),
                "CASCADE".to_string(),
            ))
        );
        assert!(
            table_foreign_keys(&connection, "source_directories").contains(&(
                "source_directories".to_string(),
                "parent_source_directory_id".to_string(),
                "CASCADE".to_string(),
            ))
        );
        assert!(table_foreign_keys(&connection, "source_files").contains(&(
            "source_directories".to_string(),
            "parent_source_directory_id".to_string(),
            "CASCADE".to_string(),
        )));
        assert_eq!(
            table_column_names(&connection, "source_files"),
            vec![
                "source_file_id",
                "source_id",
                "parent_source_directory_id",
                "name",
                "name_sort_key",
                "path_sort_key",
                "relative_path",
                "size_bytes",
                "mtime_ns",
                "file_kind",
                "file_class",
                "presence_state",
                "first_discovered_at",
                "last_observed_at",
                "last_presence_change_at",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "source_file_facts"),
            vec![
                "source_file_id",
                "basis_fingerprint",
                "basis_source_id",
                "basis_relative_path",
                "basis_size_bytes",
                "basis_mtime_ns",
                "basis_presence_state",
                "observed_at_ms",
                "content_hash_algorithm",
                "content_hash_value",
                "media_kind",
                "mime_type",
                "duration_ms",
                "sample_rate_hz",
                "channels",
                "bit_depth",
                "codec",
                "updated_at",
                "accepted_artifact_id",
            ]
        );
        assert!(
            table_index_names(&connection, "source_file_facts")
                .contains(&"source_file_facts_source_basis".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "content_attachments"),
            vec![
                "attachment_id",
                "content_hash_algorithm",
                "content_hash_value",
                "first_observed_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "source_file_attachment_links"),
            vec![
                "source_file_attachment_link_id",
                "attachment_id",
                "source_file_id",
                "source_id",
                "file_kind",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_trigger_names(&connection, "source_file_attachment_links"),
            vec![
                "source_file_attachment_links_source_match_insert".to_string(),
                "source_file_attachment_links_source_match_update".to_string(),
            ]
        );
        assert_eq!(
            table_trigger_names(&connection, "source_files"),
            vec!["source_files_attachment_link_source_match_update".to_string()]
        );
        assert_eq!(
            table_column_names(&connection, "search_filter_index_metadata"),
            vec![
                "search_filter_index_id",
                "indexer_version",
                "generation",
                "state",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "search_filter_index_source_coverage"),
            vec![
                "source_id",
                "indexer_version",
                "generation",
                "state",
                "rebuilt_at",
                "updated_at",
                "detail",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "search_filter_index_rows"),
            vec![
                "row_id",
                "generation",
                "result_kind",
                "authority_layer",
                "stable_key",
                "source_id",
                "source_location_id",
                "source_directory_id",
                "parent_source_directory_id",
                "source_file_id",
                "display_label",
                "display_path",
                "relative_path",
                "sort_key",
                "file_class",
                "file_kind",
                "media_relevance",
                "presence_state",
                "source_access_state",
                "source_scan_phase",
                "has_current_blake3",
                "has_current_probe",
                "attachment_link_state",
                "attachment_id",
                "content_hash_algorithm",
                "content_hash_value",
                "evidence_coverage_state",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "search_filter_index_rows")
                .contains(&"search_filter_index_rows_source".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "primary_media_facts"),
            vec![
                "primary_media_fact_id",
                "attachment_id",
                "evidence_source_file_id",
                "evidence_basis_fingerprint",
                "media_kind",
                "mime_type",
                "duration_ms",
                "sample_rate_hz",
                "channels",
                "bit_depth",
                "codec",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "primary_media_facts")
                .contains(&"primary_media_facts_evidence_source_file".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_candidates"),
            vec![
                "track_identity_candidate_id",
                "candidate_kind",
                "evidence_basis",
                "evidence_key_algorithm",
                "evidence_key_value",
                "status",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_candidate_members"),
            vec![
                "track_identity_candidate_member_id",
                "track_identity_candidate_id",
                "primary_media_fact_id",
                "attachment_id",
                "evidence_source_file_id",
                "evidence_basis_fingerprint",
                "content_hash_algorithm",
                "content_hash_value",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_candidate_evidence"),
            vec![
                "track_identity_candidate_evidence_id",
                "track_identity_candidate_id",
                "primary_media_fact_id",
                "attachment_id",
                "source_file_attachment_link_id",
                "source_file_id",
                "source_id",
                "evidence_basis_fingerprint",
                "content_hash_algorithm",
                "content_hash_value",
                "probe_accepted_artifact_id",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "track_identity_candidate_members")
                .contains(&"track_identity_candidate_members_candidate".to_string())
        );
        assert!(
            table_index_names(&connection, "track_identity_candidate_evidence")
                .contains(&"track_identity_candidate_evidence_source_file".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_decisions"),
            vec![
                "track_identity_decision_id",
                "track_identity_candidate_id",
                "decision_state",
                "decision_source",
                "decision_basis",
                "decision_reason",
                "candidate_kind",
                "candidate_evidence_basis",
                "candidate_status_at_decision",
                "evidence_key_algorithm",
                "evidence_key_value",
                "superseded_by_decision_id",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_decision_evidence"),
            vec![
                "track_identity_decision_evidence_id",
                "track_identity_decision_id",
                "track_identity_candidate_id",
                "track_identity_candidate_member_id",
                "track_identity_candidate_evidence_id",
                "primary_media_fact_id",
                "attachment_id",
                "source_file_attachment_link_id",
                "source_file_id",
                "source_id",
                "evidence_basis_fingerprint",
                "content_hash_algorithm",
                "content_hash_value",
                "probe_accepted_artifact_id",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "track_identity_decisions")
                .contains(&"track_identity_decisions_current_source".to_string())
        );
        assert!(
            table_index_names(&connection, "track_identity_decision_evidence")
                .contains(&"track_identity_decision_evidence_source_file".to_string())
        );
        assert_eq!(
            table_column_names(&connection, "track_identity_decision_source_scope"),
            vec![
                "track_identity_decision_source_scope_id",
                "track_identity_decision_id",
                "track_identity_candidate_id",
                "source_id",
                "scope_basis",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            table_index_names(&connection, "track_identity_decision_source_scope")
                .contains(&"track_identity_decision_source_scope_decision".to_string())
        );
        assert!(
            table_index_names(&connection, "track_identity_decision_source_scope")
                .contains(&"track_identity_decision_source_scope_source".to_string())
        );
        assert_eq!(
            table_foreign_keys(&connection, "track_identity_decision_source_scope"),
            vec![(
                "track_identity_decisions".to_string(),
                "track_identity_decision_id".to_string(),
                "CASCADE".to_string(),
            )],
            "source scope should only cascade at the decision retention boundary"
        );
        assert_eq!(
            table_foreign_keys(&connection, "track_identity_decisions"),
            vec![(
                "track_identity_decisions".to_string(),
                "superseded_by_decision_id".to_string(),
                "SET NULL".to_string(),
            )],
            "decisions retain candidate ids as copied provenance instead of cascading from live candidates"
        );
        assert_eq!(
            table_foreign_keys(&connection, "track_identity_decision_evidence"),
            vec![(
                "track_identity_decisions".to_string(),
                "track_identity_decision_id".to_string(),
                "CASCADE".to_string(),
            )],
            "decision evidence snapshots should only cascade at the decision retention boundary"
        );
        assert_eq!(
            table_column_names(&connection, "navigation_rows"),
            vec![
                "navigation_row_id",
                "stable_key",
                "parent_navigation_row_id",
                "family",
                "row_kind",
                "display_name",
                "sibling_position",
                "selectable",
                "selector_kind",
                "selector_payload",
                "updated_at",
                "row_version",
            ]
        );
    }

    #[test]
    fn source_directory_coverage_facts_accept_pending_positive_and_confirmed_negative_states() {
        let connection = install_test_baseline();
        insert_schema_test_source(&connection);

        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     has_child_directories,
                     has_primary_media_descendant,
                     has_image_media_descendant,
                     dir_scan_state,
                     dir_scan_updated_at,
                     scanned_at,
                     mtime_ns,
                     created_at,
                     updated_at
                 )
                  VALUES (10, 1, NULL, 'pending', 'v1|tptetntdtitntg', 'pending', 'present', 0, 0, 0, 'pending', 100, NULL, NULL, 100, 100)",
                [],
            )
            .expect("pending directory coverage facts are accepted");

        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     has_primary_media_descendant,
                     dir_scan_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                  VALUES (11, 1, NULL, 'scanning-media', 'v1|tstctatntntitntgt-tmtetdtita', 'scanning-media', 'present', 1, 'scanning', 101, 101, 101)",
                [],
            )
            .expect("positive media knowledge before coverage completion is accepted");

        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     has_primary_media_descendant,
                     has_image_media_descendant,
                     dir_scan_state,
                     dir_scan_updated_at,
                     scanned_at,
                     created_at,
                     updated_at
                 )
                  VALUES (12, 1, NULL, 'complete-empty', 'v1|tctomtptletettet-tetmtpttty', 'complete-empty', 'present', 0, 0, 'complete', 102, 102, 102, 102)",
                [],
            )
            .expect("confirmed no-media directory coverage facts are accepted");
    }

    #[test]
    fn source_directory_parent_deletion_cascades_to_child_directories_and_files() {
        let connection = install_test_baseline();
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        insert_schema_test_source(&connection);

        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                  VALUES (20, 1, NULL, 'parent', 'v1|tptatrtetntt', 'parent', 'present', 100, 100, 100)",
                [],
            )
            .expect("insert parent directory");
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                  VALUES (21, 1, 20, 'child', 'v1|tcthtitltd', 'parent/child', 'present', 101, 101, 101)",
                [],
            )
            .expect("insert child directory");
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     path_sort_key,
                     relative_path,
                     presence_state,
                     first_discovered_at,
                     last_observed_at,
                     last_presence_change_at,
                     created_at,
                     updated_at
                 )
                  VALUES (30, 1, 21, 'track.flac', 'v1|tttrtatctkt.tftlatc', 'v1|tptatrtetntt/tcthtitltd/v1|tttrtatctkt.tftlatc', 'parent/child/track.flac', 'present', 102, 102, 102, 102, 102)",
                [],
            )
            .expect("insert child file");

        connection
            .execute(
                "DELETE FROM source_directories WHERE source_directory_id = 20",
                [],
            )
            .expect("delete parent directory");

        let directory_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM source_directories", [], |row| {
                row.get(0)
            })
            .expect("count directories after cascade");
        let file_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
            .expect("count files after cascade");

        assert_eq!(directory_count, 0);
        assert_eq!(file_count, 0);
    }

    #[test]
    fn source_file_attachment_links_reject_source_id_drift() {
        let connection = install_test_baseline();
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        insert_schema_test_source(&connection);
        connection
            .execute(
                "INSERT INTO sources (
                     source_id,
                     source_class,
                     authority,
                     identity_key,
                     display_name,
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (2, 'internal', 'system', 'schema:test:other', 'Other Source', 1, 1, 1)",
                [],
            )
            .expect("insert other schema test source");
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     name,
                     name_sort_key,
                     path_sort_key,
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
                 VALUES (10, 1, 'track.wav', 'track.wav', 'album/track.wav',
                         'Album/track.wav', 'audio', 'audio', 'present',
                         1, 1, 1, 1, 1)",
                [],
            )
            .expect("insert schema test source file");
        connection
            .execute(
                "INSERT INTO content_attachments (
                     attachment_id,
                     content_hash_algorithm,
                     content_hash_value,
                     first_observed_at,
                     updated_at
                 )
                 VALUES (20, 'blake3', 'schema-hash', 1, 1)",
                [],
            )
            .expect("insert schema test attachment");

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
                 VALUES (20, 10, 2, 'audio', 1, 1)",
                [],
            )
            .expect_err("link source_id must match linked source_files.source_id");

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
                 VALUES (20, 10, 1, 'audio', 1, 1)",
                [],
            )
            .expect("insert matching link");
        connection
            .execute(
                "UPDATE source_file_attachment_links
                 SET source_id = 2
                 WHERE source_file_id = 10",
                [],
            )
            .expect_err("link source_id update must not drift");
        connection
            .execute(
                "UPDATE source_files
                 SET source_id = 2
                 WHERE source_file_id = 10",
                [],
            )
            .expect_err("source_file source_id update must not drift existing link");
    }

    #[test]
    fn source_navigation_user_order_uses_partial_unique_indexes_and_legal_parent_shapes() {
        let connection = install_test_baseline();
        let index_names = table_index_names(&connection, "source_navigation_user_order");
        assert_eq!(
            table_column_names(&connection, "source_navigation_user_order"),
            vec![
                "order_id",
                "item_kind",
                "item_key",
                "parent_source_key",
                "ordinal",
                "created_at",
                "updated_at",
            ]
        );
        assert!(
            index_names
                .iter()
                .any(|name| name == "source_navigation_user_order_source_item")
        );
        assert!(
            index_names
                .iter()
                .any(|name| name == "source_navigation_user_order_source_location_item")
        );

        connection
            .execute(
                "INSERT INTO source_navigation_user_order (
                     item_kind,
                     item_key,
                     parent_source_key,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source', '1', NULL, 0, 1, 1)",
                [],
            )
            .expect("insert source order row");
        connection
            .execute(
                "INSERT INTO source_navigation_user_order (
                     item_kind,
                     item_key,
                     parent_source_key,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source', '1', NULL, 1, 1, 1)",
                [],
            )
            .expect_err("partial unique index rejects duplicate source order row");
        connection
            .execute(
                "INSERT INTO source_navigation_user_order (
                     item_kind,
                     item_key,
                     parent_source_key,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source', '2', '1', 0, 1, 1)",
                [],
            )
            .expect_err("source order rows must not have parent scope");
        connection
            .execute(
                "INSERT INTO source_navigation_user_order (
                     item_kind,
                     item_key,
                     parent_source_key,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source_location', '10', NULL, 0, 1, 1)",
                [],
            )
            .expect_err("source-location order rows require parent scope");
    }

    #[test]
    fn navigation_family_and_work_kind_values_use_current_names() {
        let connection = install_test_baseline();

        connection
            .execute(
                "INSERT INTO navigation_rows (
                     navigation_row_id,
                     stable_key,
                     family,
                     row_kind,
                     display_name,
                     sibling_position,
                     selectable,
                     updated_at,
                     row_version
                 )
                 VALUES (1, 'views', 'views', 'group', 'Views', 0, 0, 1, 1)",
                [],
            )
            .expect("lowercase views family is accepted");
        connection
            .execute(
                "INSERT INTO navigation_rows (
                     navigation_row_id,
                     stable_key,
                     family,
                     row_kind,
                     display_name,
                     sibling_position,
                     selectable,
                     updated_at,
                     row_version
                 )
                 VALUES (2, 'Sources', 'Sources', 'group', 'Sources', 1, 0, 1, 1)",
                [],
            )
            .expect_err("PascalCase Sources family is rejected");

        connection
            .execute(
                "INSERT INTO work_items (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     priority_class,
                     basis_fingerprint,
                     state,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'source_file', '1', 'inspect_source_file', 'background', 'basis:1', 'queued', 1, 1)",
                [],
            )
            .expect("inspect_source_file work kind is accepted");
        connection
            .execute(
                "INSERT INTO work_items (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     priority_class,
                     basis_fingerprint,
                     state,
                     created_at,
                     updated_at
                 )
                 VALUES (2, 'source_file', '2', ?1, 'background', 'basis:2', 'queued', 1, 1)",
                [concat!("inspect_", "source")],
            )
            .expect_err("old work kind is rejected");
    }

    #[test]
    fn ensure_baseline_schema_rejects_noncanonical_database_shapes() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE IncompatiblePlaceholder (
                     placeholder_id INTEGER PRIMARY KEY,
                     created_at INTEGER NOT NULL
                 ) STRICT;",
            )
            .expect("create incompatible schema");

        let detail = malformed_detail(
            ensure_baseline_schema(&mut connection).expect_err("reject noncanonical schema"),
        );
        assert!(
            detail.starts_with("database schema does not match the canonical substrate baseline:"),
            "unexpected malformed-schema detail: {detail}"
        );
        assert!(
            detail.contains("IncompatiblePlaceholder"),
            "expected incompatible table name in diagnostic, found {detail}"
        );
    }

    #[test]
    fn canonical_snapshot_matches_live_canonical_database() {
        let connection = install_test_baseline();
        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");

        assert_eq!(canonical, live);
        compare::compare_snapshots(&canonical, &live).expect("canonical snapshots match");
    }

    #[test]
    fn tampered_compared_schema_object_reports_a_precise_structural_failure() {
        let mut connection = install_test_baseline();
        connection
            .execute_batch("DROP INDEX work_items_active_work;")
            .expect("drop compared index");

        let detail = malformed_detail(
            ensure_baseline_schema(&mut connection)
                .expect_err("reject dropped compared schema object"),
        );
        assert!(
            detail.contains("compared index set mismatch"),
            "expected compared-index diagnostic, found {detail}"
        );
        assert!(
            detail.contains("work_items_active_work"),
            "expected missing index name in diagnostic, found {detail}"
        );
    }

    #[test]
    fn validate_seed_rows_is_separate_and_catches_seed_drift() {
        let connection = install_test_baseline();
        connection
            .execute(
                "DELETE FROM search_filter_index_metadata
                 WHERE search_filter_index_id = 1",
                [],
            )
            .expect("tamper seeded search/filter metadata row");

        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");
        compare::compare_snapshots(&canonical, &live)
            .expect("seed drift should not change structural schema");

        let detail = malformed_detail(
            seeds::validate_seed_rows(&connection).expect_err("reject seed drift"),
        );
        assert!(
            detail.contains("search_filter_index_metadata"),
            "expected search/filter metadata diagnostic, found {detail}"
        );
    }

    #[test]
    fn clean_canonical_install_matches_current_seed_taxonomy() -> LibrarySqliteResult<()> {
        let connection = install_test_baseline();

        let library_metadata = connection.query_row(
            "SELECT library_id, schema_generation, created_at
                 FROM library_metadata",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        assert_eq!(
            library_metadata,
            (1, super::canonical_baseline_generation().to_string(), 0)
        );

        let metadata_row_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM library_metadata", [], |row| {
                row.get(0)
            })?;
        assert_eq!(metadata_row_count, 1);

        let search_filter_metadata = connection.query_row(
            "SELECT search_filter_index_id, indexer_version, generation, state
             FROM search_filter_index_metadata",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        assert_eq!(
            search_filter_metadata,
            (1, "search_filter_v0".to_string(), 0, "ready".to_string())
        );

        seeds::validate_seed_rows(&connection)?;
        Ok(())
    }

    #[test]
    fn auto_vacuum_enforcement_still_runs_as_a_non_structural_check() {
        let connection = install_test_baseline();
        connection
            .execute_batch("PRAGMA auto_vacuum = NONE; VACUUM;")
            .expect("rewrite auto_vacuum mode");

        let detail = malformed_detail(
            checks::validate_incremental_auto_vacuum(&connection)
                .expect_err("reject auto_vacuum drift"),
        );
        assert!(
            detail.contains("PRAGMA auto_vacuum = INCREMENTAL"),
            "expected auto_vacuum diagnostic, found {detail}"
        );
    }

    #[test]
    fn foreign_key_integrity_check_still_runs_as_a_non_structural_check() {
        let connection = install_test_baseline();
        connection
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable foreign key enforcement");
        connection
            .execute(
                "INSERT INTO source_locators (
                     source_id,
                     locator_kind,
                     absolute_path,
                     relative_suffix
                 )
                 VALUES (99, 'absolute_path', 'C:/ghost', '')",
                [],
            )
            .expect("insert foreign-key violation");

        let detail = malformed_detail(
            checks::foreign_key_integrity_check(&connection).expect_err("reject foreign key drift"),
        );
        assert!(
            detail.contains("table=source_locators"),
            "expected source_locators foreign key diagnostic, found {detail}"
        );
    }

    #[test]
    fn internal_object_policy_only_ignores_sqlite_internal_objects() {
        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");

        assert_eq!(
            canonical
                .compared_triggers
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec![
                "source_file_attachment_links_source_match_insert",
                "source_file_attachment_links_source_match_update",
                "source_files_attachment_link_source_match_update",
            ],
            "only source-file attachment link invariant triggers should be compared"
        );

        assert!(
            canonical.ignored_objects.iter().all(|object| {
                matches!(
                    object.reason,
                    introspection::IgnoredObjectReason::SqliteInternalPrefix
                        | introspection::IgnoredObjectReason::SearchFilterIndexFtsShadowObject
                ) && (object.name.starts_with("sqlite_")
                    || object.name.starts_with("search_filter_index_fts_"))
            }),
            "only sqlite-internal objects may be ignored: {:?}",
            canonical.ignored_objects
        );
    }

    #[test]
    fn partial_index_predicate_drift_is_caught_by_the_residual_fingerprint_path() {
        let connection = install_test_baseline();
        connection
            .execute_batch(
                "DROP INDEX work_items_active_work;
                 CREATE UNIQUE INDEX work_items_active_work
                     ON work_items (subject_kind, subject_id, work_kind, basis_fingerprint)
                     WHERE state IN ('queued', 'leased');",
            )
            .expect("tamper partial index predicate");

        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");
        let detail = malformed_detail(
            compare::compare_snapshots(&canonical, &live)
                .expect_err("reject partial-index predicate drift"),
        );
        assert!(
            detail.contains("work_items_active_work"),
            "expected drifting partial index name in diagnostic, found {detail}"
        );
        assert!(
            detail.contains("partial predicate fingerprint mismatch"),
            "expected residual fingerprint diagnostic, found {detail}"
        );
    }
}
