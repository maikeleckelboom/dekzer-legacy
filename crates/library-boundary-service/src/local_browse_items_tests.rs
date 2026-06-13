use std::path::{Path, PathBuf};
use std::sync::Arc;

use library_boundary_protocol as protocol;
use library_store_sqlite::{LibraryStoreContext, StoreEnvironment, durable_store_path};
use rusqlite::Connection;
use tempfile::TempDir;

use crate::local_browse_entry_points::{
    LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure,
    LocalBrowseEntryPointResolver, ResolvedLocalBrowseEntryPoint,
};
use crate::local_browse_items::{classify_item_file_for_test, window_item_names_for_test};
use crate::service::LibraryBoundaryService;

#[derive(Clone)]
struct FakeLocalBrowseEntryPointResolver {
    resolution: LocalBrowseEntryPointResolution,
}

impl LocalBrowseEntryPointResolver for FakeLocalBrowseEntryPointResolver {
    fn resolve_entry_points(
        &self,
    ) -> Result<LocalBrowseEntryPointResolution, LocalBrowseEntryPointResolveFailure> {
        Ok(self.resolution.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRowCount {
    table: String,
    rows: i64,
}

#[cfg(not(windows))]
fn open_test_service() -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
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
    let service = LibraryBoundaryService::from_store(durable_store).expect("open service");

    (tempdir, context, service)
}

fn open_test_service_with_local_browse_root(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    root: impl Into<PathBuf>,
) -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
    open_test_service_with_local_browse_entries(vec![fake_local_browse_entry(
        entry_point_kind,
        root,
        "Local Browse Root",
        protocol::LocalBrowseEntryPointStatus::Available,
    )])
}

fn open_test_service_with_local_browse_entries(
    entries: Vec<ResolvedLocalBrowseEntryPoint>,
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
        Arc::new(FakeLocalBrowseEntryPointResolver {
            resolution: LocalBrowseEntryPointResolution {
                entries,
                failure: None,
            },
        }),
    )
    .expect("open service");

    (tempdir, context, service)
}

fn fake_local_browse_entry(
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    canonical_path: impl Into<PathBuf>,
    display_name: &str,
    status: protocol::LocalBrowseEntryPointStatus,
) -> ResolvedLocalBrowseEntryPoint {
    ResolvedLocalBrowseEntryPoint {
        entry_point_kind,
        canonical_path: Some(canonical_path.into()),
        display_name: display_name.to_string(),
        status,
        platform: protocol::LocalBrowsePlatform::Windows,
        failure: None,
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

fn expect_local_browse_items_reply(
    reply: protocol::CommandReply,
) -> protocol::ReadLocalBrowseItemsReply {
    match reply {
        protocol::CommandReply::SnapshotRead(protocol::SnapshotReadReply::LocalBrowseItems(
            reply,
        )) => reply,
        other => panic!("expected local browse items reply, got {other:?}"),
    }
}

fn read_local_browse_items(
    service: &LibraryBoundaryService,
    entry_point_kind: protocol::LocalBrowseEntryPointKind,
    root: &Path,
    parent: &Path,
    offset: usize,
    limit: usize,
) -> protocol::ReadLocalBrowseItemsReply {
    expect_local_browse_items_reply(expect_success(service.handle_command(
        protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowseItems(
                protocol::ReadLocalBrowseItemsRequest {
                    entry_point_kind,
                    root_canonical_path: root.to_string_lossy().into_owned(),
                    parent_canonical_path: parent.to_string_lossy().into_owned(),
                    offset,
                    limit,
                },
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

fn item_names(reply: &protocol::ReadLocalBrowseItemsReply) -> Vec<String> {
    reply
        .items
        .iter()
        .map(|item| item.display_name.clone())
        .collect()
}

#[cfg(windows)]
#[test]
fn local_browse_item_read_validates_root_identity_before_browsing() {
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("Track.flac"), []).expect("write track");
    let wrong_root = TempDir::new().expect("create wrong local root");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        wrong_root.path(),
        wrong_root.path(),
        0,
        20,
    );

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Failed);
    assert_eq!(reply.total_items, 0);
    assert!(reply.items.is_empty());
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::RootIdentityMismatch)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn mismatched_entry_point_kind_does_not_browse_matching_path() {
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("Track.flac"), []).expect("write track");
    let (_tempdir, _context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Downloads,
        root.path(),
        root.path(),
        0,
        20,
    );

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Failed);
    assert_eq!(reply.total_items, 0);
    assert!(reply.items.is_empty());
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::RootIdentityMismatch)
    );
}

