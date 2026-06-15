use std::fs;
use std::path::Path;

use library_boundary_protocol::{
    CancelRootScanReply, CancelRootScanRequest, CancelRootScanStatus, CommandErrorEnvelope,
    CommandOutcome, CommandReply, CommandRequest, DirectoryScanState, LibraryBoundaryEvent,
    LibraryBoundaryEventStreamCommand, LibraryBoundaryEventStreamReply, LibraryRootCommand,
    LibraryRootReply, LibraryTreeCoverageState, LibraryTreeEntryPoint, LibraryTreeNodeKind,
    LibraryTreePresenceState, MaintainedSnapshotScope, NavigableChildScopeState, NavigationRow,
    NavigationRowFamily, NavigationRowKind, NavigationRowSelectorKind,
    ReadLibraryBoundaryEventsAfterReply, ReadLibraryBoundaryEventsAfterRequest,
    ReadLibraryTreeChildrenReply, ReadLibraryTreeChildrenRequest, ReadNavigationRowsRequest,
    RegisterLocalRootReply, RegisterLocalRootRequest, RegisteredLocalRoot, ScanRunPhase,
    SnapshotReadCommand, SnapshotReadReply, SourceScanEvent, SourceScanEventKind,
    StartRootScanReply, StartRootScanRequest,
};
use library_boundary_service::{LibraryBoundaryService, LibraryStoreContext, StoreEnvironment};
use library_store_sqlite::durable_store_path;
use rusqlite::Connection;
use tempfile::TempDir;

fn open_service(tempdir: &TempDir) -> LibraryBoundaryService {
    LibraryBoundaryService::open(LibraryStoreContext {
        user_data_path: tempdir.path().to_string_lossy().into_owned(),
        environment: StoreEnvironment::Development,
    })
    .expect("open boundary service")
}

fn delete_root_navigation_state(tempdir: &TempDir, root_id: i64) {
    let db_path = durable_store_path(
        &tempdir.path().to_string_lossy(),
        StoreEnvironment::Development,
    );
    let connection = Connection::open(db_path).expect("open durable store");
    connection
        .execute(
            "DELETE FROM source_root_navigation_state WHERE source_id = ?1",
            [root_id],
        )
        .expect("delete root navigation state");
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, bytes).expect("write file");
}

fn register_root(service: &LibraryBoundaryService, path: &Path) -> RegisteredLocalRoot {
    let outcome = service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::RegisterLocalRoot(RegisterLocalRootRequest {
            requested_path: path.to_string_lossy().into_owned(),
        }),
    ));
    let reply = expect_command_reply(outcome, "register local root");
    match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(
            RegisterLocalRootReply::Registered(root),
        )) => root,
        other => panic!("Expected register reply, got {other:?}"),
    }
}

fn start_scan(service: &LibraryBoundaryService, root_id: i64) -> StartRootScanReply {
    let outcome = service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::StartRootScan(StartRootScanRequest { root_id }),
    ));
    let reply = expect_command_reply(outcome, "start root scan");
    match reply {
        CommandReply::LibraryRoots(LibraryRootReply::StartRootScan(reply)) => reply,
        other => panic!("Expected start scan reply, got {other:?}"),
    }
}

fn cancel_scan(service: &LibraryBoundaryService, scan_run_id: i64) -> CancelRootScanReply {
    let outcome = service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::CancelRootScan(CancelRootScanRequest { scan_run_id }),
    ));
    let reply = expect_command_reply(outcome, "cancel root scan");
    match reply {
        CommandReply::LibraryRoots(LibraryRootReply::CancelRootScan(reply)) => reply,
        other => panic!("Expected cancel scan reply, got {other:?}"),
    }
}

