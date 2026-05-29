use std::fs;
use std::path::Path;

use library_boundary_protocol::{
    CommandErrorEnvelope, CommandOutcome, CommandReply, CommandRequest,
    LibraryBoundaryEventStreamCommand, LibraryBoundaryEventStreamReply,
    LibraryRootCommand,
    LibraryRootReply, LibraryTreeCoverageState, LibraryTreeEntryPoint,
    LibraryTreeNodeKind, LibraryTreePresenceState, MaintainedSnapshotScope,
    NavigationRow,
    NavigationRowFamily, NavigationRowKind, ReadLibraryBoundaryEventsAfterRequest,
    ReadLibraryTreeChildrenReply,
    ReadLibraryTreeChildrenRequest, ReadNavigationRowsRequest, RegisterLocalRootReply,
    RegisterLocalRootRequest, StartRootScanReply, StartRootScanRequest, SnapshotReadCommand,
    SnapshotReadReply, LibraryBoundaryEvent, SourceScanEventKind,
};
use library_boundary_service::{LibraryBoundaryService, LibraryStoreContext, StoreEnvironment};
use tempfile::TempDir;

fn open_service(tempdir: &TempDir) -> LibraryBoundaryService {
    LibraryBoundaryService::open(LibraryStoreContext {
        user_data_path: tempdir.path().to_string_lossy().into_owned(),
        environment: StoreEnvironment::Development,
    })
    .expect("open boundary service")
}

fn write_file(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, bytes).expect("write file");
}

fn register_root(service: &LibraryBoundaryService, path: &Path) -> RegisterLocalRootReply {
    let outcome = service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::RegisterLocalRoot(RegisterLocalRootRequest {
            absolute_path: path.to_string_lossy().into_owned(),
        }),
    ));
    let reply = expect_command_reply(outcome, "register local root");
    match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(reply)) => reply,
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

fn wait_for_scan_completion(service: &LibraryBoundaryService, root_id: i64) -> bool {
    let mut cursor: Option<i64> = None;
    for _ in 0..30 {
        let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(
                ReadLibraryBoundaryEventsAfterRequest {
                    last_seen_event_sequence: cursor,
                    max_events: 32,
                },
            ),
        ));
        let reply = match outcome {
            CommandOutcome::Success(env) => match env.reply {
                CommandReply::LibraryBoundaryEvents(
                    library_boundary_protocol::LibraryBoundaryEventStreamReply::ReadAfter(r),
                ) => r,
                other => panic!("Expected event stream reply, got {other:?}"),
            },
            CommandOutcome::Error(_) => continue,
        };
        cursor = reply.latest_event_sequence;
        let completed = reply.events.iter().any(|e| {
            let LibraryBoundaryEvent::SourceScanEvent(se) = e else {
                return false;
            };
            se.kind == SourceScanEventKind::SourceScanCompleted && se.root_id == root_id
        });
        if completed {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
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
        SnapshotReadCommand::ReadLibraryTreeChildren(
            ReadLibraryTreeChildrenRequest {
                entry_point: LibraryTreeEntryPoint::Source { source_id },
                parent_source_directory_id,
                offset: 0,
                limit: 50,
            },
        ),
    ));
    let reply = expect_command_reply(outcome, "read library tree");
    match reply {
        CommandReply::SnapshotRead(SnapshotReadReply::LibraryTreeChildren(r)) => r,
        other => panic!("Expected library tree reply, got {other:?}"),
    }
}

