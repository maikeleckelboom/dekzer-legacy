use super::discovery::{RootScanHierarchyObservationReason, RootScanObservation};
use super::{
    DurableStoreBootstrapStatus, DurableStoreSchemaCompatibilityState, RegisterLocalRootInput,
    SqliteDurableStore,
};
use crate::authority::ingest::{DiscoveredFileInput, DiscoveryBatch};
use crate::authority::roots::{
    ApplyRootMountedInput, ApplyRootUnmountedInput, RegisterRemovableRootInput, RootIdentityKind,
    RootMountStatus,
};
use crate::{
    AcceptSegmentationPromotionInput, ClaimMachineWorkBatchInput, CommitAcceptedSourceFactsInput,
    CommitAcceptedSourceFactsMergePolicy, CompleteMachineWorkInput, ContentHashEvidence,
    DeleteSourceLocationInput, FinishWorkRunInput, InspectSourcePromotionInput,
    RecordArtifactInput, RecordInlineArtifactInput, ReplaceAcceptedSourceSegmentSetInput,
    ResolveLibraryAssetPromotionInput, SourceFileClassFilter, StartWorkRunInput,
    StoreLiteralHierarchyCoverageState, StoreLiteralHierarchyEntryPoint, UpsertPrepPolicyInput,
    UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocationInput,
    UpsertSourceLocatorInput, UpsertSourceScanStateInput, UpsertSourceStateInput,
};
use library_domain::{
    ArtifactKind, ArtifactRole, NavigationSelector, PrepPolicyId, SourceAccessState, SourceFileId,
    SourceId, SourcePresenceState, SourceScanPhase, SourceSegmentId, SourceSegmentSetId,
    WorkItemId, WorkPriorityClass, WorkRunOutcome, encode_selector,
};
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

const FIXED_TOP_LEVEL_NAVIGATION_ROW_COUNT: usize = 6;

const FIXED_TOP_LEVEL_NAVIGATION_ROWS: &[(&str, &str, &str, &str, i64, &str)] = &[
    (
        "view:all_media",
        "Views",
        "view",
        "All Media",
        0,
        "all_media",
    ),
    (
        "view:all_audio",
        "Views",
        "view",
        "All Audio",
        1,
        "all_audio",
    ),
    (
        "view:all_videos",
        "Views",
        "view",
        "All Videos",
        2,
        "all_videos",
    ),
    (
        "view:recently_added",
        "Views",
        "view",
        "Recently Added",
        3,
        "recently_added",
    ),
    (
        "view:needs_preparation",
        "Views",
        "view",
        "Needs Preparation",
        4,
        "needs_preparation",
    ),
    (
        "collection-group:playlists",
        "Collections",
        "collection-group",
        "Playlists",
        0,
        "playlist_group",
    ),
];

const REMOVED_HIGHER_BAR_NAVIGATION_STABLE_KEYS: &[&str] = &[
    "view:top_rated",
    "view:in_rotation",
    "smart-collection-group:smart_lists",
    "crate-group:crates",
    "smart-crate-group:smart_crates",
    "set-group:sets",
    "catalog-provider:spotify",
    "catalog-provider:apple_music",
    "catalog-provider:tidal",
    "catalog-provider:soundcloud",
    "catalog-provider:beatport",
    "catalog-provider:beatsource",
    "facet-group:genres",
    "facet-group:year",
];

fn open_mutation_connection(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).expect("open mutation database");
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign keys");
    connection
}

fn assert_fixed_top_level_navigation_rows(rows: &[crate::NavigationRow]) {
    for &(stable_key, family, row_kind, display_name, sibling_position, selector_kind) in
        FIXED_TOP_LEVEL_NAVIGATION_ROWS
    {
        let row = rows
            .iter()
            .find(|row| row.stable_key == stable_key)
            .unwrap_or_else(|| panic!("fixed navigation row {stable_key} exists"));
        assert_eq!(row.parent_navigation_row_id, None);
        assert_eq!(row.family.as_deref(), Some(family));
        assert_eq!(row.row_kind, row_kind);
        assert_eq!(row.display_name, display_name);
        assert_eq!(row.sibling_position, sibling_position);
        assert!(row.selectable);
        assert_eq!(row.selector_kind.as_deref(), Some(selector_kind));
        assert_eq!(row.selector_payload.as_deref(), Some(""));
    }
}

fn assert_removed_higher_bar_navigation_rows_absent(connection: &Connection) {
    for stable_key in REMOVED_HIGHER_BAR_NAVIGATION_STABLE_KEYS {
        let row_count: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM navigation_rows
                 WHERE stable_key = ?1",
                [stable_key],
                |row| row.get(0),
            )
            .unwrap_or_else(|error| panic!("count removed navigation row {stable_key}: {error}"));
        assert_eq!(
            row_count, 0,
            "removed navigation row {stable_key} must not be projected"
        );
    }
}

fn navigation_projection_change_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM ProjectionChangeLog
             WHERE projection_domain = 'navigation'",
            [],
            |row| row.get(0),
        )
        .expect("count navigation projection changes")
}

fn upsert_empty_prep_policy(
    durable_store: &SqliteDurableStore,
    prep_policy_id: i64,
    policy_name: &str,
    is_system_policy: bool,
    changed_at: i64,
) -> PrepPolicyId {
    durable_store
        .upsert_prep_policy(UpsertPrepPolicyInput {
            prep_policy_id: Some(PrepPolicyId::new(prep_policy_id).expect("positive id")),
            policy_name: policy_name.to_string(),
            is_system_policy,
            is_user_editable: !is_system_policy,
            changed_at,
            targets: Vec::new(),
        })
        .expect("upsert prep policy")
}

fn insert_library_browser_asset(connection: &Connection, library_asset_id: i64, title: &str) {
    connection
        .execute(
            "INSERT INTO LibraryAssets (
                 library_asset_id,
                 equivalence_fingerprint,
                 retention_policy,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, 'keep_metadata', 1, 1)",
            rusqlite::params![library_asset_id, format!("eq:browser:{library_asset_id}")],
        )
        .expect("insert library asset");
    connection
        .execute(
            "INSERT INTO LibraryBrowserRows (
                 library_asset_id,
                 row_version,
                 primary_source_file_id,
                 availability_state,
                 title,
                 prep_readiness_summary,
                 updated_at
             )
             VALUES (?1, 1, NULL, 'available', ?2, 'not_required', 1)",
            rusqlite::params![library_asset_id, title],
        )
        .expect("insert library browser row");
    connection
        .execute(
            "INSERT INTO LibraryBrowserRows_fts (rowid, title, artist, album)
             VALUES (?1, ?2, '', '')",
            rusqlite::params![library_asset_id, title],
        )
        .expect("insert library browser search row");
}

fn set_library_browser_prep_readiness(
    connection: &Connection,
    library_asset_id: i64,
    prep_readiness_summary: &str,
) {
    connection
        .execute(
            "UPDATE LibraryBrowserRows
             SET prep_readiness_summary = ?2
             WHERE library_asset_id = ?1",
            rusqlite::params![library_asset_id, prep_readiness_summary],
        )
        .expect("set library browser prep readiness");
}

