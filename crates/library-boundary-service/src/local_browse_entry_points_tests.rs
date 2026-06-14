use std::path::{Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;
use library_store_sqlite::{LibraryStoreContext, StoreEnvironment, durable_store_path};
use rusqlite::Connection;
use tempfile::TempDir;

#[cfg(windows)]
use crate::local_browse_entry_points::windows_path_status_probe_from_error;
use crate::local_browse_entry_points::{
    LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure,
    LocalBrowseEntryPointResolver, LocalBrowsePathStatusProbe, LocalBrowsePathStatusResolver,
    ResolvedLocalBrowseEntryPoint, status_for_path, status_for_path_with_resolver,
};
use crate::service::LibraryBoundaryService;

#[derive(Clone)]
struct FakeLocalBrowseEntryPointResolver {
    resolution: Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure>,
}

impl LocalBrowseEntryPointResolver for FakeLocalBrowseEntryPointResolver {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure> {
        self.resolution.clone()
    }
}

struct FakeLocalBrowsePathStatusResolver {
    probe: LocalBrowsePathStatusProbe,
}

impl LocalBrowsePathStatusResolver for FakeLocalBrowsePathStatusResolver {
    fn resolve_status(&self, _path: &Path) -> LocalBrowsePathStatusProbe {
        self.probe
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRowCount {
    table: String,
    rows: i64,
}

fn open_service_with_fake_local_browse_entries(
    resolution: LocalBrowseEntryPointResolution,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    open_service_with_local_browse_resolution(Ok(resolution))
}

fn open_service_with_failing_local_browse_resolver(
    failure: LocalBrowseEntryPointResolveFailure,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    open_service_with_local_browse_resolution(Err(failure))
}

fn open_service_with_local_browse_resolution(
    resolution: Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure>,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    let tempdir = TempDir::new().expect("create tempdir");
    let context = LibraryStoreContext {
        user_data_path: tempdir.path().to_string_lossy().into_owned(),
        environment: StoreEnvironment::Development,
    };
    let db_path = durable_store_path(&context.user_data_path, context.environment);
    std::fs::create_dir_all(db_path.parent().expect("database parent"))
        .expect("create durable store parent");
    let durable_store =
        library_store_sqlite::SqliteDurableStore::open(&db_path).expect("open durable store");
    let service = LibraryBoundaryService::from_store_with_local_browse_entry_point_resolver(
        durable_store,
        Arc::new(FakeLocalBrowseEntryPointResolver { resolution }),
    )
    .expect("open service with fake resolver");

    (tempdir, context, service)
}

fn fake_local_browse_resolution(
    entries: Vec<ResolvedLocalBrowseEntryPoint>,
) -> LocalBrowseEntryPointResolution {
    LocalBrowseEntryPointResolution {
        entries,
        failure: None,
    }
}

fn fake_local_browse_entry(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    resolved_path: impl Into<PathBuf>,
    display_name: &str,
    status: protocol::LocalBrowseEntryPointStatus,
) -> ResolvedLocalBrowseEntryPoint {
    ResolvedLocalBrowseEntryPoint {
        entry_point_kind,
        resolved_path: Some(resolved_path.into()),
        display_name: display_name.to_string(),
        status,
        platform: protocol::LocalBrowsePlatform::Windows,
        failure: None,
    }
}

fn fake_unresolved_local_browse_entry(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    display_name: &str,
    status: protocol::LocalBrowseEntryPointStatus,
    failure_code: protocol::LocalBrowseEntryPointFailureCode,
) -> ResolvedLocalBrowseEntryPoint {
    ResolvedLocalBrowseEntryPoint {
        entry_point_kind,
        resolved_path: None,
        display_name: display_name.to_string(),
        status,
        platform: protocol::LocalBrowsePlatform::Windows,
        failure: Some(LocalBrowseEntryPointResolveFailure {
            code: failure_code,
            detail: "fixture failure".to_string(),
        }),
    }
}

fn open_test_read_connection(context: &LibraryStoreContext) -> Connection {
    Connection::open(durable_store_path(
        &context.user_data_path,
        context.environment,
    ))
    .expect("open test read connection")
}

fn count_rows(context: &LibraryStoreContext, table: &str) -> i64 {
    open_test_read_connection(context)
        .query_row(
            &format!("SELECT COUNT(*) FROM {}", quote_sql_identifier(table)),
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("count rows in {table}: {error}"))
}

fn application_table_row_counts(context: &LibraryStoreContext) -> Vec<TableRowCount> {
    let connection = open_test_read_connection(context);
    let mut statement = connection
        .prepare(
            "SELECT name
             FROM sqlite_schema
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("prepare application table query");
    let tables = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query application tables")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect application tables");

    tables
        .into_iter()
        .map(|table| {
            let rows = connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM {}", quote_sql_identifier(&table)),
                    [],
                    |row| row.get(0),
                )
                .unwrap_or_else(|error| panic!("count rows in {table}: {error}"));
            TableRowCount { table, rows }
        })
        .collect()
}

fn quote_sql_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn expect_success(outcome: protocol::CommandOutcome) -> protocol::CommandReply {
    match outcome {
        protocol::CommandOutcome::Success(envelope) => envelope.reply,
        protocol::CommandOutcome::Error(envelope) => {
            panic!("expected success, got error {:?}", envelope.error)
        }
    }
}

fn expect_register_local_root_reply(
    reply: protocol::CommandReply,
) -> protocol::RegisterLocalRootReply {
    match reply {
        protocol::CommandReply::LibraryRoots(protocol::LibraryRootReply::RegisterLocalRoot(
            reply,
        )) => reply,
        other => panic!("expected register local root reply, got {other:?}"),
    }
}

fn expect_unregister_local_root_reply(
    reply: protocol::CommandReply,
) -> protocol::UnregisterLocalRootReply {
    match reply {
        protocol::CommandReply::LibraryRoots(protocol::LibraryRootReply::UnregisterLocalRoot(
            reply,
        )) => reply,
        other => panic!("expected unregister local root reply, got {other:?}"),
    }
}

fn expect_local_browse_entry_points_reply(
    reply: protocol::CommandReply,
) -> protocol::ReadLocalBrowseEntryPointsReply {
    match reply {
        protocol::CommandReply::SnapshotRead(
            protocol::SnapshotReadReply::LocalBrowseEntryPoints(reply),
        ) => reply,
        other => panic!("expected local browse entry points reply, got {other:?}"),
    }
}

fn read_local_browse_entry_points(
    service: &LibraryBoundaryService,
) -> protocol::ReadLocalBrowseEntryPointsReply {
    expect_local_browse_entry_points_reply(expect_success(service.handle_command(
        protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowseEntryPoints(
                protocol::ReadLocalBrowseEntryPointsRequest,
            ),
        ),
    )))
}

fn register_local_root(
    service: &LibraryBoundaryService,
    requested_path: String,
) -> protocol::RegisteredLocalRoot {
    match expect_register_local_root_reply(expect_success(service.handle_command(
        protocol::CommandRequest::LibraryRoots(protocol::LibraryRootCommand::RegisterLocalRoot(
            protocol::RegisterLocalRootRequest { requested_path },
        )),
    ))) {
        protocol::RegisterLocalRootReply::Registered(root) => root,
        other => panic!("expected registered local root reply, got {other:?}"),
    }
}

fn unregister_local_root(service: &LibraryBoundaryService, root_id: i64) {
    let reply = expect_unregister_local_root_reply(expect_success(service.handle_command(
        protocol::CommandRequest::LibraryRoots(protocol::LibraryRootCommand::UnregisterLocalRoot(
            protocol::UnregisterLocalRootRequest { root_id },
        )),
    )));
    assert!(reply.unregistered);
}

fn has_browse_children_operation(operations: &[protocol::LocalBrowseOperation]) -> bool {
    operations
        .iter()
        .any(|operation| matches!(operation, protocol::LocalBrowseOperation::BrowseChildren))
}

fn has_source_admission_operation(operations: &[protocol::LocalBrowseOperation]) -> bool {
    operations.iter().any(|operation| {
        matches!(
            operation,
            protocol::LocalBrowseOperation::RequestSourceAdmission { .. }
        )
    })
}

fn has_source_admission_request_kind(
    operations: &[protocol::LocalBrowseOperation],
    expected: protocol::LocalBrowseSourceAdmissionRequestKind,
) -> bool {
    operations.iter().any(|operation| {
        matches!(
            operation,
            protocol::LocalBrowseOperation::RequestSourceAdmission { request_kind, .. }
                if *request_kind == expected
        )
    })
}

#[test]
fn local_browse_path_status_resolver_maps_probe_results() {
    let cases = [
        (
            LocalBrowsePathStatusProbe::Directory,
            protocol::LocalBrowseEntryPointStatus::Available,
        ),
        (
            LocalBrowsePathStatusProbe::NonDirectory,
            protocol::LocalBrowseEntryPointStatus::Unavailable,
        ),
        (
            LocalBrowsePathStatusProbe::NotFound,
            protocol::LocalBrowseEntryPointStatus::Missing,
        ),
        (
            LocalBrowsePathStatusProbe::PermissionBlocked,
            protocol::LocalBrowseEntryPointStatus::PermissionBlocked,
        ),
        (
            LocalBrowsePathStatusProbe::Unavailable,
            protocol::LocalBrowseEntryPointStatus::Unavailable,
        ),
    ];

    for (probe, expected_status) in cases {
        let resolver = FakeLocalBrowsePathStatusResolver { probe };
        assert_eq!(
            status_for_path_with_resolver(&resolver, Path::new("fixture")),
            expected_status
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_local_browse_path_status_resolver_maps_win32_errors() {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_BAD_NETPATH, ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE,
        ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND,
    };

    assert_eq!(
        windows_path_status_probe_from_error(ERROR_FILE_NOT_FOUND),
        LocalBrowsePathStatusProbe::NotFound
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_PATH_NOT_FOUND),
        LocalBrowsePathStatusProbe::NotFound
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_ACCESS_DENIED),
        LocalBrowsePathStatusProbe::PermissionBlocked
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_BAD_NETPATH),
        LocalBrowsePathStatusProbe::Unavailable
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE),
        LocalBrowsePathStatusProbe::Unavailable
    );
}

