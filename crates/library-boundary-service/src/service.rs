use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use library_boundary_protocol as protocol;
use library_domain::{LibraryAssetId, PlaylistId};
use library_store_sqlite::{
    AppendLibraryAssetToPlaylistInput, CreatePlaylistInput, DeletePlaylistInput,
    LibraryStoreContext, LocalRootAvailability, MovePlaylistEntryInput, ReadLocalRootsResult,
    RegisterLocalRootInput, RemoveLibraryAssetFromPlaylistInput, RenamePlaylistInput,
    RootScanObservation, SourceFileBlake3HashAdmissionScope, SqliteDurableStore,
    UnregisterLocalRootInput,
};

use crate::session_events::{LibraryBoundaryEventStream, ScanEventInput};
use crate::snapshot_read_protocol::{
    map_load_navigation_row_by_stable_key_reply, map_load_navigation_row_reply,
    map_maintained_read_model_revisions, map_read_contents_reply,
    map_read_library_asset_preparation_detail_reply,
    map_read_library_asset_waveform_overview_reply, map_read_library_tree_children_reply,
    map_read_navigation_node_library_browser_window_reply, map_read_navigation_rows_reply,
    map_read_source_lifecycle_reply, map_search_navigation_node_library_browser_window_reply,
    store_contents_policy, store_contents_recursion, store_contents_scope,
    store_library_tree_entry_point,
};
use crate::source_file_hash_protocol::{
    empty_hash_source_files_blake3_reply, map_hash_lifecycle_source_failure,
    map_hash_source_files_blake3_reply,
};
use crate::storage_environment::resolve_library_storage_environment;

struct ActiveScanJob {
    #[allow(dead_code)]
    root_id: i64,
    scan_run_id: i64,
    terminal_publication_complete: Arc<AtomicBool>,
    _handle: JoinHandle<()>,
}

#[derive(Default)]
struct ScanJobRegistry {
    jobs: HashMap<i64, ActiveScanJob>,
    scan_run_id_to_root_id: HashMap<i64, i64>,
    terminal_scan_run_ids: HashSet<i64>,
    next_scan_run_id: i64,
}

impl ScanJobRegistry {
    fn next_scan_run_id(&mut self) -> i64 {
        if self.next_scan_run_id == 0 {
            self.next_scan_run_id = 1;
        }
        let id = self.next_scan_run_id;
        self.next_scan_run_id += 1;
        id
    }

    fn cleanup_terminal_scan(&mut self, root_id: i64) {
        if let Some(job) = self.jobs.remove(&root_id) {
            self.scan_run_id_to_root_id.remove(&job.scan_run_id);
            self.terminal_scan_run_ids.insert(job.scan_run_id);
        }
    }
}

pub struct LibraryBoundaryService {
    durable_store: SqliteDurableStore,
    session_events: LibraryBoundaryEventStream,
    scan_registry: Mutex<ScanJobRegistry>,
}

impl LibraryBoundaryService {
    pub fn open(store_context: LibraryStoreContext) -> protocol::ProtocolResult<Self> {
        let storage_environment =
            resolve_library_storage_environment(&store_context).map_err(|error| {
                protocol::ProtocolError::InvalidRequest {
                    detail: error.to_string(),
                }
            })?;
        fs::create_dir_all(storage_environment.storage_root_path()).map_err(|error| {
            protocol::ProtocolError::HostFailure {
                detail: format!("failed to prepare durable store directory: {error}"),
            }
        })?;

        let durable_store = SqliteDurableStore::open(storage_environment.durable_store_path())
            .map_err(map_store_error)?;
        Self::from_store(durable_store)
    }

    pub fn from_store(durable_store: SqliteDurableStore) -> protocol::ProtocolResult<Self> {
        let initial_revisions = durable_store
            .read_maintained_read_model_revisions()
            .map_err(map_store_error)?;
        let session_events =
            LibraryBoundaryEventStream::new(map_maintained_read_model_revisions(initial_revisions));

        Ok(Self {
            durable_store,
            session_events,
            scan_registry: Mutex::new(ScanJobRegistry::default()),
        })
    }

    pub fn handle_command(&self, request: protocol::CommandRequest) -> protocol::CommandOutcome {
        match self.try_handle_command(request) {
            Ok(reply) => {
                protocol::CommandOutcome::Success(protocol::CommandSuccessEnvelope { reply })
            }
            Err(error) => protocol::CommandOutcome::Error(protocol::CommandErrorEnvelope { error }),
        }
    }

    pub fn try_handle_command(
        &self,
        request: protocol::CommandRequest,
    ) -> protocol::ProtocolResult<protocol::CommandReply> {
        match request {
            protocol::CommandRequest::LibraryBoundaryEvents(command) => self
                .handle_library_boundary_event_command(command)
                .map(protocol::CommandReply::LibraryBoundaryEvents),
            protocol::CommandRequest::LibraryRoots(command) => self
                .handle_library_root_command(command)
                .map(protocol::CommandReply::LibraryRoots),
            protocol::CommandRequest::PlaylistWrite(command) => self
                .handle_playlist_write_command(command)
                .map(protocol::CommandReply::PlaylistWrite),
            protocol::CommandRequest::SourceFileHash(command) => self
                .handle_source_file_hash_command(command)
                .map(protocol::CommandReply::SourceFileHash),
            protocol::CommandRequest::SnapshotRead(command) => self
                .handle_snapshot_read_command(command)
                .map(protocol::CommandReply::SnapshotRead),
        }
    }