fn insert_resolved_prep_target(
    connection: &Connection,
    library_asset_id: i64,
    prep_policy_id: PrepPolicyId,
    capability_kind: &str,
    target_profile_key: &str,
) {
    connection
        .execute(
            "INSERT INTO ResolvedLibraryAssetPrepTargets (
                 library_asset_id,
                 capability_kind,
                 target_profile_key,
                 target_quality,
                 target_stability_class,
                 priority_class,
                 resolved_from_policy_id,
                 updated_at
             )
             VALUES (?1, ?2, ?3, 100, 'stable', 'interactive', ?4, 1)",
            rusqlite::params![
                library_asset_id,
                capability_kind,
                target_profile_key,
                prep_policy_id.get(),
            ],
        )
        .expect("insert resolved prep target");
}

fn register_removable_root(
    durable_store: &SqliteDurableStore,
    relative_suffix: &str,
    identity_value: &str,
) -> i64 {
    durable_store
        .register_removable_root(RegisterRemovableRootInput {
            root_identity_kind: "stable_volume_identity".to_string(),
            root_identity_value: format!("root:{identity_value}:{relative_suffix}"),
            label: Some("USB".to_string()),
            locator_identity_kind: RootIdentityKind::FilesystemUuid,
            locator_identity_value: identity_value.to_string(),
            relative_suffix: relative_suffix.to_string(),
        })
        .expect("register removable root")
}

fn write_valid_wav_file(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create wav parent directory");
    }

    let data = [0_u8, 127, 255, 127];
    let sample_rate = 8_000_u32;
    let channels = 1_u16;
    let bits_per_sample = 8_u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * u32::from(block_align);
    let data_len = data.len() as u32;
    let chunk_size = 36 + data_len;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&chunk_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(&data);
    std::fs::write(path, bytes).expect("write valid wav file");
}

fn claim_single_work_item(
    durable_store: &SqliteDurableStore,
    claimed_at: i64,
) -> crate::ClaimedMachineWorkItem {
    let claimed = durable_store
        .claim_machine_work_batch(ClaimMachineWorkBatchInput {
            limit: 1,
            lease_duration_ms: 30_000,
            claimed_at,
        })
        .expect("claim work item");
    assert_eq!(claimed.len(), 1);
    claimed
        .into_iter()
        .next()
        .expect("single claimed work item")
}

fn record_completed_inline_artifact(
    durable_store: &SqliteDurableStore,
    work_item_id: WorkItemId,
    artifact_kind: ArtifactKind,
    basis_fingerprint: &str,
    started_at: i64,
    payload_hash: &str,
) -> i64 {
    let work_run = durable_store
        .start_work_run(StartWorkRunInput {
            work_item_id,
            adapter_key: "test.adapter".to_string(),
            adapter_version: "1.0.0".to_string(),
            started_at,
        })
        .expect("start work run");
    let artifact = durable_store
        .record_inline_artifact(RecordInlineArtifactInput {
            artifact: RecordArtifactInput {
                work_run_id: work_run.work_run_id,
                artifact_kind,
                artifact_role: ArtifactRole::PrimaryResult,
                media_type: "application/json".to_string(),
                basis_fingerprint: basis_fingerprint.to_string(),
                payload_hash: payload_hash.to_string(),
                created_at: started_at + 1,
            },
            payload: b"{}".to_vec(),
        })
        .expect("record inline artifact");
    durable_store
        .finish_work_run(FinishWorkRunInput {
            work_run_id: work_run.work_run_id,
            finished_at: started_at + 2,
            outcome: WorkRunOutcome::Completed,
            failure_kind: None,
            error_detail: None,
        })
        .expect("finish work run");
    artifact.artifact_id.get()
}

#[test]
fn bootstrap_or_validate_installs_the_canonical_baseline_for_empty_databases() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");

    let status =
        SqliteDurableStore::bootstrap_or_validate(&db_path).expect("bootstrap empty database");

    assert_eq!(
        status,
        DurableStoreBootstrapStatus::InstalledCanonicalBaseline
    );

    let reopened =
        SqliteDurableStore::bootstrap_or_validate(&db_path).expect("reopen canonical database");

    assert_eq!(reopened, DurableStoreBootstrapStatus::OpenedCanonicalStore);
}

#[test]
fn schema_compatibility_reports_missing_without_creating_a_database() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");

    let compatibility = SqliteDurableStore::schema_compatibility(&db_path);

    assert_eq!(
        compatibility.state(),
        DurableStoreSchemaCompatibilityState::Missing
    );
    assert_eq!(compatibility.detail(), None);
    assert!(!db_path.exists());
}

#[test]
fn schema_compatibility_reports_compatible_for_canonical_databases() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    SqliteDurableStore::bootstrap_or_validate(&db_path).expect("bootstrap canonical database");

    let compatibility = SqliteDurableStore::schema_compatibility(&db_path);

    assert_eq!(
        compatibility.state(),
        DurableStoreSchemaCompatibilityState::Compatible
    );
    assert_eq!(compatibility.detail(), None);
}

#[test]
fn schema_compatibility_reports_incompatible_for_noncanonical_databases() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let connection = Connection::open(&db_path).expect("open database");
    connection
        .execute_batch(
            "CREATE TABLE IncompatiblePlaceholder (
                 placeholder_id INTEGER PRIMARY KEY
             ) STRICT;",
        )
        .expect("create incompatible table");

    let compatibility = SqliteDurableStore::schema_compatibility(&db_path);

    assert_eq!(
        compatibility.state(),
        DurableStoreSchemaCompatibilityState::Incompatible
    );
    let detail = compatibility.detail().expect("incompatibility detail");
    assert!(detail.contains("canonical substrate baseline"));
    assert!(detail.contains("IncompatiblePlaceholder"));
}

#[test]
fn schema_compatibility_reports_unreadable_for_non_sqlite_files() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    fs::write(&db_path, b"not sqlite").expect("write invalid database bytes");

    let compatibility = SqliteDurableStore::schema_compatibility(&db_path);

    assert_eq!(
        compatibility.state(),
        DurableStoreSchemaCompatibilityState::Unreadable
    );
    assert!(
        compatibility
            .detail()
            .is_some_and(|detail| !detail.is_empty())
    );
}

#[test]
fn repeated_upsert_source_keeps_one_order_row_and_one_navigation_row() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let first_source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "repeat-source".to_string(),
            display_name: "Repeated Source".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(5),
            changed_at: 10,
        })
        .expect("insert source");
    let second_source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "repeat-source".to_string(),
            display_name: "Repeated Source Renamed".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(7),
            changed_at: 20,
        })
        .expect("update source");

    assert_eq!(first_source_id, second_source_id);

    let connection = open_mutation_connection(&db_path);
    let order_row: (i64, i64) = connection
        .query_row(
            "SELECT COUNT(*), MAX(ordinal)
             FROM browser_user_order
             WHERE node_domain = 'source'
               AND node_id = ?1
               AND parent_scope IS NULL",
            [first_source_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load source order rows");
    assert_eq!(order_row, (1, 7));

    let source_navigation_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM navigation_rows
             WHERE stable_key = ?1
               AND row_kind = 'source'",
            [format!("source:{first_source_id}")],
            |row| row.get(0),
        )
        .expect("count source navigation rows");
    assert_eq!(source_navigation_count, 1);
}