#[test]
fn production_local_browse_path_status_resolver_reads_shallow_path_state() {
    let tempdir = TempDir::new().expect("create tempdir");
    let file_path = tempdir.path().join("not-a-directory.txt");
    std::fs::write(&file_path, b"fixture").expect("write fixture file");

    assert_eq!(
        status_for_path(tempdir.path()),
        protocol::LocalBrowseEntryPointStatus::Available
    );
    assert_eq!(
        status_for_path(&file_path),
        protocol::LocalBrowseEntryPointStatus::Unavailable
    );
    assert_eq!(
        status_for_path(&tempdir.path().join("missing")),
        protocol::LocalBrowseEntryPointStatus::Missing
    );
}

#[test]
fn local_browse_entry_points_read_returns_entries_without_source_rows() {
    let fixture_tempdir = TempDir::new().expect("create fixture paths");
    let system_root = fixture_tempdir.path().join("system-root");
    let music_root = fixture_tempdir.path().join("Music");
    let home_root = fixture_tempdir.path().join("home");
    std::fs::create_dir_all(&system_root).expect("create system root fixture");
    std::fs::create_dir_all(&music_root).expect("create music root fixture");
    std::fs::create_dir_all(&home_root).expect("create home root fixture");

    let (_tempdir, context, service) =
        open_service_with_fake_local_browse_entries(fake_local_browse_resolution(vec![
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
                system_root,
                "C:\\",
                protocol::LocalBrowseEntryPointStatus::Available,
            ),
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::Music,
                music_root,
                "Music",
                protocol::LocalBrowseEntryPointStatus::Available,
            ),
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::UserHome,
                home_root,
                "Home",
                protocol::LocalBrowseEntryPointStatus::Available,
            ),
        ]));
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browse_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowseEntryPointsReadStatus::Complete
    );
    assert_eq!(reply.entries.len(), 3);
    assert_eq!(
        reply.entries[0].identity.entry_point_kind,
        protocol::LocalBrowseEntryPointKind::SystemDriveRoot
    );
    assert!(has_browse_children_operation(
        &reply.entries[0].available_operations
    ));
    assert!(!has_source_admission_operation(
        &reply.entries[0].available_operations
    ));
    assert_eq!(
        reply.entries[1].identity.entry_point_kind,
        protocol::LocalBrowseEntryPointKind::Music
    );
    assert!(has_source_admission_request_kind(
        &reply.entries[1].available_operations,
        protocol::LocalBrowseSourceAdmissionRequestKind::DefaultMusicFolder
    ));
    assert!(has_source_admission_request_kind(
        &reply.entries[2].available_operations,
        protocol::LocalBrowseSourceAdmissionRequestKind::SelectedDirectory
    ));
    assert_eq!(count_rows(&context, "sources"), 0);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "local browse entry point read must not mutate any durable application table"
    );
}

