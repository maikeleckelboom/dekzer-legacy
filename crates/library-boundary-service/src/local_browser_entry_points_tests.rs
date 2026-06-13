use std::path::{Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;
use library_store_sqlite::{LibraryStoreContext, StoreEnvironment, durable_store_path};
use rusqlite::Connection;
use tempfile::TempDir;

#[cfg(windows)]
use crate::local_browser_entry_points::windows_path_status_probe_from_error;
use crate::local_browser_entry_points::{
    LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure,
    LocalBrowserEntryPointResolver, LocalBrowserPathStatusProbe, LocalBrowserPathStatusResolver,
    ResolvedLocalBrowserEntryPoint, status_for_path, status_for_path_with_resolver,
};
use crate::service::LibraryBoundaryService;

#[derive(Clone)]
struct FakeLocalBrowserEntryPointResolver {
    resolution: Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure>,
}

impl LocalBrowserEntryPointResolver for FakeLocalBrowserEntryPointResolver {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure> {
        self.resolution.clone()
    }
}

struct FakeLocalBrowserPathStatusResolver {
    probe: LocalBrowserPathStatusProbe,
}

impl LocalBrowserPathStatusResolver for FakeLocalBrowserPathStatusResolver {
    fn resolve_status(&self, _path: &Path) -> LocalBrowserPathStatusProbe {
        self.probe
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRowCount {
    table: String,
    rows: i64,
}

fn open_service_with_fake_local_browser_entries(
    resolution: LocalBrowserEntryPointResolution,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    open_service_with_local_browser_resolution(Ok(resolution))
}

fn open_service_with_failing_local_browser_resolver(
    failure: LocalBrowserEntryPointResolveFailure,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    open_service_with_local_browser_resolution(Err(failure))
}

fn open_service_with_local_browser_resolution(
    resolution: Result<LocalBrowserEntryPointResolution, LocalBrowserEntryPointResolveFailure>,
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
    let service = LibraryBoundaryService::from_store_with_local_browser_entry_point_resolver(
        durable_store,
        Arc::new(FakeLocalBrowserEntryPointResolver { resolution }),
    )
    .expect("open service with fake resolver");

    (tempdir, context, service)
}

fn fake_local_browser_resolution(
    entries: Vec<ResolvedLocalBrowserEntryPoint>,
) -> LocalBrowserEntryPointResolution {
    LocalBrowserEntryPointResolution {
        entries,
        failure: None,
    }
}

fn fake_local_browser_entry(
    entry_point_kind: protocol::LocalBrowserEntryPointKind,
    canonical_path: impl Into<PathBuf>,
    display_name: &str,
    status: protocol::LocalBrowserEntryPointStatus,
) -> ResolvedLocalBrowserEntryPoint {
    ResolvedLocalBrowserEntryPoint {
        entry_point_kind,
        canonical_path: Some(canonical_path.into()),
        display_name: display_name.to_string(),
        status,
        platform: protocol::LocalBrowserEntryPointPlatform::Windows,
        failure: None,
    }
}

fn fake_unresolved_local_browser_entry(
    entry_point_kind: protocol::LocalBrowserEntryPointKind,
    display_name: &str,
    status: protocol::LocalBrowserEntryPointStatus,
    failure_code: protocol::LocalBrowserEntryPointFailureCode,
) -> ResolvedLocalBrowserEntryPoint {
    ResolvedLocalBrowserEntryPoint {
        entry_point_kind,
        canonical_path: None,
        display_name: display_name.to_string(),
        status,
        platform: protocol::LocalBrowserEntryPointPlatform::Windows,
        failure: Some(LocalBrowserEntryPointResolveFailure {
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

fn expect_local_browser_entry_points_reply(
    reply: protocol::CommandReply,
) -> protocol::ReadLocalBrowserEntryPointsReply {
    match reply {
        protocol::CommandReply::SnapshotRead(
            protocol::SnapshotReadReply::LocalBrowserEntryPoints(reply),
        ) => reply,
        other => panic!("expected local browser entry points reply, got {other:?}"),
    }
}

fn read_local_browser_entry_points(
    service: &LibraryBoundaryService,
) -> protocol::ReadLocalBrowserEntryPointsReply {
    expect_local_browser_entry_points_reply(expect_success(service.handle_command(
        protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowserEntryPoints(
                protocol::ReadLocalBrowserEntryPointsRequest,
            ),
        ),
    )))
}

fn register_local_root(
    service: &LibraryBoundaryService,
    absolute_path: String,
) -> protocol::RegisteredLocalRoot {
    match expect_register_local_root_reply(expect_success(service.handle_command(
        protocol::CommandRequest::LibraryRoots(protocol::LibraryRootCommand::RegisterLocalRoot(
            protocol::RegisterLocalRootRequest { absolute_path },
        )),
    ))) {
        protocol::RegisterLocalRootReply::Registered(root) => root,
        other => panic!("expected registered local root reply, got {other:?}"),
    }
}

#[test]
fn local_browser_path_status_resolver_maps_probe_results() {
    let cases = [
        (
            LocalBrowserPathStatusProbe::Directory,
            protocol::LocalBrowserEntryPointStatus::Available,
        ),
        (
            LocalBrowserPathStatusProbe::NonDirectory,
            protocol::LocalBrowserEntryPointStatus::Unavailable,
        ),
        (
            LocalBrowserPathStatusProbe::NotFound,
            protocol::LocalBrowserEntryPointStatus::Missing,
        ),
        (
            LocalBrowserPathStatusProbe::PermissionBlocked,
            protocol::LocalBrowserEntryPointStatus::PermissionBlocked,
        ),
        (
            LocalBrowserPathStatusProbe::Unavailable,
            protocol::LocalBrowserEntryPointStatus::Unavailable,
        ),
    ];

    for (probe, expected_status) in cases {
        let resolver = FakeLocalBrowserPathStatusResolver { probe };
        assert_eq!(
            status_for_path_with_resolver(&resolver, Path::new("fixture")),
            expected_status
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_local_browser_path_status_resolver_maps_win32_errors() {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_BAD_NETPATH, ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE,
        ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND,
    };

    assert_eq!(
        windows_path_status_probe_from_error(ERROR_FILE_NOT_FOUND),
        LocalBrowserPathStatusProbe::NotFound
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_PATH_NOT_FOUND),
        LocalBrowserPathStatusProbe::NotFound
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_ACCESS_DENIED),
        LocalBrowserPathStatusProbe::PermissionBlocked
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_BAD_NETPATH),
        LocalBrowserPathStatusProbe::Unavailable
    );
    assert_eq!(
        windows_path_status_probe_from_error(ERROR_CLOUD_FILE_NETWORK_UNAVAILABLE),
        LocalBrowserPathStatusProbe::Unavailable
    );
}

#[test]
fn production_local_browser_path_status_resolver_reads_shallow_path_state() {
    let tempdir = TempDir::new().expect("create tempdir");
    let file_path = tempdir.path().join("not-a-directory.txt");
    std::fs::write(&file_path, b"fixture").expect("write fixture file");

    assert_eq!(
        status_for_path(tempdir.path()),
        protocol::LocalBrowserEntryPointStatus::Available
    );
    assert_eq!(
        status_for_path(&file_path),
        protocol::LocalBrowserEntryPointStatus::Unavailable
    );
    assert_eq!(
        status_for_path(&tempdir.path().join("missing")),
        protocol::LocalBrowserEntryPointStatus::Missing
    );
}

#[test]
fn local_browser_entry_points_read_returns_candidates_without_source_rows() {
    let fixture_tempdir = TempDir::new().expect("create fixture paths");
    let system_root = fixture_tempdir.path().join("system-root");
    let music_root = fixture_tempdir.path().join("Music");
    let home_root = fixture_tempdir.path().join("home");
    std::fs::create_dir_all(&system_root).expect("create system root fixture");
    std::fs::create_dir_all(&music_root).expect("create music root fixture");
    std::fs::create_dir_all(&home_root).expect("create home root fixture");

    let (_tempdir, context, service) =
        open_service_with_fake_local_browser_entries(fake_local_browser_resolution(vec![
            fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::SystemDriveRoot,
                system_root,
                "C:\\",
                protocol::LocalBrowserEntryPointStatus::Available,
            ),
            fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::Music,
                music_root,
                "Music",
                protocol::LocalBrowserEntryPointStatus::Available,
            ),
            fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::UserHome,
                home_root,
                "Home",
                protocol::LocalBrowserEntryPointStatus::Available,
            ),
        ]));
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browser_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowserEntryPointsReadStatus::Complete
    );
    assert_eq!(reply.entries.len(), 3);
    assert_eq!(
        reply.entries[0].identity.entry_point_kind,
        protocol::LocalBrowserEntryPointKind::SystemDriveRoot
    );
    assert_eq!(
        reply.entries[0].admission_hint,
        protocol::LocalBrowserEntryPointAdmissionHint::NotDirectlyAdmissible
    );
    assert!(reply.entries[0].affordances.can_browse);
    assert!(!reply.entries[0].affordances.can_request_admission);
    assert_eq!(
        reply.entries[1].identity.entry_point_kind,
        protocol::LocalBrowserEntryPointKind::Music
    );
    assert_eq!(
        reply.entries[1].admission_hint,
        protocol::LocalBrowserEntryPointAdmissionHint::DefaultMusicFolder
    );
    assert!(reply.entries[1].affordances.can_request_admission);
    assert!(!reply.entries[1].affordances.requires_confirmation);
    assert_eq!(
        reply.entries[2].admission_hint,
        protocol::LocalBrowserEntryPointAdmissionHint::RequiresConfirmation
    );
    assert!(reply.entries[2].affordances.requires_confirmation);
    assert_eq!(count_rows(&context, "sources"), 0);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "local browser entry point read must not mutate any durable application table"
    );
}