#[test]
fn source_navigation_projection_uses_active_top_level_substrate_rows() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let internal_source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "internal-source".to_string(),
            display_name: "Internal Library".to_string(),
            medium_label: Some("SSD".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 10,
        })
        .expect("insert internal source");
    let removable_source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "removable_mounted".to_string(),
            authority: "device".to_string(),
            identity_kind: "stable_volume_identity".to_string(),
            identity_value: "usb-source".to_string(),
            display_name: "Rekordbox Export".to_string(),
            medium_label: Some("USB Drive".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(1),
            changed_at: 11,
        })
        .expect("insert removable source");

    let top_level_rows = durable_store
        .read_navigation_rows(None)
        .expect("read top-level rows");
    let connection = open_mutation_connection(&db_path);
    assert_removed_higher_bar_navigation_rows_absent(&connection);
    assert_eq!(
        top_level_rows.len(),
        FIXED_TOP_LEVEL_NAVIGATION_ROW_COUNT + 2
    );
    assert_fixed_top_level_navigation_rows(&top_level_rows);

    let source_rows = top_level_rows
        .iter()
        .filter(|row| row.row_kind == "source")
        .collect::<Vec<_>>();
    assert_eq!(source_rows.len(), 2);
    assert!(source_rows.iter().all(|row| {
        row.parent_navigation_row_id.is_none() && row.family.as_deref() == Some("Sources")
    }));
    assert!(
        source_rows
            .iter()
            .any(|row| row.stable_key == format!("source:{internal_source_id}"))
    );
    assert!(
        source_rows
            .iter()
            .any(|row| row.stable_key == format!("source:{removable_source_id}"))
    );
}

#[test]
fn prep_policy_navigation_group_is_absent_without_prep_policies() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let top_level_rows = durable_store
        .read_navigation_rows(None)
        .expect("read top-level rows");

    assert!(
        !top_level_rows
            .iter()
            .any(|row| row.stable_key == "prep-policy-group:policies")
    );
}

#[test]
fn prep_policy_rows_are_projected_and_node_scoped_browser_reads_use_resolved_targets() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let user_policy = upsert_empty_prep_policy(&durable_store, 101, "alpha policy", false, 10);
    let system_policy = upsert_empty_prep_policy(&durable_store, 102, "z system policy", true, 11);
    let second_user_policy =
        upsert_empty_prep_policy(&durable_store, 103, "Alpha policy", false, 12);
    let empty_policy = upsert_empty_prep_policy(&durable_store, 104, "empty policy", false, 13);
    let source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "prep-policy-order-source".to_string(),
            display_name: "Source After Preparation".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 14,
        })
        .expect("insert source");

    let top_level_rows = durable_store
        .read_navigation_rows(None)
        .expect("read top-level rows");
    let expected_source_key = format!("source:{source_id}");
    assert_eq!(
        top_level_rows
            .iter()
            .map(|row| row.stable_key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "view:all_media",
            "view:all_audio",
            "view:all_videos",
            "view:recently_added",
            "view:needs_preparation",
            "collection-group:playlists",
            "prep-policy-group:policies",
            expected_source_key.as_str(),
        ]
    );
    let group = top_level_rows
        .iter()
        .find(|row| row.stable_key == "prep-policy-group:policies")
        .expect("prep policy group is projected");
    assert_eq!(group.row_kind, "prep-policy-group");
    assert_eq!(group.family.as_deref(), Some("Preparation"));
    assert_eq!(group.display_name, "Policies");
    assert!(!group.selectable);
    assert_eq!(group.selector_kind, None);
    assert_eq!(group.selector_payload, None);

    let policy_rows = durable_store
        .read_navigation_rows(Some(group.navigation_row_id))
        .expect("read prep policy rows");
    assert_eq!(
        policy_rows
            .iter()
            .map(|row| row.stable_key.as_str())
            .collect::<Vec<_>>(),
        vec![
            "prep_policy_scope:102",
            "prep_policy_scope:101",
            "prep_policy_scope:103",
            "prep_policy_scope:104",
        ]
    );
    for row in &policy_rows {
        assert_eq!(row.row_kind, "prep-policy-scope");
        assert_eq!(row.parent_navigation_row_id, Some(group.navigation_row_id));
        assert!(row.selectable);
        assert_eq!(row.selector_kind.as_deref(), Some("prep_policy_scope"));
    }
    assert_eq!(policy_rows[0].selector_payload.as_deref(), Some("102"));
    assert_eq!(policy_rows[1].selector_payload.as_deref(), Some("101"));
    assert_eq!(policy_rows[2].selector_payload.as_deref(), Some("103"));
    assert_eq!(policy_rows[3].selector_payload.as_deref(), Some("104"));

    let connection = open_mutation_connection(&db_path);
    assert_removed_higher_bar_navigation_rows_absent(&connection);
    insert_library_browser_asset(&connection, 1, "Alpha Track");
    insert_library_browser_asset(&connection, 2, "Beta Track");
    insert_library_browser_asset(&connection, 3, "Gamma Track");
    insert_resolved_prep_target(&connection, 1, user_policy, "waveform", "warm-waveform");
    insert_resolved_prep_target(&connection, 1, user_policy, "stems", "warm-stems");
    insert_resolved_prep_target(
        &connection,
        2,
        second_user_policy,
        "waveform",
        "other-waveform",
    );
    insert_resolved_prep_target(&connection, 3, user_policy, "waveform", "warm-video");

    let user_policy_row = policy_rows
        .iter()
        .find(|row| row.stable_key == "prep_policy_scope:101")
        .expect("user policy row exists");
    let user_policy_window = durable_store
        .read_navigation_node_library_browser_window(user_policy_row.navigation_row_id, 0, 10)
        .expect("read prep policy navigation node")
        .expect("prep policy row resolves to browser scope");
    assert_eq!(user_policy_window.total_rows, 2);
    assert_eq!(
        user_policy_window
            .rows
            .iter()
            .map(|row| row.library_asset_id)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );

    let scoped_search = durable_store
        .search_navigation_node_library_browser_window(
            user_policy_row.navigation_row_id,
            "Beta",
            0,
            10,
        )
        .expect("search prep policy navigation node")
        .expect("prep policy row resolves to browser scope");
    assert_eq!(scoped_search.total_rows, 0);
    assert!(scoped_search.rows.is_empty());

    let empty_policy_row = policy_rows
        .iter()
        .find(|row| row.stable_key == "prep_policy_scope:104")
        .expect("empty policy row exists");
    let empty_window = durable_store
        .read_navigation_node_library_browser_window(empty_policy_row.navigation_row_id, 0, 10)
        .expect("read empty prep policy navigation node")
        .expect("empty prep policy row resolves to browser scope");
    assert_eq!(empty_window.total_rows, 0);
    assert!(empty_window.rows.is_empty());

    assert_eq!(system_policy.get(), 102);
    assert_eq!(empty_policy.get(), 104);
}