fn read_boundary_events_after(
    service: &LibraryBoundaryService,
    cursor: Option<i64>,
    max_events: usize,
) -> ReadLibraryBoundaryEventsAfterReply {
    let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
        LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
            last_seen_event_sequence: cursor,
            max_events,
        }),
    ));
    match outcome {
        CommandOutcome::Success(env) => match env.reply {
            CommandReply::LibraryBoundaryEvents(LibraryBoundaryEventStreamReply::ReadAfter(r)) => r,
            other => panic!("Expected event stream reply, got {other:?}"),
        },
        CommandOutcome::Error(env) => panic!("read boundary events failed: {:?}", env.error),
    }
}

fn wait_for_scan_event<F>(
    service: &LibraryBoundaryService,
    max_events: usize,
    mut matches_event: F,
) -> Option<SourceScanEvent>
where
    F: FnMut(&SourceScanEvent) -> bool,
{
    let mut cursor: Option<i64> = None;
    for _ in 0..60 {
        let reply = read_boundary_events_after(service, cursor, max_events);
        cursor = reply.latest_event_sequence;

        for event in reply.events {
            let LibraryBoundaryEvent::SourceScanEvent(scan_event) = event else {
                continue;
            };
            if matches_event(&scan_event) {
                return Some(scan_event);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    None
}

fn wait_for_scan_completion(service: &LibraryBoundaryService, root_id: i64) -> bool {
    wait_for_scan_event(service, 32, |se| {
        se.kind == SourceScanEventKind::SourceScanCompleted && se.root_id == root_id
    })
    .is_some()
}

fn expect_command_reply(outcome: CommandOutcome, context: &str) -> CommandReply {
    match outcome {
        CommandOutcome::Success(env) => env.reply,
        CommandOutcome::Error(env) => panic!("{context} failed: {:?}", env.error),
    }
}

fn expect_command_error(outcome: CommandOutcome) -> CommandErrorEnvelope {
    match outcome {
        CommandOutcome::Success(env) => panic!("Expected command error, got {:?}", env.reply),
        CommandOutcome::Error(env) => env,
    }
}

fn read_navigation_rows(
    service: &LibraryBoundaryService,
    parent_id: Option<i64>,
) -> Vec<NavigationRow> {
    let outcome = service.handle_command(CommandRequest::SnapshotRead(
        SnapshotReadCommand::ReadNavigationRows(ReadNavigationRowsRequest {
            parent_navigation_row_id: parent_id,
        }),
    ));
    let reply = expect_command_reply(outcome, "read navigation rows");
    match reply {
        CommandReply::SnapshotRead(SnapshotReadReply::NavigationRows(r)) => r.rows,
        other => panic!("Expected navigation rows reply, got {other:?}"),
    }
}

fn read_library_tree(
    service: &LibraryBoundaryService,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
) -> ReadLibraryTreeChildrenReply {
    let outcome = service.handle_command(CommandRequest::SnapshotRead(
        SnapshotReadCommand::ReadLibraryTreeChildren(ReadLibraryTreeChildrenRequest {
            entry_point: LibraryTreeEntryPoint::Source { source_id },
            parent_source_directory_id,
            row_policy: None,
            offset: 0,
            limit: 50,
        }),
    ));
    let reply = expect_command_reply(outcome, "read library tree");
    match reply {
        CommandReply::SnapshotRead(SnapshotReadReply::LibraryTreeChildren(r)) => r,
        other => panic!("Expected library tree reply, got {other:?}"),
    }
}

#[test]
fn registered_source_is_navigation_readable_before_recursive_scan() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    fs::create_dir_all(music_root.join("artists")).expect("create root child directory");
    fs::create_dir_all(music_root.join("crates")).expect("create sibling root child directory");
    write_file(&music_root.join("loose.mp3"), b"not-real-mp3");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    let root_id_text = registered.root_id.to_string();

    let root_nav_rows = read_navigation_rows(&service, None);
    let source_row = root_nav_rows
        .iter()
        .find(|row| {
            matches!(row.family, Some(NavigationRowFamily::Sources))
                && row.row_kind == NavigationRowKind::Source
                && row.selector_kind == Some(NavigationRowSelectorKind::Source)
                && row.selector_payload.as_deref() == Some(root_id_text.as_str())
        })
        .expect("registered source appears in navigation before scan");
    assert!(
        source_row.selectable,
        "registered source must be selectable before recursive scan"
    );

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window before scan");

    assert_eq!(
        root_window.coverage.state,
        LibraryTreeCoverageState::Pending,
        "pre-scan source hierarchy coverage must remain pending instead of empty"
    );
    assert!(
        !root_window.coverage.subtree_coverage_complete,
        "pre-scan source hierarchy coverage cannot claim complete subtree coverage"
    );
    assert!(
        !root_window.coverage.empty_result_authoritative,
        "pre-scan source hierarchy coverage cannot claim authoritative empty"
    );
    assert_eq!(
        root_window.total_rows, 2,
        "pre-scan root window must expose immediate child directories before recursive scan"
    );
    assert_eq!(
        root_window
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect::<Vec<_>>(),
        vec!["artists", "crates"]
    );
    assert!(
        !root_window
            .rows
            .iter()
            .any(|row| row.display_name == "loose.mp3"),
        "root hierarchy establishment must not return source-file rows"
    );
    for row in &root_window.rows {
        assert_eq!(row.node_kind, LibraryTreeNodeKind::Directory);
        assert_eq!(row.parent_source_directory_id, None);
        assert_eq!(row.directory_scan_state, Some(DirectoryScanState::Pending));
        assert_eq!(
            row.navigable_child_scope_state,
            Some(NavigableChildScopeState::Unknown),
            "pre-scan child directories must keep descendant readiness unknown"
        );
    }
}

#[test]
fn repeated_non_empty_root_reads_do_not_refresh_established_navigation_window() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    fs::create_dir_all(music_root.join("artists")).expect("create root child directory");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let first_reply = read_library_tree(&service, registered.root_id, None);
    let first_window = first_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");
    assert_eq!(first_window.total_rows, 1);
    assert_eq!(first_window.rows[0].display_name, "artists");

    fs::remove_dir_all(music_root.join("artists")).expect("remove established child directory");

    let second_reply = read_library_tree(&service, registered.root_id, None);
    let second_window = second_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");

    assert_eq!(
        second_window.total_rows, 1,
        "normal repeated root reads must not refresh or mark missing established child directories"
    );
    assert_eq!(second_window.rows[0].display_name, "artists");
    assert_eq!(
        second_window.rows[0].presence_state,
        LibraryTreePresenceState::Present
    );
}

#[test]
fn repeated_empty_root_reads_do_not_refresh_established_navigation_window() {
    let tempdir = TempDir::new().expect("create tempdir");
    let empty_root = tempdir.path().join("empty-root");
    fs::create_dir_all(&empty_root).expect("create empty root");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &empty_root);

    let first_reply = read_library_tree(&service, registered.root_id, None);
    let first_window = first_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");
    assert_eq!(first_window.total_rows, 0);
    assert_eq!(
        first_window.coverage.state,
        LibraryTreeCoverageState::Pending
    );
    assert!(
        !first_window.coverage.empty_result_authoritative,
        "registered tree reads do not promote pre-scan empty roots into authoritative scan coverage"
    );

    fs::create_dir_all(empty_root.join("late-child")).expect("create child after establishment");

    let second_reply = read_library_tree(&service, registered.root_id, None);
    let second_window = second_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");

    assert_eq!(
        second_window.total_rows, 0,
        "normal repeated root reads must not refresh established empty root windows"
    );
    assert!(second_window.rows.is_empty());
    assert!(
        !second_window.coverage.empty_result_authoritative,
        "empty remains a persisted zero-row skeleton until scanner coverage completes"
    );
}