    pub fn read_library_boundary_events_after(
        &self,
        request: protocol::ReadLibraryBoundaryEventsAfterRequest,
    ) -> protocol::ProtocolResult<protocol::ReadLibraryBoundaryEventsAfterReply> {
        if request.max_events == 0 {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "libraryBoundaryEvents.readAfter maxEvents must be greater than zero"
                    .to_string(),
            });
        }

        self.publish_maintained_snapshot_invalidations()?;
        let (events, latest_event_sequence, earliest_retained_sequence, gap_detected) = self
            .session_events
            .read_after(request.last_seen_event_sequence, request.max_events);
        Ok(protocol::ReadLibraryBoundaryEventsAfterReply {
            events,
            latest_event_sequence,
            earliest_retained_sequence,
            gap_detected,
        })
    }

    pub fn register_local_root(
        &self,
        request: protocol::RegisterLocalRootRequest,
    ) -> protocol::ProtocolResult<protocol::RegisterLocalRootReply> {
        if request.absolute_path.trim().is_empty() {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "libraryRoots.registerLocalRoot absolutePath must not be empty".to_string(),
            });
        }

        let registered = self
            .durable_store
            .register_local_root(RegisterLocalRootInput {
                absolute_path: PathBuf::from(request.absolute_path),
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::RegisterLocalRootReply {
            root_id: registered.root_id,
            canonical_path: registered.canonical_path.to_string_lossy().into_owned(),
        })
    }

    pub fn start_root_scan(
        &self,
        request: protocol::StartRootScanRequest,
    ) -> protocol::ProtocolResult<protocol::StartRootScanReply> {
        let root_id = require_positive_i64(request.root_id, "rootId")?;
        let scan_started_at_ms = unix_time_ms()?;

        let mut registry = self.scan_registry.lock().expect("scan registry poisoned");

        if let Some(existing) = registry.jobs.get(&root_id) {
            if !existing
                .terminal_publication_complete
                .load(Ordering::Acquire)
            {
                return Err(protocol::ProtocolError::InvalidRequest {
                    detail: "A scan is already running for this root".to_string(),
                });
            }

            registry.cleanup_terminal_scan(root_id);
        }

        self.durable_store
            .read_root_scan_path(root_id)
            .map_err(|e| protocol::ProtocolError::DurableStoreFailure {
                detail: e.to_string(),
            })?;

        let scan_run_id = registry.next_scan_run_id();

        let store = self.durable_store.clone();
        let events = self.session_events.clone();
        let terminal_publication_complete = Arc::new(AtomicBool::new(false));
        let thread_terminal_publication_complete = Arc::clone(&terminal_publication_complete);

        events.publish_scan_event(ScanEventInput {
            kind: library_boundary_protocol::SourceScanEventKind::SourceScanStarted,
            root_id,
            scan_run_id,
            phase: library_boundary_protocol::ScanRunPhase::Scanning,
            directories_visited: 0,
            files_visited: 0,
            files_discovered: 0,
            media_candidates: 0,
            queued_work_items: 0,
            detail: None,
        });

        let handle = std::thread::spawn(move || {
            execute_scan_job(
                store,
                events,
                root_id,
                scan_run_id,
                scan_started_at_ms,
                thread_terminal_publication_complete,
            );
        });

        registry.scan_run_id_to_root_id.insert(scan_run_id, root_id);
        registry.jobs.insert(
            root_id,
            ActiveScanJob {
                root_id,
                scan_run_id,
                terminal_publication_complete,
                _handle: handle,
            },
        );

        Ok(protocol::StartRootScanReply { scan_run_id })
    }

    pub fn cancel_root_scan(
        &self,
        request: protocol::CancelRootScanRequest,
    ) -> protocol::ProtocolResult<protocol::CancelRootScanReply> {
        let scan_run_id = require_positive_i64(request.scan_run_id, "scanRunId")?;

        let mut registry = self.scan_registry.lock().expect("scan registry poisoned");

        let root_id = match registry.scan_run_id_to_root_id.get(&scan_run_id).copied() {
            Some(id) => id,
            None => {
                let status = if registry.terminal_scan_run_ids.contains(&scan_run_id) {
                    protocol::CancelRootScanStatus::AlreadyTerminal
                } else {
                    protocol::CancelRootScanStatus::NotFound
                };
                return Ok(protocol::CancelRootScanReply { status });
            }
        };

        let job = match registry.jobs.get(&root_id) {
            Some(job) if job.scan_run_id == scan_run_id => job,
            _ => {
                return Ok(protocol::CancelRootScanReply {
                    status: protocol::CancelRootScanStatus::NotFound,
                });
            }
        };

        if job.terminal_publication_complete.load(Ordering::Acquire) {
            registry.cleanup_terminal_scan(root_id);
            return Ok(protocol::CancelRootScanReply {
                status: protocol::CancelRootScanStatus::AlreadyTerminal,
            });
        }

        self.durable_store.cancel_root_work(&[root_id]);

        Ok(protocol::CancelRootScanReply {
            status: protocol::CancelRootScanStatus::Accepted,
        })
    }

    pub fn create_playlist(
        &self,
        request: protocol::CreatePlaylistRequest,
    ) -> protocol::ProtocolResult<protocol::CreatePlaylistReply> {
        let created_at = unix_time_ms()?;
        let playlist_id = self
            .durable_store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: request.display_name,
                created_at,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::CreatePlaylistReply {
            playlist_id: playlist_id.get(),
        })
    }

    pub fn rename_playlist(
        &self,
        request: protocol::RenamePlaylistRequest,
    ) -> protocol::ProtocolResult<protocol::RenamePlaylistReply> {
        let playlist_id = require_playlist_id(request.playlist_id, "playlistId")?;
        let renamed_at = unix_time_ms()?;
        let renamed = self
            .durable_store
            .rename_playlist(RenamePlaylistInput {
                playlist_id,
                display_name: request.display_name,
                renamed_at,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::RenamePlaylistReply { renamed })
    }

    pub fn delete_playlist(
        &self,
        request: protocol::DeletePlaylistRequest,
    ) -> protocol::ProtocolResult<protocol::DeletePlaylistReply> {
        let playlist_id = require_playlist_id(request.playlist_id, "playlistId")?;
        let deleted = self
            .durable_store
            .delete_playlist(DeletePlaylistInput { playlist_id })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::DeletePlaylistReply { deleted })
    }

    pub fn append_library_asset_to_playlist(
        &self,
        request: protocol::AppendLibraryAssetToPlaylistRequest,
    ) -> protocol::ProtocolResult<protocol::AppendLibraryAssetToPlaylistReply> {
        let playlist_id = require_playlist_id(request.playlist_id, "playlistId")?;
        let library_asset_id =
            require_library_asset_id(request.library_asset_id, "libraryAssetId")?;
        let appended_at = unix_time_ms()?;
        let playlist_entry_id = self
            .durable_store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id,
                appended_at,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::AppendLibraryAssetToPlaylistReply { playlist_entry_id })
    }

    pub fn remove_library_asset_from_playlist(
        &self,
        request: protocol::RemoveLibraryAssetFromPlaylistRequest,
    ) -> protocol::ProtocolResult<protocol::RemoveLibraryAssetFromPlaylistReply> {
        let playlist_id = require_playlist_id(request.playlist_id, "playlistId")?;
        let library_asset_id =
            require_library_asset_id(request.library_asset_id, "libraryAssetId")?;
        let removed_at = unix_time_ms()?;
        let removed = self
            .durable_store
            .remove_library_asset_from_playlist(RemoveLibraryAssetFromPlaylistInput {
                playlist_id,
                library_asset_id,
                removed_at,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::RemoveLibraryAssetFromPlaylistReply { removed })
    }

    pub fn move_playlist_entry(
        &self,
        request: protocol::MovePlaylistEntryRequest,
    ) -> protocol::ProtocolResult<protocol::MovePlaylistEntryReply> {
        let playlist_id = require_playlist_id(request.playlist_id, "playlistId")?;
        let playlist_entry_id = require_positive_i64(request.playlist_entry_id, "playlistEntryId")?;
        if request.new_position < 0 {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "newPosition must be non-negative".to_string(),
            });
        }

        let moved_at = unix_time_ms()?;
        let moved = self
            .durable_store
            .move_playlist_entry(MovePlaylistEntryInput {
                playlist_id,
                playlist_entry_id,
                new_position: request.new_position,
                moved_at,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::MovePlaylistEntryReply { moved })
    }

    pub fn read_navigation_rows(
        &self,
        request: protocol::ReadNavigationRowsRequest,
    ) -> protocol::ProtocolResult<protocol::ReadNavigationRowsReply> {
        let rows = self
            .durable_store
            .read_navigation_rows(request.parent_navigation_row_id)
            .map_err(map_store_error)?;
        map_read_navigation_rows_reply(rows).map_err(map_store_error)
    }

    pub fn load_navigation_row(
        &self,
        request: protocol::LoadNavigationRowRequest,
    ) -> protocol::ProtocolResult<protocol::LoadNavigationRowReply> {
        let row = self
            .durable_store
            .load_navigation_row(request.navigation_row_id)
            .map_err(map_store_error)?;
        map_load_navigation_row_reply(row).map_err(map_store_error)
    }

    pub fn load_navigation_row_by_stable_key(
        &self,
        request: protocol::LoadNavigationRowByStableKeyRequest,
    ) -> protocol::ProtocolResult<protocol::LoadNavigationRowByStableKeyReply> {
        let row = self
            .durable_store
            .load_navigation_row_by_stable_key(&request.stable_key)
            .map_err(map_store_error)?;
        map_load_navigation_row_by_stable_key_reply(row).map_err(map_store_error)
    }

    pub fn read_library_tree_children(
        &self,
        request: protocol::ReadLibraryTreeChildrenRequest,
    ) -> protocol::ProtocolResult<protocol::ReadLibraryTreeChildrenReply> {
        let window = self
            .durable_store
            .read_literal_hierarchy_children(
                store_library_tree_entry_point(request.entry_point),
                request.parent_source_directory_id,
                request.offset,
                request.limit,
                library_store_sqlite::LibraryTreeRowAdmission::Performance,
            )
            .map_err(map_store_error)?;
        map_read_library_tree_children_reply(window).map_err(map_store_error)
    }

    pub fn read_source_lifecycle(
        &self,
        request: protocol::ReadSourceLifecycleRequest,
    ) -> protocol::ProtocolResult<protocol::ReadSourceLifecycleReply> {
        let source_id = require_positive_i64(request.source_id, "sourceId")?;
        let lifecycle = self
            .durable_store
            .read_source_lifecycle(source_id)
            .map_err(map_store_error)?;
        map_read_source_lifecycle_reply(lifecycle).map_err(map_store_error)
    }

    pub fn read_navigation_node_library_browser_window(
        &self,
        request: protocol::ReadNavigationNodeLibraryBrowserWindowRequest,
    ) -> protocol::ProtocolResult<protocol::ReadNavigationNodeLibraryBrowserWindowReply> {
        let window = self
            .durable_store
            .read_navigation_node_library_browser_window(
                request.navigation_row_id,
                request.offset,
                request.limit,
            )
            .map_err(map_store_error)?;
        map_read_navigation_node_library_browser_window_reply(window).map_err(map_store_error)
    }

    pub fn search_navigation_node_library_browser_window(
        &self,
        request: protocol::SearchNavigationNodeLibraryBrowserWindowRequest,
    ) -> protocol::ProtocolResult<protocol::SearchNavigationNodeLibraryBrowserWindowReply> {
        let window = self
            .durable_store
            .search_navigation_node_library_browser_window(
                request.navigation_row_id,
                &request.query,
                request.offset,
                request.limit,
            )
            .map_err(map_store_error)?;
        map_search_navigation_node_library_browser_window_reply(window).map_err(map_store_error)
    }

    pub fn read_contents(
        &self,
        request: protocol::ContentsReadRequest,
    ) -> protocol::ProtocolResult<protocol::ContentsReadReply> {
        validate_contents_scope(&request.scope)?;
        validate_contents_policy(&request.policy)?;
        let limit = request.limit.unwrap_or(100);
        validate_contents_limit(limit)?;
        let result = self
            .durable_store
            .read_contents(
                store_contents_scope(request.scope),
                store_contents_policy(request.policy),
                store_contents_recursion(request.recursion),
                limit,
                request.cursor.as_deref(),
            )
            .map_err(map_store_error)?;
        map_read_contents_reply(result).map_err(map_store_error)
    }

    pub fn read_library_asset_waveform_overview(
        &self,
        request: protocol::ReadLibraryAssetWaveformOverviewRequest,
    ) -> protocol::ProtocolResult<protocol::ReadLibraryAssetWaveformOverviewReply> {
        let overview = self
            .durable_store
            .read_library_asset_waveform_overview(request.library_asset_id)
            .map_err(map_store_error)?;
        Ok(map_read_library_asset_waveform_overview_reply(overview))
    }

    pub fn read_library_asset_preparation_detail(
        &self,
        request: protocol::ReadLibraryAssetPreparationDetailRequest,
    ) -> protocol::ProtocolResult<protocol::ReadLibraryAssetPreparationDetailReply> {
        let detail = self
            .durable_store
            .read_library_asset_preparation_detail(request.library_asset_id)
            .map_err(map_store_error)?;
        map_read_library_asset_preparation_detail_reply(detail).map_err(map_store_error)
    }

    pub fn hash_source_files_blake3(
        &self,
        request: protocol::HashSourceFilesBlake3Request,
    ) -> protocol::ProtocolResult<protocol::HashSourceFilesBlake3Reply> {
        let source_id = require_positive_i64(request.source_id, "sourceId")?;
        if matches!(request.limit, Some(0)) {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "hashSourceFilesBlake3 limit must be greater than zero".to_string(),
            });
        }

        let lifecycle = self
            .durable_store
            .read_source_lifecycle(source_id)
            .map_err(map_store_error)?;
        let source_failure =
            map_hash_lifecycle_source_failure(lifecycle.as_ref()).map_err(map_store_error)?;
        let effective_limit = library_store_sqlite::effective_hash_batch_limit(request.limit);

        if let Some(source_failure) = source_failure {
            return Ok(empty_hash_source_files_blake3_reply(
                effective_limit,
                source_failure,
            ));
        }

        let result = self
            .durable_store
            .hash_source_file_blake3_batch(library_store_sqlite::HashSourceFileBlake3BatchInput {
                scope: SourceFileBlake3HashAdmissionScope::Source { source_id },
                limit: request.limit,
                observed_at_ms: unix_time_ms()?,
            })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(map_hash_source_files_blake3_reply(result, None))
    }

    fn handle_library_boundary_event_command(
        &self,
        command: protocol::LibraryBoundaryEventStreamCommand,
    ) -> protocol::ProtocolResult<protocol::LibraryBoundaryEventStreamReply> {
        match command {
            protocol::LibraryBoundaryEventStreamCommand::ReadAfter(request) => self
                .read_library_boundary_events_after(request)
                .map(protocol::LibraryBoundaryEventStreamReply::ReadAfter),
        }
    }

    pub fn read_local_roots(&self) -> protocol::ProtocolResult<protocol::ReadLocalRootsReply> {
        let ReadLocalRootsResult { roots } = self
            .durable_store
            .read_local_roots()
            .map_err(map_store_error)?;
        Ok(protocol::ReadLocalRootsReply {
            roots: roots
                .into_iter()
                .map(|root| protocol::LocalRoot {
                    root_id: root.root_id,
                    canonical_path: root.canonical_path.to_string_lossy().into_owned(),
                    availability: match root.availability {
                        LocalRootAvailability::Available => {
                            protocol::LocalRootAvailability::Available
                        }
                        LocalRootAvailability::Unavailable => {
                            protocol::LocalRootAvailability::Unavailable
                        }
                    },
                })
                .collect(),
        })
    }

    pub fn unregister_local_root(
        &self,
        request: protocol::UnregisterLocalRootRequest,
    ) -> protocol::ProtocolResult<protocol::UnregisterLocalRootReply> {
        let root_id = require_positive_i64(request.root_id, "rootId")?;
        let result = self
            .durable_store
            .unregister_local_root(UnregisterLocalRootInput { root_id })
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::UnregisterLocalRootReply {
            unregistered: result.unregistered,
        })
    }

    fn handle_library_root_command(
        &self,
        command: protocol::LibraryRootCommand,
    ) -> protocol::ProtocolResult<protocol::LibraryRootReply> {
        match command {
            protocol::LibraryRootCommand::RegisterLocalRoot(request) => self
                .register_local_root(request)
                .map(protocol::LibraryRootReply::RegisterLocalRoot),
            protocol::LibraryRootCommand::StartRootScan(request) => self
                .start_root_scan(request)
                .map(protocol::LibraryRootReply::StartRootScan),
            protocol::LibraryRootCommand::CancelRootScan(request) => self
                .cancel_root_scan(request)
                .map(protocol::LibraryRootReply::CancelRootScan),
            protocol::LibraryRootCommand::ReadLocalRoots(_) => self
                .read_local_roots()
                .map(protocol::LibraryRootReply::ReadLocalRoots),
            protocol::LibraryRootCommand::UnregisterLocalRoot(request) => self
                .unregister_local_root(request)
                .map(protocol::LibraryRootReply::UnregisterLocalRoot),
        }
    }

    fn handle_playlist_write_command(
        &self,
        command: protocol::PlaylistWriteCommand,
    ) -> protocol::ProtocolResult<protocol::PlaylistWriteReply> {
        match command {
            protocol::PlaylistWriteCommand::CreatePlaylist(request) => self
                .create_playlist(request)
                .map(protocol::PlaylistWriteReply::CreatePlaylist),
            protocol::PlaylistWriteCommand::RenamePlaylist(request) => self
                .rename_playlist(request)
                .map(protocol::PlaylistWriteReply::RenamePlaylist),
            protocol::PlaylistWriteCommand::DeletePlaylist(request) => self
                .delete_playlist(request)
                .map(protocol::PlaylistWriteReply::DeletePlaylist),
            protocol::PlaylistWriteCommand::AppendLibraryAssetToPlaylist(request) => self
                .append_library_asset_to_playlist(request)
                .map(protocol::PlaylistWriteReply::AppendLibraryAssetToPlaylist),
            protocol::PlaylistWriteCommand::RemoveLibraryAssetFromPlaylist(request) => self
                .remove_library_asset_from_playlist(request)
                .map(protocol::PlaylistWriteReply::RemoveLibraryAssetFromPlaylist),
            protocol::PlaylistWriteCommand::MovePlaylistEntry(request) => self
                .move_playlist_entry(request)
                .map(protocol::PlaylistWriteReply::MovePlaylistEntry),
        }
    }

    fn handle_source_file_hash_command(
        &self,
        command: protocol::SourceFileHashCommand,
    ) -> protocol::ProtocolResult<protocol::SourceFileHashReply> {
        match command {
            protocol::SourceFileHashCommand::HashSourceFilesBlake3(request) => self
                .hash_source_files_blake3(request)
                .map(protocol::SourceFileHashReply::HashSourceFilesBlake3),
        }
    }

    fn handle_snapshot_read_command(
        &self,
        command: protocol::SnapshotReadCommand,
    ) -> protocol::ProtocolResult<protocol::SnapshotReadReply> {
        match command {
            protocol::SnapshotReadCommand::ReadNavigationRows(request) => self
                .read_navigation_rows(request)
                .map(protocol::SnapshotReadReply::NavigationRows),
            protocol::SnapshotReadCommand::LoadNavigationRow(request) => self
                .load_navigation_row(request)
                .map(protocol::SnapshotReadReply::NavigationRow),
            protocol::SnapshotReadCommand::LoadNavigationRowByStableKey(request) => self
                .load_navigation_row_by_stable_key(request)
                .map(protocol::SnapshotReadReply::NavigationRowByStableKey),
            protocol::SnapshotReadCommand::ReadLibraryTreeChildren(request) => self
                .read_library_tree_children(request)
                .map(protocol::SnapshotReadReply::LibraryTreeChildren),
            protocol::SnapshotReadCommand::ReadSourceLifecycle(request) => self
                .read_source_lifecycle(request)
                .map(protocol::SnapshotReadReply::SourceLifecycle),
            protocol::SnapshotReadCommand::ReadNavigationNodeLibraryBrowserWindow(request) => self
                .read_navigation_node_library_browser_window(request)
                .map(protocol::SnapshotReadReply::NavigationNodeLibraryBrowserWindow),
            protocol::SnapshotReadCommand::SearchNavigationNodeLibraryBrowserWindow(request) => {
                self.search_navigation_node_library_browser_window(request)
                    .map(protocol::SnapshotReadReply::NavigationNodeLibraryBrowserSearch)
            }
            protocol::SnapshotReadCommand::ContentsRead(request) => self
                .read_contents(request)
                .map(protocol::SnapshotReadReply::Contents),
            protocol::SnapshotReadCommand::ReadLibraryAssetWaveformOverview(request) => self
                .read_library_asset_waveform_overview(request)
                .map(protocol::SnapshotReadReply::LibraryAssetWaveformOverview),
            protocol::SnapshotReadCommand::ReadLibraryAssetPreparationDetail(request) => self
                .read_library_asset_preparation_detail(request)
                .map(protocol::SnapshotReadReply::LibraryAssetPreparationDetail),
        }
    }

    fn publish_maintained_snapshot_invalidations(&self) -> protocol::ProtocolResult<()> {
        let revisions = self
            .durable_store
            .read_maintained_read_model_revisions()
            .map_err(map_store_error)?;
        self.session_events
            .publish_revisions(map_maintained_read_model_revisions(revisions));
        Ok(())
    }
}