#[test]
fn needs_preparation_view_is_projected_and_node_scoped_browser_reads_use_readiness_summary() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let needs_preparation_row = durable_store
        .load_navigation_row_by_stable_key("view:needs_preparation")
        .expect("load needs-preparation row")
        .expect("needs-preparation row exists");
    assert_eq!(needs_preparation_row.parent_navigation_row_id, None);
    assert_eq!(needs_preparation_row.family.as_deref(), Some("Views"));
    assert_eq!(needs_preparation_row.row_kind, "view");
    assert_eq!(needs_preparation_row.display_name, "Needs Preparation");
    assert_eq!(needs_preparation_row.sibling_position, 4);
    assert!(needs_preparation_row.selectable);
    assert_eq!(
        needs_preparation_row.selector_kind.as_deref(),
        Some("needs_preparation")
    );
    assert_eq!(needs_preparation_row.selector_payload.as_deref(), Some(""));

    let connection = open_mutation_connection(&db_path);
    assert_removed_higher_bar_navigation_rows_absent(&connection);
    insert_library_browser_asset(&connection, 1, "A Preparing");
    insert_library_browser_asset(&connection, 2, "B Underprepared");
    insert_library_browser_asset(&connection, 3, "C Blocked");
    insert_library_browser_asset(&connection, 4, "D Failed");
    insert_library_browser_asset(&connection, 5, "E Ready");
    insert_library_browser_asset(&connection, 6, "F Not Required");
    set_library_browser_prep_readiness(&connection, 1, "preparing");
    set_library_browser_prep_readiness(&connection, 2, "underprepared");
    set_library_browser_prep_readiness(&connection, 3, "blocked");
    set_library_browser_prep_readiness(&connection, 4, "failed");
    set_library_browser_prep_readiness(&connection, 5, "ready");

    let window = durable_store
        .read_navigation_node_library_browser_window(needs_preparation_row.navigation_row_id, 0, 10)
        .expect("read needs-preparation navigation node")
        .expect("needs-preparation row resolves to browser scope");
    assert_eq!(window.total_rows, 4);
    assert_eq!(
        window
            .rows
            .iter()
            .map(|row| (row.library_asset_id, row.prep_readiness_summary.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (1, "preparing"),
            (2, "underprepared"),
            (3, "blocked"),
            (4, "failed")
        ]
    );

    let excluded_ready = durable_store
        .search_navigation_node_library_browser_window(
            needs_preparation_row.navigation_row_id,
            "Ready",
            0,
            10,
        )
        .expect("search needs-preparation node for ready asset")
        .expect("needs-preparation row resolves to browser scope");
    assert_eq!(excluded_ready.total_rows, 0);
    assert!(excluded_ready.rows.is_empty());

    let included_underprepared = durable_store
        .search_navigation_node_library_browser_window(
            needs_preparation_row.navigation_row_id,
            "Underprepared",
            0,
            10,
        )
        .expect("search needs-preparation node for underprepared asset")
        .expect("needs-preparation row resolves to browser scope");
    assert_eq!(included_underprepared.total_rows, 1);
    assert_eq!(included_underprepared.rows[0].library_asset_id, 2);
}

#[test]
fn source_locations_write_side_validates_paths_and_enforces_uniqueness() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "location-source".to_string(),
            display_name: "Location Source".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 10,
        })
        .expect("insert source");

    let source_location_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Music/DJ Pool".to_string(),
            display_name: Some("DJ Pool".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(3),
            first_created_at: Some(20),
            changed_at: 20,
        })
        .expect("insert source location");
    let same_source_location_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Music/DJ Pool".to_string(),
            display_name: Some("DJ Pool Renamed".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(4),
            first_created_at: None,
            changed_at: 21,
        })
        .expect("upsert same canonical source location");
    assert_eq!(source_location_id, same_source_location_id);

    for invalid_path in [
        "/Music",
        "C:/Music",
        ".",
        "Music/../Pool",
        r"Music\Pool",
        "Music/Pool/",
        "Music//Pool",
        "   ",
    ] {
        durable_store
            .upsert_source_location(UpsertSourceLocationInput {
                source_location_id: None,
                source_id,
                authority: "user".to_string(),
                location_kind: "registered_subpath".to_string(),
                relative_path: invalid_path.to_string(),
                display_name: None,
                is_user_visible: true,
                browser_order_ordinal: None,
                first_created_at: None,
                changed_at: 22,
            })
            .expect_err("invalid source-location path is rejected");
    }

    let connection = open_mutation_connection(&db_path);
    let row: (i64, String, String, i64) = connection
        .query_row(
            "SELECT COUNT(*),
                    MAX(relative_path),
                    MAX(display_name),
                    MAX(browser_user_order.ordinal)
             FROM source_locations
             LEFT JOIN browser_user_order
               ON browser_user_order.node_domain = 'source_location'
              AND browser_user_order.node_id = CAST(source_locations.source_location_id AS TEXT)
              AND browser_user_order.parent_scope = CAST(source_locations.source_id AS TEXT)
             WHERE source_locations.source_id = ?1",
            [source_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load source location");
    assert_eq!(
        row,
        (
            1,
            "Music/DJ Pool".to_string(),
            "DJ Pool Renamed".to_string(),
            4
        )
    );

    assert!(
        durable_store
            .delete_source_location(DeleteSourceLocationInput { source_location_id })
            .expect("delete source location")
    );
    let connection = open_mutation_connection(&db_path);
    let remaining_rows: (i64, i64) = connection
        .query_row(
            "SELECT
                 (SELECT COUNT(*) FROM source_locations),
                 (SELECT COUNT(*) FROM browser_user_order WHERE node_domain = 'source_location')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("count source-location rows after delete");
    assert_eq!(remaining_rows, (0, 0));
}

#[test]
fn source_locations_reject_nested_accepted_locations_but_allow_siblings_and_observed_paths() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "nested-location-source".to_string(),
            display_name: "Nested Location Source".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 10,
        })
        .expect("insert source");

    let albums_location_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Albums".to_string(),
            display_name: Some("Albums".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            first_created_at: Some(20),
            changed_at: 20,
        })
        .expect("insert accepted source location");
    let observed_same_path_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "device".to_string(),
            location_kind: "observed_path".to_string(),
            relative_path: "Albums".to_string(),
            display_name: Some("Observed Albums".to_string()),
            is_user_visible: true,
            browser_order_ordinal: None,
            first_created_at: Some(21),
            changed_at: 21,
        })
        .expect("observed write over accepted path is ignored");
    assert_eq!(observed_same_path_id, albums_location_id);

    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Albums/1998".to_string(),
            display_name: Some("Albums 1998".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(1),
            first_created_at: Some(22),
            changed_at: 22,
        })
        .expect_err("accepted descendant source location is rejected");

    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Compilations".to_string(),
            display_name: Some("Compilations".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(1),
            first_created_at: Some(23),
            changed_at: 23,
        })
        .expect("accepted sibling source location is allowed");

    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "device".to_string(),
            location_kind: "observed_path".to_string(),
            relative_path: "Albums/1998".to_string(),
            display_name: Some("Observed Albums 1998".to_string()),
            is_user_visible: true,
            browser_order_ordinal: None,
            first_created_at: Some(24),
            changed_at: 24,
        })
        .expect("observed paths do not participate in accepted-location nesting");

    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Singles/1998".to_string(),
            display_name: Some("Singles 1998".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(2),
            first_created_at: Some(25),
            changed_at: 25,
        })
        .expect("accepted source location in unrelated subtree is allowed");

    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "Singles".to_string(),
            display_name: Some("Singles".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(3),
            first_created_at: Some(26),
            changed_at: 26,
        })
        .expect_err("accepted ancestor source location is rejected");
}

