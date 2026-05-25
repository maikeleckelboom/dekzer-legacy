use std::fs;
use std::path::Path;

use library_boundary_protocol::{
    CommandErrorEnvelope, CommandOutcome, CommandReply, CommandRequest, LibraryRootCommand,
    LibraryRootReply, RegisterLocalRootReply, RegisterLocalRootRequest, RunRootScanReply,
    RunRootScanRequest,
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

fn run_scan(service: &LibraryBoundaryService, root_id: i64) -> RunRootScanReply {
    let outcome = service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::RunRootScan(RunRootScanRequest { root_id }),
    ));
    let reply = expect_command_reply(outcome, "run root scan");
    match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(reply)) => reply,
        other => panic!("Expected scan reply, got {other:?}"),
    }
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
    write_file(
        &music_root.join("artwork").join("cover.png"),
        b"fake-png-data",
    );
    write_file(&music_root.join("loose.mp3"), b"fake-mp3-data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);
    let scanned = run_scan(&service, registered.root_id);

    assert!(registered.root_id > 0);
    assert!(!registered.canonical_path.is_empty());
    assert_eq!(scanned.root_id, registered.root_id);
    assert!(scanned.scan_run_id > 0);
    assert_eq!(scanned.discovered_file_count, 4);
    assert!(scanned.queued_source_work_items > 0);
}

#[test]
fn scan_empty_folder_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let empty_root = tempdir.path().join("empty-root");
    fs::create_dir_all(&empty_root).expect("create empty root");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &empty_root);
    let scanned = run_scan(&service, registered.root_id);

    assert_eq!(scanned.root_id, registered.root_id);
    assert!(scanned.scan_run_id > 0);
    assert_eq!(scanned.discovered_file_count, 0);
    assert_eq!(scanned.queued_source_work_items, 0);
}

#[test]
fn rescan_after_successful_scan_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    write_file(&music_root.join("track.wav"), b"data");

    let service = open_service(&tempdir);
    let registered = register_root(&service, &music_root);

    let first_scan = run_scan(&service, registered.root_id);
    assert_eq!(first_scan.root_id, registered.root_id);
    assert!(first_scan.scan_run_id > 0);
    assert_eq!(first_scan.discovered_file_count, 1);

    let rescan = run_scan(&service, registered.root_id);
    assert_eq!(rescan.root_id, registered.root_id);
    assert!(rescan.scan_run_id > 0);
    assert_eq!(rescan.discovered_file_count, 1);
}

#[test]
fn scan_with_invalid_root_id_rejects() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let zero_root_error =
        expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::RunRootScan(RunRootScanRequest { root_id: 0 }),
        )));
    assert_eq!(zero_root_error.error.code(), "INVALID_REQUEST");
    assert!(
        zero_root_error.error.to_string().contains("rootId"),
        "Error should mention rootId, got: {}",
        zero_root_error.error,
    );

    let negative_root_error =
        expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::RunRootScan(RunRootScanRequest { root_id: -5 }),
        )));
    assert_eq!(negative_root_error.error.code(), "INVALID_REQUEST");
}

#[test]
fn scan_with_unknown_root_id_returns_current_store_error() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let error = expect_command_error(service.handle_command(CommandRequest::LibraryRoots(
        LibraryRootCommand::RunRootScan(RunRootScanRequest { root_id: 99999 }),
    )));

    assert_eq!(error.error.code(), "DURABLE_STORE_FAILURE");
    assert!(
        error.error.to_string().contains("does not exist"),
        "Current store error should mention root does not exist, got: {}",
        error.error,
    );
}