#[test]
fn local_browse_entry_points_exact_admitted_source_match_is_duplicate_entry() {
    let fixture_tempdir = TempDir::new().expect("create fixture paths");
    let source_root = fixture_tempdir.path().join("Music");
    std::fs::create_dir_all(&source_root).expect("create source root");

    let (_tempdir, context, service) =
        open_service_with_fake_local_browse_entries(fake_local_browse_resolution(vec![
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::Music,
                source_root.clone(),
                "Music",
                protocol::LocalBrowseEntryPointStatus::Available,
            ),
        ]));
    let registered = register_local_root(&service, source_root.to_string_lossy().into_owned());
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browse_entry_points(&service);

    assert_eq!(reply.entries.len(), 1);
    assert_eq!(
        reply.entries[0].status,
        protocol::LocalBrowseEntryPointStatus::DuplicateOfAdmittedSource
    );
    assert_eq!(reply.entries[0].matched_source_id, Some(registered.root_id));
    assert!(!has_source_admission_operation(
        &reply.entries[0].available_operations
    ));
    assert_eq!(count_rows(&context, "sources"), 1);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "duplicate entry point read must not mutate any durable application table"
    );
}

#[test]
fn local_browse_entry_points_removed_source_match_is_restorable_entry() {
    let fixture_tempdir = TempDir::new().expect("create fixture paths");
    let source_root = fixture_tempdir.path().join("Music");
    std::fs::create_dir_all(&source_root).expect("create source root");

    let (_tempdir, context, service) =
        open_service_with_fake_local_browse_entries(fake_local_browse_resolution(vec![
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::Music,
                source_root.clone(),
                "Music",
                protocol::LocalBrowseEntryPointStatus::Available,
            ),
        ]));
    let registered = register_local_root(&service, source_root.to_string_lossy().into_owned());
    unregister_local_root(&service, registered.root_id);
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browse_entry_points(&service);

    assert_eq!(reply.entries.len(), 1);
    assert_eq!(
        reply.entries[0].status,
        protocol::LocalBrowseEntryPointStatus::RestorableSource
    );
    assert_eq!(reply.entries[0].matched_source_id, Some(registered.root_id));
    assert!(has_source_admission_operation(
        &reply.entries[0].available_operations
    ));
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "restorable entry point read must not mutate any durable application table"
    );

    let restored = register_local_root(&service, source_root.to_string_lossy().into_owned());
    assert_eq!(restored.root_id, registered.root_id);
    let active_reply = read_local_browse_entry_points(&service);
    assert_eq!(
        active_reply.entries[0].status,
        protocol::LocalBrowseEntryPointStatus::DuplicateOfAdmittedSource
    );
}