#[test]
fn source_directory_writes_do_not_reseed_navigation_projection() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "directory-source".to_string(),
            display_name: "Directory Source".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 10,
        })
        .expect("insert source");

    let connection = open_mutation_connection(&db_path);
    let navigation_row_count_before: i64 = connection
        .query_row("SELECT COUNT(*) FROM navigation_rows", [], |row| row.get(0))
        .expect("count navigation rows before source directory write");
    let navigation_change_count_before = navigation_projection_change_count(&connection);

    durable_store
        .upsert_source_directory(UpsertSourceDirectoryInput {
            source_directory_id: None,
            source_id,
            parent_source_directory_id: None,
            name: "inventory".to_string(),
            relative_path: "inventory".to_string(),
            presence_state: SourcePresenceState::Present,
            dir_scan_state: None,
            dir_scan_issue_kind: None,
            dir_scan_error_detail: None,
            scanned_at: None,
            mtime_ns: None,
            first_created_at: Some(20),
            changed_at: 20,
        })
        .expect("upsert source directory");

    let connection = open_mutation_connection(&db_path);
    let source_directory_row: (i64, String) = connection
        .query_row(
            "SELECT COUNT(*), MIN(name_browse_sort_key)
             FROM source_directories",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read source directory row");
    let navigation_row_count_after: i64 = connection
        .query_row("SELECT COUNT(*) FROM navigation_rows", [], |row| row.get(0))
        .expect("count navigation rows after source directory write");
    let navigation_change_count_after = navigation_projection_change_count(&connection);

    assert_eq!(source_directory_row.0, 1);
    assert!(!source_directory_row.1.is_empty());
    assert_eq!(navigation_row_count_after, navigation_row_count_before);
    assert_eq!(
        navigation_change_count_after,
        navigation_change_count_before
    );
}

#[test]
fn root_lifecycle_syncs_removable_roots_into_source_navigation_rows() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let mount_root = tempdir.path().join("mounted-volume");
    fs::create_dir_all(mount_root.join("Library")).expect("create mounted source directory");

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root_id = register_removable_root(&durable_store, "Library", "root-sync-volume");
    durable_store
        .apply_root_mounted(ApplyRootMountedInput {
            identity_kind: RootIdentityKind::FilesystemUuid,
            identity_value: "root-sync-volume".to_string(),
            mount_root: mount_root.clone(),
            observed_volume_label: Some("USB".to_string()),
            filesystem_type: Some("exfat".to_string()),
            event_at_ms: 1_000,
        })
        .expect("mount removable root");

    let connection = open_mutation_connection(&db_path);
    let source_row = connection
        .query_row(
            "SELECT source_id, identity_key, display_name
                 FROM sources",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("read source");
    assert_eq!(
        source_row.1,
        "stable_volume_identity:root:root-sync-volume:Library"
    );
    assert_eq!(source_row.2, "USB");

    let locator_row = connection
        .query_row(
            "SELECT locator_kind, device_identity_value, relative_suffix
                 FROM source_locators
                 WHERE source_id = ?1",
            [source_row.0],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("read source locator");
    assert_eq!(locator_row.0, "removable_volume");
    assert_eq!(locator_row.1.as_deref(), Some("root-sync-volume"));
    assert_eq!(locator_row.2, "Library");

    let state_row = connection
        .query_row(
            "SELECT ss.access_state, sss.scan_phase
                 FROM source_state ss
                 JOIN source_scan_state sss ON sss.source_id = ss.source_id
                 WHERE ss.source_id = ?1",
            [source_row.0],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read source state");
    assert_eq!(
        state_row,
        (
            SourceAccessState::Accessible.as_str().to_string(),
            SourceScanPhase::Idle.as_str().to_string(),
        )
    );

    let navigation_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM navigation_rows", [], |row| row.get(0))
        .expect("count navigation rows");
    assert_eq!(
        navigation_count,
        (FIXED_TOP_LEVEL_NAVIGATION_ROW_COUNT + 1) as i64
    );

    durable_store
        .apply_root_unmounted(ApplyRootUnmountedInput {
            identity_kind: RootIdentityKind::FilesystemUuid,
            identity_value: "root-sync-volume".to_string(),
            event_at_ms: 1_100,
        })
        .expect("unmount removable root");

    let unmounted_state: String = connection
        .query_row(
            "SELECT mount_status
                 FROM source_state
                 WHERE source_id = ?1",
            [source_row.0],
            |row| row.get(0),
        )
        .expect("read unmounted source state");
    assert_eq!(unmounted_state, RootMountStatus::Unmounted.as_str());
    assert!(root_id > 0);
}

#[test]
fn commit_discovery_syncs_source_rows_and_queues_inspection_work() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let root_path = tempdir.path().join("music");
    fs::create_dir_all(&root_path).expect("create root path");

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root = durable_store
        .bootstrap_root(&root_path)
        .expect("bootstrap root");

    durable_store
        .commit_discovery(DiscoveryBatch {
            root_id: root.root_id,
            scan_started_at_ms: 990,
            files: vec![DiscoveredFileInput {
                canonical_path: "artist/track.wav".to_string(),
                file_size_bytes: Some(4_200_000),
                modified_at_ns: Some(1_000_000),
                observed_at_ms: 1_000,
            }],
        })
        .expect("commit discovery");

    let connection = open_mutation_connection(&db_path);
    let source_directory_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM source_directories", [], |row| {
            row.get(0)
        })
        .expect("count source directories");
    let source_file_row = connection
        .query_row(
            "SELECT relative_path, presence_state
                 FROM source_files",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .expect("read source file row");
    let inspection_work_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
                 FROM WorkItems
                 WHERE subject_kind = 'source_file'
                   AND work_kind = 'inspect_source'",
            [],
            |row| row.get(0),
        )
        .expect("count inspection work");
    let navigation_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM navigation_rows", [], |row| row.get(0))
        .expect("count navigation rows");

    assert_eq!(source_directory_count, 1);
    assert_eq!(
        source_file_row,
        (
            "artist/track.wav".to_string(),
            SourcePresenceState::Present.as_str().to_string(),
        )
    );
    assert_eq!(inspection_work_count, 1);
    assert_eq!(
        navigation_count,
        (FIXED_TOP_LEVEL_NAVIGATION_ROW_COUNT + 1) as i64
    );
}