#[test]
fn empty_registered_source_root_is_pending_empty_until_scan_coverage_completes() {
    let tempdir = TempDir::new().expect("create tempdir");
    let empty_root = tempdir.path().join("empty-root");
    fs::create_dir_all(&empty_root).expect("create empty root");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &empty_root);

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window before scan");

    assert_eq!(root_window.total_rows, 0);
    assert!(root_window.rows.is_empty());
    assert_eq!(
        root_window.coverage.state,
        LibraryTreeCoverageState::Pending
    );
    assert!(
        !root_window.coverage.subtree_coverage_complete,
        "an empty immediate root window must not claim recursive subtree completion"
    );
    assert!(
        !root_window.coverage.empty_result_authoritative,
        "registered hierarchy disclosure must not synthesize authoritative empty coverage before scan"
    );
}

#[test]
fn unestablished_missing_registered_source_root_is_not_probed_by_tree_read() {
    let tempdir = TempDir::new().expect("create tempdir");
    let root_path = tempdir.path().join("missing-after-register");
    fs::create_dir_all(&root_path).expect("create root before registration");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &root_path);
    delete_root_navigation_state(&tempdir, registered.root_id);
    fs::remove_dir_all(&root_path).expect("remove root after registration");

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");

    assert_eq!(
        root_window.coverage.state,
        LibraryTreeCoverageState::Pending
    );
    assert!(!root_window.coverage.subtree_coverage_complete);
    assert!(!root_window.coverage.empty_result_authoritative);
    assert!(root_window.rows.is_empty());
}