fn execute_scan_job(
    store: SqliteDurableStore,
    events: LibraryBoundaryEventStream,
    root_id: i64,
    scan_run_id: i64,
    scan_started_at_ms: i64,
    terminal_publication_complete: Arc<AtomicBool>,
) {
    let committed_chunk_count = Arc::new(AtomicUsize::new(0));
    let chunk_count = Arc::clone(&committed_chunk_count);

    let result = store.run_root_scan_with_observer(root_id, scan_started_at_ms, |observation| {
        match observation {
            RootScanObservation::HierarchyPublished {
                root_id: obs_root_id,
                scan_run_id: obs_scan_run_id,
                reason: _reason,
            } => {
                chunk_count.fetch_add(1, Ordering::Relaxed);
                events.publish_scan_event(ScanEventInput {
                    kind: library_boundary_protocol::SourceScanEventKind::SourceScanProgressed,
                    root_id: obs_root_id,
                    scan_run_id: obs_scan_run_id,
                    phase: library_boundary_protocol::ScanRunPhase::Scanning,
                    directories_visited: 0,
                    files_visited: 0,
                    files_discovered: 0,
                    media_candidates: 0,
                    queued_work_items: 0,
                    detail: None,
                });
            }
            RootScanObservation::SourceWorkQueued { .. } => {}
        }
    });

    match result {
        Ok(scan) => {
            publish_job_terminal_scan_event(
                &store,
                &events,
                &terminal_publication_complete,
                ScanEventInput {
                    kind: library_boundary_protocol::SourceScanEventKind::SourceScanCompleted,
                    root_id,
                    scan_run_id,
                    phase: library_boundary_protocol::ScanRunPhase::Scanning,
                    directories_visited: 0,
                    files_visited: scan.discovered_file_count,
                    files_discovered: scan.discovered_file_count,
                    media_candidates: scan.discovered_file_count,
                    queued_work_items: scan.queued_source_work_items,
                    detail: None,
                },
            );
        }
        Err(error) => {
            if matches!(
                &error,
                library_store_sqlite::LibrarySqliteError::RootWorkCancelled { .. }
                    | library_store_sqlite::LibrarySqliteError::RootWorkAdmissionDenied { .. }
            ) {
                publish_job_terminal_scan_event(
                    &store,
                    &events,
                    &terminal_publication_complete,
                    job_scan_cancelled_event(
                        root_id,
                        scan_run_id,
                        committed_chunk_count.load(Ordering::Relaxed),
                    ),
                );
            } else {
                publish_job_terminal_scan_event(
                    &store,
                    &events,
                    &terminal_publication_complete,
                    job_scan_failed_or_blocked_event(root_id, scan_run_id, &error),
                );
            }
        }
    }
}