#[test]
fn register_local_root_initializes_lifecycle_side_rows() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let root_path = tempdir.path().join("registered-root");
    fs::create_dir_all(&root_path).expect("create root path");

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root = durable_store
        .register_local_root(RegisterLocalRootInput {
            absolute_path: root_path.clone(),
        })
        .expect("register local root");

    let connection = open_mutation_connection(&db_path);
    let state_row = connection
        .query_row(
            "SELECT ss.mount_status,
                    ss.access_state,
                    ss.effective_path,
                    sss.scan_phase
             FROM source_state ss
             JOIN source_scan_state sss ON sss.source_id = ss.source_id
             WHERE ss.source_id = ?1",
            [root.root_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .expect("read lifecycle side rows");

    assert_eq!(state_row.0, RootMountStatus::Mounted.as_str());
    assert_eq!(state_row.1, SourceAccessState::Accessible.as_str());
    let canonical_root_path = fs::canonicalize(&root_path)
        .expect("canonicalize root path")
        .to_string_lossy()
        .into_owned();
    assert_eq!(state_row.2.as_deref(), Some(canonical_root_path.as_str()));
    assert_eq!(state_row.3, SourceScanPhase::Idle.as_str());

    let lifecycle = durable_store
        .read_source_lifecycle(root.root_id)
        .expect("read source lifecycle")
        .expect("registered source lifecycle exists");
    assert_eq!(lifecycle.source_id, root.root_id);
    assert_eq!(lifecycle.mount_status, RootMountStatus::Mounted.as_str());
    assert_eq!(
        lifecycle.access_state,
        SourceAccessState::Accessible.as_str()
    );
    assert_eq!(lifecycle.scan_phase, SourceScanPhase::Idle.as_str());
}

#[test]
fn register_local_root_establishes_immediate_root_child_directories_before_scan() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let root_path = tempdir.path().join("registered-root");
    fs::create_dir_all(root_path.join("artists")).expect("create artists directory");
    fs::create_dir_all(root_path.join("crates")).expect("create crates directory");
    fs::write(root_path.join("loose.wav"), b"not-real-wav").expect("write loose file");

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root = durable_store
        .register_local_root(RegisterLocalRootInput {
            absolute_path: root_path.clone(),
        })
        .expect("register local root");

    let window = durable_store
        .read_literal_hierarchy_children(
            StoreLiteralHierarchyEntryPoint::Source {
                source_id: root.root_id,
            },
            None,
            0,
            50,
            SourceFileClassFilter::NavigationOnly,
        )
        .expect("read literal hierarchy")
        .expect("registered source has hierarchy window");

    assert_eq!(window.total_rows, 2);
    assert_eq!(
        window
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect::<Vec<_>>(),
        vec!["artists", "crates"]
    );
    assert!(window.rows.iter().all(|row| row.node_kind == "directory"));
    assert!(
        window
            .rows
            .iter()
            .all(|row| row.dir_scan_state.as_deref() == Some("pending"))
    );
    assert_eq!(
        window.coverage.state,
        StoreLiteralHierarchyCoverageState::Pending
    );
    assert!(!window.coverage.subtree_coverage_complete);
    assert!(!window.coverage.empty_result_authoritative);

    let connection = open_mutation_connection(&db_path);
    let source_file_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
        .expect("count source files");
    assert_eq!(
        source_file_count, 0,
        "root navigation establishment must not ingest files"
    );
}

#[test]
fn root_scan_materialization_records_files_and_queues_source_work() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let root_path = tempdir.path().join("scan-root");
    write_valid_wav_file(&root_path.join("artist").join("one.wav"));
    write_valid_wav_file(&root_path.join("artist").join("two.wav"));

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root = durable_store
        .bootstrap_root(&root_path)
        .expect("bootstrap root");
    durable_store
        .mark_root_scan_started(root.root_id, 1_000)
        .expect("mark scan started");

    let mut observations = Vec::new();
    let result = durable_store
        .execute_root_scan_materialization_with_observer(
            root.root_id,
            &root_path,
            1_000,
            |observation| observations.push(observation),
        )
        .expect("execute root scan");

    let connection = open_mutation_connection(&db_path);
    let source_file_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
        .expect("count source files");
    let inspection_work_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
                 FROM WorkItems
                 WHERE subject_kind = 'source_file'
                   AND work_kind = 'inspect_source'",
            [],
            |row| row.get(0),
        )
        .expect("count inspection work");
    let scan_phase: String = connection
        .query_row(
            "SELECT scan_phase
             FROM source_scan_state
             WHERE source_id = ?1",
            [root.root_id],
            |row| row.get(0),
        )
        .expect("read scan phase");
    let incomplete_directory_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM source_directories
             WHERE source_id = ?1
               AND dir_scan_state <> 'complete'",
            [root.root_id],
            |row| row.get(0),
        )
        .expect("count incomplete directories");

    assert_eq!(result.discovered_file_count, 2);
    assert_eq!(result.queued_source_work_items, 2);
    assert_eq!(source_file_count, 2);
    assert_eq!(inspection_work_count, 2);
    assert_eq!(scan_phase, SourceScanPhase::Complete.as_str());
    assert_eq!(incomplete_directory_count, 0);
    assert!(observations.iter().any(|observation| matches!(
        observation,
        RootScanObservation::HierarchyPublished {
            reason: RootScanHierarchyObservationReason::Finalized,
            ..
        }
    )));
    assert!(observations.iter().any(|observation| matches!(
        observation,
        RootScanObservation::SourceWorkQueued {
            queued_work_items: 2,
            ..
        }
    )));
}

#[test]
fn stale_scan_completion_still_rejects_old_mount_epochs() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let mount_root = tempdir.path().join("mounted-volume");
    fs::create_dir_all(&mount_root).expect("create mounted-volume directory");
    for index in 0..300 {
        let file_path = mount_root
            .join("Library")
            .join("artist")
            .join(format!("track-{index}.wav"));
        write_valid_wav_file(&file_path);
    }

    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");
    let root_id = register_removable_root(&durable_store, "Library", "scan-volume");
    durable_store
        .apply_root_mounted(ApplyRootMountedInput {
            identity_kind: RootIdentityKind::FilesystemUuid,
            identity_value: "scan-volume".to_string(),
            mount_root: mount_root.clone(),
            observed_volume_label: Some("USB".to_string()),
            filesystem_type: Some("exfat".to_string()),
            event_at_ms: 1_000,
        })
        .expect("mount removable root");
    durable_store
        .mark_root_scan_started(root_id, 1_100)
        .expect("mark root scanning");

    let durable_store_for_observer = durable_store.clone();
    let remounted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let remounted_for_observer = std::sync::Arc::clone(&remounted);
    let error = durable_store
        .execute_root_scan_materialization_with_observer(
            root_id,
            &mount_root.join("Library"),
            1_100,
            move |observation| {
                let RootScanObservation::HierarchyPublished {
                    reason: RootScanHierarchyObservationReason::ChunkCommitted,
                    ..
                } = observation
                else {
                    return;
                };
                if remounted_for_observer.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                durable_store_for_observer
                    .apply_root_unmounted(ApplyRootUnmountedInput {
                        identity_kind: RootIdentityKind::FilesystemUuid,
                        identity_value: "scan-volume".to_string(),
                        event_at_ms: 1_200,
                    })
                    .expect("unmount during scan");
                durable_store_for_observer
                    .apply_root_mounted(ApplyRootMountedInput {
                        identity_kind: RootIdentityKind::FilesystemUuid,
                        identity_value: "scan-volume".to_string(),
                        mount_root: mount_root.clone(),
                        observed_volume_label: Some("USB".to_string()),
                        filesystem_type: Some("exfat".to_string()),
                        event_at_ms: 1_300,
                    })
                    .expect("remount during scan");
            },
        )
        .expect_err("stale scan completion must be rejected");

    assert!(matches!(
        error,
        crate::LibrarySqliteError::StaleMountEpochStamp {
            root_id: stale_root_id,
            admitted_mount_epoch: 1,
            current_mount_epoch: 3,
        } if stale_root_id == root_id
    ));
}