#[test]
fn unestablished_blocked_registered_source_root_is_not_probed_by_tree_read() {
    let tempdir = TempDir::new().expect("create tempdir");
    let root_path = tempdir.path().join("blocked-after-register");
    fs::create_dir_all(&root_path).expect("create root before registration");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &root_path);
    delete_root_navigation_state(&tempdir, registered.root_id);
    fs::remove_dir_all(&root_path).expect("remove root directory after registration");
    fs::write(&root_path, b"not-a-directory").expect("replace root with file");

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("registered source root resolves to a hierarchy window");

    assert_eq!(
        root_window.coverage.state,
        LibraryTreeCoverageState::Pending
    );
    assert!(!root_window.coverage.subtree_coverage_complete);
    assert!(!root_window.coverage.empty_result_authoritative);
    assert!(root_window.rows.is_empty());
}

#[test]
fn scan_empty_folder_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let empty_root = tempdir.path().join("empty-root");
    fs::create_dir_all(&empty_root).expect("create empty root");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &empty_root);
    let scanned = start_scan(&service, registered.root_id);

    assert!(scanned.scan_run_id > 0);

    let completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(completed, "scan of empty folder should complete");
}

#[test]
fn scan_with_invalid_root_id_rejects() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let zero_root_error =
        expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::StartRootScan(StartRootScanRequest { root_id: 0 }),
        )));
    assert_eq!(zero_root_error.error.code(), "INVALID_REQUEST");
    assert!(
        zero_root_error.error.to_string().contains("rootId"),
        "Error should mention rootId, got: {}",
        zero_root_error.error,
    );

    let negative_root_error =
        expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::StartRootScan(StartRootScanRequest { root_id: -5 }),
        )));
    assert_eq!(negative_root_error.error.code(), "INVALID_REQUEST");
}

#[test]
fn scan_with_unknown_root_id_returns_current_store_error() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let error = expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::StartRootScan(StartRootScanRequest { root_id: 99999 }),
    )));

    assert_eq!(error.error.code(), "DURABLE_STORE_FAILURE");
    assert!(
        error.error.to_string().contains("does not exist"),
        "Current store error should mention root does not exist, got: {}",
        error.error,
    );
}

#[test]
fn missing_registered_root_is_rejected_before_scan_start() {
    let tempdir = TempDir::new().expect("create tempdir");
    let missing_root = tempdir.path().join("missing-after-register");
    fs::create_dir_all(&missing_root).expect("create root before registration");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &missing_root);
    fs::remove_dir_all(&missing_root).expect("remove root after registration");

    let error = expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::StartRootScan(StartRootScanRequest {
            root_id: registered.root_id,
        }),
    )));

    assert_eq!(error.error.code(), "INVALID_REQUEST");
    assert!(
        error.error.to_string().contains("unknown_root"),
        "expected unknown-root admission rejection, got: {}",
        error.error,
    );

    let events = read_boundary_events_after(&service, None, 32);
    assert!(
        events.events.iter().all(|event| !matches!(
            event,
            LibraryBoundaryEvent::SourceScanEvent(scan_event)
                if scan_event.root_id == registered.root_id
                    && scan_event.kind == SourceScanEventKind::SourceScanStarted
        )),
        "admission-rejected roots must not publish SourceScanStarted"
    );
}

