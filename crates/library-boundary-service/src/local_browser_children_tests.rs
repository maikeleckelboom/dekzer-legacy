use std::path::Path;

use library_boundary_protocol as protocol;
use library_store_sqlite::{LibraryStoreContext, StoreEnvironment, durable_store_path};
use rusqlite::Connection;
use tempfile::TempDir;

use crate::local_browser_children::classify_candidate_file_for_test;
use crate::service::LibraryBoundaryService;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRowCount {
    table: String,
    rows: i64,
}

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

fn expect_local_browser_children_reply(
    reply: protocol::CommandReply,
) -> protocol::ReadLocalBrowserChildrenReply {
    match reply {
        protocol::CommandReply::SnapshotRead(
            protocol::SnapshotReadReply::LocalBrowserChildren(reply),
        ) => reply,
        other => panic!("expected local browser children reply, got {other:?}"),
    }
}

fn read_local_browser_children(
    service: &LibraryBoundaryService,
    entry_point_kind: protocol::LocalBrowserEntryPointKind,
    root: &Path,
    parent: &Path,
    offset: usize,
    limit: usize,
) -> protocol::ReadLocalBrowserChildrenReply {
    expect_local_browser_children_reply(expect_success(service.handle_command(
        protocol::CommandRequest::SnapshotRead(
            protocol::SnapshotReadCommand::ReadLocalBrowserChildren(
                protocol::ReadLocalBrowserChildrenRequest {
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

fn child_names(reply: &protocol::ReadLocalBrowserChildrenReply) -> Vec<String> {
    reply
        .rows
        .iter()
        .map(|row| row.display_name.clone())
        .collect()
}

#[cfg(windows)]
#[test]
fn local_browser_children_returns_bounded_immediate_children_only() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Alpha")).expect("create Alpha");
    std::fs::create_dir(root.path().join("Beta")).expect("create Beta");
    std::fs::write(root.path().join("Track.flac"), []).expect("write track");
    std::fs::write(root.path().join("Alpha").join("Nested.mp3"), []).expect("write nested track");

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );

    assert_eq!(
        reply.status,
        protocol::LocalBrowserChildrenReadStatus::Complete
    );
    assert_eq!(reply.total_rows, 3);
    assert_eq!(child_names(&reply), vec!["Alpha", "Beta", "Track.flac"]);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn local_browser_children_limit_offset_and_sort_directories_before_files() {
    let (_tempdir, _context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("B")).expect("create B");
    std::fs::create_dir(root.path().join("a")).expect("create a");
    std::fs::write(root.path().join("z.flac"), []).expect("write z");
    std::fs::write(root.path().join("A.mp3"), []).expect("write A");
    std::fs::write(root.path().join("notes.txt"), []).expect("write notes");

    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        1,
        3,
    );

    assert_eq!(
        reply.status,
        protocol::LocalBrowserChildrenReadStatus::Complete
    );
    assert_eq!(reply.total_rows, 5);
    assert_eq!(child_names(&reply), vec!["B", "A.mp3", "z.flac"]);
    assert_eq!(reply.offset, 1);
    assert_eq!(reply.limit, 3);
}

#[cfg(windows)]
#[test]
fn media_file_candidates_are_not_source_files_and_can_choose_parent() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("Track.mp3"), []).expect("write track");

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let row = reply
        .rows
        .iter()
        .find(|row| row.display_name == "Track.mp3")
        .expect("track row");

    assert_eq!(
        row.row_kind,
        protocol::LocalBrowserCandidateRowKind::MediaFileCandidate
    );
    assert_eq!(row.file_kind, Some(protocol::ContentsFileKind::Audio));
    assert_eq!(
        row.media_relevance,
        Some(protocol::LocalBrowserCandidateMediaRelevance::MediaRelevant)
    );
    assert_eq!(
        row.admission_hint,
        protocol::LocalBrowserCandidateAdmissionHint::ChooseParentDirectory
    );
    assert!(!row.affordances.can_request_admission);
    assert!(row.affordances.can_request_parent_admission);
    assert_eq!(count_rows(&context, "source_files"), 0);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn unsupported_files_are_marked_unsupported_candidates() {
    let (_tempdir, _context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::write(root.path().join("notes.txt"), []).expect("write notes");

    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let row = reply
        .rows
        .iter()
        .find(|row| row.display_name == "notes.txt")
        .expect("notes row");

    assert_eq!(
        row.row_kind,
        protocol::LocalBrowserCandidateRowKind::UnsupportedFileCandidate
    );
    assert_eq!(row.file_kind, Some(protocol::ContentsFileKind::TextDoc));
    assert_eq!(
        row.media_relevance,
        Some(protocol::LocalBrowserCandidateMediaRelevance::Unsupported)
    );
    assert_eq!(
        row.admission_hint,
        protocol::LocalBrowserCandidateAdmissionHint::NotDirectlyAdmissible
    );
}

#[cfg(windows)]
#[test]
fn child_read_does_not_recurse() {
    let (_tempdir, _context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Album")).expect("create album");
    std::fs::write(root.path().join("Album").join("Nested.flac"), []).expect("write nested");

    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );

    assert_eq!(reply.total_rows, 1);
    assert_eq!(child_names(&reply), vec!["Album"]);
}

#[cfg(windows)]
#[test]
fn exact_admitted_source_path_marks_duplicate_without_mutation() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    let admitted = root.path().join("Admitted");
    std::fs::create_dir(&admitted).expect("create admitted candidate");
    register_local_root(&service, admitted.to_string_lossy().into_owned());

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let row = reply
        .rows
        .iter()
        .find(|row| row.display_name == "Admitted")
        .expect("admitted row");

    assert_eq!(
        row.status,
        protocol::LocalBrowserCandidateStatus::DuplicateOfAdmittedSource
    );
    assert_eq!(
        row.admission_hint,
        protocol::LocalBrowserCandidateAdmissionHint::DuplicateOfAdmittedSource
    );
    assert!(!row.affordances.can_request_admission);
    assert!(row.affordances.can_browse);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn system_drive_child_browsing_remains_read_only_and_rejects_system_roots() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    std::fs::create_dir(root.path().join("Windows")).expect("create Windows candidate");

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::SystemDriveRoot,
        root.path(),
        root.path(),
        0,
        20,
    );
    let row = reply
        .rows
        .iter()
        .find(|row| row.display_name == "Windows")
        .expect("Windows row");

    assert_eq!(
        reply.status,
        protocol::LocalBrowserChildrenReadStatus::Complete
    );
    assert_eq!(
        row.row_kind,
        protocol::LocalBrowserCandidateRowKind::RejectedRootCandidate
    );
    assert_eq!(row.status, protocol::LocalBrowserCandidateStatus::Rejected);
    assert_eq!(
        row.admission_hint,
        protocol::LocalBrowserCandidateAdmissionHint::Rejected
    );
    assert!(!row.affordances.can_request_admission);
    assert_eq!(
        row.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowserChildFailureCode::RejectedRoot)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn missing_parent_maps_to_read_failure_without_lifecycle_state() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    let missing = root.path().join("missing");

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        &missing,
        0,
        20,
    );

    assert_eq!(
        reply.status,
        protocol::LocalBrowserChildrenReadStatus::Missing
    );
    assert_eq!(reply.total_rows, 0);
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowserChildFailureCode::ParentMissing)
    );
    assert_eq!(count_rows(&context, "source_state"), 0);
    assert_eq!(count_rows(&context, "source_scan_state"), 0);
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(windows)]
#[test]
fn symlink_escape_candidate_is_not_followed_when_symlinks_are_available() {
    let (_tempdir, context, service) = open_test_service();
    let root = TempDir::new().expect("create local root");
    let outside = TempDir::new().expect("create outside root");
    std::fs::write(outside.path().join("Outside.flac"), []).expect("write outside file");
    let link = root.path().join("OutsideLink");
    if std::os::windows::fs::symlink_dir(outside.path(), &link).is_err() {
        return;
    }

    let before = application_table_row_counts(&context);
    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        root.path(),
        root.path(),
        0,
        20,
    );
    let row = reply
        .rows
        .iter()
        .find(|row| row.display_name == "OutsideLink")
        .expect("link row");

    assert_eq!(reply.total_rows, 1);
    assert_eq!(
        row.row_kind,
        protocol::LocalBrowserCandidateRowKind::InaccessibleCandidate
    );
    assert_eq!(
        row.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowserChildFailureCode::ReparsePointSkipped)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[cfg(not(windows))]
