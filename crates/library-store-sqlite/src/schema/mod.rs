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

fn validate_existing_canonical_baseline_schema(connection: &Connection) -> LibrarySqliteResult<()> {
    validate_baseline_schema(connection).map_err(|error| match error {
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
        "source_directories",
        "SourceFacts",
        "source_files",
        "source_locators",
        "SourceSegmentSets",
        "SourceSegments",
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
                "relative_path",
                "presence_state",
                "media_browseability",
                "created_at",
                "updated_at",
            ]
        );
        assert_eq!(
            table_column_names(&connection, "source_files"),
            vec![
                "source_file_id",
                "source_id",
                "parent_source_directory_id",
                "name",
                "relative_path",
                "size_bytes",
                "mtime_ns",
                "presence_state",
                "media_class",
                "first_discovered_at",
                "last_observed_at",
                "last_presence_change_at",
                "created_at",
                "updated_at",
            ]
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