#[cfg(windows)]
#[test]
fn local_browse_items_returns_bounded_immediate_children_only() {
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Alpha")).expect("create Alpha");
    std::fs::create_dir(root.path().join("Beta")).expect("create Beta");
    std::fs::write(root.path().join("Track.flac"), []).expect("write track");
    std::fs::write(root.path().join("Alpha").join("Nested.mp3"), []).expect("write nested track");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Complete);
    assert_eq!(reply.total_items, 3);
    assert_eq!(item_names(&reply), vec!["Alpha", "Beta", "Track.flac"]);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn local_browse_items_limit_offset_and_sort_directories_before_files() {
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("B")).expect("create B");
    std::fs::create_dir(root.path().join("a")).expect("create a");
    std::fs::write(root.path().join("z.flac"), []).expect("write z");
    std::fs::write(root.path().join("A.mp3"), []).expect("write A");
    std::fs::write(root.path().join("notes.txt"), []).expect("write notes");
    let (_tempdir, _context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        1,
        3,
    );

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Complete);
    assert_eq!(reply.total_items, 5);
    assert_eq!(item_names(&reply), vec!["B", "A.mp3", "z.flac"]);
    assert_eq!(reply.offset, 1);
    assert_eq!(reply.limit, 3);
}

#[cfg(windows)]
#[test]
fn media_file_items_are_not_source_files_and_request_parent_admission_only() {
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("Track.mp3"), []).expect("write track");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let item = reply
        .items
        .iter()
        .find(|item| item.display_name == "Track.mp3")
        .expect("track item");

    assert_eq!(item.item_kind, protocol::LocalBrowseItemKind::MediaFile);
    assert_eq!(item.file_kind, Some(protocol::ContentsFileKind::Audio));
    assert_eq!(
        item.media_relevance,
        Some(protocol::LocalBrowseItemMediaRelevance::MediaRelevant)
    );
    assert_eq!(
        item.admission_action,
        Some(protocol::LocalBrowseAdmissionAction::RequestParentAdmission)
    );
    assert!(!item.available_actions.can_request_admission);
    assert!(item.available_actions.can_request_parent_admission);
    assert_eq!(count_rows(&context, "source_files"), 0);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn unsupported_files_are_marked_unsupported_items() {
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("notes.txt"), []).expect("write notes");
    let (_tempdir, _context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let item = reply
        .items
        .iter()
        .find(|item| item.display_name == "notes.txt")
        .expect("notes item");

    assert_eq!(
        item.item_kind,
        protocol::LocalBrowseItemKind::UnsupportedFile
    );
    assert_eq!(item.file_kind, Some(protocol::ContentsFileKind::TextDoc));
    assert_eq!(
        item.media_relevance,
        Some(protocol::LocalBrowseItemMediaRelevance::Unsupported)
    );
    assert_eq!(item.admission_action, None);
}

#[cfg(windows)]
#[test]
fn item_read_does_not_recurse() {
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Album")).expect("create album");
    std::fs::write(root.path().join("Album").join("Nested.flac"), []).expect("write nested");
    let (_tempdir, _context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );

    assert_eq!(reply.total_items, 1);
    assert_eq!(item_names(&reply), vec!["Album"]);
}