fn publish_job_terminal_scan_event(
    store: &SqliteDurableStore,
    events: &LibraryBoundaryEventStream,
    terminal_publication_complete: &AtomicBool,
    event: ScanEventInput,
) {
    let revisions = store
        .read_maintained_read_model_revisions()
        .map(map_maintained_read_model_revisions)
        .unwrap_or_default();
    events.publish_terminal_scan_event(event, revisions, terminal_publication_complete);
}

fn job_scan_failed_or_blocked_event(
    root_id: i64,
    scan_run_id: i64,
    error: &library_store_sqlite::LibrarySqliteError,
) -> ScanEventInput {
    use library_store_sqlite::CanonicalErrorCode;

    let detail = error.to_string();
    let is_blocked = matches!(
        error,
        library_store_sqlite::LibrarySqliteError::Canonical(canonical)
            if matches!(canonical.code, CanonicalErrorCode::NotFound)
    );

    if is_blocked {
        ScanEventInput {
            kind: library_boundary_protocol::SourceScanEventKind::SourceScanBlocked,
            root_id,
            scan_run_id,
            phase: library_boundary_protocol::ScanRunPhase::Blocked,
            directories_visited: 0,
            files_visited: 0,
            files_discovered: 0,
            media_candidates: 0,
            queued_work_items: 0,
            detail: Some(detail),
        }
    } else {
        ScanEventInput {
            kind: library_boundary_protocol::SourceScanEventKind::SourceScanFailed,
            root_id,
            scan_run_id,
            phase: library_boundary_protocol::ScanRunPhase::Scanning,
            directories_visited: 0,
            files_visited: 0,
            files_discovered: 0,
            media_candidates: 0,
            queued_work_items: 0,
            detail: Some(detail),
        }
    }
}

fn job_scan_cancelled_event(
    root_id: i64,
    scan_run_id: i64,
    committed_chunks: usize,
) -> ScanEventInput {
    let approx_files = committed_chunks * 256;
    ScanEventInput {
        kind: library_boundary_protocol::SourceScanEventKind::SourceScanCancelled,
        root_id,
        scan_run_id,
        phase: library_boundary_protocol::ScanRunPhase::Interrupted,
        directories_visited: 0,
        files_visited: approx_files,
        files_discovered: approx_files,
        media_candidates: 0,
        queued_work_items: 0,
        detail: Some("cancelled by user".to_string()),
    }
}

fn require_playlist_id(value: i64, field_name: &str) -> protocol::ProtocolResult<PlaylistId> {
    PlaylistId::new(value).ok_or_else(|| protocol::ProtocolError::InvalidRequest {
        detail: format!("{field_name} must be positive"),
    })
}

fn require_library_asset_id(
    value: i64,
    field_name: &str,
) -> protocol::ProtocolResult<LibraryAssetId> {
    LibraryAssetId::new(value).ok_or_else(|| protocol::ProtocolError::InvalidRequest {
        detail: format!("{field_name} must be positive"),
    })
}

fn require_positive_i64(value: i64, field_name: &str) -> protocol::ProtocolResult<i64> {
    if value > 0 {
        Ok(value)
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("{field_name} must be positive"),
        })
    }
}

fn validate_contents_scope(scope: &protocol::ContentsScope) -> protocol::ProtocolResult<()> {
    match scope {
        protocol::ContentsScope::Source { source_id } => {
            require_positive_i64(*source_id, "contents sourceId")?;
        }
        protocol::ContentsScope::SourceLocation { source_location_id } => {
            require_positive_i64(*source_location_id, "contents sourceLocationId")?;
        }
        protocol::ContentsScope::Directory {
            source_id,
            source_directory_id,
        } => {
            require_positive_i64(*source_id, "contents sourceId")?;
            require_positive_i64(*source_directory_id, "contents sourceDirectoryId")?;
        }
    }

    Ok(())
}

fn validate_contents_policy(policy: &protocol::ContentsReadPolicy) -> protocol::ProtocolResult<()> {
    if policy.media_classes.is_empty() {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "contents mediaClasses must not be empty".to_string(),
        });
    }

    Ok(())
}

fn validate_contents_limit(limit: usize) -> protocol::ProtocolResult<()> {
    if (1..=200).contains(&limit) {
        Ok(())
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: "contents limit must be between 1 and 200".to_string(),
        })
    }
}

fn unix_time_ms() -> protocol::ProtocolResult<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| protocol::ProtocolError::HostFailure {
            detail: format!("system clock is before UNIX epoch: {error}"),
        })?;
    i64::try_from(duration.as_millis()).map_err(|error| protocol::ProtocolError::HostFailure {
        detail: format!("current UNIX timestamp does not fit i64 milliseconds: {error}"),
    })
}