#[test]
fn scanned_literal_hierarchy_survives_service_reopen() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_one.flac"),
        b"not-real-flac",
    );
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_two.wav"),
        b"not-real-wav",
    );
    write_file(&music_root.join("loose.mp3"), b"not-real-mp3");
    write_file(&music_root.join("artwork").join("cover.png"), b"fake-png");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    assert!(registered.root_id > 0);

    let scanned = start_scan(&service, registered.root_id);
    assert!(scanned.scan_run_id > 0);

    let completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(completed, "scan should complete on a small directory");

    let root_nav_rows = read_navigation_rows(&service, None);
    root_nav_rows
        .iter()
        .find(|row| {
            matches!(row.family, Some(NavigationRowFamily::Sources))
                && row.row_kind == NavigationRowKind::Source
        })
        .expect("registered source appears in navigation at root level");

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("source root resolves to hierarchy window");
    assert!(
        root_window.total_rows >= 1,
        "root window must contain at least artists/"
    );

    let artists_dir = root_window
        .rows
        .iter()
        .find(|row| {
            row.display_name == "artists" && row.node_kind == LibraryTreeNodeKind::Directory
        })
        .expect("artists directory in root hierarchy");

    assert!(
        !root_window
            .rows
            .iter()
            .any(|row| row.display_name == "loose.mp3"),
        "loose.mp3 remains source-file inventory and must not appear in tree hierarchy"
    );

    for row in &root_window.rows {
        assert_eq!(
            row.parent_source_directory_id, None,
            "root-level row '{}' must have parent_source_directory_id == None",
            row.display_name,
        );
    }

    for row in &root_window.rows {
        assert_eq!(
            row.node_kind,
            LibraryTreeNodeKind::Directory,
            "root tree row '{}' must be a navigation directory",
            row.display_name,
        );
    }

    for row in &root_window.rows {
        assert_eq!(
            row.presence_state,
            LibraryTreePresenceState::Present,
            "root row '{}' must be Present, got {:?}",
            row.display_name,
            row.presence_state,
        );
    }

    for row in &root_window.rows {
        assert!(
            row.directory_scan_state.is_some(),
            "directory '{}' must have directory_scan_state",
            row.display_name,
        );
        assert!(
            row.has_child_directories.is_some(),
            "directory '{}' must have has_child_directories",
            row.display_name,
        );
    }

    let artists_dir_id = artists_dir
        .source_directory_id
        .expect("artists directory has source_directory_id");
    let artists_reply = read_library_tree(&service, registered.root_id, Some(artists_dir_id));
    let artists_window = artists_reply
        .window
        .as_ref()
        .expect("artists directory resolves to hierarchy window");
    let alpha_dir = artists_window
        .rows
        .iter()
        .find(|row| row.display_name == "alpha")
        .expect("alpha directory inside artists");
    assert_eq!(
        alpha_dir.navigable_child_scope_state,
        Some(NavigableChildScopeState::NoNavigableChildScopes),
        "alpha contains tracks but no navigable child directories"
    );

    let alpha_dir_id = alpha_dir
        .source_directory_id
        .expect("alpha directory has source_directory_id");
    let alpha_reply = read_library_tree(&service, registered.root_id, Some(alpha_dir_id));
    let alpha_window = alpha_reply
        .window
        .as_ref()
        .expect("alpha directory resolves to hierarchy window");
    assert_eq!(alpha_window.total_rows, 0);
    assert!(
        alpha_window.rows.is_empty(),
        "track files in alpha remain contents/source-file inventory, not tree children"
    );

    let original_root_total = root_window.total_rows;
    let original_artists_row_count = artists_window.rows.len();
    let original_alpha_row_count = alpha_window.rows.len();

    drop(service);

    let reopened = open_service(&tempdir);

    let reopened_root_nav = read_navigation_rows(&reopened, None);
    reopened_root_nav
        .iter()
        .find(|row| {
            matches!(row.family, Some(NavigationRowFamily::Sources))
                && row.row_kind == NavigationRowKind::Source
        })
        .expect("registered source survives in navigation after reopen");

    let reopened_root_reply = read_library_tree(&reopened, registered.root_id, None);
    let reopened_root_window = reopened_root_reply
        .window
        .as_ref()
        .expect("root hierarchy survives service reopen");
    assert_eq!(
        reopened_root_window.total_rows, original_root_total,
        "root-level row count must survive reopen",
    );

    reopened_root_window
        .rows
        .iter()
        .find(|row| row.display_name == "artists")
        .expect("artists directory survives reopen");
    assert!(
        !reopened_root_window
            .rows
            .iter()
            .any(|row| row.display_name == "loose.mp3"),
        "loose.mp3 must remain absent from reopened tree hierarchy"
    );

    let reopened_artists_reply =
        read_library_tree(&reopened, registered.root_id, Some(artists_dir_id));
    let reopened_artists_window = reopened_artists_reply
        .window
        .as_ref()
        .expect("artists directory survives reopen");
    assert_eq!(
        reopened_artists_window.rows.len(),
        original_artists_row_count,
        "artists directory child count must survive reopen",
    );

    let reopened_alpha_reply = read_library_tree(&reopened, registered.root_id, Some(alpha_dir_id));
    let reopened_alpha_window = reopened_alpha_reply
        .window
        .as_ref()
        .expect("alpha directory survives reopen");
    assert_eq!(
        reopened_alpha_window.rows.len(),
        original_alpha_row_count,
        "alpha directory child count must survive reopen",
    );

    for window in [&root_reply.window, &reopened_root_reply.window] {
        let w = window.as_ref().expect("hierarchy window exists");
        assert_eq!(
            w.coverage.state,
            LibraryTreeCoverageState::Complete,
            "post-scan coverage state must be Complete",
        );
        assert!(
            w.coverage.subtree_coverage_complete,
            "post-scan scope coverage must be complete",
        );
    }
}