#[test]
fn non_windows_child_read_returns_unsupported_consistently() {
    let (_tempdir, context, service) = open_test_service();
    let before = application_table_row_counts(&context);
    let root = std::path::PathBuf::from("/tmp/dekzer-local-browser-root");

    let reply = read_local_browser_children(
        &service,
        protocol::LocalBrowserEntryPointKind::Music,
        &root,
        &root,
        0,
        20,
    );

    assert_eq!(
        reply.status,
        protocol::LocalBrowserChildrenReadStatus::UnsupportedPlatform
    );
    assert_eq!(
        reply.failure.as_ref().map(|failure| failure.code),
        Some(protocol::LocalBrowserChildFailureCode::UnsupportedPlatform)
    );
    assert_eq!(application_table_row_counts(&context), before);
}

#[test]
fn local_browser_candidate_file_classification_is_provisional_display_state() {
    assert_eq!(
        classify_candidate_file_for_test(Path::new("Track.flac")),
        (
            protocol::LocalBrowserCandidateRowKind::MediaFileCandidate,
            protocol::ContentsFileKind::Audio,
            protocol::LocalBrowserCandidateMediaRelevance::MediaRelevant,
            protocol::LocalBrowserCandidateAdmissionHint::ChooseParentDirectory,
        )
    );
    assert_eq!(
        classify_candidate_file_for_test(Path::new("Album.cue")),
        (
            protocol::LocalBrowserCandidateRowKind::MediaFileCandidate,
            protocol::ContentsFileKind::CueSheet,
            protocol::LocalBrowserCandidateMediaRelevance::CompanionMetadata,
            protocol::LocalBrowserCandidateAdmissionHint::ChooseParentDirectory,
        )
    );
    assert_eq!(
        classify_candidate_file_for_test(Path::new("notes.txt")),
        (
            protocol::LocalBrowserCandidateRowKind::UnsupportedFileCandidate,
            protocol::ContentsFileKind::TextDoc,
            protocol::LocalBrowserCandidateMediaRelevance::Unsupported,
            protocol::LocalBrowserCandidateAdmissionHint::NotDirectlyAdmissible,
        )
    );
}
