use std::fs;
use std::io::Write;

use library_boundary_protocol::{
    CommandReply, CommandRequest, LibraryRootCommand, LibraryRootReply,
    RegisterLocalRootRequest, RunRootScanRequest,
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

fn create_music_folder(root: &std::path::Path) {
    let nested = root.join("artists").join("alpha");
    fs::create_dir_all(&nested).expect("create nested dir");
    let mut f = fs::File::create(nested.join("track_one.wav")).expect("create wav");
    f.write_all(b"not-real-audio-data").expect("write wav data");
    let mut f = fs::File::create(nested.join("track_two.flac")).expect("create flac");
    f.write_all(b"not-real-audio-data").expect("write flac data");

    let images = root.join("artwork");
    fs::create_dir_all(&images).expect("create artwork dir");
    let mut f = fs::File::create(images.join("cover.png")).expect("create png");
    f.write_all(b"fake-png-data").expect("write png data");

    let mut f = fs::File::create(root.join("loose.mp3")).expect("create mp3");
    f.write_all(b"fake-mp3-data").expect("write mp3 data");
}

#[test]
fn register_and_scan_with_nested_directories_and_audio_files() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    std::fs::create_dir_all(&music_root).expect("create music root");
    create_music_folder(&music_root);

    let service = open_service(&tempdir);

    let register_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
        RegisterLocalRootRequest {
            absolute_path: music_root.to_string_lossy().into_owned(),
        },
    ));
    let outcome = service.handle_command(register_cmd);
    let reply = match outcome {
        library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
        library_boundary_protocol::CommandOutcome::Error(env) => {
            panic!("Registration failed: {:?}", env.error);
        }
    };
    let registered = match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(reply)) => reply,
        other => panic!("Expected register reply, got {:?}", other),
    };
    assert!(registered.root_id > 0);
    assert!(!registered.canonical_path.is_empty());

    let scan_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(
        RunRootScanRequest {
            root_id: registered.root_id,
        },
    ));
    let outcome = service.handle_command(scan_cmd);
    let reply = match outcome {
        library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
        library_boundary_protocol::CommandOutcome::Error(env) => {
            panic!(
                "Scan failed with protocol error: {:?}",
                env.error
            );
        }
    };
    let scanned = match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(reply)) => reply,
        other => panic!("Expected scan reply, got {:?}", other),
    };

    assert_eq!(scanned.root_id, registered.root_id);
    assert!(scanned.scan_run_id > 0);
    assert_eq!(
        scanned.discovered_file_count, 4,
        "Expected 4 files (3 audio + 1 image), got {}",
        scanned.discovered_file_count,
    );
    assert!(scanned.queued_source_work_items > 0);
}

#[test]
fn scan_empty_folder_succeeds_with_zero_files() {
    let tempdir = TempDir::new().expect("create tempdir");
    let empty_root = tempdir.path().join("empty-root");
    std::fs::create_dir_all(&empty_root).expect("create empty root");

    let service = open_service(&tempdir);

    let register_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
        RegisterLocalRootRequest {
            absolute_path: empty_root.to_string_lossy().into_owned(),
        },
    ));
    let outcome = service.handle_command(register_cmd);
    let reply = match outcome {
        library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
        library_boundary_protocol::CommandOutcome::Error(env) => {
            panic!("Registration failed: {:?}", env.error);
        }
    };
    let registered = match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(reply)) => reply,
        other => panic!("Expected register reply, got {:?}", other),
    };

    let scan_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(
        RunRootScanRequest {
            root_id: registered.root_id,
        },
    ));
    let outcome = service.handle_command(scan_cmd);
    let reply = match outcome {
        library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
        library_boundary_protocol::CommandOutcome::Error(env) => {
            panic!("Scan failed: {:?}", env.error);
        }
    };
    let scanned = match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(reply)) => reply,
        other => panic!("Expected scan reply, got {:?}", other),
    };

    assert_eq!(scanned.root_id, registered.root_id);
    assert!(scanned.scan_run_id > 0);
    assert_eq!(scanned.discovered_file_count, 0);
    assert_eq!(scanned.queued_source_work_items, 0);
}

#[test]
fn rescan_after_first_scan_succeeds() {
    let tempdir = TempDir::new().expect("create tempdir");
    let music_root = tempdir.path().join("music-root");
    std::fs::create_dir_all(&music_root).expect("create music root");
    let mut f = fs::File::create(music_root.join("track.wav")).expect("create wav");
    f.write_all(b"data").expect("write data");

    let service = open_service(&tempdir);

    let register_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
        RegisterLocalRootRequest {
            absolute_path: music_root.to_string_lossy().into_owned(),
        },
    ));
    let outcome = service.handle_command(register_cmd);
    let reply = match outcome {
        library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
        library_boundary_protocol::CommandOutcome::Error(_env) => panic!("Register fail"),
    };
    let registered = match reply {
        CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(reply)) => reply,
        other => panic!("Expected register reply, got {:?}", other),
    };

    let do_scan = |service: &LibraryBoundaryService| -> usize {
        let outcome = service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::RunRootScan(RunRootScanRequest {
                root_id: registered.root_id,
            }),
        ));
        let reply = match outcome {
            library_boundary_protocol::CommandOutcome::Success(env) => env.reply,
            library_boundary_protocol::CommandOutcome::Error(env) => {
                panic!("Scan failed: {:?}", env.error);
            }
        };
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(reply)) => {
                reply.discovered_file_count
            }
            other => panic!("Expected scan reply, got {:?}", other),
        }
    };

    let first_count = do_scan(&service);
    assert_eq!(first_count, 1, "First scan should find 1 file");

    let second_count = do_scan(&service);
    assert_eq!(second_count, 1, "Rescan should find 1 file");
}

#[test]
fn scan_with_stale_root_id_returns_error() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let scan_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(
        RunRootScanRequest { root_id: 99999 },
    ));
    let outcome = service.handle_command(scan_cmd);
    match outcome {
        library_boundary_protocol::CommandOutcome::Success(_) => {
            panic!("Scan with unknown root should fail");
        }
        library_boundary_protocol::CommandOutcome::Error(env) => {
            assert_eq!(env.error.code(), "DURABLE_STORE_FAILURE");
            let msg = env.error.to_string();
            assert!(
                msg.contains("does not exist"),
                "Error message should mention root does not exist, got: {msg}",
            );
        }
    }
}

#[test]
fn scan_with_invalid_root_id_returns_protocol_error() {
    let tempdir = TempDir::new().expect("create tempdir");
    let service = open_service(&tempdir);

    let scan_cmd = CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(
        RunRootScanRequest { root_id: 0 },
    ));
    let outcome = service.handle_command(scan_cmd);
    match outcome {
        library_boundary_protocol::CommandOutcome::Success(_) => {
            panic!("Scan with zero root ID should fail");
        }
        library_boundary_protocol::CommandOutcome::Error(env) => {
            assert_eq!(env.error.code(), "INVALID_REQUEST");
            let msg = env.error.to_string();
            assert!(
                msg.contains("rootId"),
                "Error should mention rootId, got: {msg}"
            );
        }
    }

    let scan_cmd2 = CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(
        RunRootScanRequest { root_id: -5 },
    ));
    let outcome2 = service.handle_command(scan_cmd2);
    match outcome2 {
        library_boundary_protocol::CommandOutcome::Success(_) => {
            panic!("Scan with negative root ID should fail");
        }
        library_boundary_protocol::CommandOutcome::Error(env) => {
            assert_eq!(env.error.code(), "INVALID_REQUEST");
        }
    }
}