#[test]
fn registered_tree_children_are_served_from_persisted_scan_after_source_path_disappears() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_one.wav"),
        b"not-real-wav",
    );

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    let scanned = start_scan(&service, registered.root_id);
    assert!(scanned.scan_run_id > 0);

    let completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(completed, "scan should complete on a small directory");

    fs::remove_dir_all(&music_root).expect("remove source folder after scan");

    let root_reply = read_library_tree(&service, registered.root_id, None);
    let root_window = root_reply
        .window
        .as_ref()
        .expect("registered source tree uses persisted root window");
    assert_eq!(
        root_window.coverage.state,
        LibraryTreeCoverageState::Complete
    );
    let artists_dir = root_window
        .rows
        .iter()
        .find(|row| row.display_name == "artists")
        .expect("persisted root child directory");
    assert_eq!(artists_dir.node_kind, LibraryTreeNodeKind::Directory);
    assert_eq!(artists_dir.parent_source_directory_id, None);
    assert_eq!(
        artists_dir.directory_scan_state,
        Some(DirectoryScanState::Complete)
    );
    assert_eq!(
        artists_dir.navigable_child_scope_state,
        Some(NavigableChildScopeState::HasNavigableChildScopes)
    );
    let artists_dir_id = artists_dir
        .source_directory_id
        .expect("artists directory has persisted id");

    let artists_reply = read_library_tree(&service, registered.root_id, Some(artists_dir_id));
    let artists_window = artists_reply
        .window
        .as_ref()
        .expect("registered child folder tree uses persisted window");
    assert_eq!(
        artists_window.coverage.state,
        LibraryTreeCoverageState::Complete
    );
    assert_eq!(
        artists_window
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha"]
    );
}

