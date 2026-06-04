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
        "ArtifactClaims",
        "ArtifactFileStoreEntries",
        "ArtifactInlinePayloads",
        "Artifacts",
        "CapabilityDependencies",
        "CapabilitySpecs",
        "LibraryMetadata",
        "browser_user_order",
        "browser_user_prefs",
        "source_state",
        "source_scan_state",
        "source_locations",
        "sources",
        "navigation_rows",
        "LibraryBrowserRows",
        "LibraryBrowserRows_fts",
        "LibraryAssetAttachments",
        "LibraryAssetCapabilities",
        "LibraryAssetMetadataCorrections",
        "LibraryAssets",
        "PlaylistEntries",
        "Playlists",
        "PrepAssignments",
        "PrepPolicies",
        "PrepPolicyTargets",
        "ProjectionChangeLog",
        "ProjectionCursors",
        "ProjectionRetentionWatermarks",
        "ProjectionSubscribers",
        "ResolvedLibraryAssetPrepTargets",
        "content_attachments",
        "primary_media_candidates",
        "source_directories",
        "source_file_attachment_links",
        "SourceFacts",
        "source_files",
        "source_locators",
        "SourceSegmentSets",
        "SourceSegments",
        "track_identity_candidate_evidence",
        "track_identity_candidate_members",
        "track_identity_candidates",
        "track_identity_decision_evidence",
        "track_identity_decision_source_scope",
        "track_identity_decisions",
        "WorkItems",
        "WorkRuns",
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
                   AND name NOT LIKE 'LibraryBrowserRows_fts_%'
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
                 FROM LibraryMetadata
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
    fn canonical_baseline_includes_durable_playlist_authority_tables() {
        let connection = install_test_baseline();

        assert_eq!(
            table_column_names(&connection, "Playlists"),
            vec!["playlist_id", "display_name", "created_at", "updated_at"]
        );
        assert_eq!(
            table_column_names(&connection, "PlaylistEntries"),
            vec![
                "playlist_entry_id",
                "playlist_id",
                "library_asset_id",
                "position",
                "created_at",
                "updated_at",
            ]
        );

        let foreign_keys = table_foreign_keys(&connection, "PlaylistEntries");
        assert!(foreign_keys.contains(&(
            "Playlists".to_string(),
            "playlist_id".to_string(),
            "CASCADE".to_string(),
        )));
        assert!(foreign_keys.contains(&(
            "LibraryAssets".to_string(),
            "library_asset_id".to_string(),
            "CASCADE".to_string(),
        )));
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
            "source_locations",
            "source_directories",
            "source_files",
            "browser_user_order",
            "browser_user_prefs",
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
                "name_browse_sort_key",
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
                .contains(&"source_files_source_browse_order".to_string())
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
                "name_browse_sort_key",
                "relative_path_browse_sort_key",
                "relative_path",
                "size_bytes",
                "mtime_ns",
                "file_kind",
                "media_class",
                "presence_state",
                "first_discovered_at",
                "last_observed_at",
                "last_presence_change_at",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "SourceFacts"),
            vec![
                "source_file_id",
                "fact_kind",
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
            table_index_names(&connection, "SourceFacts")
                .contains(&"SourceFacts_source_basis".to_string())
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
            table_column_names(&connection, "primary_media_candidates"),
            vec![
                "primary_media_candidate_id",
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
            table_index_names(&connection, "primary_media_candidates")
                .contains(&"primary_media_candidates_evidence_source_file".to_string())
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
                "primary_media_candidate_id",
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
                "primary_media_candidate_id",
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
                "primary_media_candidate_id",
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
                     name_browse_sort_key,
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
                     name_browse_sort_key,
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
                     name_browse_sort_key,
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
                     name_browse_sort_key,
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
                     name_browse_sort_key,
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
                     name_browse_sort_key,
                     relative_path_browse_sort_key,
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
    fn browser_user_order_uses_partial_unique_indexes_and_legal_parent_shapes() {
        let connection = install_test_baseline();
        let index_names = table_index_names(&connection, "browser_user_order");
        assert!(
            index_names
                .iter()
                .any(|name| name == "browser_user_order_source_node")
        );
        assert!(
            index_names
                .iter()
                .any(|name| name == "browser_user_order_source_location_node")
        );

        connection
            .execute(
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
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
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
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
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
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
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
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
    fn baseline_schema_defines_library_asset_durable_tables() {
        let connection = install_test_baseline();
        let tables = sorted_user_table_names(&connection);

        for table_name in [
            "LibraryAssets",
            "LibraryAssetAttachments",
            "LibraryAssetMetadataCorrections",
            "LibraryAssetCapabilities",
            "ResolvedLibraryAssetPrepTargets",
            "LibraryBrowserRows",
            "LibraryBrowserRows_fts",
        ] {
            assert!(
                tables.iter().any(|table| table == table_name),
                "canonical library-asset table {table_name} is missing"
            );
        }

        for table_name in [
            "LibraryAssets",
            "LibraryAssetAttachments",
            "LibraryAssetMetadataCorrections",
            "LibraryAssetCapabilities",
            "ResolvedLibraryAssetPrepTargets",
            "LibraryBrowserRows",
        ] {
            let columns = table_column_names(&connection, table_name);
            assert!(columns.iter().any(|column| column == "library_asset_id"));
        }

        let subject_kinds = connection
            .prepare(
                "SELECT sql
                 FROM sqlite_master
                 WHERE sql LIKE '%library_asset%'",
            )
            .expect("prepare schema sql scan")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query schema sql")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect schema sql")
            .join("\n");

        assert!(
            subject_kinds
                .contains("subject_kind IN ('source_file', 'library_asset', 'projection_domain')")
        );
        assert!(subject_kinds.contains("scope_kind IN ('library', 'source', 'library_asset')"));
        assert!(subject_kinds.contains("resolve_library_asset"));
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
            .execute_batch("DROP INDEX WorkItems_active_non_capability;")
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
            detail.contains("WorkItems_active_non_capability"),
            "expected missing index name in diagnostic, found {detail}"
        );
    }

    #[test]
    fn validate_seed_rows_is_separate_and_catches_seed_drift() {
        let connection = install_test_baseline();
        connection
            .execute(
                "DELETE FROM CapabilitySpecs WHERE capability_kind = 'waveform'",
                [],
            )
            .expect("tamper seeded capability row");

        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");
        compare::compare_snapshots(&canonical, &live)
            .expect("seed drift should not change structural schema");

        let detail = malformed_detail(
            seeds::validate_seed_rows(&connection).expect_err("reject seed drift"),
        );
        assert!(
            detail.contains("CapabilitySpecs"),
            "expected capability-spec diagnostic, found {detail}"
        );
    }

    #[test]
    fn clean_canonical_install_matches_current_seed_taxonomy() -> LibrarySqliteResult<()> {
        let connection = install_test_baseline();

        let library_metadata = connection.query_row(
            "SELECT library_id, schema_generation, created_at
                 FROM LibraryMetadata",
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
            connection.query_row("SELECT COUNT(*) FROM LibraryMetadata", [], |row| row.get(0))?;
        assert_eq!(metadata_row_count, 1);

        let capability_specs = connection
            .prepare(
                "SELECT capability_kind, display_name, quality_aware, default_profile_key
                 FROM CapabilitySpecs
                 ORDER BY capability_kind",
            )?
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? != 0,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            capability_specs,
            vec![
                (
                    "beatgrid".to_string(),
                    "Beatgrid".to_string(),
                    false,
                    "default".to_string(),
                ),
                (
                    "musical_key".to_string(),
                    "Musical Key".to_string(),
                    false,
                    "default".to_string(),
                ),
                (
                    "stems".to_string(),
                    "Stems".to_string(),
                    true,
                    "default".to_string(),
                ),
                (
                    "tempo".to_string(),
                    "Tempo".to_string(),
                    false,
                    "default".to_string(),
                ),
                (
                    "waveform".to_string(),
                    "Waveform".to_string(),
                    true,
                    "default".to_string(),
                ),
            ]
        );

        let dependency_row_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM CapabilityDependencies", [], |row| {
                row.get(0)
            })?;
        assert_eq!(dependency_row_count, 0);

        seeds::validate_seed_rows(&connection)?;
        Ok(())
    }

    #[test]
    fn validate_seed_rows_rejects_unexpected_capability_dependency_rows() {
        let connection = install_test_baseline();
        connection
            .execute(
                "INSERT INTO CapabilityDependencies (
                     upstream_capability_kind,
                     downstream_capability_kind,
                     invalidation_mode,
                     created_at,
                     updated_at
                 )
                 VALUES ('beatgrid', 'tempo', 'mark_stale', 1, 1)",
                [],
            )
            .expect("tamper capability-dependency seed rows");

        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");
        compare::compare_snapshots(&canonical, &live)
            .expect("seed drift should not change structural schema");

        let detail = malformed_detail(
            seeds::validate_seed_rows(&connection)
                .expect_err("reject unexpected capability-dependency seed drift"),
        );
        assert!(
            detail.contains("CapabilityDependencies"),
            "expected CapabilityDependencies diagnostic, found {detail}"
        );
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

        assert!(
            canonical.compared_triggers.is_empty(),
            "the canonical baseline should not install compared triggers: {:?}",
            canonical.compared_triggers.keys().collect::<Vec<_>>()
        );

        assert!(
            canonical.ignored_objects.iter().all(|object| {
                matches!(
                    object.reason,
                    introspection::IgnoredObjectReason::SqliteInternalPrefix
                        | introspection::IgnoredObjectReason::LibraryBrowserFtsShadowObject
                ) && (object.name.starts_with("sqlite_")
                    || object.name.starts_with("LibraryBrowserRows_fts_"))
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
                "DROP INDEX WorkItems_active_non_capability;
                 CREATE UNIQUE INDEX WorkItems_active_non_capability
                     ON WorkItems (subject_kind, subject_id, work_kind, basis_fingerprint)
                     WHERE work_kind <> 'compute_capability'
                       AND state IN ('queued', 'leased');",
            )
            .expect("tamper partial index predicate");

        let canonical = snapshot::canonical_snapshot().expect("build canonical snapshot");
        let live = snapshot::live_snapshot(&connection).expect("build live snapshot");
        let detail = malformed_detail(
            compare::compare_snapshots(&canonical, &live)
                .expect_err("reject partial-index predicate drift"),
        );
        assert!(
            detail.contains("WorkItems_active_non_capability"),
            "expected drifting partial index name in diagnostic, found {detail}"
        );
        assert!(
            detail.contains("partial predicate fingerprint mismatch"),
            "expected residual fingerprint diagnostic, found {detail}"
        );
    }
}