#[test]
fn local_browser_entry_points_exact_admitted_source_match_is_duplicate_candidate() {
    let fixture_tempdir = TempDir::new().expect("create fixture paths");
    let source_root = fixture_tempdir.path().join("Music");
    std::fs::create_dir_all(&source_root).expect("create source root");

    let (_tempdir, context, service) =
        open_service_with_fake_local_browser_entries(fake_local_browser_resolution(vec![
            fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::Music,
                source_root.clone(),
                "Music",
                protocol::LocalBrowserEntryPointStatus::Available,
            ),
        ]));
    let _registered = register_local_root(&service, source_root.to_string_lossy().into_owned());
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browser_entry_points(&service);

    assert_eq!(reply.entries.len(), 1);
    assert_eq!(
        reply.entries[0].status,
        protocol::LocalBrowserEntryPointStatus::DuplicateOfAdmittedSource
    );
    assert_eq!(
        reply.entries[0].admission_hint,
        protocol::LocalBrowserEntryPointAdmissionHint::DuplicateOfAdmittedSource
    );
    assert!(!reply.entries[0].affordances.can_request_admission);
    assert_eq!(count_rows(&context, "sources"), 1);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "duplicate candidate read must not mutate any durable application table"
    );
}

#[test]
fn local_browser_entry_points_statuses_do_not_create_lifecycle_state() {
    let missing_path = PathBuf::from("Z:\\Missing-Music");
    let (_tempdir, context, service) =
        open_service_with_fake_local_browser_entries(fake_local_browser_resolution(vec![
            fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::Downloads,
                missing_path,
                "Downloads",
                protocol::LocalBrowserEntryPointStatus::Missing,
            ),
            fake_unresolved_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::Desktop,
                "Desktop",
                protocol::LocalBrowserEntryPointStatus::PermissionBlocked,
                protocol::LocalBrowserEntryPointFailureCode::MetadataUnavailable,
            ),
            fake_unresolved_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::RemovableVolumeRoot,
                "USB",
                protocol::LocalBrowserEntryPointStatus::Unavailable,
                protocol::LocalBrowserEntryPointFailureCode::VolumeEnumerationUnavailable,
            ),
        ]));
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browser_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowserEntryPointsReadStatus::Complete
    );
    assert_eq!(
        reply
            .entries
            .iter()
            .map(|entry| entry.status)
            .collect::<Vec<_>>(),
        vec![
            protocol::LocalBrowserEntryPointStatus::Missing,
            protocol::LocalBrowserEntryPointStatus::PermissionBlocked,
            protocol::LocalBrowserEntryPointStatus::Unavailable
        ]
    );
    assert!(
        reply.entries.iter().all(|entry| entry.admission_hint
            == protocol::LocalBrowserEntryPointAdmissionHint::Unavailable)
    );
    assert!(
        reply
            .entries
            .iter()
            .all(|entry| !entry.affordances.can_request_admission)
    );
    assert_eq!(count_rows(&context, "source_state"), 0);
    assert_eq!(count_rows(&context, "source_scan_state"), 0);
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "permission, missing, and unavailable candidate statuses are not source lifecycle state"
    );
}