#[test]
fn library_tree_children_excludes_image_only_directories() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_one.wav"),
        b"not-real-wav",
    );
    write_file(&music_root.join("artwork").join("cover.png"), b"fake-png");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    start_scan(&service, registered.root_id);
    wait_for_scan_completion(&service, registered.root_id);

    let reply = read_library_tree(&service, registered.root_id, None);
    let window = reply
        .window
        .as_ref()
        .expect("Library tree children resolves hierarchy");

    let has_artists = window.rows.iter().any(|row| row.display_name == "artists");
    assert!(
        has_artists,
        "artists directory must appear in library tree children"
    );

    let has_artwork = window.rows.iter().any(|row| row.display_name == "artwork");
    assert!(
        !has_artwork,
        "artwork directory must NOT appear in library tree children (contains only images)"
    );
}

#[test]
fn start_root_scan_publishes_started_and_completed_events_in_order() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let start_reply = start_scan(&service, registered.root_id);
    assert!(start_reply.scan_run_id > 0);

    let mut cursor: Option<i64> = None;
    let mut started = false;
    let mut completed = false;

    for _ in 0..30 {
        let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: cursor,
                max_events: 32,
            }),
        ));
        let reply = match outcome {
            CommandOutcome::Success(env) => match env.reply {
                CommandReply::LibraryBoundaryEvents(
                    LibraryBoundaryEventStreamReply::ReadAfter(r),
                ) => r,
                other => panic!("Expected event stream reply, got {other:?}"),
            },
            CommandOutcome::Error(_) => continue,
        };
        cursor = reply.latest_event_sequence;

        for event in &reply.events {
            let LibraryBoundaryEvent::SourceScanEvent(se) = event else {
                continue;
            };
            if se.root_id != registered.root_id {
                continue;
            }
            match se.kind {
                SourceScanEventKind::SourceScanStarted => started = true,
                SourceScanEventKind::SourceScanCompleted => {
                    assert!(started, "completed must appear after started");
                    completed = true;
                }
                _ => {}
            }
        }

        if completed {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    assert!(started, "SourceScanStarted must be published");
    assert!(completed, "SourceScanCompleted must be published");
}

#[test]
fn scan_publishes_maintained_snapshot_invalidations_on_completion() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let scanned = start_scan(&service, registered.root_id);

    let mut cursor: Option<i64> = None;
    let mut has_navigation_invalidation = false;
    let mut scan_completed = false;

    for _ in 0..30 {
        let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: cursor,
                max_events: 32,
            }),
        ));
        let reply = match outcome {
            CommandOutcome::Success(env) => match env.reply {
                CommandReply::LibraryBoundaryEvents(
                    LibraryBoundaryEventStreamReply::ReadAfter(r),
                ) => r,
                other => panic!("Expected event stream reply, got {other:?}"),
            },
            CommandOutcome::Error(_) => continue,
        };
        cursor = reply.latest_event_sequence;

        for event in &reply.events {
            match event {
                LibraryBoundaryEvent::SourceScanEvent(se) => {
                    if se.kind == SourceScanEventKind::SourceScanCompleted
                        && se.root_id == registered.root_id
                    {
                        scan_completed = true;
                    }
                }
                LibraryBoundaryEvent::MaintainedSnapshotInvalidated(inv) => {
                    if inv.invalidation.scope == MaintainedSnapshotScope::NavigationRows {
                        has_navigation_invalidation = true;
                    }
                }
            }
        }

        if scan_completed && has_navigation_invalidation {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    assert!(scan_completed, "SourceScanCompleted must be published");
    assert!(
        has_navigation_invalidation,
        "MaintainedSnapshotInvalidated for NavigationRows must be published after scan"
    );

    let first_cancel = cancel_scan(&service, scanned.scan_run_id);
    assert_eq!(
        first_cancel.status,
        CancelRootScanStatus::AlreadyTerminal,
        "cancel after scan completion must return AlreadyTerminal"
    );

    let second_cancel = cancel_scan(&service, scanned.scan_run_id);
    assert_eq!(
        second_cancel.status,
        CancelRootScanStatus::AlreadyTerminal,
        "second cancel after terminal scan must also return AlreadyTerminal"
    );
}