#[test]
fn local_browse_entry_points_statuses_do_not_create_lifecycle_state() {
    let missing_path = PathBuf::from("Z:\\Missing-Music");
    let (_tempdir, context, service) =
        open_service_with_fake_local_browse_entries(fake_local_browse_resolution(vec![
            fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::Downloads,
                missing_path,
                "Downloads",
                protocol::LocalBrowseEntryPointStatus::Missing,
            ),
            fake_unresolved_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::Desktop,
                "Desktop",
                protocol::LocalBrowseEntryPointStatus::PermissionBlocked,
                protocol::LocalBrowseEntryPointFailureCode::MetadataUnavailable,
            ),
            fake_unresolved_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::RemovableVolumeRoot,
                "USB",
                protocol::LocalBrowseEntryPointStatus::Unavailable,
                protocol::LocalBrowseEntryPointFailureCode::VolumeEnumerationUnavailable,
            ),
        ]));
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browse_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowseEntryPointsReadStatus::Complete
    );
    assert_eq!(
        reply
            .entries
            .iter()
            .map(|entry| entry.status)
            .collect::<Vec<_>>(),
        vec![
            protocol::LocalBrowseEntryPointStatus::Missing,
            protocol::LocalBrowseEntryPointStatus::PermissionBlocked,
            protocol::LocalBrowseEntryPointStatus::Unavailable
        ]
    );
    assert!(
        reply
            .entries
            .iter()
            .all(|entry| !has_source_admission_operation(&entry.available_operations))
    );
    assert!(
        reply
            .entries
            .iter()
            .all(|entry| !has_browse_children_operation(&entry.available_operations))
    );
    assert_eq!(count_rows(&context, "source_state"), 0);
    assert_eq!(count_rows(&context, "source_scan_state"), 0);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "permission, missing, and unavailable entry point statuses are not source lifecycle state"
    );
}