fn map_store_error(error: library_store_sqlite::LibrarySqliteError) -> protocol::ProtocolError {
    protocol::ProtocolError::DurableStoreFailure {
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use library_boundary_protocol::{
        CancelRootScanReply, CancelRootScanRequest, CancelRootScanStatus, CommandOutcome,
        CommandReply, CommandRequest, ContentsReadPolicy, ContentsRowProfile, CreatePlaylistReply,
        CreatePlaylistRequest, DeletePlaylistReply, DeletePlaylistRequest,
        DirectoryImageMediaState, DirectoryPrimaryMediaState, DirectoryScanState,
        HashSourceFilesBlake3OutcomeStatus, HashSourceFilesBlake3Reply,
        HashSourceFilesBlake3Request, HashSourceFilesBlake3SourceFailure, LibraryBoundaryEvent,
        LibraryBoundaryEventStreamCommand, LibraryBoundaryEventStreamReply, LibraryRootCommand,
        LibraryRootReply, LibraryTreeEntryPoint, LibraryTreeNodeKind, LibraryTreePresenceState,
        LoadNavigationRowByStableKeyReply, LoadNavigationRowByStableKeyRequest,
        MaintainedSnapshotScope, PlaylistWriteCommand, PlaylistWriteReply, ProtocolError,
        ReadLibraryBoundaryEventsAfterReply, ReadLibraryBoundaryEventsAfterRequest,
        ReadLibraryTreeChildrenRequest, ReadSourceLifecycleReply, ReadSourceLifecycleRequest,
        RegisterLocalRootReply, RegisterLocalRootRequest, RenamePlaylistReply,
        RenamePlaylistRequest, SnapshotReadCommand, SnapshotReadReply, SourceFileHashCommand,
        SourceFileHashReply, StartRootScanReply, StartRootScanRequest, UnregisterLocalRootReply,
        UnregisterLocalRootRequest,
    };
    use serde_json::json;
    use tempfile::TempDir;

    use library_domain::{SourceAccessIssueKind, SourceAccessState, SourceScanPhase};
    use library_store_sqlite::{
        LibraryStoreContext, StoreEnvironment, UpsertSourceInput, UpsertSourceScanStateInput,
        UpsertSourceStateInput, durable_store_path,
    };

    use super::{LibraryBoundaryService, validate_contents_policy};

    fn open_service_with_context() -> (TempDir, LibraryStoreContext, LibraryBoundaryService) {
        let tempdir = TempDir::new().expect("create tempdir");
        let context = LibraryStoreContext {
            user_data_path: tempdir.path().to_string_lossy().into_owned(),
            environment: StoreEnvironment::Development,
        };
        let service = LibraryBoundaryService::open(context.clone()).expect("open service");

        (tempdir, context, service)
    }

    #[test]
    fn open_creates_the_explicit_storage_parent_before_sqlite_bootstrap() {
        let tempdir = TempDir::new().expect("create tempdir");
        let user_data_path = tempdir.path().join("missing").join("user-data");
        let context = LibraryStoreContext {
            user_data_path: user_data_path.to_string_lossy().into_owned(),
            environment: StoreEnvironment::Development,
        };
        let database_path = durable_store_path(&context.user_data_path, context.environment);

        assert!(!database_path.exists());
        let _service = LibraryBoundaryService::open(context).expect("open boundary service");

        assert!(database_path.exists());
        assert!(database_path.parent().expect("database parent").exists());
    }

    #[test]
    fn open_rejects_relative_user_data_paths() {
        let result = LibraryBoundaryService::open(LibraryStoreContext {
            user_data_path: "relative-user-data".to_string(),
            environment: StoreEnvironment::Development,
        });
        let error = match result {
            Ok(_) => panic!("relative user data path should be rejected"),
            Err(error) => error,
        };

        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
    }

    #[test]
    fn contents_policy_validation_rejects_empty_media_classes() {
        let error = validate_contents_policy(&ContentsReadPolicy {
            media_classes: Vec::new(),
            row_profile: ContentsRowProfile::SourceFile,
        })
        .expect_err("empty contents media classes should be rejected");

        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
    }

    fn expect_success(outcome: CommandOutcome) -> CommandReply {
        match outcome {
            CommandOutcome::Success(envelope) => envelope.reply,
            CommandOutcome::Error(envelope) => panic!("expected success, got {:?}", envelope.error),
        }
    }

    fn expect_register_local_root_reply(reply: CommandReply) -> RegisterLocalRootReply {
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(reply)) => reply,
            other => panic!("expected register local root reply, got {other:?}"),
        }
    }

    fn expect_read_local_roots_reply(
        reply: CommandReply,
    ) -> library_boundary_protocol::ReadLocalRootsReply {
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::ReadLocalRoots(reply)) => reply,
            other => panic!("expected read local roots reply, got {other:?}"),
        }
    }

    fn expect_start_root_scan_reply(reply: CommandReply) -> StartRootScanReply {
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::StartRootScan(reply)) => reply,
            other => panic!("expected start root scan reply, got {other:?}"),
        }
    }

    fn expect_unregister_local_root_reply(reply: CommandReply) -> UnregisterLocalRootReply {
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::UnregisterLocalRoot(reply)) => reply,
            other => panic!("expected unregister local root reply, got {other:?}"),
        }
    }

    fn expect_library_tree_reply(
        reply: CommandReply,
    ) -> library_boundary_protocol::ReadLibraryTreeChildrenReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::LibraryTreeChildren(reply)) => reply,
            other => panic!("expected library tree reply, got {other:?}"),
        }
    }

    fn expect_source_lifecycle_reply(reply: CommandReply) -> ReadSourceLifecycleReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::SourceLifecycle(reply)) => reply,
            other => panic!("expected source lifecycle reply, got {other:?}"),
        }
    }

    fn expect_hash_source_files_blake3_reply(reply: CommandReply) -> HashSourceFilesBlake3Reply {
        match reply {
            CommandReply::SourceFileHash(SourceFileHashReply::HashSourceFilesBlake3(reply)) => {
                reply
            }
            other => panic!("expected hash source files BLAKE3 reply, got {other:?}"),
        }
    }

    fn expect_event_stream_read_after_reply(
        reply: CommandReply,
    ) -> ReadLibraryBoundaryEventsAfterReply {
        match reply {
            CommandReply::LibraryBoundaryEvents(LibraryBoundaryEventStreamReply::ReadAfter(
                reply,
            )) => reply,
            other => panic!("expected event stream readAfter reply, got {other:?}"),
        }
    }

    fn expect_create_playlist_reply(reply: CommandReply) -> CreatePlaylistReply {
        match reply {
            CommandReply::PlaylistWrite(PlaylistWriteReply::CreatePlaylist(reply)) => reply,
            other => panic!("expected create playlist reply, got {other:?}"),
        }
    }

    fn expect_rename_playlist_reply(reply: CommandReply) -> RenamePlaylistReply {
        match reply {
            CommandReply::PlaylistWrite(PlaylistWriteReply::RenamePlaylist(reply)) => reply,
            other => panic!("expected rename playlist reply, got {other:?}"),
        }
    }

    fn expect_delete_playlist_reply(reply: CommandReply) -> DeletePlaylistReply {
        match reply {
            CommandReply::PlaylistWrite(PlaylistWriteReply::DeletePlaylist(reply)) => reply,
            other => panic!("expected delete playlist reply, got {other:?}"),
        }
    }

    fn expect_navigation_row_by_stable_key_reply(
        reply: CommandReply,
    ) -> LoadNavigationRowByStableKeyReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::NavigationRowByStableKey(reply)) => reply,
            other => panic!("expected load navigation row by stable key reply, got {other:?}"),
        }
    }

    fn register_local_root(
        service: &LibraryBoundaryService,
        absolute_path: String,
    ) -> (serde_json::Value, RegisterLocalRootReply) {
        let outcome = service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::RegisterLocalRoot(RegisterLocalRootRequest { absolute_path }),
        ));
        let json = serde_json::to_value(&outcome).expect("serialize register outcome");
        let reply = expect_register_local_root_reply(expect_success(outcome));
        (json, reply)
    }

    fn create_playlist(
        service: &LibraryBoundaryService,
        display_name: &str,
    ) -> (serde_json::Value, CreatePlaylistReply) {
        let outcome = service.handle_command(CommandRequest::PlaylistWrite(
            PlaylistWriteCommand::CreatePlaylist(CreatePlaylistRequest {
                display_name: display_name.to_string(),
            }),
        ));
        let json = serde_json::to_value(&outcome).expect("serialize create playlist outcome");
        let reply = expect_create_playlist_reply(expect_success(outcome));
        (json, reply)
    }

    fn rename_playlist(
        service: &LibraryBoundaryService,
        playlist_id: i64,
        display_name: &str,
    ) -> RenamePlaylistReply {
        expect_rename_playlist_reply(expect_success(service.handle_command(
            CommandRequest::PlaylistWrite(PlaylistWriteCommand::RenamePlaylist(
                RenamePlaylistRequest {
                    playlist_id,
                    display_name: display_name.to_string(),
                },
            )),
        )))
    }

    fn delete_playlist(service: &LibraryBoundaryService, playlist_id: i64) -> DeletePlaylistReply {
        expect_delete_playlist_reply(expect_success(service.handle_command(
            CommandRequest::PlaylistWrite(PlaylistWriteCommand::DeletePlaylist(
                DeletePlaylistRequest { playlist_id },
            )),
        )))
    }

    fn start_root_scan(service: &LibraryBoundaryService, root_id: i64) -> StartRootScanReply {
        expect_start_root_scan_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::StartRootScan(StartRootScanRequest {
                root_id,
            })),
        )))
    }

    fn cancel_root_scan(service: &LibraryBoundaryService, scan_run_id: i64) -> CancelRootScanReply {
        let outcome = service.handle_command(CommandRequest::LibraryRoots(
            LibraryRootCommand::CancelRootScan(CancelRootScanRequest { scan_run_id }),
        ));
        match expect_success(outcome) {
            CommandReply::LibraryRoots(LibraryRootReply::CancelRootScan(reply)) => reply,
            other => panic!("expected cancel root scan reply, got {other:?}"),
        }
    }

    fn read_after_events(
        service: &LibraryBoundaryService,
        last_seen_event_sequence: Option<i64>,
        max_events: usize,
    ) -> ReadLibraryBoundaryEventsAfterReply {
        expect_event_stream_read_after_reply(expect_success(service.handle_command(
            CommandRequest::LibraryBoundaryEvents(LibraryBoundaryEventStreamCommand::ReadAfter(
                ReadLibraryBoundaryEventsAfterRequest {
                    last_seen_event_sequence,
                    max_events,
                },
            )),
        )))
    }

    fn hash_source_files_blake3(
        service: &LibraryBoundaryService,
        source_id: i64,
        limit: Option<usize>,
    ) -> HashSourceFilesBlake3Reply {
        expect_hash_source_files_blake3_reply(expect_success(service.handle_command(
            CommandRequest::SourceFileHash(SourceFileHashCommand::HashSourceFilesBlake3(
                HashSourceFilesBlake3Request { source_id, limit },
            )),
        )))
    }

    fn wait_for_scan_completed(service: &LibraryBoundaryService, root_id: i64) {
        for _ in 0..30 {
            let events_reply = read_after_events(service, None, 64);
            if events_reply.events.iter().any(|e| {
                let LibraryBoundaryEvent::SourceScanEvent(se) = e else {
                    return false;
                };
                se.kind == library_boundary_protocol::SourceScanEventKind::SourceScanCompleted
                    && se.root_id == root_id
            }) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        panic!("scan should complete on a tiny directory");
    }

    fn read_library_tree_children(
        service: &LibraryBoundaryService,
        entry_point: LibraryTreeEntryPoint,
        parent_source_directory_id: Option<i64>,
    ) -> library_boundary_protocol::ReadLibraryTreeChildrenReply {
        expect_library_tree_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadLibraryTreeChildren(
                ReadLibraryTreeChildrenRequest {
                    entry_point,
                    parent_source_directory_id,
                    offset: 0,
                    limit: 10,
                },
            )),
        )))
    }

    fn read_source_lifecycle(
        service: &LibraryBoundaryService,
        source_id: i64,
    ) -> ReadSourceLifecycleReply {
        expect_source_lifecycle_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadSourceLifecycle(
                ReadSourceLifecycleRequest { source_id },
            )),
        )))
    }

    fn load_navigation_row_by_stable_key(
        service: &LibraryBoundaryService,
        stable_key: String,
    ) -> LoadNavigationRowByStableKeyReply {
        expect_navigation_row_by_stable_key_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::LoadNavigationRowByStableKey(
                LoadNavigationRowByStableKeyRequest { stable_key },
            )),
        )))
    }

    #[test]
    fn source_lifecycle_read_returns_authoritative_source_level_facts() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("source-lifecycle-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());

        service
            .durable_store
            .upsert_source_state(UpsertSourceStateInput {
                source_id: registered.root_id,
                mount_status: "unmounted".to_string(),
                mount_epoch: 2,
                access_state: SourceAccessState::Blocked,
                access_issue_kind: Some(SourceAccessIssueKind::UnavailableMount),
                access_error_detail: None,
                access_checked_at: Some(30),
                mount_root: None,
                effective_path: Some(source_root.to_string_lossy().into_owned()),
                observed_volume_label: None,
                filesystem_type: None,
                last_seen_at: Some(25),
                updated_at: 31,
            })
            .expect("update source state");
        service
            .durable_store
            .upsert_source_scan_state(UpsertSourceScanStateInput {
                source_id: registered.root_id,
                scan_phase: SourceScanPhase::Blocked,
                last_scan_started_at: Some(10),
                last_scan_finished_at: Some(20),
                last_successful_scan_at: None,
                scan_issue_kind: Some(SourceAccessIssueKind::PermissionDenied),
                error_detail: None,
                updated_at: 32,
            })
            .expect("update scan state");

        let reply = read_source_lifecycle(&service, registered.root_id);
        let lifecycle = reply.lifecycle.expect("known source lifecycle");
        assert_eq!(lifecycle.source_id, registered.root_id);
        assert!(lifecycle.is_user_visible);
        assert_eq!(
            lifecycle.mount_status,
            library_boundary_protocol::SourceMountStatus::Unmounted
        );
        assert_eq!(
            lifecycle.access_state,
            library_boundary_protocol::SourceAccessState::Blocked
        );
        assert_eq!(
            lifecycle.access_issue_kind,
            Some(library_boundary_protocol::SourceLifecycleIssueKind::UnavailableMount)
        );
        assert_eq!(
            lifecycle.scan_phase,
            library_boundary_protocol::SourceScanPhase::Blocked
        );
        assert_eq!(
            lifecycle.scan_issue_kind,
            Some(library_boundary_protocol::SourceLifecycleIssueKind::PermissionDenied)
        );
        assert_eq!(lifecycle.last_scan_started_at_ms, Some(10));
        assert_eq!(lifecycle.last_scan_finished_at_ms, Some(20));
        assert_eq!(lifecycle.last_successful_scan_at_ms, None);
        assert_eq!(lifecycle.last_seen_at_ms, Some(25));
        assert!(lifecycle.updated_at_ms >= 32);

        let missing = read_source_lifecycle(&service, 99_999);
        assert!(missing.lifecycle.is_none());

        let invalid = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadSourceLifecycle(ReadSourceLifecycleRequest {
                    source_id: 0,
                }),
            ))
            .expect_err("zero sourceId is invalid");
        assert!(matches!(invalid, ProtocolError::InvalidRequest { .. }));
    }

    #[test]
    fn source_lifecycle_read_defaults_missing_side_rows_for_known_source() {
        let (_tempdir, _context, service) = open_service_with_context();
        let source_id = service
            .durable_store
            .upsert_source(UpsertSourceInput {
                source_id: Some(77),
                source_class: "external_mounted".to_string(),
                authority: "device".to_string(),
                identity_kind: "fixture".to_string(),
                identity_value: "missing-side-rows".to_string(),
                display_name: "Fixture".to_string(),
                medium_label: None,
                is_user_visible: true,
                browser_order_ordinal: None,
                changed_at: 100,
            })
            .expect("insert source without lifecycle side rows");

        let reply = read_source_lifecycle(&service, source_id);
        let lifecycle = reply.lifecycle.expect("known source lifecycle");

        assert_eq!(lifecycle.source_id, source_id);
        assert_eq!(
            lifecycle.mount_status,
            library_boundary_protocol::SourceMountStatus::Unknown
        );
        assert_eq!(
            lifecycle.access_state,
            library_boundary_protocol::SourceAccessState::Unknown
        );
        assert_eq!(
            lifecycle.scan_phase,
            library_boundary_protocol::SourceScanPhase::Idle
        );
        assert_eq!(lifecycle.updated_at_ms, 100);
    }

    #[test]
    fn protocol_commands_register_scan_read_and_reopen_real_store() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("source-root");
        let crate_dir = source_root.join("Crate");
        std::fs::create_dir_all(&crate_dir).expect("create source root");
        std::fs::write(crate_dir.join("amen.wav"), b"not-real-audio").expect("write source file");

        let (register_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        assert!(registered.root_id > 0);
        assert_eq!(
            register_json.pointer("/payload/reply/payload/payload/rootId"),
            Some(&json!(registered.root_id.to_string()))
        );
        assert_eq!(
            register_json.pointer("/payload/reply/payload/payload/canonicalPath"),
            Some(&json!(registered.canonical_path.clone()))
        );

        let scan = start_root_scan(&service, registered.root_id);
        assert!(scan.scan_run_id > 0);

        let mut scan_completed = false;
        for _ in 0..20 {
            let events_reply = read_after_events(&service, None, 32);
            scan_completed = events_reply.events.iter().any(|e| {
                let LibraryBoundaryEvent::SourceScanEvent(se) = e else {
                    return false;
                };
                se.kind == library_boundary_protocol::SourceScanEventKind::SourceScanCompleted
                    && se.root_id == registered.root_id
            });
            if scan_completed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(scan_completed, "scan should complete on a tiny directory");

        let root_reply = read_library_tree_children(
            &service,
            LibraryTreeEntryPoint::Source {
                source_id: registered.root_id,
            },
            None,
        );
        let root_window = root_reply
            .window
            .expect("registered source resolves to library tree window");
        assert_eq!(root_window.total_rows, 1);
        let crate_row = root_window
            .rows
            .iter()
            .find(|row| row.display_name == "Crate")
            .expect("top-level folder is browsable");
        assert_eq!(crate_row.node_kind, LibraryTreeNodeKind::Directory);
        assert_eq!(crate_row.presence_state, LibraryTreePresenceState::Present);
        assert_eq!(crate_row.has_child_directories, Some(false));
        assert_eq!(
            crate_row.directory_primary_media_state,
            Some(DirectoryPrimaryMediaState::HasPrimaryMediaDescendants)
        );
        assert_eq!(
            crate_row.directory_image_media_state,
            Some(DirectoryImageMediaState::NoImageMediaDescendants)
        );
        assert_eq!(
            crate_row.directory_scan_state,
            Some(DirectoryScanState::Complete)
        );
        let crate_directory_id = crate_row
            .source_directory_id
            .expect("directory rows carry durable ids");

        let crate_reply = read_library_tree_children(
            &service,
            LibraryTreeEntryPoint::Source {
                source_id: registered.root_id,
            },
            Some(crate_directory_id),
        );
        let crate_window = crate_reply
            .window
            .expect("nested directory resolves to library tree window");
        assert_eq!(crate_window.total_rows, 1);
        let file_row = crate_window.rows.first().expect("scanned file row exists");
        assert_eq!(file_row.node_kind, LibraryTreeNodeKind::File);
        assert_eq!(file_row.display_name, "amen.wav");
        assert_eq!(file_row.relative_path, "Crate/amen.wav");
        assert!(file_row.source_file_id.is_some());
        assert_eq!(file_row.has_child_directories, None);
        assert_eq!(file_row.directory_primary_media_state, None);
        assert_eq!(file_row.directory_image_media_state, None);
        assert_eq!(file_row.directory_scan_state, None);

        drop(service);
        let reopened = LibraryBoundaryService::open(context).expect("reopen boundary service");
        let reopened_crate_reply = read_library_tree_children(
            &reopened,
            LibraryTreeEntryPoint::Source {
                source_id: registered.root_id,
            },
            Some(crate_directory_id),
        );
        let reopened_crate_window = reopened_crate_reply
            .window
            .expect("persisted hierarchy survives service reopen");
        assert_eq!(reopened_crate_window.rows, crate_window.rows);
    }

    #[test]
    fn hash_source_files_blake3_updates_observed_facts_and_publishes_invalidation() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("hash-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("B.flac"), b"b bytes").expect("write b");
        std::fs::write(source_root.join("a.flac"), b"a bytes").expect("write a");
        std::fs::write(source_root.join("notes.txt"), b"not media").expect("write notes");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);
        let cursor_after_scan = read_after_events(&service, None, 64).latest_event_sequence;

        let first = hash_source_files_blake3(&service, registered.root_id, Some(1));
        assert_eq!(first.effective_limit, 1);
        assert_eq!(first.hashed_count, 1);
        assert_eq!(first.skipped_count, 0);
        assert_eq!(first.failed_count, 0);
        assert_eq!(first.remaining_candidates, 1);
        assert_eq!(first.outcomes.len(), 1);
        assert_eq!(first.outcomes[0].relative_path, "a.flac");
        let HashSourceFilesBlake3OutcomeStatus::Hashed(hashed) = &first.outcomes[0].status else {
            panic!("expected hashed outcome");
        };
        assert_eq!(hashed.content_hash_algorithm, "blake3");
        assert_eq!(hashed.content_hash_value.len(), 64);

        let facts = service
            .durable_store
            .read_observed_file_facts_for_source_file(first.outcomes[0].source_file_id)
            .expect("read observed facts")
            .expect("hash command writes observed facts");
        assert_eq!(
            facts.content_hash.expect("content hash").algorithm,
            "blake3"
        );

        let events_after_hash = read_after_events(&service, cursor_after_scan, 16);
        assert!(
            events_after_hash.events.iter().any(|e| {
                let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(event) = e else {
                    return false;
                };
                event.invalidation.scope == MaintainedSnapshotScope::LibraryBrowser
            }),
            "hash evidence changes must publish the narrow current maintained scope"
        );

        let second = hash_source_files_blake3(&service, registered.root_id, Some(10));
        assert_eq!(second.hashed_count, 1);
        assert_eq!(second.remaining_candidates, 0);
        assert_eq!(second.outcomes[0].relative_path, "B.flac");

        let third = hash_source_files_blake3(&service, registered.root_id, Some(10));
        assert_eq!(third.hashed_count, 0);
        assert_eq!(third.outcomes.len(), 0);
        assert_eq!(third.remaining_candidates, 0);
    }

    #[test]
    fn hash_source_files_blake3_reports_source_failures_without_empty_success() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("blocked-hash-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        service
            .durable_store
            .upsert_source_state(UpsertSourceStateInput {
                source_id: registered.root_id,
                mount_status: "unmounted".to_string(),
                mount_epoch: 2,
                access_state: SourceAccessState::Blocked,
                access_issue_kind: Some(SourceAccessIssueKind::UnavailableMount),
                access_error_detail: None,
                access_checked_at: Some(30),
                mount_root: None,
                effective_path: Some(source_root.to_string_lossy().into_owned()),
                observed_volume_label: None,
                filesystem_type: None,
                last_seen_at: Some(25),
                updated_at: 31,
            })
            .expect("block source state");

        let blocked = hash_source_files_blake3(&service, registered.root_id, Some(4));
        assert_eq!(blocked.effective_limit, 4);
        assert_eq!(blocked.hashed_count, 0);
        assert_eq!(blocked.outcomes.len(), 0);
        assert!(matches!(
            blocked.source_failure,
            Some(HashSourceFilesBlake3SourceFailure::SourceRootBlocked(_))
        ));

        let missing = hash_source_files_blake3(&service, 99_999, Some(4));
        assert!(matches!(
            missing.source_failure,
            Some(HashSourceFilesBlake3SourceFailure::SourceNotFound)
        ));

        let invalid = service
            .try_handle_command(CommandRequest::SourceFileHash(
                SourceFileHashCommand::HashSourceFilesBlake3(HashSourceFilesBlake3Request {
                    source_id: 0,
                    limit: Some(4),
                }),
            ))
            .expect_err("zero sourceId is invalid");
        assert!(matches!(invalid, ProtocolError::InvalidRequest { .. }));

        let invalid_limit = service
            .try_handle_command(CommandRequest::SourceFileHash(
                SourceFileHashCommand::HashSourceFilesBlake3(HashSourceFilesBlake3Request {
                    source_id: registered.root_id,
                    limit: Some(0),
                }),
            ))
            .expect_err("zero limit is invalid");
        assert!(matches!(
            invalid_limit,
            ProtocolError::InvalidRequest { .. }
        ));
    }

    #[test]
    fn protocol_commands_read_snapshot_invalidation_events_via_cursor() {
        let (_tempdir, _context, service) = open_service_with_context();

        let first_read = read_after_events(&service, None, 16);
        assert!(first_read.events.is_empty());
        assert!(
            first_read.latest_event_sequence.is_none(),
            "no events published yet, cursor must be None"
        );
        assert!(!first_read.gap_detected);

        let (_create_json, created) = create_playlist(&service, "Event Test");
        let deleted = delete_playlist(&service, created.playlist_id);
        assert!(deleted.deleted);

        let second_read = read_after_events(&service, first_read.latest_event_sequence, 16);
        assert!(
            !second_read.events.is_empty(),
            "create and delete must produce invalidation events"
        );
        assert!(
            second_read.events.iter().any(|e| {
                let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(payload) = e else {
                    return false;
                };
                payload.invalidation.scope == MaintainedSnapshotScope::NavigationRows
            }),
            "events must include NavigationRows invalidation"
        );
        assert!(
            second_read.events.iter().any(|e| {
                let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(payload) = e else {
                    return false;
                };
                payload.invalidation.scope == MaintainedSnapshotScope::LibraryBrowser
            }),
            "events must include LibraryBrowser invalidation"
        );
        assert!(second_read.latest_event_sequence.is_some());
        assert!(!second_read.gap_detected);

        let third_read = read_after_events(&service, second_read.latest_event_sequence, 16);
        assert!(
            third_read.events.is_empty(),
            "cursor advance must not replay consumed events"
        );
        assert!(!third_read.gap_detected);

        let error = service
            .try_handle_command(CommandRequest::LibraryBoundaryEvents(
                LibraryBoundaryEventStreamCommand::ReadAfter(
                    ReadLibraryBoundaryEventsAfterRequest {
                        last_seen_event_sequence: None,
                        max_events: 0,
                    },
                ),
            ))
            .expect_err("zero maxEvents is invalid");
        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
    }

    #[test]
    fn cursor_batch_truncation_does_not_skip_events() {
        let (_tempdir, _context, service) = open_service_with_context();

        let _created = create_playlist(&service, "Trunc A");
        let _created = create_playlist(&service, "Trunc B");
        let _created = create_playlist(&service, "Trunc C");

        let first = read_after_events(&service, None, 1);
        assert_eq!(first.events.len(), 1);
        assert!(first.latest_event_sequence.is_some());
        let cursor0 = first.latest_event_sequence;

        let second = read_after_events(&service, cursor0, 1);
        assert_eq!(second.events.len(), 1);
        assert!(second.latest_event_sequence.is_some());
        let cursor1 = second.latest_event_sequence;

        let third = read_after_events(&service, cursor1, 1);
        assert_eq!(third.events.len(), 1);
        assert!(third.latest_event_sequence.is_some());

        assert!(!first.gap_detected);
        assert!(!second.gap_detected);
        assert!(!third.gap_detected);
    }

    #[test]
    fn first_empty_read_does_not_skip_future_events() {
        let (_tempdir, _context, service) = open_service_with_context();

        let first = read_after_events(&service, None, 16);
        assert!(first.events.is_empty());
        assert!(first.latest_event_sequence.is_none());

        let _created = create_playlist(&service, "Late event");
        let second = read_after_events(&service, first.latest_event_sequence, 16);
        assert!(!second.events.is_empty());
        assert!(!second.gap_detected);
    }

    #[test]
    fn multiple_consumers_advance_independently() {
        let (_tempdir, _context, service) = open_service_with_context();

        let _created = create_playlist(&service, "Indy A");
        let _created = create_playlist(&service, "Indy B");

        let consumer1_first = read_after_events(&service, None, 16);
        assert!(!consumer1_first.events.is_empty());
        let c1_cursor = consumer1_first.latest_event_sequence;

        let consumer2_first = read_after_events(&service, None, 16);
        assert_eq!(consumer2_first.events.len(), consumer1_first.events.len());
        assert_eq!(consumer2_first.latest_event_sequence, c1_cursor);

        let _created = create_playlist(&service, "Indy C");
        let consumer1_second = read_after_events(&service, c1_cursor, 16);
        assert!(!consumer1_second.events.is_empty());

        let consumer2_second =
            read_after_events(&service, consumer2_first.latest_event_sequence, 16);
        assert!(!consumer2_second.events.is_empty());
    }

    #[test]
    fn cursor_older_than_earliest_retained_reports_gap() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("gap-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, _registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());

        for i in 0..300 {
            let _created = create_playlist(&service, &format!("Flood {}", i));
        }

        let reply = read_after_events(&service, Some(0), 16);
        assert!(reply.gap_detected);
    }

    #[test]
    fn protocol_playlist_write_commands_create_rename_and_delete_real_playlists() {
        let (_tempdir, _context, service) = open_service_with_context();

        let (create_json, created) = create_playlist(&service, "Warmups");
        assert!(created.playlist_id > 0);
        assert_eq!(
            create_json.pointer("/payload/reply/payload/payload/playlistId"),
            Some(&json!(created.playlist_id.to_string()))
        );

        let stable_key = format!("playlist:{}", created.playlist_id);
        let created_row = load_navigation_row_by_stable_key(&service, stable_key.clone())
            .row
            .expect("created playlist has navigation row");
        assert_eq!(created_row.display_name, "Warmups");

        let renamed = rename_playlist(&service, created.playlist_id, "Peak Hour");
        assert!(renamed.renamed);
        let renamed_row = load_navigation_row_by_stable_key(&service, stable_key.clone())
            .row
            .expect("renamed playlist still has navigation row");
        assert_eq!(renamed_row.display_name, "Peak Hour");

        let deleted = delete_playlist(&service, created.playlist_id);
        assert!(deleted.deleted);
        assert!(
            load_navigation_row_by_stable_key(&service, stable_key)
                .row
                .is_none()
        );

        let error = service
            .try_handle_command(CommandRequest::PlaylistWrite(
                PlaylistWriteCommand::RenamePlaylist(RenamePlaylistRequest {
                    playlist_id: 0,
                    display_name: "Invalid".to_string(),
                }),
            ))
            .expect_err("zero playlist id is invalid");
        match error {
            ProtocolError::InvalidRequest { detail } => {
                assert!(detail.contains("playlistId"));
            }
            other => panic!("expected InvalidRequest, got {other:?}"),
        }
    }

    #[test]
    fn real_store_read_failure_maps_to_durable_store_failure() {
        let (tempdir, context, service) = open_service_with_context();
        let database_path = durable_store_path(&context.user_data_path, context.environment);
        drop(tempdir);
        if database_path.exists() {
            std::fs::remove_file(&database_path).expect("remove durable store file");
        }

        let error = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadNavigationRows(
                    library_boundary_protocol::ReadNavigationRowsRequest {
                        parent_navigation_row_id: None,
                    },
                ),
            ))
            .expect_err("missing schema should fail as a durable store error");
        assert!(matches!(error, ProtocolError::DurableStoreFailure { .. }));
        assert_eq!(error.code(), "DURABLE_STORE_FAILURE");
    }

    #[test]
    fn invalid_command_input_maps_to_protocol_error_outcome() {
        let (_tempdir, _context, service) = open_service_with_context();
        let command =
            CommandRequest::LibraryRoots(LibraryRootCommand::StartRootScan(StartRootScanRequest {
                root_id: 0,
            }));

        let error = service
            .try_handle_command(command.clone())
            .expect_err("zero root id is invalid");
        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");

        match service.handle_command(command) {
            CommandOutcome::Error(envelope) => match envelope.error {
                ProtocolError::InvalidRequest { detail } => {
                    assert!(detail.contains("rootId"));
                }
                other => panic!("expected invalid request, got {other:?}"),
            },
            CommandOutcome::Success(envelope) => {
                panic!("expected error outcome, got {:?}", envelope.reply);
            }
        }
    }

    #[test]
    fn read_local_roots_returns_empty_before_registration() {
        let (_tempdir, _context, service) = open_service_with_context();
        let reply = expect_read_local_roots_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::ReadLocalRoots(
                library_boundary_protocol::ReadLocalRootsRequest,
            )),
        )));
        assert!(reply.roots.is_empty());
    }

    #[test]
    fn read_local_roots_returns_registered_root_after_registration() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        assert!(registered.root_id > 0);

        let reply = expect_read_local_roots_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::ReadLocalRoots(
                library_boundary_protocol::ReadLocalRootsRequest,
            )),
        )));
        assert_eq!(reply.roots.len(), 1);
        assert_eq!(reply.roots[0].root_id, registered.root_id);
        assert_eq!(reply.roots[0].canonical_path, registered.canonical_path);
        assert!(
            matches!(
                reply.roots[0].availability,
                library_boundary_protocol::LocalRootAvailability::Available
            ),
            "registered root must be available"
        );
    }

    #[test]
    fn unregister_local_root_removes_from_read_local_roots() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        assert!(registered.root_id > 0);

        let roots_after_register =
            expect_read_local_roots_reply(expect_success(service.handle_command(
                CommandRequest::LibraryRoots(LibraryRootCommand::ReadLocalRoots(
                    library_boundary_protocol::ReadLocalRootsRequest,
                )),
            )));
        assert_eq!(roots_after_register.roots.len(), 1);

        let unregistered = expect_unregister_local_root_reply(expect_success(
            service.handle_command(CommandRequest::LibraryRoots(
                LibraryRootCommand::UnregisterLocalRoot(UnregisterLocalRootRequest {
                    root_id: registered.root_id,
                }),
            )),
        ));
        assert!(unregistered.unregistered);

        let roots_after_unregister =
            expect_read_local_roots_reply(expect_success(service.handle_command(
                CommandRequest::LibraryRoots(LibraryRootCommand::ReadLocalRoots(
                    library_boundary_protocol::ReadLocalRootsRequest,
                )),
            )));
        assert!(
            roots_after_unregister.roots.is_empty(),
            "unregistered root must not appear in readLocalRoots"
        );
    }

    #[test]
    fn unregister_nonexistent_root_returns_unregistered_false() {
        let (_tempdir, _context, service) = open_service_with_context();

        let unregistered =
            expect_unregister_local_root_reply(expect_success(service.handle_command(
                CommandRequest::LibraryRoots(LibraryRootCommand::UnregisterLocalRoot(
                    UnregisterLocalRootRequest { root_id: 99999 },
                )),
            )));
        assert!(!unregistered.unregistered);
    }

    #[test]
    fn unregister_already_unregistered_root_is_idempotent() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());

        let first = expect_unregister_local_root_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::UnregisterLocalRoot(
                UnregisterLocalRootRequest {
                    root_id: registered.root_id,
                },
            )),
        )));
        assert!(first.unregistered);

        let second = expect_unregister_local_root_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::UnregisterLocalRoot(
                UnregisterLocalRootRequest {
                    root_id: registered.root_id,
                },
            )),
        )));
        assert!(
            second.unregistered,
            "unregistering already-unregistered root must return unregistered true"
        );
    }

    #[test]
    fn re_register_same_path_after_unregister_restores_visibility() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, first_registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());

        let _unregistered = expect_unregister_local_root_reply(expect_success(
            service.handle_command(CommandRequest::LibraryRoots(
                LibraryRootCommand::UnregisterLocalRoot(UnregisterLocalRootRequest {
                    root_id: first_registered.root_id,
                }),
            )),
        ));

        let (_json, re_registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        assert_eq!(
            re_registered.root_id, first_registered.root_id,
            "re-registering same path must return same root_id"
        );

        let roots = expect_read_local_roots_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::ReadLocalRoots(
                library_boundary_protocol::ReadLocalRootsRequest,
            )),
        )));
        assert_eq!(roots.roots.len(), 1);
        assert_eq!(roots.roots[0].root_id, first_registered.root_id);
    }

    #[test]
    fn cancel_unknown_scan_run_id_returns_not_found() {
        let (_tempdir, _context, service) = open_service_with_context();

        let reply = cancel_root_scan(&service, 99999);
        assert_eq!(reply.status, CancelRootScanStatus::NotFound);
    }

    #[test]
    fn cancel_active_scan_returns_accepted_or_already_terminal() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.wav"), b"data").expect("write test file");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let scanned = start_root_scan(&service, registered.root_id);

        let reply = cancel_root_scan(&service, scanned.scan_run_id);
        assert!(
            matches!(
                reply.status,
                CancelRootScanStatus::Accepted | CancelRootScanStatus::AlreadyTerminal
            ),
            "active scan cancellation must return Accepted or AlreadyTerminal, not {:?}",
            reply.status,
        );
    }

    #[test]
    fn cancel_after_scan_completes_returns_already_terminal() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("music-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.wav"), b"data").expect("write test file");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let scanned = start_root_scan(&service, registered.root_id);

        for _ in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let reply = cancel_root_scan(&service, scanned.scan_run_id);
            if reply.status == CancelRootScanStatus::AlreadyTerminal {
                return;
            }
        }

        panic!(
            "expected cancel after scan completion to return AlreadyTerminal, got NotCancelable"
        );
    }

    #[test]
    fn cancel_invalid_scan_run_id_rejects() {
        let (_tempdir, _context, service) = open_service_with_context();

        let error = service
            .try_handle_command(CommandRequest::LibraryRoots(
                LibraryRootCommand::CancelRootScan(CancelRootScanRequest { scan_run_id: 0 }),
            ))
            .expect_err("zero scanRunId is invalid");
        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
    }
}