#[test]
fn store_source_and_promotion_flows_drive_navigation_and_library_browser() {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("library.sqlite3");
    let durable_store = SqliteDurableStore::open(&db_path).expect("open durable store");

    let source_id = durable_store
        .upsert_source(UpsertSourceInput {
            source_id: None,
            source_class: "internal".to_string(),
            authority: "system".to_string(),
            identity_kind: "filesystem_uuid".to_string(),
            identity_value: "store-flow-volume".to_string(),
            display_name: "Store Flow".to_string(),
            medium_label: None,
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            changed_at: 10,
        })
        .expect("upsert source");
    durable_store
        .upsert_source_locator(UpsertSourceLocatorInput {
            source_id,
            locator: crate::SourceLocatorInput::AbsolutePath {
                absolute_path: "C:/music".to_string(),
            },
        })
        .expect("upsert source locator");
    durable_store
        .upsert_source_state(UpsertSourceStateInput {
            source_id,
            mount_status: RootMountStatus::Mounted.as_str().to_string(),
            mount_epoch: 0,
            access_state: SourceAccessState::Accessible,
            access_issue_kind: None,
            access_error_detail: None,
            access_checked_at: Some(13),
            mount_root: Some("C:/music".to_string()),
            effective_path: Some("C:/music".to_string()),
            observed_volume_label: Some("Volume".to_string()),
            filesystem_type: Some("exfat".to_string()),
            last_seen_at: Some(11),
            updated_at: 13,
        })
        .expect("upsert source state");
    durable_store
        .upsert_source_scan_state(UpsertSourceScanStateInput {
            source_id,
            scan_phase: SourceScanPhase::Idle,
            last_scan_started_at: Some(12),
            last_scan_finished_at: Some(13),
            last_successful_scan_at: Some(13),
            scan_issue_kind: None,
            error_detail: None,
            updated_at: 13,
        })
        .expect("upsert source state");
    let source_directory_id = durable_store
        .upsert_source_directory(UpsertSourceDirectoryInput {
            source_directory_id: None,
            source_id,
            parent_source_directory_id: None,
            name: "album".to_string(),
            relative_path: "album".to_string(),
            presence_state: SourcePresenceState::Present,
            dir_scan_state: None,
            dir_scan_issue_kind: None,
            dir_scan_error_detail: None,
            scanned_at: None,
            mtime_ns: None,
            first_created_at: Some(14),
            changed_at: 14,
        })
        .expect("upsert source directory");
    let source_file_id = durable_store
        .record_source_file_observation(crate::RecordSourceFileObservationInput {
            source_file_id: None,
            source_id,
            parent_source_directory_id: Some(source_directory_id),
            name: "track.wav".to_string(),
            relative_path: "album/track.wav".to_string(),
            size_bytes: Some(12_345),
            mtime_ns: Some(555),
            presence_state: SourcePresenceState::Present,
            first_discovered_at: Some(15),
            observed_at: Some(15),
            presence_changed_at: 15,
            updated_at: 15,
        })
        .expect("record source file observation");
    let connection = open_mutation_connection(&db_path);
    let source_file_sort_keys: (String, String) = connection
        .query_row(
            "SELECT name_browse_sort_key, relative_path_browse_sort_key
             FROM source_files
             WHERE source_file_id = ?1",
            [source_file_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read source file sort keys");
    assert!(!source_file_sort_keys.0.is_empty());
    assert!(!source_file_sort_keys.1.is_empty());
    let source_basis_fingerprint = super::sources::source_observation_basis_fingerprint(
        source_file_id,
        "album/track.wav",
        Some(12_345),
        Some(555),
        15,
    );

    let claimed_inspection = claim_single_work_item(&durable_store, 20);
    let inspection_artifact_id = record_completed_inline_artifact(
        &durable_store,
        claimed_inspection.work_item_id,
        ArtifactKind::InspectionResult,
        &source_basis_fingerprint,
        21,
        "hash:inspect:1",
    );
    durable_store
        .inspect_source(InspectSourcePromotionInput {
            source_facts: CommitAcceptedSourceFactsInput {
                source_file_id: SourceFileId::new(source_file_id).expect("positive source file id"),
                accepted_artifact_id: library_domain::ArtifactId::new(inspection_artifact_id)
                    .expect("positive artifact id"),
                basis_fingerprint: source_basis_fingerprint.clone(),
                observed_at_ms: 24,
                content_hash: Some(ContentHashEvidence {
                    algorithm: "sha256".to_string(),
                    value: "store-flow".to_string(),
                }),
                media_kind: "audio".to_string(),
                mime_type: Some("audio/wav".to_string()),
                duration_ms: Some(180_000),
                sample_rate_hz: Some(44_100),
                channels: Some(2),
                bit_depth: Some(16),
                codec: Some("pcm".to_string()),
                updated_at: 24,
            },
            source_facts_merge_policy: CommitAcceptedSourceFactsMergePolicy::replacement(),
            rebuild_projection_domains: vec![library_domain::ProjectionDomain::LibraryBrowser],
            rebuild_priority: WorkPriorityClass::Interactive,
        })
        .expect("promote inspection");
    durable_store
        .complete_machine_work_item(CompleteMachineWorkInput {
            work_item_id: claimed_inspection.work_item_id,
            completed_at: 25,
        })
        .expect("complete inspection work item");
    let claimed_projection = claim_single_work_item(&durable_store, 26);
    assert_eq!(
        claimed_projection.work_kind,
        library_domain::MachineWorkKind::RebuildProjection
    );
    record_completed_inline_artifact(
        &durable_store,
        claimed_projection.work_item_id,
        ArtifactKind::ProjectionSnapshot,
        &source_basis_fingerprint,
        27,
        "hash:projection:inspect:1",
    );
    durable_store
        .complete_machine_work_item(CompleteMachineWorkInput {
            work_item_id: claimed_projection.work_item_id,
            completed_at: 29,
        })
        .expect("complete projection rebuild work item");

    durable_store
        .queue_accept_segmentation_work(crate::QueueAcceptSegmentationWorkInput {
            source_file_id: SourceFileId::new(source_file_id).expect("positive source file id"),
            basis_fingerprint: source_basis_fingerprint.clone(),
            priority_class: WorkPriorityClass::Interactive,
            queued_at: 30,
        })
        .expect("queue segmentation work");
    let claimed_segmentation = claim_single_work_item(&durable_store, 31);
    let segmentation_artifact_id = record_completed_inline_artifact(
        &durable_store,
        claimed_segmentation.work_item_id,
        ArtifactKind::SegmentationResult,
        &source_basis_fingerprint,
        32,
        "hash:segments:1",
    );
    durable_store
        .accept_segmentation(AcceptSegmentationPromotionInput {
            segment_set: ReplaceAcceptedSourceSegmentSetInput {
                source_segment_set_id: Some(
                    SourceSegmentSetId::new(400).expect("positive source segment set id"),
                ),
                source_file_id: SourceFileId::new(source_file_id).expect("positive source file id"),
                segment_set_kind: "accepted_primary".to_string(),
                basis_fingerprint: source_basis_fingerprint.clone(),
                accepted_artifact_id: library_domain::ArtifactId::new(segmentation_artifact_id)
                    .expect("positive artifact id"),
                accepted_at: 33,
                updated_at: 33,
                segments: vec![crate::AcceptedSourceSegmentInput {
                    source_segment_id: Some(
                        SourceSegmentId::new(500).expect("positive source segment id"),
                    ),
                    segment_kind: "track_span".to_string(),
                    ordinal: 0,
                    start_offset_ms: 0,
                    end_offset_ms: Some(180_000),
                    display_title: Some("Store Track".to_string()),
                    display_artist: Some("Store Artist".to_string()),
                    display_album: Some("Store Album".to_string()),
                }],
            },
            rebuild_projection_domains: vec![library_domain::ProjectionDomain::LibraryBrowser],
            rebuild_priority: WorkPriorityClass::Interactive,
        })
        .expect("promote segmentation");
    durable_store
        .complete_machine_work_item(CompleteMachineWorkInput {
            work_item_id: claimed_segmentation.work_item_id,
            completed_at: 34,
        })
        .expect("complete segmentation work item");

    durable_store
        .resolve_library_asset(ResolveLibraryAssetPromotionInput {
            equivalence_fingerprint: "eq:store-flow".to_string(),
            retention_policy: library_domain::LibraryAssetRetentionPolicy::KeepMetadata,
            source_segment_ids: vec![
                SourceSegmentId::new(500).expect("positive source segment id"),
            ],
            accepted_at: 40,
            updated_at: 40,
            rebuild_projection_domains: vec![
                library_domain::ProjectionDomain::Navigation,
                library_domain::ProjectionDomain::LibraryBrowser,
            ],
            rebuild_priority: WorkPriorityClass::Interactive,
        })
        .expect("resolve library asset");

    let top_level_navigation_rows = durable_store
        .read_navigation_rows(None)
        .expect("read top-level navigation rows");
    let all_media_row = top_level_navigation_rows
        .iter()
        .find(|row| row.stable_key == "view:all_media")
        .expect("all-media view row exists");
    let source_row = top_level_navigation_rows
        .iter()
        .find(|row| row.stable_key == format!("source:{source_id}"))
        .expect("source navigation row exists");
    let source_children = durable_store
        .read_navigation_rows(Some(source_row.navigation_row_id))
        .expect("read source child rows");
    let library_browser = durable_store
        .read_library_browser_window(0, 10)
        .expect("read library browser rows");

    assert_eq!(
        top_level_navigation_rows.len(),
        FIXED_TOP_LEVEL_NAVIGATION_ROW_COUNT + 1
    );
    assert_fixed_top_level_navigation_rows(&top_level_navigation_rows);
    assert_eq!(all_media_row.family.as_deref(), Some("Views"));
    assert_eq!(source_row.family.as_deref(), Some("Sources"));
    let expected_selector = encode_selector(&NavigationSelector::Source(
        SourceId::new(source_id).expect("positive source id"),
    ));
    assert_eq!(
        source_row.selector_kind.as_deref(),
        Some(expected_selector.kind)
    );
    assert_eq!(
        source_row.selector_payload.as_deref(),
        Some(expected_selector.payload.as_str())
    );
    assert!(
        source_children.is_empty(),
        "source directories must remain inventory substrate and not project as browser locations"
    );
    durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "device".to_string(),
            location_kind: "observed_path".to_string(),
            relative_path: "observed-album".to_string(),
            display_name: Some("Observed Album".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            first_created_at: Some(40),
            changed_at: 40,
        })
        .expect("upsert observed source location");
    let source_children = durable_store
        .read_navigation_rows(Some(source_row.navigation_row_id))
        .expect("read source children with observed location");
    assert!(
        source_children.is_empty(),
        "observed source locations must not project as accepted navigation rows"
    );
    let album_location_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "album".to_string(),
            display_name: Some("Album".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(1),
            first_created_at: Some(41),
            changed_at: 41,
        })
        .expect("upsert explicit source location");
    let crates_location_id = durable_store
        .upsert_source_location(UpsertSourceLocationInput {
            source_location_id: None,
            source_id,
            authority: "user".to_string(),
            location_kind: "registered_subpath".to_string(),
            relative_path: "crates".to_string(),
            display_name: Some("Crates".to_string()),
            is_user_visible: true,
            browser_order_ordinal: Some(0),
            first_created_at: Some(42),
            changed_at: 42,
        })
        .expect("upsert ordered source location");
    let source_children = durable_store
        .read_navigation_rows(Some(source_row.navigation_row_id))
        .expect("read source children with source location");
    assert!(
        source_children
            .iter()
            .all(|row| row.row_kind != "location-group"),
        "accepted source locations must project directly under the source"
    );
    assert_eq!(source_children.len(), 2);
    assert_eq!(source_children[0].row_kind, "location");
    assert_eq!(
        source_children[0].stable_key,
        format!("source_location:{crates_location_id}")
    );
    assert_eq!(source_children[0].sibling_position, 0);
    assert_eq!(
        source_children[0].selector_kind.as_deref(),
        Some("source_location")
    );
    assert_eq!(source_children[1].row_kind, "location");
    assert_eq!(
        source_children[1].stable_key,
        format!("source_location:{album_location_id}")
    );
    assert_eq!(source_children[1].sibling_position, 1);
    assert_eq!(
        source_children[1].selector_kind.as_deref(),
        Some("source_location")
    );
    assert_eq!(library_browser.total_rows, 1);
    assert_eq!(library_browser.rows.len(), 1);
    assert_eq!(
        library_browser.rows[0].title.as_deref(),
        Some("Store Track")
    );

    let connection = open_mutation_connection(&db_path);
    let accepted_facts_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM SourceFacts", [], |row| row.get(0))
        .expect("count source facts");
    let library_asset_row_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM LibraryBrowserRows", [], |row| {
            row.get(0)
        })
        .expect("count library browser rows");
    assert_eq!(accepted_facts_count, 1);
    assert_eq!(library_asset_row_count, 1);
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*)
                     FROM WorkItems
                     WHERE subject_kind = 'source_file'
                       AND work_kind = 'inspect_source'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("count inspection work items"),
        1
    );
}