#[test]
fn cancel_active_scan_publishes_cancelled_and_registry_cleans_on_terminal() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("large-root");
    fs::create_dir_all(&music_root).expect("create large root");

    for d in 0..20 {
        let dir = music_root.join(format!("dir-{:03}", d));
        fs::create_dir_all(&dir).expect("create subdir");
        for f in 0..20 {
            fs::write(
                dir.join(format!("track-{:03}.wav", f)),
                b"not-real-audio-data",
            )
            .expect("write file");
        }
    }

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    let scanned = start_scan(&service, registered.root_id);

    assert!(scanned.scan_run_id > 0);

    let reply = cancel_scan(&service, scanned.scan_run_id);
    assert_eq!(
        reply.status,
        CancelRootScanStatus::Accepted,
        "cancel of active scan should return Accepted"
    );

    let mut cursor: Option<i64> = None;
    let mut started = false;
    let mut cancelled = false;
    let mut completed = false;

    for _ in 0..60 {
        let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: cursor,
                max_events: 64,
            }),
        ));
        let reply = match outcome {
            CommandOutcome::Success(env) => match env.reply {
                CommandReply::LibraryBoundaryEvents(
                    LibraryBoundaryEventStreamReply::ReadAfter(r),
                ) => r,
                other => panic!("Expected event stream reply, got {other:?}"),
            },
            CommandOutcome::Error(_) => continue,
        };
        cursor = reply.latest_event_sequence;

        for event in &reply.events {
            let LibraryBoundaryEvent::SourceScanEvent(se) = event else {
                continue;
            };
            if se.root_id != registered.root_id || se.scan_run_id != scanned.scan_run_id {
                continue;
            }
            match se.kind {
                SourceScanEventKind::SourceScanStarted => started = true,
                SourceScanEventKind::SourceScanCancelled => {
                    assert!(
                        started,
                        "SourceScanCancelled must appear after SourceScanStarted"
                    );
                    assert!(
                        !completed,
                        "SourceScanCancelled must not be preceded by SourceScanCompleted"
                    );
                    assert_eq!(se.phase, ScanRunPhase::Interrupted);
                    cancelled = true;
                }
                SourceScanEventKind::SourceScanCompleted => {
                    completed = true;
                }
                _ => {}
            }
        }

        if cancelled {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(
        cancelled,
        "SourceScanCancelled must be published after cancel accepted"
    );
    assert!(
        !completed,
        "SourceScanCompleted must not be published for cancelled scan"
    );

    let second_cancel = cancel_scan(&service, scanned.scan_run_id);
    assert_eq!(
        second_cancel.status,
        CancelRootScanStatus::AlreadyTerminal,
        "second cancel after scan termination must return AlreadyTerminal"
    );
}

#[test]
fn duplicate_start_rejects_then_rescan_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let first = start_scan(&service, registered.root_id);
    assert!(first.scan_run_id > 0);

    let error = expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::StartRootScan(StartRootScanRequest {
            root_id: registered.root_id,
        }),
    )));
    assert_eq!(error.error.code(), "INVALID_REQUEST");
    assert!(
        error.error.to_string().contains("already running"),
        "Duplicate start should be rejected while scan is running"
    );

    assert!(wait_for_scan_completion(&service, registered.root_id));

    let rescan = start_scan(&service, registered.root_id);
    assert!(rescan.scan_run_id > 0);
    assert_ne!(rescan.scan_run_id, first.scan_run_id);
    assert!(wait_for_scan_completion(&service, registered.root_id));
}