#[test]
fn register_and_scan_mixed_nested_folder_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_one.wav"),
        b"not-real-audio-data",
    );
    write_file(
        &music_root
            .join("artists")
            .join("alpha")
            .join("track_two.flac"),
        b"not-real-audio-data",
    );
    write_file(&music_root.join("artwork").join("cover.png"), b"fake-png-data");
    write_file(&music_root.join("loose.mp3"), b"fake-mp3-data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    let scanned = start_scan(&service, registered.root_id);

    assert!(registered.root_id > 0);
    assert!(!registered.canonical_path.is_empty());
    assert!(scanned.scan_run_id > 0);

    let completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(completed, "scan should finish on a small directory");
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
fn rescan_after_successful_scan_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let first_scan = start_scan(&service, registered.root_id);
    assert!(first_scan.scan_run_id > 0);
    let first_completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(first_completed, "first scan should complete");

    let rescan = start_scan(&service, registered.root_id);
    assert!(rescan.scan_run_id > 0);
    assert_ne!(rescan.scan_run_id, first_scan.scan_run_id);
    let second_completed = wait_for_scan_completion(&service, registered.root_id);
    assert!(second_completed, "rescan should complete");
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
    write_file(
        &music_root.join("artwork").join("cover.png"),
        b"fake-png",
    );

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
        root_window.total_rows >= 2,
        "root window must contain at least artists/ and loose.mp3"
    );

    let artists_dir = root_window
        .rows
        .iter()
        .find(|row| row.display_name == "artists" && row.node_kind == LibraryTreeNodeKind::Directory)
        .expect("artists directory in root hierarchy");

    root_window
        .rows
        .iter()
        .find(|row| row.display_name == "loose.mp3" && row.node_kind == LibraryTreeNodeKind::File)
        .expect("loose.mp3 file in root hierarchy");

    for row in &root_window.rows {
        assert_eq!(
            row.parent_source_directory_id, None,
            "root-level row '{}' must have parent_source_directory_id == None",
            row.display_name,
        );
    }

    for row in &root_window.rows {
        assert!(
            matches!(
                row.node_kind,
                LibraryTreeNodeKind::Directory | LibraryTreeNodeKind::File,
            ),
            "root row '{}' has unexpected node_kind {:?}",
            row.display_name,
            row.node_kind,
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
        match row.node_kind {
            LibraryTreeNodeKind::Directory => {
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
            LibraryTreeNodeKind::File => {
                assert!(
                    row.directory_scan_state.is_none(),
                    "file '{}' must not have directory_scan_state",
                    row.display_name,
                );
                assert!(
                    row.has_child_directories.is_none(),
                    "file '{}' must not have has_child_directories",
                    row.display_name,
                );
                assert!(
                    row.directory_primary_media_state.is_none(),
                    "file '{}' must not have directory_primary_media_state",
                    row.display_name,
                );
                assert!(
                    row.directory_image_media_state.is_none(),
                    "file '{}' must not have directory_image_media_state",
                    row.display_name,
                );
            }
        }
    }

    let artists_dir_id = artists_dir
        .source_directory_id
        .expect("artists directory has source_directory_id");
    let artists_reply =
        read_library_tree(&service, registered.root_id, Some(artists_dir_id));
    let artists_window = artists_reply
        .window
        .as_ref()
        .expect("artists directory resolves to hierarchy window");
    let alpha_dir = artists_window
        .rows
        .iter()
        .find(|row| row.display_name == "alpha")
        .expect("alpha directory inside artists");

    let alpha_dir_id = alpha_dir
        .source_directory_id
        .expect("alpha directory has source_directory_id");
    let alpha_reply =
        read_library_tree(&service, registered.root_id, Some(alpha_dir_id));
    let alpha_window = alpha_reply
        .window
        .as_ref()
        .expect("alpha directory resolves to hierarchy window");
    assert!(
        alpha_window
            .rows
            .iter()
            .any(|r| r.display_name == "track_one.flac"),
        "track_one.flac in alpha directory"
    );
    assert!(
        alpha_window
            .rows
            .iter()
            .any(|r| r.display_name == "track_two.wav"),
        "track_two.wav in alpha directory"
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
    reopened_root_window
        .rows
        .iter()
        .find(|row| row.display_name == "loose.mp3")
        .expect("loose.mp3 file survives reopen");

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

    let reopened_alpha_reply =
        read_library_tree(&reopened, registered.root_id, Some(alpha_dir_id));
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
            w.coverage.recursive_scope_complete,
            "post-scan recursive scope must be complete",
        );
    }
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
    write_file(
        &music_root.join("artwork").join("cover.png"),
        b"fake-png",
    );

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    start_scan(&service, registered.root_id);
    wait_for_scan_completion(&service, registered.root_id);

    let reply = read_library_tree(&service, registered.root_id, None);
    let window = reply
        .window
        .as_ref()
        .expect("Library tree children resolves hierarchy");

    let has_artists = window
        .rows
        .iter()
        .any(|row| row.display_name == "artists");
    assert!(has_artists, "artists directory must appear in library tree children");

    let has_artwork = window
        .rows
        .iter()
        .any(|row| row.display_name == "artwork");
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
            LibraryBoundaryEventStreamCommand::ReadAfter(
                ReadLibraryBoundaryEventsAfterRequest {
                    last_seen_event_sequence: cursor,
                    max_events: 32,
                },
            ),
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
fn duplicate_start_root_scan_rejects_with_already_running_error() {
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
        "Error should mention already running, got: {}",
        error.error
    );
}

#[test]
fn scan_publishes_maintained_snapshot_invalidations_on_completion() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    start_scan(&service, registered.root_id);

    let mut cursor: Option<i64> = None;
    let mut has_navigation_invalidation = false;
    let mut scan_completed = false;

    for _ in 0..30 {
        let outcome = service.handle_command(CommandRequest::LibraryBoundaryEvents(
            LibraryBoundaryEventStreamCommand::ReadAfter(
                ReadLibraryBoundaryEventsAfterRequest {
                    last_seen_event_sequence: cursor,
                    max_events: 32,
                },
            ),
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
}