#[test]
fn local_browser_entry_points_read_reports_partial_failure_without_writes() {
    let (_tempdir, context, service) =
        open_service_with_fake_local_browser_entries(LocalBrowserEntryPointResolution {
            entries: vec![fake_local_browser_entry(
                protocol::LocalBrowserEntryPointKind::UserHome,
                "C:\\Users\\DJ",
                "Home",
                protocol::LocalBrowserEntryPointStatus::Available,
            )],
            failure: Some(LocalBrowserEntryPointResolveFailure {
                code: protocol::LocalBrowserEntryPointFailureCode::VolumeEnumerationUnavailable,
                detail: "volume enumeration failed".to_string(),
            }),
        });
    let before_counts = application_table_row_counts(&context);

    let reply = read_local_browser_entry_points(&service);

    assert_eq!(
        reply.status,
        protocol::LocalBrowserEntryPointsReadStatus::PartialFailure
    );
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowserEntryPointFailureCode::VolumeEnumerationUnavailable)
    );
    assert_eq!(
        application_table_row_counts(&context),
        before_counts,
        "platform resolver partial failures must not be converted into durable writes"
    );
}

#[test]
fn local_browser_entry_points_read_failure_has_no_durable_writes() {
    let (_tempdir, context, service) =
        open_service_with_failing_local_browser_resolver(LocalBrowserEntryPointResolveFailure {
            code: protocol::LocalBrowserEntryPointFailureCode::MetadataUnavailable,
            detail: "metadata probe failed".to_string(),
        });
    let before_counts = application_table_row_counts(&context);

    let error = service
        .try_handle_command(protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowserEntryPoints(
                protocol::ReadLocalBrowserEntryPointsRequest,
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