#[cfg(windows)]
#[test]
fn exact_admitted_source_path_marks_duplicate_without_mutation() {
    let root = TempDir::new().expect("create local root");
    let admitted = root.path().join("Admitted");
    std::fs::create_dir(&admitted).expect("create admitted item");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );
    register_local_root(&service, admitted.to_string_lossy().into_owned());

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let item = reply
        .items
        .iter()
        .find(|item| item.display_name == "Admitted")
        .expect("admitted item");

    assert_eq!(
        item.status,
        protocol::LocalBrowseItemStatus::DuplicateOfAdmittedSource
    );
    assert_eq!(item.admission_action, None);
    assert!(!item.available_actions.can_request_admission);
    assert!(item.available_actions.can_browse);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn system_drive_item_browsing_remains_read_only_and_rejects_system_roots() {
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Windows")).expect("create Windows item");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::SystemDriveRoot,
        root.path(),
        root.path(),
        0,
        20,
    );
    let item = reply
        .items
        .iter()
        .find(|item| item.display_name == "Windows")
        .expect("Windows item");

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Complete);
    assert_eq!(item.item_kind, protocol::LocalBrowseItemKind::RejectedRoot);
    assert_eq!(item.status, protocol::LocalBrowseItemStatus::Rejected);
    assert_eq!(item.admission_action, None);
    assert!(!item.available_actions.can_request_admission);
    assert_eq!(
        item.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::RejectedRoot)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn missing_parent_maps_to_read_failure_without_lifecycle_state() {
    let root = TempDir::new().expect("create local root");
    let missing = root.path().join("missing");
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        &missing,
        0,
        20,
    );

    assert_eq!(reply.status, protocol::LocalBrowseItemsReadStatus::Missing);
    assert_eq!(reply.total_items, 0);
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::ParentMissing)
    );
    assert_eq!(count_rows(&context, "source_state"), 0);
    assert_eq!(count_rows(&context, "source_scan_state"), 0);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn symlink_escape_item_is_not_followed_when_symlinks_are_available() {
    let root = TempDir::new().expect("create local root");
    let outside = TempDir::new().expect("create outside root");
    std::fs::write(outside.path().join("Outside.flac"), []).expect("write outside file");
    let link = root.path().join("OutsideLink");
    if std::os::windows::fs::symlink_dir(outside.path(), &link).is_err() {
        return;
    }
    let (_tempdir, context, service) = open_test_service_with_local_browse_root(
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
    );

    let before = application_table_row_counts(&context);
    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let item = reply
        .items
        .iter()
        .find(|item| item.display_name == "OutsideLink")
        .expect("link item");

    assert_eq!(reply.total_items, 1);
    assert_eq!(item.item_kind, protocol::LocalBrowseItemKind::Inaccessible);
    assert_eq!(
        item.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::ReparsePointSkipped)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(not(windows))]
#[test]
fn non_windows_child_read_returns_unsupported_consistently() {
    let (_tempdir, context, service) = open_test_service();
    let before = application_table_row_counts(&context);
    let root = std::path::PathBuf::from("/tmp/dekzer-local-browse-root");

    let reply = read_local_browse_items(
        &service,
        protocol::LocalBrowseEntryPointKind::Music,
        &root,
        &root,
        0,
        20,
    );

    assert_eq!(
        reply.status,
        protocol::LocalBrowseItemsReadStatus::UnsupportedPlatform
    );
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowseItemFailureCode::UnsupportedPlatform)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[test]
fn local_browse_item_file_classification_is_provisional_display_state() {
    assert_eq!(
        classify_item_file_for_test(Path::new("Track.flac")),
        (
            protocol::LocalBrowseItemKind::MediaFile,
            protocol::ContentsFileKind::Audio,
            protocol::LocalBrowseItemMediaRelevance::MediaRelevant,
            Some(protocol::LocalBrowseAdmissionAction::RequestParentAdmission),
        )
    );
    assert_eq!(
        classify_item_file_for_test(Path::new("Album.cue")),
        (
            protocol::LocalBrowseItemKind::MediaFile,
            protocol::ContentsFileKind::CueSheet,
            protocol::LocalBrowseItemMediaRelevance::CompanionMetadata,
            Some(protocol::LocalBrowseAdmissionAction::RequestParentAdmission),
        )
    );
    assert_eq!(
        classify_item_file_for_test(Path::new("notes.txt")),
        (
            protocol::LocalBrowseItemKind::UnsupportedFile,
            protocol::ContentsFileKind::TextDoc,
            protocol::LocalBrowseItemMediaRelevance::Unsupported,
            None,
        )
    );
}

#[test]
fn local_browse_item_windowing_sorts_lightweight_keys_before_materializing_window() {
    let (total, window) = window_item_names_for_test(
        vec![
            ("z.flac", protocol::LocalBrowseItemKind::MediaFile),
            ("Beta", protocol::LocalBrowseItemKind::Directory),
            ("notes.txt", protocol::LocalBrowseItemKind::UnsupportedFile),
            ("alpha", protocol::LocalBrowseItemKind::Directory),
        ],
        1,
        2,
    );

    assert_eq!(total, 4);
    assert_eq!(window, vec!["Beta", "z.flac"]);
}