#[test]
fn local_browse_entry_points_read_reports_partial_failure_without_writes() {
    let (_tempdir, context, service) =
        open_service_with_fake_local_browse_entries(LocalBrowseEntryPointResolution {
            entries: vec![fake_local_browse_entry(
                protocol::LocalBrowseEntryPointKind::UserHome,
                "C:\\Users\\DJ",
                "Home",
                protocol::LocalBrowseEntryPointStatus::Available,
            )],
            failure: Some(LocalBrowseEntryPointResolveFailure {
                code: protocol::LocalBrowseEntryPointFailureCode::VolumeEnumerationUnavailable,
                detail: "volume enumeration failed".to_string(),
            }),
        });
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browse_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowseEntryPointsReadStatus::PartialFailure
    );
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseEntryPointFailureCode::VolumeEnumerationUnavailable)
    );
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "platform resolver partial failures must not be converted into durable writes"
    );
}

#[test]
fn local_browse_entry_points_read_failure_has_no_durable_writes() {
    let (_tempdir, context, service) =
        open_service_with_failing_local_browse_resolver(LocalBrowseEntryPointResolveFailure {
            code: protocol::LocalBrowseEntryPointFailureCode::MetadataUnavailable,
            detail: "metadata probe failed".to_string(),
        });
    let before_counts = application_table_row_counts(&context);

    let error = service
        .try_handle_command(protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowseEntryPoints(
                protocol::ReadLocalBrowseEntryPointsRequest,
            ),
        ))
        .expect_err("resolver failure must be a read failure");

    match error {
        protocol::ProtocolError::HostFailure { detail } => {
            assert_eq!(detail, "metadata probe failed");
        }
        other => panic!("expected host failure, got {other:?}"),
    }
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "platform resolver failures must not be converted into durable writes"
    );
}
