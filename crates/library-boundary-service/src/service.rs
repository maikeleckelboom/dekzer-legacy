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
    RootNavigationWindowEstablishment, RootNavigationWindowEstablishmentState, RootScanObservation,
    SourceFileBlake3HashAdmissionScope, SqliteDurableStore, UnregisterLocalRootInput,
};

use crate::session_events::{LibraryBoundaryEventStream, ScanEventInput};
use crate::snapshot_read_protocol::{
    map_load_navigation_row_by_stable_key_reply, map_load_navigation_row_reply,
    map_maintained_read_model_revisions, map_read_attachment_source_files_reply,
    map_read_contents_reply, map_read_library_asset_preparation_detail_reply,
    map_read_library_asset_waveform_overview_reply, map_read_library_tree_children_reply,
    map_read_navigation_node_library_browser_window_reply, map_read_navigation_rows_reply,
    map_read_source_attachment_summary_reply, map_read_source_file_attachment_reply,
    map_read_source_lifecycle_reply, map_read_track_identity_review_candidates_reply,
    map_search_filter_read_reply, map_search_navigation_node_library_browser_window_reply,
    store_contents_policy, store_contents_scope, store_contents_scope_depth,
    store_library_tree_entry_point, store_search_filter_request,
    store_track_identity_review_state_filter,
};
use crate::source_file_hash_protocol::{
    empty_hash_source_files_blake3_reply, map_hash_lifecycle_source_failure,
    map_hash_source_files_blake3_reply,
};
use crate::source_maintenance::{
    SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT, SourceMaintenanceController,
    SourceMaintenanceRun, SourceMaintenanceRunInput,
};
use crate::storage_environment::resolve_library_storage_environment;

struct ActiveScanJob {
    #[allow(dead_code)]
    root_id: i64,
    scan_run_id: i64,
    terminal_publication_complete: Arc<AtomicBool>,
    handle: JoinHandle<()>,
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

    fn cleanup_terminal_scan(&mut self, root_id: i64) -> Option<JoinHandle<()>> {
        if let Some(job) = self.jobs.remove(&root_id) {
            self.scan_run_id_to_root_id.remove(&job.scan_run_id);
            self.terminal_scan_run_ids.insert(job.scan_run_id);
            return Some(job.handle);
        }
        None
    }

    fn drain_jobs(&mut self) -> Vec<ActiveScanJob> {
        self.scan_run_id_to_root_id.clear();
        self.jobs.drain().map(|(_, job)| job).collect()
    }
}

pub struct LibraryBoundaryService {
    durable_store: SqliteDurableStore,
    session_events: LibraryBoundaryEventStream,
    scan_registry: Mutex<ScanJobRegistry>,
    source_maintenance: SourceMaintenanceController,
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
            source_maintenance: SourceMaintenanceController::new(),
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
            protocol::CommandRequest::SourceMaintenance(command) => self
                .handle_source_maintenance_command(command)
                .map(Box::new)
                .map(protocol::CommandReply::SourceMaintenance),
            protocol::CommandRequest::TrackIdentityDecisions(command) => {
                crate::track_identity_decisions::handle_track_identity_decisions_command(
                    &self.durable_store,
                    command,
                )
                .map(protocol::CommandReply::TrackIdentityDecisions)
            }
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

        let terminal_handle = if let Some(existing) = registry.jobs.get(&root_id) {
            if !existing
                .terminal_publication_complete
                .load(Ordering::Acquire)
            {
                return Err(protocol::ProtocolError::InvalidRequest {
                    detail: "A scan is already running for this root".to_string(),
                });
            }

            registry.cleanup_terminal_scan(root_id)
        } else {
            None
        };
        if let Some(handle) = terminal_handle {
            let _ = handle.join();
        }

        self.durable_store
            .read_root_scan_path(root_id)
            .map_err(|e| protocol::ProtocolError::DurableStoreFailure {
                detail: e.to_string(),
            })?;

        let scan_run_id = registry.next_scan_run_id();

        let store = self.durable_store.clone();
        let events = self.session_events.clone();
        let source_maintenance = self.source_maintenance.clone();
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
                source_maintenance,
            );
        });

        registry.scan_run_id_to_root_id.insert(scan_run_id, root_id);
        registry.jobs.insert(
            root_id,
            ActiveScanJob {
                root_id,
                scan_run_id,
                terminal_publication_complete,
                handle,
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
            let handle = registry.cleanup_terminal_scan(root_id);
            drop(registry);
            if let Some(handle) = handle {
                let _ = handle.join();
            }
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
        let root_navigation_window_establishment =
            root_navigation_window_establishment_target(&request)
                .map(|source_id| {
                    self.durable_store
                        .establish_root_navigation_window(source_id)
                })
                .transpose()
                .map_err(map_store_error)?;
        let window = self
            .durable_store
            .read_literal_hierarchy_children(
                store_library_tree_entry_point(request.entry_point),
                request.parent_source_directory_id,
                request.offset,
                request.limit,
                library_store_sqlite::SourceFileClassFilter::NavigationOnly,
            )
            .map_err(map_store_error)?;
        let mut reply = map_read_library_tree_children_reply(window).map_err(map_store_error)?;
        if let Some(establishment) = root_navigation_window_establishment {
            apply_root_navigation_window_establishment(&mut reply, &establishment);
        }
        Ok(reply)
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

    pub fn read_source_file_attachment(
        &self,
        request: protocol::ReadSourceFileAttachmentRequest,
    ) -> protocol::ProtocolResult<protocol::ReadSourceFileAttachmentReply> {
        let source_file_id = require_positive_i64(request.source_file_id, "sourceFileId")?;
        let attachment_link = self
            .durable_store
            .read_attachment_for_source_file(source_file_id)
            .map_err(map_store_error)?;
        map_read_source_file_attachment_reply(attachment_link).map_err(map_store_error)
    }

    pub fn read_attachment_source_files(
        &self,
        request: protocol::ReadAttachmentSourceFilesRequest,
    ) -> protocol::ProtocolResult<protocol::ReadAttachmentSourceFilesReply> {
        let attachment_id = require_positive_i64(request.attachment_id, "attachmentId")?;
        let limit = request.limit.unwrap_or(100);
        validate_attachment_source_files_limit(limit)?;
        let source_files = self
            .durable_store
            .read_source_files_for_attachment(attachment_id, limit)
            .map_err(map_store_error)?;
        map_read_attachment_source_files_reply(source_files).map_err(map_store_error)
    }

    pub fn read_source_attachment_summary(
        &self,
        request: protocol::ReadSourceAttachmentSummaryRequest,
    ) -> protocol::ProtocolResult<protocol::ReadSourceAttachmentSummaryReply> {
        let source_id = require_positive_i64(request.source_id, "sourceId")?;
        let summary = self
            .durable_store
            .read_source_attachment_summary(source_id)
            .map_err(map_store_error)?;
        Ok(map_read_source_attachment_summary_reply(summary))
    }

    pub fn read_track_identity_review_candidates(
        &self,
        request: protocol::ReadTrackIdentityReviewCandidatesRequest,
    ) -> protocol::ProtocolResult<protocol::ReadTrackIdentityReviewCandidatesReply> {
        validate_track_identity_review_candidates_limit(request.limit)?;
        let source_id = request
            .source_id
            .map(|source_id| require_positive_i64(source_id, "sourceId"))
            .transpose()?;

        if let Some(source_id) = source_id {
            let lifecycle = self
                .durable_store
                .read_source_lifecycle(source_id)
                .map_err(map_store_error)?;
            if lifecycle.is_none() {
                return Ok(protocol::ReadTrackIdentityReviewCandidatesReply {
                    status: protocol::TrackIdentityReviewReadStatus::SourceNotFound,
                    candidates: Vec::new(),
                });
            }
        }

        let candidates = self
            .durable_store
            .read_track_identity_review_candidates(
                source_id,
                store_track_identity_review_state_filter(request.review_state),
                request.limit,
            )
            .map_err(map_store_error)?;
        map_read_track_identity_review_candidates_reply(candidates).map_err(map_store_error)
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
                store_contents_scope_depth(request.scope_depth),
                limit,
                request.cursor.as_deref(),
            )
            .map_err(map_store_error)?;
        map_read_contents_reply(result).map_err(map_store_error)
    }

    pub fn read_search_filter(
        &self,
        request: protocol::SearchFilterReadRequest,
    ) -> protocol::ProtocolResult<protocol::SearchFilterReadReply> {
        validate_search_filter_scope(&request.scope)?;
        let limit = request.limit.unwrap_or(100);
        validate_search_filter_limit(limit)?;
        if matches!(request.sort, protocol::SearchFilterSort::Relevance)
            && request
                .text_query
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "searchFilterRead relevance sort requires a textQuery".to_string(),
            });
        }
        let result = self
            .durable_store
            .read_search_filter(store_search_filter_request(request, limit))
            .map_err(map_store_error)?;
        map_search_filter_read_reply(result).map_err(map_store_error)
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
        let _attachment_materialization = self
            .durable_store
            .materialize_attachments_for_source(
                source_id,
                SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT,
            )
            .map_err(map_store_error)?;
        self.durable_store
            .rebuild_search_filter_index_for_source(source_id)
            .map_err(map_store_error)?;
        self.publish_maintained_snapshot_invalidations()?;
        Ok(map_hash_source_files_blake3_reply(result, None))
    }

    pub fn run_source_maintenance(
        &self,
        request: protocol::RunSourceMaintenanceRequest,
    ) -> protocol::ProtocolResult<protocol::RunSourceMaintenanceReply> {
        let source_id = require_positive_i64(request.source_id, "sourceId")?;
        require_optional_positive_limit(request.hash_limit, "runSourceMaintenance hashLimit")?;
        require_optional_positive_limit(
            request.attachment_limit,
            "runSourceMaintenance attachmentLimit",
        )?;
        require_optional_positive_limit(request.probe_limit, "runSourceMaintenance probeLimit")?;
        require_optional_positive_limit(
            request.promotion_limit,
            "runSourceMaintenance promotionLimit",
        )?;
        require_optional_positive_limit(
            request.identity_candidate_limit,
            "runSourceMaintenance identityCandidateLimit",
        )?;
        require_optional_positive_limit(
            request.identity_decision_limit,
            "runSourceMaintenance identityDecisionLimit",
        )?;

        let run = self.source_maintenance.run_manual(
            &self.durable_store,
            &self.session_events,
            SourceMaintenanceRunInput {
                source_id,
                hash_limit: request.hash_limit,
                attachment_limit: request.attachment_limit,
                probe_limit: request.probe_limit,
                promotion_limit: request.promotion_limit,
                identity_candidate_limit: request.identity_candidate_limit,
                identity_decision_limit: request.identity_decision_limit,
            },
        )?;
        Ok(map_run_source_maintenance_reply(run))
    }

    pub fn read_source_maintenance(
        &self,
        request: protocol::ReadSourceMaintenanceRequest,
    ) -> protocol::ProtocolResult<protocol::ReadSourceMaintenanceReply> {
        let source_id = require_positive_i64(request.source_id, "sourceId")?;
        let snapshot = self
            .source_maintenance
            .read_snapshot(&self.durable_store, source_id)?;
        Ok(protocol::ReadSourceMaintenanceReply {
            source_id: snapshot.source_id,
            status: snapshot.status,
            remaining_hash_candidates: snapshot.remaining_hash_candidates,
            remaining_probe_candidates: snapshot.remaining_probe_candidates,
            remaining_primary_media_promotion_candidates: snapshot
                .remaining_primary_media_promotion_candidates,
            remaining_track_identity_candidate_production_candidates: snapshot
                .remaining_track_identity_candidate_production_candidates,
            remaining_track_identity_decision_production_candidates: snapshot
                .remaining_track_identity_decision_production_candidates,
            attachment_links: snapshot.attachment_links,
            source_failure: snapshot.source_failure,
            last_run: snapshot.last_run,
        })
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

    fn handle_source_maintenance_command(
        &self,
        command: protocol::SourceMaintenanceCommand,
    ) -> protocol::ProtocolResult<protocol::SourceMaintenanceReply> {
        match command {
            protocol::SourceMaintenanceCommand::RunSourceMaintenance(request) => self
                .run_source_maintenance(request)
                .map(protocol::SourceMaintenanceReply::RunSourceMaintenance),
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
            protocol::SnapshotReadCommand::ReadSourceMaintenance(request) => self
                .read_source_maintenance(request)
                .map(Box::new)
                .map(protocol::SnapshotReadReply::SourceMaintenance),
            protocol::SnapshotReadCommand::ReadSourceFileAttachment(request) => self
                .read_source_file_attachment(request)
                .map(protocol::SnapshotReadReply::SourceFileAttachment),
            protocol::SnapshotReadCommand::ReadAttachmentSourceFiles(request) => self
                .read_attachment_source_files(request)
                .map(protocol::SnapshotReadReply::AttachmentSourceFiles),
            protocol::SnapshotReadCommand::ReadSourceAttachmentSummary(request) => self
                .read_source_attachment_summary(request)
                .map(protocol::SnapshotReadReply::SourceAttachmentSummary),
            protocol::SnapshotReadCommand::ReadTrackIdentityReviewCandidates(request) => self
                .read_track_identity_review_candidates(request)
                .map(protocol::SnapshotReadReply::TrackIdentityReviewCandidates),
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
            protocol::SnapshotReadCommand::SearchFilterRead(request) => self
                .read_search_filter(request)
                .map(Box::new)
                .map(protocol::SnapshotReadReply::SearchFilter),
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

fn root_navigation_window_establishment_target(
    request: &protocol::ReadLibraryTreeChildrenRequest,
) -> Option<i64> {
    if request.parent_source_directory_id.is_some() {
        return None;
    }

    match &request.entry_point {
        protocol::LibraryTreeEntryPoint::Source { source_id } => Some(*source_id),
        protocol::LibraryTreeEntryPoint::SourceLocation { .. } => None,
    }
}

fn apply_root_navigation_window_establishment(
    reply: &mut protocol::ReadLibraryTreeChildrenReply,
    establishment: &RootNavigationWindowEstablishment,
) {
    let Some(window) = reply.window.as_mut() else {
        return;
    };

    match establishment.state {
        RootNavigationWindowEstablishmentState::NotRequired => {}
        RootNavigationWindowEstablishmentState::Established => {
            window.coverage.empty_result_authoritative = false;
        }
        RootNavigationWindowEstablishmentState::Empty => {
            window.coverage.empty_result_authoritative = true;
            if window.coverage.detail.is_none() {
                window.coverage.detail =
                    Some("The immediate source root child-directory window is empty.".to_string());
            }
        }
        RootNavigationWindowEstablishmentState::Missing => {
            apply_root_navigation_window_terminal_coverage(
                window,
                protocol::LibraryTreeCoverageState::LocationMissing,
                establishment
                    .detail
                    .clone()
                    .unwrap_or_else(|| "The selected source root is missing.".to_string()),
            );
        }
        RootNavigationWindowEstablishmentState::Blocked => {
            apply_root_navigation_window_terminal_coverage(
                window,
                protocol::LibraryTreeCoverageState::Blocked,
                establishment
                    .detail
                    .clone()
                    .unwrap_or_else(|| "The selected source root is blocked.".to_string()),
            );
        }
        RootNavigationWindowEstablishmentState::Failed => {
            apply_root_navigation_window_terminal_coverage(
                window,
                protocol::LibraryTreeCoverageState::Failed,
                establishment.detail.clone().unwrap_or_else(|| {
                    "The source root child-directory window read failed.".to_string()
                }),
            );
        }
    }
}

fn apply_root_navigation_window_terminal_coverage(
    window: &mut protocol::LibraryTreeWindow,
    state: protocol::LibraryTreeCoverageState,
    detail: String,
) {
    window.rows.clear();
    window.total_rows = 0;
    window.coverage = protocol::LibraryTreeCoverage {
        state,
        subtree_coverage_complete: false,
        empty_result_authoritative: false,
        detail: Some(detail),
    };
}

impl Drop for LibraryBoundaryService {
    fn drop(&mut self) {
        self.source_maintenance.stop();
        let jobs = {
            let mut registry = self.scan_registry.lock().expect("scan registry poisoned");
            let root_ids = registry.jobs.keys().copied().collect::<Vec<_>>();
            if !root_ids.is_empty() {
                self.durable_store.cancel_root_work(&root_ids);
            }
            registry.drain_jobs()
        };

        for job in jobs {
            let _ = job.handle.join();
        }
    }
}

fn execute_scan_job(
    store: SqliteDurableStore,
    events: LibraryBoundaryEventStream,
    root_id: i64,
    scan_run_id: i64,
    scan_started_at_ms: i64,
    terminal_publication_complete: Arc<AtomicBool>,
    source_maintenance: SourceMaintenanceController,
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
            // Registered local root ids are durable source ids; source
            // maintenance is source-scoped and bounded.
            source_maintenance.request_source(root_id);
            let _ = source_maintenance.run_queued(&store, &events);
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

pub(crate) fn require_positive_i64(value: i64, field_name: &str) -> protocol::ProtocolResult<i64> {
    if value > 0 {
        Ok(value)
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("{field_name} must be positive"),
        })
    }
}

fn require_optional_positive_limit(
    value: Option<usize>,
    field_name: &str,
) -> protocol::ProtocolResult<()> {
    if matches!(value, Some(0)) {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: format!("{field_name} must be greater than zero"),
        })
    } else {
        Ok(())
    }
}

fn map_run_source_maintenance_reply(
    run: SourceMaintenanceRun,
) -> protocol::RunSourceMaintenanceReply {
    protocol::RunSourceMaintenanceReply {
        source_id: run.source_id,
        status: run.status,
        effective_limits: run.effective_limits,
        hash: run.hash,
        attachment_materialization: run.attachment_materialization,
        probe: run.probe,
        primary_media_promotion: run.primary_media_promotion,
        track_identity_candidates: run.track_identity_candidates,
        track_identity_decisions: run.track_identity_decisions,
        remaining_hash_candidates: run.remaining_hash_candidates,
        remaining_probe_candidates: run.remaining_probe_candidates,
        remaining_primary_media_promotion_candidates: run
            .remaining_primary_media_promotion_candidates,
        remaining_track_identity_candidate_production_candidates: run
            .remaining_track_identity_candidate_production_candidates,
        remaining_track_identity_decision_production_candidates: run
            .remaining_track_identity_decision_production_candidates,
        attachment_links: run.attachment_links,
        source_failure: run.source_failure,
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
    let empty_filter = match policy {
        protocol::ContentsReadPolicy::PlayableMediaBrowse
        | protocol::ContentsReadPolicy::AudioBrowse => false,
        protocol::ContentsReadPolicy::SourceFileInventory { file_classes } => {
            file_classes.is_empty()
        }
        protocol::ContentsReadPolicy::PrimaryMedia { media_kinds } => media_kinds.is_empty(),
    };
    if empty_filter {
        return Err(protocol::ProtocolError::InvalidRequest {
            detail: "contents policy filter must not be empty".to_string(),
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

fn validate_search_filter_scope(
    scope: &protocol::SearchFilterScope,
) -> protocol::ProtocolResult<()> {
    match scope {
        protocol::SearchFilterScope::Library => {}
        protocol::SearchFilterScope::Source { source_id } => {
            require_positive_i64(*source_id, "searchFilter sourceId")?;
        }
        protocol::SearchFilterScope::SourceLocation { source_location_id } => {
            require_positive_i64(*source_location_id, "searchFilter sourceLocationId")?;
        }
        protocol::SearchFilterScope::Directory {
            source_id,
            source_directory_id,
        } => {
            require_positive_i64(*source_id, "searchFilter sourceId")?;
            require_positive_i64(*source_directory_id, "searchFilter sourceDirectoryId")?;
        }
    }

    Ok(())
}

fn validate_search_filter_limit(limit: usize) -> protocol::ProtocolResult<()> {
    if (1..=200).contains(&limit) {
        Ok(())
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: "searchFilter limit must be between 1 and 200".to_string(),
        })
    }
}

fn validate_attachment_source_files_limit(limit: usize) -> protocol::ProtocolResult<()> {
    if (1..=200).contains(&limit) {
        Ok(())
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: "readAttachmentSourceFiles limit must be between 1 and 200".to_string(),
        })
    }
}

fn validate_track_identity_review_candidates_limit(limit: usize) -> protocol::ProtocolResult<()> {
    if (1..=200).contains(&limit) {
        Ok(())
    } else {
        Err(protocol::ProtocolError::InvalidRequest {
            detail: "readTrackIdentityReviewCandidates limit must be between 1 and 200".to_string(),
        })
    }
}

pub(crate) fn unix_time_ms() -> protocol::ProtocolResult<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| protocol::ProtocolError::HostFailure {
            detail: format!("system clock is before UNIX epoch: {error}"),
        })?;
    i64::try_from(duration.as_millis()).map_err(|error| protocol::ProtocolError::HostFailure {
        detail: format!("current UNIX timestamp does not fit i64 milliseconds: {error}"),
    })
}

pub(crate) fn map_store_error(
    error: library_store_sqlite::LibrarySqliteError,
) -> protocol::ProtocolError {
    protocol::ProtocolError::DurableStoreFailure {
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use library_boundary_protocol::{
        AcceptTrackIdentityCandidateRequest, AttachmentIdentityReadStatus, CancelRootScanReply,
        CancelRootScanRequest, CancelRootScanStatus, CommandOutcome, CommandReply, CommandRequest,
        ContentsFileClass, ContentsReadPolicy, ContentsReadRequest, ContentsScope,
        ContentsScopeDepth, CreatePlaylistReply, CreatePlaylistRequest,
        DeferTrackIdentityCandidateRequest, DeletePlaylistReply, DeletePlaylistRequest,
        DirectoryImageMediaState, DirectoryPrimaryMediaState, DirectoryScanState,
        HashSourceFilesBlake3Reply, HashSourceFilesBlake3Request,
        HashSourceFilesBlake3SourceFailure, LibraryBoundaryEvent,
        LibraryBoundaryEventStreamCommand, LibraryBoundaryEventStreamReply, LibraryRootCommand,
        LibraryRootReply, LibraryTreeEntryPoint, LibraryTreeNodeKind, LibraryTreePresenceState,
        LoadNavigationRowByStableKeyReply, LoadNavigationRowByStableKeyRequest,
        MaintainedSnapshotScope, PlaylistWriteCommand, PlaylistWriteReply, ProtocolError,
        ReadAttachmentSourceFilesReply, ReadAttachmentSourceFilesRequest,
        ReadLibraryBoundaryEventsAfterReply, ReadLibraryBoundaryEventsAfterRequest,
        ReadLibraryTreeChildrenRequest, ReadSourceAttachmentSummaryReply,
        ReadSourceAttachmentSummaryRequest, ReadSourceFileAttachmentReply,
        ReadSourceFileAttachmentRequest, ReadSourceLifecycleReply, ReadSourceLifecycleRequest,
        ReadSourceMaintenanceReply, ReadSourceMaintenanceRequest,
        ReadTrackIdentityReviewCandidatesRequest, RegisterLocalRootReply, RegisterLocalRootRequest,
        RejectTrackIdentityCandidateRequest, RenamePlaylistReply, RenamePlaylistRequest,
        RunSourceMaintenanceReply, RunSourceMaintenanceRequest, SearchFilterFileClass,
        SearchFilterReadReply, SearchFilterReadRequest, SearchFilterRecursion,
        SearchFilterResultKind, SearchFilterScope, SearchFilterSet, SearchFilterSort,
        SearchFilterState, SnapshotReadCommand, SnapshotReadReply, SourceFileAttachmentLinkStatus,
        SourceFileHashCommand, SourceFileHashReply, SourceMaintenanceCommand,
        SourceMaintenanceReply, StartRootScanReply, StartRootScanRequest,
        TrackIdentityDecisionCommand, TrackIdentityDecisionCommandFailure,
        TrackIdentityDecisionCommandResult, TrackIdentityDecisionReply, TrackIdentityDecisionState,
        TrackIdentityEffectiveDecisionCurrentStatus, TrackIdentityEffectiveDecisionPrecedence,
        TrackIdentityReviewReadStatus, TrackIdentityReviewState,
        TrackIdentityUserBlockingDecisionState, UnregisterLocalRootReply,
        UnregisterLocalRootRequest,
    };
    use rusqlite::Connection;
    use serde_json::json;
    use tempfile::TempDir;

    use library_domain::{
        SourceAccessIssueKind, SourceAccessState, SourcePresenceState, SourceScanPhase,
    };
    use library_store_sqlite::{
        LibraryStoreContext, ReadSourceFileBlake3HashCandidatesInput,
        RecordSourceFileObservationInput, SourceFileBlake3HashAdmissionScope, StoreEnvironment,
        UpsertSourceInput, UpsertSourceScanStateInput, UpsertSourceStateInput, durable_store_path,
    };

    use crate::source_maintenance::{
        SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT, SOURCE_HASH_MAINTENANCE_BATCH_LIMIT,
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

    fn open_test_read_connection(context: &LibraryStoreContext) -> Connection {
        Connection::open(durable_store_path(
            &context.user_data_path,
            context.environment,
        ))
        .expect("open test read connection")
    }

    fn count_rows(context: &LibraryStoreContext, table: &str) -> i64 {
        open_test_read_connection(context)
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count rows in {table}: {error}"))
    }

    fn count_table_if_exists(context: &LibraryStoreContext, table: &str) -> Option<i64> {
        let connection = open_test_read_connection(context);
        let exists: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM sqlite_schema
                 WHERE type IN ('table', 'view')
                   AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("check optional table existence");
        if exists == 0 {
            return None;
        }
        Some(
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap_or_else(|error| panic!("count rows in optional {table}: {error}")),
        )
    }

    fn clear_attachment_identity_rows(context: &LibraryStoreContext) {
        let connection = open_test_read_connection(context);
        connection
            .execute("DELETE FROM source_file_attachment_links", [])
            .expect("clear source-file attachment links");
        connection
            .execute("DELETE FROM content_attachments", [])
            .expect("clear content attachments");
    }

    fn record_present_source_file(
        service: &LibraryBoundaryService,
        source_id: i64,
        source_file_id: i64,
        relative_path: &str,
        size_bytes: usize,
    ) {
        service
            .durable_store
            .record_source_file_observation(RecordSourceFileObservationInput {
                source_file_id: Some(source_file_id),
                source_id,
                parent_source_directory_id: None,
                name: relative_path
                    .rsplit('/')
                    .next()
                    .expect("relative path has file name")
                    .to_string(),
                relative_path: relative_path.to_string(),
                size_bytes: Some(i64::try_from(size_bytes).expect("fixture file length fits i64")),
                mtime_ns: Some(source_file_id),
                presence_state: SourcePresenceState::Present,
                first_discovered_at: Some(10 + source_file_id),
                observed_at: Some(10 + source_file_id),
                presence_changed_at: 10 + source_file_id,
                updated_at: 10 + source_file_id,
            })
            .expect("record source file");
    }

    fn tiny_wav_bytes(
        sample_rate: u32,
        channels: u16,
        bits_per_sample: u16,
        frames: u32,
    ) -> Vec<u8> {
        let bytes_per_sample = bits_per_sample / 8;
        let block_align = channels * bytes_per_sample;
        let byte_rate = sample_rate * u32::from(block_align);
        let data_len = frames * u32::from(block_align);
        let riff_len = 36 + data_len;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_len.to_le_bytes());
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
        bytes.resize(
            bytes.len() + usize::try_from(data_len).expect("data length fits usize"),
            0,
        );
        bytes
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
    fn contents_policy_validation_rejects_empty_profile_filter() {
        let error = validate_contents_policy(&ContentsReadPolicy::SourceFileInventory {
            file_classes: Vec::new(),
        })
        .expect_err("empty contents file classes should be rejected");

        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
    }

    #[test]
    fn contents_service_preserves_playable_media_and_audio_browse_policy_semantics() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("contents-policy-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.m4a", 10);
        record_present_source_file(&service, registered.root_id, 101, "clip.mp4", 10);
        record_present_source_file(&service, registered.root_id, 102, "cover.jpg", 10);
        service
            .durable_store
            .upsert_source_scan_state(UpsertSourceScanStateInput {
                source_id: registered.root_id,
                scan_phase: SourceScanPhase::Complete,
                last_scan_started_at: Some(10),
                last_scan_finished_at: Some(20),
                last_successful_scan_at: Some(20),
                scan_issue_kind: None,
                error_detail: None,
                updated_at: 20,
            })
            .expect("complete source scan");

        let playable = service
            .read_contents(ContentsReadRequest {
                scope: ContentsScope::Source {
                    source_id: registered.root_id,
                },
                policy: ContentsReadPolicy::PlayableMediaBrowse,
                scope_depth: ContentsScopeDepth::Recursive,
                limit: Some(10),
                cursor: None,
            })
            .expect("read playable media")
            .result;
        assert_eq!(
            playable
                .rows
                .iter()
                .map(|row| row.file_name.as_str())
                .collect::<Vec<_>>(),
            vec!["clip.mp4", "track.m4a"]
        );
        assert_eq!(
            playable
                .rows
                .iter()
                .map(|row| row.file_class)
                .collect::<Vec<_>>(),
            vec![ContentsFileClass::Video, ContentsFileClass::Audio]
        );
        assert!(playable.has_policy_omitted_rows);

        let audio = service
            .read_contents(ContentsReadRequest {
                scope: ContentsScope::Source {
                    source_id: registered.root_id,
                },
                policy: ContentsReadPolicy::AudioBrowse,
                scope_depth: ContentsScopeDepth::Recursive,
                limit: Some(10),
                cursor: None,
            })
            .expect("read audio browse")
            .result;
        assert_eq!(audio.rows.len(), 1);
        assert_eq!(audio.rows[0].file_name, "track.m4a");
        assert_eq!(audio.rows[0].file_class, ContentsFileClass::Audio);
        assert!(audio.has_policy_omitted_rows);
    }

    #[test]
    fn search_filter_snapshot_read_uses_backend_index_rows() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("search-filter-root");
        let set_dir = source_root.join("sets");
        std::fs::create_dir_all(&set_dir).expect("create source root");
        std::fs::write(
            set_dir.join("Amen Break.wav"),
            tiny_wav_bytes(44_100, 2, 16, 128),
        )
        .expect("write audio file");
        std::fs::write(set_dir.join("cover.jpg"), b"cover").expect("write image file");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);

        let text_reply = expect_search_filter_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::SearchFilterRead(
                SearchFilterReadRequest {
                    scope: SearchFilterScope::Source {
                        source_id: registered.root_id,
                    },
                    recursion: SearchFilterRecursion::Recursive,
                    text_query: Some("amen".to_string()),
                    target_kinds: vec![SearchFilterResultKind::SourceFile],
                    filters: SearchFilterSet::default(),
                    sort: SearchFilterSort::PathName,
                    limit: Some(10),
                    cursor: None,
                },
            )),
        )));
        assert_eq!(text_reply.result.state, SearchFilterState::Ready);
        assert_eq!(text_reply.result.rows.len(), 1);
        let row = &text_reply.result.rows[0];
        assert_eq!(row.result_kind, SearchFilterResultKind::SourceFile);
        assert_eq!(row.display_label, "Amen Break.wav");
        assert_eq!(row.file_class, Some(SearchFilterFileClass::Audio));
        assert_eq!(row.source_id, Some(registered.root_id));
        assert_eq!(row.authority_layer, "source_file_inventory");
        assert_eq!(
            text_reply.result.query_identity.text_query.as_deref(),
            Some("amen")
        );
        assert_eq!(text_reply.result.query_identity.page_size, 10);

        let audio_reply = expect_search_filter_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::SearchFilterRead(
                SearchFilterReadRequest {
                    scope: SearchFilterScope::Source {
                        source_id: registered.root_id,
                    },
                    recursion: SearchFilterRecursion::Recursive,
                    text_query: None,
                    target_kinds: vec![SearchFilterResultKind::SourceFile],
                    filters: SearchFilterSet {
                        file_classes: vec![SearchFilterFileClass::Audio],
                        ..SearchFilterSet::default()
                    },
                    sort: SearchFilterSort::PathName,
                    limit: Some(10),
                    cursor: None,
                },
            )),
        )));
        assert_eq!(audio_reply.result.state, SearchFilterState::Ready);
        assert_eq!(
            audio_reply
                .result
                .rows
                .iter()
                .map(|row| row.display_label.as_str())
                .collect::<Vec<_>>(),
            vec!["Amen Break.wav"]
        );
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

    fn expect_source_file_attachment_reply(reply: CommandReply) -> ReadSourceFileAttachmentReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::SourceFileAttachment(reply)) => reply,
            other => panic!("expected source-file attachment reply, got {other:?}"),
        }
    }

    fn expect_attachment_source_files_reply(reply: CommandReply) -> ReadAttachmentSourceFilesReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::AttachmentSourceFiles(reply)) => reply,
            other => panic!("expected attachment source-files reply, got {other:?}"),
        }
    }

    fn expect_source_attachment_summary_reply(
        reply: CommandReply,
    ) -> ReadSourceAttachmentSummaryReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::SourceAttachmentSummary(reply)) => reply,
            other => panic!("expected source attachment summary reply, got {other:?}"),
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

    fn expect_run_source_maintenance_reply(reply: CommandReply) -> RunSourceMaintenanceReply {
        match reply {
            CommandReply::SourceMaintenance(reply) => match *reply {
                SourceMaintenanceReply::RunSourceMaintenance(reply) => reply,
            },
            other => panic!("expected run source maintenance reply, got {other:?}"),
        }
    }

    fn expect_track_identity_decisions_reply(reply: CommandReply) -> TrackIdentityDecisionReply {
        match reply {
            CommandReply::TrackIdentityDecisions(reply) => reply,
            other => panic!("expected track identity decision authority reply, got {other:?}"),
        }
    }

    fn expect_read_source_maintenance_reply(reply: CommandReply) -> ReadSourceMaintenanceReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::SourceMaintenance(reply)) => *reply,
            other => panic!("expected source maintenance snapshot reply, got {other:?}"),
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

    fn expect_search_filter_reply(reply: CommandReply) -> SearchFilterReadReply {
        match reply {
            CommandReply::SnapshotRead(SnapshotReadReply::SearchFilter(reply)) => *reply,
            other => panic!("expected search filter reply, got {other:?}"),
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

    fn run_source_maintenance(
        service: &LibraryBoundaryService,
        source_id: i64,
        hash_limit: Option<usize>,
        attachment_limit: Option<usize>,
        probe_limit: Option<usize>,
        promotion_limit: Option<usize>,
    ) -> RunSourceMaintenanceReply {
        run_source_maintenance_with_request(
            service,
            RunSourceMaintenanceRequest {
                source_id,
                hash_limit,
                attachment_limit,
                probe_limit,
                promotion_limit,
                identity_candidate_limit: None,
                identity_decision_limit: None,
            },
        )
    }

    fn run_source_maintenance_with_request(
        service: &LibraryBoundaryService,
        request: RunSourceMaintenanceRequest,
    ) -> RunSourceMaintenanceReply {
        expect_run_source_maintenance_reply(expect_success(service.handle_command(
            CommandRequest::SourceMaintenance(SourceMaintenanceCommand::RunSourceMaintenance(
                request,
            )),
        )))
    }

    fn read_source_maintenance(
        service: &LibraryBoundaryService,
        source_id: i64,
    ) -> ReadSourceMaintenanceReply {
        expect_read_source_maintenance_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadSourceMaintenance(
                ReadSourceMaintenanceRequest { source_id },
            )),
        )))
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
        wait_for_scan_terminal_kind(
            service,
            root_id,
            library_boundary_protocol::SourceScanEventKind::SourceScanCompleted,
        );
    }

    fn wait_for_scan_terminal_kind(
        service: &LibraryBoundaryService,
        root_id: i64,
        expected_kind: library_boundary_protocol::SourceScanEventKind,
    ) {
        for _ in 0..30 {
            let events_reply = read_after_events(service, None, 64);
            let scan_run_id = events_reply.events.iter().find_map(|e| {
                let LibraryBoundaryEvent::SourceScanEvent(se) = e else {
                    return None;
                };
                (se.kind == expected_kind && se.root_id == root_id).then_some(se.scan_run_id)
            });
            if let Some(scan_run_id) = scan_run_id {
                let cancelled = cancel_root_scan(service, scan_run_id);
                assert_eq!(
                    cancelled.status,
                    CancelRootScanStatus::AlreadyTerminal,
                    "completed scan should be joined through the deterministic cleanup path"
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        panic!("scan should reach {expected_kind:?} on a tiny directory");
    }

    fn wait_for_scan_terminal_run(
        service: &LibraryBoundaryService,
        root_id: i64,
        scan_run_id: i64,
        expected_kind: library_boundary_protocol::SourceScanEventKind,
    ) {
        for _ in 0..30 {
            let events_reply = read_after_events(service, None, 64);
            let found = events_reply.events.iter().any(|e| {
                let LibraryBoundaryEvent::SourceScanEvent(se) = e else {
                    return false;
                };
                se.kind == expected_kind && se.root_id == root_id && se.scan_run_id == scan_run_id
            });
            if found {
                let cancelled = cancel_root_scan(service, scan_run_id);
                assert_eq!(
                    cancelled.status,
                    CancelRootScanStatus::AlreadyTerminal,
                    "completed scan should be joined through the deterministic cleanup path"
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        panic!("scan {scan_run_id} should reach {expected_kind:?} on a tiny directory");
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

    fn read_source_file_attachment(
        service: &LibraryBoundaryService,
        source_file_id: i64,
    ) -> ReadSourceFileAttachmentReply {
        expect_source_file_attachment_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadSourceFileAttachment(
                ReadSourceFileAttachmentRequest { source_file_id },
            )),
        )))
    }

    fn read_attachment_source_files(
        service: &LibraryBoundaryService,
        attachment_id: i64,
        limit: Option<usize>,
    ) -> ReadAttachmentSourceFilesReply {
        expect_attachment_source_files_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadAttachmentSourceFiles(
                ReadAttachmentSourceFilesRequest {
                    attachment_id,
                    limit,
                },
            )),
        )))
    }

    fn read_source_attachment_summary(
        service: &LibraryBoundaryService,
        source_id: i64,
    ) -> ReadSourceAttachmentSummaryReply {
        expect_source_attachment_summary_reply(expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadSourceAttachmentSummary(
                ReadSourceAttachmentSummaryRequest { source_id },
            )),
        )))
    }

    fn read_source_contents_file_ids(service: &LibraryBoundaryService, source_id: i64) -> Vec<i64> {
        service
            .read_contents(ContentsReadRequest {
                scope: ContentsScope::Source { source_id },
                policy: ContentsReadPolicy::SourceFileInventory {
                    file_classes: vec![
                        ContentsFileClass::Audio,
                        ContentsFileClass::Video,
                        ContentsFileClass::Image,
                        ContentsFileClass::Unsupported,
                    ],
                },
                scope_depth: ContentsScopeDepth::Recursive,
                limit: Some(200),
                cursor: None,
            })
            .expect("read source contents")
            .result
            .rows
            .into_iter()
            .map(|row| row.source_file_id)
            .collect()
    }

    fn read_hash_candidate_count(service: &LibraryBoundaryService, source_id: i64) -> usize {
        service
            .durable_store
            .read_source_file_blake3_hash_candidates(ReadSourceFileBlake3HashCandidatesInput {
                scope: SourceFileBlake3HashAdmissionScope::Source { source_id },
                limit: Some(512),
            })
            .expect("read hash candidates")
            .len()
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
        assert_eq!(
            crate_row.navigable_child_scope_state,
            Some(library_boundary_protocol::NavigableChildScopeState::NoNavigableChildScopes)
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
        assert_eq!(crate_window.total_rows, 0);
        assert!(
            crate_window.rows.is_empty(),
            "tree children are navigation-only; files stay in contents/inventory reads"
        );

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
    fn scan_completion_runs_service_owned_source_maintenance() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("hash-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let run_bound = SOURCE_HASH_MAINTENANCE_BATCH_LIMIT;
        let candidate_count = run_bound + 1;
        for index in 0..candidate_count {
            std::fs::write(
                source_root.join(format!("track-{index:02}.flac")),
                format!("track {index} bytes"),
            )
            .expect("write media file");
        }
        std::fs::write(source_root.join("notes.txt"), b"not media").expect("write notes");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);

        let file_ids = read_source_contents_file_ids(&service, registered.root_id);
        assert_eq!(file_ids.len(), candidate_count);
        assert_eq!(read_hash_candidate_count(&service, registered.root_id), 1);

        let hashed_after_scan = file_ids
            .iter()
            .filter(|source_file_id| {
                service
                    .durable_store
                    .read_observed_file_facts_for_source_file(**source_file_id)
                    .expect("read observed facts")
                    .is_some()
            })
            .count();
        assert_eq!(
            hashed_after_scan, run_bound,
            "scan-triggered maintenance must only run the bounded maintenance unit"
        );
        assert_eq!(
            count_rows(&context, "source_file_attachment_links"),
            i64::try_from(SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT)
                .expect("attachment limit fits i64"),
            "scan-triggered maintenance must only run one bounded attachment materialization unit"
        );
        assert_eq!(
            count_rows(&context, "content_attachments"),
            i64::try_from(SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT)
                .expect("attachment limit fits i64"),
            "test fixture uses unique bytes, so one bounded unit creates one attachment per link"
        );

        let events_after_hash = read_after_events(&service, None, 128);
        assert!(
            events_after_hash.events.iter().any(|e| {
                let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(event) = e else {
                    return false;
                };
                event.invalidation.scope == MaintainedSnapshotScope::LibraryBrowser
            }),
            "hash evidence changes must publish the narrow current maintained scope"
        );

        let runs = service.source_maintenance.completed_runs_for_test();
        let run = runs
            .iter()
            .find(|run| run.source_id == registered.root_id)
            .expect("scan completion requests source maintenance");
        assert_eq!(run.hash.hashed_count, run_bound);
        assert_eq!(run.remaining_hash_candidates, 1);
        assert!(!run.stopped);
        assert_eq!(
            run.attachment_materialization.links_created,
            SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT
        );
        assert_eq!(
            run.attachment_materialization.remaining_candidates,
            run_bound - SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT
        );
        assert_eq!(run.probe.failed_count, run.probe.effective_limit);

        let manually_hashed = hash_source_files_blake3(&service, registered.root_id, Some(10));
        assert_eq!(manually_hashed.hashed_count, 1);
        assert_eq!(manually_hashed.remaining_candidates, 0);
        assert_eq!(
            count_rows(&context, "source_file_attachment_links"),
            i64::try_from(SOURCE_ATTACHMENT_MATERIALIZATION_BATCH_LIMIT * 2)
                .expect("attachment limit fits i64"),
            "manual source maintenance must also trigger one bounded attachment materialization unit"
        );

        for source_file_id in &file_ids {
            let facts = service
                .durable_store
                .read_observed_file_facts_for_source_file(*source_file_id)
                .expect("read observed facts")
                .expect("scan-triggered and manual maintenance write observed facts");
            assert_eq!(
                facts.content_hash.expect("content hash").algorithm,
                "blake3"
            );
        }

        for table in [
            "LibraryAssets",
            "LibraryAssetAttachments",
            "LibraryBrowserRows",
            "SourceSegmentSets",
            "SourceSegments",
            "PrepAssignments",
            "ResolvedLibraryAssetPrepTargets",
        ] {
            assert_eq!(
                count_rows(&context, table),
                0,
                "{table} must not be created by hash/attachment maintenance"
            );
        }
        for absent_or_future_table in [
            "Tracks",
            "TrackRows",
            "LibraryTracks",
            "PrepRows",
            "PreparationRows",
            "primaryMedia",
        ] {
            assert!(
                matches!(
                    count_table_if_exists(&context, absent_or_future_table),
                    None | Some(0)
                ),
                "{absent_or_future_table} must be absent or empty"
            );
        }
    }

    #[test]
    fn scan_completion_materializes_existing_blake3_candidates_without_hash_work() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("existing-attachment-candidates-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("a.flac"), b"a").expect("write a");
        std::fs::write(source_root.join("b.flac"), b"b").expect("write b");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _first_scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);
        assert_eq!(read_hash_candidate_count(&service, registered.root_id), 0);
        clear_attachment_identity_rows(&context);
        assert_eq!(count_rows(&context, "source_file_attachment_links"), 0);

        let second_scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_terminal_run(
            &service,
            registered.root_id,
            second_scan.scan_run_id,
            library_boundary_protocol::SourceScanEventKind::SourceScanCompleted,
        );

        assert_eq!(read_hash_candidate_count(&service, registered.root_id), 0);
        assert_eq!(
            count_rows(&context, "source_file_attachment_links"),
            2,
            "scan-triggered source maintenance should run one attachment unit even when hash work is already current"
        );
        let runs = service.source_maintenance.completed_runs_for_test();
        let run = runs
            .iter()
            .rev()
            .find(|run| run.source_id == registered.root_id)
            .expect("second scan completion records source maintenance");
        assert_eq!(run.hash.hashed_count, 0);
        assert_eq!(run.attachment_materialization.links_created, 2);
        assert_eq!(run.attachment_materialization.remaining_candidates, 0);
    }

    #[test]
    fn run_source_maintenance_hashes_materializes_and_probes_one_bounded_unit() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("source-maintenance-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let wav_bytes = tiny_wav_bytes(44_100, 2, 16, 4_410);
        std::fs::write(source_root.join("track.wav"), &wav_bytes).expect("write wav");
        std::fs::write(source_root.join("video.mp4"), b"video bytes").expect("write video");
        std::fs::write(source_root.join("album.cue"), b"FILE track.wav WAVE").expect("write cue");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(
            &service,
            registered.root_id,
            100,
            "track.wav",
            wav_bytes.len(),
        );
        record_present_source_file(&service, registered.root_id, 101, "video.mp4", 11);
        record_present_source_file(&service, registered.root_id, 102, "album.cue", 19);

        let run = run_source_maintenance(
            &service,
            registered.root_id,
            Some(10),
            Some(10),
            Some(10),
            Some(10),
        );

        assert_eq!(
            run.status,
            library_boundary_protocol::SourceMaintenanceRunStatus::Completed
        );
        assert_eq!(run.effective_limits.hash_limit, 10);
        assert_eq!(run.hash.hashed_count, 3);
        assert_eq!(run.remaining_hash_candidates, 0);
        assert_eq!(run.attachment_materialization.links_created, 3);
        assert_eq!(run.probe.probed_count, 1);
        assert_eq!(run.probe.failed_count, 0);
        assert_eq!(run.remaining_probe_candidates, 0);
        let links = run.attachment_links.expect("attachment summary exists");
        assert_eq!(links.current_links_count, 3);
        assert_eq!(links.stale_links_count, 0);
        assert_eq!(links.source_files_missing_attachment_links_count, 0);

        let audio_facts = service
            .durable_store
            .read_observed_file_facts_for_source_file(100)
            .expect("read audio facts")
            .expect("audio facts exist");
        assert_eq!(
            audio_facts.content_hash.expect("audio hash").algorithm,
            "blake3"
        );
        assert_eq!(audio_facts.duration_ms, Some(100));
        assert_eq!(audio_facts.sample_rate_hz, Some(44_100));

        let video_facts = service
            .durable_store
            .read_observed_file_facts_for_source_file(101)
            .expect("read video facts")
            .expect("video facts exist");
        assert_eq!(
            video_facts.content_hash.expect("video hash").algorithm,
            "blake3"
        );
        assert_eq!(video_facts.duration_ms, None);

        let cue_facts = service
            .durable_store
            .read_observed_file_facts_for_source_file(102)
            .expect("read cue facts")
            .expect("cue facts exist");
        assert_eq!(cue_facts.media_kind, "cue_sheet");
        assert_eq!(
            cue_facts.content_hash.expect("cue hash").algorithm,
            "blake3"
        );
        assert_eq!(cue_facts.duration_ms, None);

        let snapshot = read_source_maintenance(&service, registered.root_id);
        assert_eq!(
            snapshot.status,
            library_boundary_protocol::SourceMaintenanceSnapshotStatus::Idle
        );
        assert_eq!(snapshot.remaining_hash_candidates, 0);
        assert_eq!(snapshot.remaining_probe_candidates, 0);
        assert_eq!(
            snapshot.last_run.expect("last run summary exists").status,
            library_boundary_protocol::SourceMaintenanceRunStatus::Completed
        );

        for table in [
            "LibraryAssets",
            "LibraryAssetAttachments",
            "LibraryBrowserRows",
            "SourceSegmentSets",
            "SourceSegments",
            "PrepAssignments",
            "ResolvedLibraryAssetPrepTargets",
        ] {
            assert_eq!(
                count_rows(&context, table),
                0,
                "{table} must not be created by source maintenance"
            );
        }
    }

    #[test]
    fn source_maintenance_track_identity_candidate_phase_respects_limit() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("track-identity-candidate-limit-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let first_wav = tiny_wav_bytes(44_100, 2, 16, 4_410);
        let second_wav = tiny_wav_bytes(44_100, 2, 16, 2_205);
        std::fs::write(source_root.join("track-a.wav"), &first_wav).expect("write first wav");
        std::fs::write(source_root.join("track-b.wav"), &second_wav).expect("write second wav");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(
            &service,
            registered.root_id,
            100,
            "track-a.wav",
            first_wav.len(),
        );
        record_present_source_file(
            &service,
            registered.root_id,
            101,
            "track-b.wav",
            second_wav.len(),
        );

        let run = run_source_maintenance_with_request(
            &service,
            RunSourceMaintenanceRequest {
                source_id: registered.root_id,
                hash_limit: Some(10),
                attachment_limit: Some(10),
                probe_limit: Some(10),
                promotion_limit: Some(10),
                identity_candidate_limit: Some(1),
                identity_decision_limit: None,
            },
        );

        assert_eq!(
            run.status,
            library_boundary_protocol::SourceMaintenanceRunStatus::Partial
        );
        assert_eq!(run.track_identity_candidates.effective_limit, 1);
        assert_eq!(run.track_identity_candidates.candidates_created, 1);
        assert_eq!(
            run.remaining_track_identity_candidate_production_candidates,
            1
        );
        assert_eq!(count_rows(&context, "track_identity_candidates"), 1);
        assert_eq!(count_rows(&context, "track_identity_candidate_members"), 1);
    }

    #[test]
    fn source_maintenance_track_identity_decision_phase_respects_limit() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("track-identity-decision-limit-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let first_wav = tiny_wav_bytes(44_100, 2, 16, 4_410);
        let second_wav = tiny_wav_bytes(44_100, 2, 16, 2_205);
        std::fs::write(source_root.join("track-a.wav"), &first_wav).expect("write first wav");
        std::fs::write(source_root.join("track-b.wav"), &second_wav).expect("write second wav");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(
            &service,
            registered.root_id,
            100,
            "track-a.wav",
            first_wav.len(),
        );
        record_present_source_file(
            &service,
            registered.root_id,
            101,
            "track-b.wav",
            second_wav.len(),
        );

        let run = run_source_maintenance_with_request(
            &service,
            RunSourceMaintenanceRequest {
                source_id: registered.root_id,
                hash_limit: Some(10),
                attachment_limit: Some(10),
                probe_limit: Some(10),
                promotion_limit: Some(10),
                identity_candidate_limit: Some(10),
                identity_decision_limit: Some(1),
            },
        );

        assert_eq!(
            run.status,
            library_boundary_protocol::SourceMaintenanceRunStatus::Partial
        );
        assert_eq!(run.track_identity_candidates.candidates_created, 2);
        assert_eq!(run.track_identity_decisions.effective_limit, 1);
        assert_eq!(run.track_identity_decisions.decisions_created, 1);
        assert_eq!(
            run.remaining_track_identity_decision_production_candidates,
            1
        );
        assert_eq!(count_rows(&context, "track_identity_decisions"), 1);
        assert_eq!(count_rows(&context, "track_identity_decision_evidence"), 1);
    }

    #[test]
    fn track_identity_decisions_commands_are_service_owned_and_candidate_scoped() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir
            .path()
            .join("track-identity-decision-authority-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let wav = tiny_wav_bytes(44_100, 2, 16, 4_410);
        std::fs::write(source_root.join("track.wav"), &wav).expect("write wav");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.wav", wav.len());

        let maintenance = run_source_maintenance_with_request(
            &service,
            RunSourceMaintenanceRequest {
                source_id: registered.root_id,
                hash_limit: Some(10),
                attachment_limit: Some(10),
                probe_limit: Some(10),
                promotion_limit: Some(10),
                identity_candidate_limit: Some(10),
                identity_decision_limit: Some(10),
            },
        );
        assert_eq!(maintenance.track_identity_candidates.candidates_created, 1);
        assert_eq!(maintenance.track_identity_decisions.decisions_created, 1);

        let candidate_id = service
            .durable_store
            .read_track_identity_candidates_for_source(registered.root_id, 10)
            .expect("read track identity candidates")
            .into_iter()
            .next()
            .expect("candidate exists")
            .track_identity_candidate_id;

        let reject_reply = expect_track_identity_decisions_reply(expect_success(
            service.handle_command(CommandRequest::TrackIdentityDecisions(
                TrackIdentityDecisionCommand::RejectTrackIdentityCandidate(
                    RejectTrackIdentityCandidateRequest {
                        candidate_id,
                        reason: Some("  not this identity  ".to_string()),
                    },
                ),
            )),
        ));
        let reject_success = match reject_reply {
            TrackIdentityDecisionReply::RejectTrackIdentityCandidate(
                TrackIdentityDecisionCommandResult::Written(success),
            ) => success,
            other => panic!("expected reject success, got {other:?}"),
        };
        assert_eq!(reject_success.candidate_id, candidate_id);
        assert_eq!(
            reject_success.decision_state,
            TrackIdentityDecisionState::Rejected
        );
        assert_eq!(reject_success.decision_source, "user_local_v0");
        assert_eq!(reject_success.evidence_snapshot_count, 1);
        assert_eq!(
            reject_success
                .effective_decision
                .effective_decision_current_status,
            TrackIdentityEffectiveDecisionCurrentStatus::Current
        );
        assert_eq!(
            reject_success
                .effective_decision
                .effective_decision_precedence,
            TrackIdentityEffectiveDecisionPrecedence::User
        );
        assert_eq!(
            reject_success
                .effective_decision
                .user_blocking_decision_state,
            TrackIdentityUserBlockingDecisionState::Rejected
        );
        assert!(
            reject_success
                .effective_decision
                .masked_system_decision_id
                .is_some()
        );

        let decisions_after_reject = service
            .durable_store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read decisions after reject");
        let user_reject = decisions_after_reject
            .iter()
            .find(|decision| decision.decision_source == "user_local_v0")
            .expect("user reject decision exists");
        assert_eq!(user_reject.decision_reason, "not this identity");

        let blocked_maintenance = run_source_maintenance_with_request(
            &service,
            RunSourceMaintenanceRequest {
                source_id: registered.root_id,
                hash_limit: Some(10),
                attachment_limit: Some(10),
                probe_limit: Some(10),
                promotion_limit: Some(10),
                identity_candidate_limit: Some(10),
                identity_decision_limit: Some(10),
            },
        );
        assert_eq!(
            blocked_maintenance
                .track_identity_decisions
                .decisions_created,
            0
        );
        assert_eq!(
            blocked_maintenance
                .track_identity_decisions
                .skipped_user_blocked_candidates,
            1
        );

        let accept_reply = expect_track_identity_decisions_reply(expect_success(
            service.handle_command(CommandRequest::TrackIdentityDecisions(
                TrackIdentityDecisionCommand::AcceptTrackIdentityCandidate(
                    AcceptTrackIdentityCandidateRequest {
                        candidate_id,
                        reason: Some("same identity".to_string()),
                    },
                ),
            )),
        ));
        let accept_success = match accept_reply {
            TrackIdentityDecisionReply::AcceptTrackIdentityCandidate(
                TrackIdentityDecisionCommandResult::Written(success),
            ) => success,
            other => panic!("expected accept success, got {other:?}"),
        };
        assert_eq!(
            accept_success.decision_state,
            TrackIdentityDecisionState::Accepted
        );
        assert_eq!(
            accept_success
                .effective_decision
                .effective_decision_precedence,
            TrackIdentityEffectiveDecisionPrecedence::User
        );
        assert_eq!(
            accept_success
                .effective_decision
                .user_blocking_decision_state,
            TrackIdentityUserBlockingDecisionState::None
        );
        assert_eq!(
            accept_success.effective_decision.masked_system_decision_id,
            None
        );

        let decisions_after_accept = service
            .durable_store
            .read_track_identity_decisions_for_candidate(candidate_id, 10)
            .expect("read decisions after accept");
        assert!(
            decisions_after_accept
                .iter()
                .any(
                    |decision| decision.track_identity_decision_id == reject_success.decision_id
                        && decision.superseded_by_decision_id == Some(accept_success.decision_id)
                ),
            "user accept should supersede the prior current user reject"
        );

        let missing_reply = expect_track_identity_decisions_reply(expect_success(
            service.handle_command(CommandRequest::TrackIdentityDecisions(
                TrackIdentityDecisionCommand::AcceptTrackIdentityCandidate(
                    AcceptTrackIdentityCandidateRequest {
                        candidate_id: 99_999,
                        reason: None,
                    },
                ),
            )),
        ));
        assert!(matches!(
            missing_reply,
            TrackIdentityDecisionReply::AcceptTrackIdentityCandidate(
                TrackIdentityDecisionCommandResult::Failed(
                    TrackIdentityDecisionCommandFailure::CandidateNotFound
                )
            )
        ));

        let invalid = service.try_handle_command(CommandRequest::TrackIdentityDecisions(
            TrackIdentityDecisionCommand::DeferTrackIdentityCandidate(
                DeferTrackIdentityCandidateRequest {
                    candidate_id: 0,
                    reason: None,
                },
            ),
        ));
        assert!(matches!(invalid, Err(ProtocolError::InvalidRequest { .. })));
    }

    #[test]
    fn track_identity_review_candidates_snapshot_read_maps_store_decisions() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("track-identity-review-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let wav = tiny_wav_bytes(44_100, 2, 16, 4_410);
        std::fs::write(source_root.join("track.wav"), &wav).expect("write wav");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.wav", wav.len());

        let maintenance = run_source_maintenance_with_request(
            &service,
            RunSourceMaintenanceRequest {
                source_id: registered.root_id,
                hash_limit: Some(10),
                attachment_limit: Some(10),
                probe_limit: Some(10),
                promotion_limit: Some(10),
                identity_candidate_limit: Some(10),
                identity_decision_limit: Some(10),
            },
        );
        assert_eq!(maintenance.track_identity_candidates.candidates_created, 1);
        assert_eq!(maintenance.track_identity_decisions.decisions_created, 1);

        let candidate_id = service
            .durable_store
            .read_track_identity_candidates_for_source(registered.root_id, 10)
            .expect("read track identity candidates")
            .into_iter()
            .next()
            .expect("candidate exists")
            .track_identity_candidate_id;

        let _reject_reply = expect_track_identity_decisions_reply(expect_success(
            service.handle_command(CommandRequest::TrackIdentityDecisions(
                TrackIdentityDecisionCommand::RejectTrackIdentityCandidate(
                    RejectTrackIdentityCandidateRequest {
                        candidate_id,
                        reason: None,
                    },
                ),
            )),
        ));

        let review_reply = match expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadTrackIdentityReviewCandidates(
                ReadTrackIdentityReviewCandidatesRequest {
                    source_id: Some(registered.root_id),
                    review_state: Some(TrackIdentityReviewState::UserRejected),
                    limit: 10,
                },
            )),
        )) {
            CommandReply::SnapshotRead(SnapshotReadReply::TrackIdentityReviewCandidates(reply)) => {
                reply
            }
            other => panic!("expected review candidates reply, got {other:?}"),
        };

        assert_eq!(review_reply.status, TrackIdentityReviewReadStatus::Ok);
        assert_eq!(review_reply.candidates.len(), 1);
        let candidate = &review_reply.candidates[0];
        assert_eq!(candidate.candidate_id, candidate_id);
        assert_eq!(
            candidate.review_state,
            TrackIdentityReviewState::UserRejected
        );
        assert_eq!(candidate.evidence_summary.member_count, 1);
        assert_eq!(candidate.evidence_summary.evidence_count, 1);
        assert_eq!(candidate.source_summary.source_count, 1);
        let decision = candidate
            .effective_decision
            .as_ref()
            .expect("effective decision");
        assert_eq!(
            decision.decision_state,
            TrackIdentityDecisionState::Rejected
        );
        assert_eq!(decision.decision_source, "user_local_v0");
        assert_eq!(
            decision.current_status,
            TrackIdentityEffectiveDecisionCurrentStatus::Current
        );
        assert_eq!(
            decision.user_blocking_decision_state,
            TrackIdentityUserBlockingDecisionState::Rejected
        );
        assert!(decision.masked_system_decision_id.is_some());

        let missing_source = match expect_success(service.handle_command(
            CommandRequest::SnapshotRead(SnapshotReadCommand::ReadTrackIdentityReviewCandidates(
                ReadTrackIdentityReviewCandidatesRequest {
                    source_id: Some(99_999),
                    review_state: None,
                    limit: 10,
                },
            )),
        )) {
            CommandReply::SnapshotRead(SnapshotReadReply::TrackIdentityReviewCandidates(reply)) => {
                reply
            }
            other => panic!("expected review candidates reply, got {other:?}"),
        };
        assert_eq!(
            missing_source.status,
            TrackIdentityReviewReadStatus::SourceNotFound
        );
        assert!(missing_source.candidates.is_empty());

        let invalid = service.try_handle_command(CommandRequest::SnapshotRead(
            SnapshotReadCommand::ReadTrackIdentityReviewCandidates(
                ReadTrackIdentityReviewCandidatesRequest {
                    source_id: Some(registered.root_id),
                    review_state: None,
                    limit: 0,
                },
            ),
        ));
        assert!(matches!(invalid, Err(ProtocolError::InvalidRequest { .. })));

        let invalid = service.try_handle_command(CommandRequest::SnapshotRead(
            SnapshotReadCommand::ReadTrackIdentityReviewCandidates(
                ReadTrackIdentityReviewCandidatesRequest {
                    source_id: Some(registered.root_id),
                    review_state: None,
                    limit: 201,
                },
            ),
        ));
        assert!(matches!(invalid, Err(ProtocolError::InvalidRequest { .. })));
    }

    #[test]
    fn attachment_identity_snapshot_reads_expose_current_stale_occurrences_and_summary() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("attachment-read-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("a.flac"), b"same bytes").expect("write a");
        std::fs::write(source_root.join("b.flac"), b"same bytes").expect("write b");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);
        let file_ids = read_source_contents_file_ids(&service, registered.root_id);
        assert_eq!(file_ids.len(), 2);

        let first = read_source_file_attachment(&service, file_ids[0]);
        assert_eq!(first.status, AttachmentIdentityReadStatus::Ok);
        let first_link = first
            .attachment_link
            .expect("first file has attachment link");
        assert_eq!(first_link.source_id, registered.root_id);
        assert_eq!(
            first_link.link_status,
            SourceFileAttachmentLinkStatus::Current
        );
        assert_eq!(first_link.content_hash_algorithm, "blake3");
        assert_eq!(
            first_link.file_kind,
            library_boundary_protocol::ContentsFileKind::Audio
        );

        let second = read_source_file_attachment(&service, file_ids[1]);
        assert_eq!(second.status, AttachmentIdentityReadStatus::Ok);
        assert_eq!(
            second
                .attachment_link
                .as_ref()
                .expect("second file has attachment link")
                .attachment_id,
            first_link.attachment_id,
            "same bytes should point at one attachment identity"
        );

        let limited = read_attachment_source_files(&service, first_link.attachment_id, Some(1));
        assert_eq!(limited.status, AttachmentIdentityReadStatus::Ok);
        assert_eq!(
            limited
                .attachment
                .as_ref()
                .expect("attachment identity is included")
                .content_hash_value,
            first_link.content_hash_value
        );
        assert_eq!(limited.source_file_links.len(), 1);
        assert_eq!(limited.effective_limit, 1);
        assert_eq!(limited.remaining_source_file_links, 1);

        let all_links = read_attachment_source_files(&service, first_link.attachment_id, Some(10));
        assert_eq!(all_links.source_file_links.len(), 2);
        assert!(
            all_links
                .source_file_links
                .iter()
                .all(|link| link.link_status == SourceFileAttachmentLinkStatus::Current)
        );

        let summary = read_source_attachment_summary(&service, registered.root_id);
        assert_eq!(summary.status, AttachmentIdentityReadStatus::Ok);
        let summary = summary.summary.expect("source summary exists");
        assert_eq!(summary.source_id, registered.root_id);
        assert_eq!(summary.current_links_count, 2);
        assert_eq!(summary.stale_links_count, 0);
        assert_eq!(summary.source_files_with_current_blake3_facts_count, 2);
        assert_eq!(summary.source_files_with_attachment_links_count, 2);
        assert_eq!(summary.source_files_missing_attachment_links_count, 0);
        assert_eq!(summary.unmaterialized_blake3_facts_count, 0);

        service
            .durable_store
            .record_source_file_observation(RecordSourceFileObservationInput {
                source_file_id: Some(file_ids[1]),
                source_id: registered.root_id,
                parent_source_directory_id: None,
                name: "b.flac".to_string(),
                relative_path: "b.flac".to_string(),
                size_bytes: Some(13),
                mtime_ns: Some(999),
                presence_state: SourcePresenceState::Present,
                first_discovered_at: Some(9_000_000_000_000),
                observed_at: Some(9_000_000_000_000),
                presence_changed_at: 9_000_000_000_000,
                updated_at: 9_000_000_000_000,
            })
            .expect("change source-file basis");
        std::fs::write(source_root.join("b.flac"), b"different now").expect("rewrite b");
        service
            .durable_store
            .hash_source_file_blake3_batch(library_store_sqlite::HashSourceFileBlake3BatchInput {
                scope: SourceFileBlake3HashAdmissionScope::Source {
                    source_id: registered.root_id,
                },
                limit: Some(1),
                observed_at_ms: 50,
            })
            .expect("rehash changed source file without attachment materialization");

        let stale = read_source_file_attachment(&service, file_ids[1]);
        assert_eq!(stale.status, AttachmentIdentityReadStatus::Ok);
        assert_eq!(
            stale
                .attachment_link
                .expect("stale link still exists")
                .link_status,
            SourceFileAttachmentLinkStatus::Stale
        );

        let summary = read_source_attachment_summary(&service, registered.root_id);
        let summary = summary
            .summary
            .expect("source summary exists after stale basis");
        assert_eq!(summary.current_links_count, 1);
        assert_eq!(summary.stale_links_count, 1);
        assert_eq!(summary.source_files_with_current_blake3_facts_count, 1);
        assert_eq!(summary.source_files_with_attachment_links_count, 2);
        assert_eq!(summary.source_files_missing_attachment_links_count, 0);

        service
            .durable_store
            .record_source_file_observation(RecordSourceFileObservationInput {
                source_file_id: Some(999),
                source_id: registered.root_id,
                parent_source_directory_id: None,
                name: "unlinked.flac".to_string(),
                relative_path: "unlinked.flac".to_string(),
                size_bytes: Some(7),
                mtime_ns: Some(7),
                presence_state: SourcePresenceState::Present,
                first_discovered_at: Some(9_000_000_000_100),
                observed_at: Some(9_000_000_000_100),
                presence_changed_at: 9_000_000_000_100,
                updated_at: 9_000_000_000_100,
            })
            .expect("record unlinked source file");
        let unlinked = read_source_file_attachment(&service, 999);
        assert_eq!(unlinked.status, AttachmentIdentityReadStatus::NotFound);
        assert!(unlinked.attachment_link.is_none());

        let missing_attachment = read_attachment_source_files(&service, 99_999, Some(10));
        assert_eq!(
            missing_attachment.status,
            AttachmentIdentityReadStatus::NotFound
        );
        assert!(missing_attachment.attachment.is_none());
        assert!(missing_attachment.source_file_links.is_empty());

        let missing_source = read_source_attachment_summary(&service, 99_999);
        assert_eq!(
            missing_source.status,
            AttachmentIdentityReadStatus::NotFound
        );
        assert!(missing_source.summary.is_none());

        let before_counts = (
            count_rows(&context, "SourceFacts"),
            count_rows(&context, "content_attachments"),
            count_rows(&context, "source_file_attachment_links"),
            count_rows(&context, "LibraryAssets"),
            count_rows(&context, "LibraryAssetAttachments"),
            count_rows(&context, "LibraryBrowserRows"),
            count_rows(&context, "SourceSegmentSets"),
            count_rows(&context, "SourceSegments"),
            service.source_maintenance.completed_runs_for_test().len(),
        );
        let _ = read_source_file_attachment(&service, file_ids[0]);
        let _ = read_attachment_source_files(&service, first_link.attachment_id, Some(10));
        let _ = read_source_attachment_summary(&service, registered.root_id);
        let after_counts = (
            count_rows(&context, "SourceFacts"),
            count_rows(&context, "content_attachments"),
            count_rows(&context, "source_file_attachment_links"),
            count_rows(&context, "LibraryAssets"),
            count_rows(&context, "LibraryAssetAttachments"),
            count_rows(&context, "LibraryBrowserRows"),
            count_rows(&context, "SourceSegmentSets"),
            count_rows(&context, "SourceSegments"),
            service.source_maintenance.completed_runs_for_test().len(),
        );
        assert_eq!(
            after_counts, before_counts,
            "attachment identity reads must not trigger source maintenance, materialization, or old identity rows"
        );
        for absent_or_future_table in [
            "Tracks",
            "TrackRows",
            "LibraryTracks",
            "PrepRows",
            "PreparationRows",
            "primaryMedia",
            "Playlists",
            "PlaylistEntries",
        ] {
            assert!(
                matches!(
                    count_table_if_exists(&context, absent_or_future_table),
                    None | Some(0)
                ),
                "{absent_or_future_table} must be absent or empty after attachment reads"
            );
        }
    }

    #[test]
    fn attachment_identity_snapshot_reads_reject_invalid_ids_and_limits() {
        let (_tempdir, _context, service) = open_service_with_context();

        let invalid_source_file = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadSourceFileAttachment(ReadSourceFileAttachmentRequest {
                    source_file_id: 0,
                }),
            ))
            .expect_err("zero sourceFileId is invalid");
        assert!(matches!(
            invalid_source_file,
            ProtocolError::InvalidRequest { .. }
        ));

        let invalid_attachment = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest {
                    attachment_id: -1,
                    limit: Some(10),
                }),
            ))
            .expect_err("negative attachmentId is invalid");
        assert!(matches!(
            invalid_attachment,
            ProtocolError::InvalidRequest { .. }
        ));

        let invalid_limit = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadAttachmentSourceFiles(ReadAttachmentSourceFilesRequest {
                    attachment_id: 1,
                    limit: Some(0),
                }),
            ))
            .expect_err("zero attachment read limit is invalid");
        assert!(matches!(
            invalid_limit,
            ProtocolError::InvalidRequest { .. }
        ));

        let invalid_source = service
            .try_handle_command(CommandRequest::SnapshotRead(
                SnapshotReadCommand::ReadSourceAttachmentSummary(
                    ReadSourceAttachmentSummaryRequest { source_id: 0 },
                ),
            ))
            .expect_err("zero sourceId is invalid");
        assert!(matches!(
            invalid_source,
            ProtocolError::InvalidRequest { .. }
        ));
    }

    #[test]
    fn duplicate_source_maintenance_requests_dedupe() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("dedupe-hash-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);

        let completed_before = service.source_maintenance.completed_runs_for_test().len();
        assert!(
            service
                .source_maintenance
                .request_source(registered.root_id)
        );
        assert!(
            !service
                .source_maintenance
                .request_source(registered.root_id),
            "duplicate pending source maintenance requests must be deduped"
        );

        let runs = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].source_id, registered.root_id);
        assert_eq!(runs[0].hash.hashed_count, 0);
        assert_eq!(runs[0].remaining_hash_candidates, 0);
        assert_eq!(
            service.source_maintenance.completed_runs_for_test().len(),
            completed_before + 1
        );
    }

    #[test]
    fn active_manual_source_maintenance_returns_skipped_without_work() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("active-manual-maintenance-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.flac", 5);

        service
            .source_maintenance
            .set_source_active_for_test(registered.root_id, true);
        let completed_before = service.source_maintenance.completed_runs_for_test().len();

        let run = run_source_maintenance(
            &service,
            registered.root_id,
            Some(10),
            Some(10),
            Some(10),
            Some(10),
        );

        assert_eq!(
            run.status,
            library_boundary_protocol::SourceMaintenanceRunStatus::Skipped
        );
        assert_eq!(run.hash.hashed_count, 0);
        assert_eq!(run.attachment_materialization.links_created, 0);
        assert_eq!(run.probe.probed_count, 0);
        assert_eq!(count_rows(&context, "SourceFacts"), 0);
        assert_eq!(count_rows(&context, "content_attachments"), 0);
        assert_eq!(
            service.source_maintenance.completed_runs_for_test().len(),
            completed_before,
            "duplicate active manual runs are scheduler skips, not completed maintenance units"
        );

        let snapshot = read_source_maintenance(&service, registered.root_id);
        assert!(
            snapshot.last_run.is_none(),
            "skipped duplicate active runs must not overwrite last_run"
        );
        service
            .source_maintenance
            .set_source_active_for_test(registered.root_id, false);
        assert!(
            service
                .source_maintenance
                .active_source_ids_for_test()
                .is_empty()
        );
    }

    #[test]
    fn manual_source_maintenance_takes_pending_source_without_back_to_back_queue_run() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("manual-pending-maintenance-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.flac", 5);

        assert!(
            service
                .source_maintenance
                .request_source(registered.root_id)
        );
        assert_eq!(
            service.source_maintenance.pending_source_ids_for_test(),
            vec![registered.root_id]
        );

        let manual = run_source_maintenance(
            &service,
            registered.root_id,
            Some(10),
            Some(10),
            Some(10),
            Some(10),
        );
        assert_eq!(manual.hash.hashed_count, 1);

        let queued = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");
        assert!(
            queued.is_empty(),
            "manual ownership must remove the pending entry for the same source"
        );
        assert!(
            service
                .source_maintenance
                .pending_source_ids_for_test()
                .is_empty()
        );
    }

    #[test]
    fn pending_source_maintenance_runs_in_source_id_order() {
        let (_tempdir, _context, service) = open_service_with_context();

        assert!(service.source_maintenance.request_source(30));
        assert!(service.source_maintenance.request_source(10));
        assert!(service.source_maintenance.request_source(20));
        assert!(!service.source_maintenance.request_source(10));
        assert_eq!(
            service.source_maintenance.pending_source_ids_for_test(),
            vec![10, 20, 30]
        );

        let runs = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");
        let run_source_ids = runs.iter().map(|run| run.source_id).collect::<Vec<_>>();
        assert_eq!(run_source_ids, vec![10, 20, 30]);
        assert!(runs.iter().all(|run| matches!(
            run.source_failure,
            Some(library_boundary_protocol::SourceMaintenanceSourceFailure::SourceNotFound)
        )));
    }

    #[test]
    fn stop_clears_pending_source_maintenance_without_stuck_active_sources() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("stop-cleanup-maintenance-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        record_present_source_file(&service, registered.root_id, 100, "track.flac", 5);
        assert!(
            service
                .source_maintenance
                .request_source(registered.root_id)
        );
        service
            .source_maintenance
            .request_stop_before_attachment_materialization_for_test();

        let runs = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");

        assert_eq!(runs.len(), 1);
        assert!(runs[0].stopped);
        assert!(
            service
                .source_maintenance
                .pending_source_ids_for_test()
                .is_empty()
        );
        assert!(
            service
                .source_maintenance
                .active_source_ids_for_test()
                .is_empty()
        );
    }

    #[test]
    fn source_maintenance_observes_stop_before_attachment_materialization() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("stop-before-attachment-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        service
            .durable_store
            .record_source_file_observation(RecordSourceFileObservationInput {
                source_file_id: Some(100),
                source_id: registered.root_id,
                parent_source_directory_id: None,
                name: "track.flac".to_string(),
                relative_path: "track.flac".to_string(),
                size_bytes: Some(5),
                mtime_ns: Some(1),
                presence_state: SourcePresenceState::Present,
                first_discovered_at: Some(10),
                observed_at: Some(10),
                presence_changed_at: 10,
                updated_at: 10,
            })
            .expect("record source file");

        assert!(
            service
                .source_maintenance
                .request_source(registered.root_id)
        );
        service
            .source_maintenance
            .request_stop_before_attachment_materialization_for_test();
        let runs = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].hash.hashed_count, 1);
        assert!(runs[0].stopped);
        assert!(
            runs[0].attachment_materialization.links_created == 0,
            "stop must be observed before starting the attachment materialization unit"
        );
        assert_eq!(count_rows(&context, "SourceFacts"), 1);
        assert_eq!(count_rows(&context, "source_file_attachment_links"), 0);
        assert_eq!(count_rows(&context, "content_attachments"), 0);
    }

    #[test]
    fn blocked_scan_does_not_request_source_maintenance() {
        let (tempdir, _context, service) = open_service_with_context();
        let source_root = tempdir.path().join("blocked-scan-root");
        std::fs::create_dir_all(&source_root).expect("create source root");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        std::fs::remove_dir_all(&source_root).expect("remove source root before scan");

        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_terminal_kind(
            &service,
            registered.root_id,
            library_boundary_protocol::SourceScanEventKind::SourceScanBlocked,
        );

        assert!(
            service
                .source_maintenance
                .completed_runs_for_test()
                .iter()
                .all(|run| run.source_id != registered.root_id),
            "blocked scans must not automatically request source maintenance"
        );
    }

    #[test]
    fn failed_scan_does_not_request_source_maintenance() {
        let (_tempdir, _context, service) = open_service_with_context();
        let missing_source_id = 99_999;
        let scan_run_id = 7;
        let terminal_publication_complete =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        super::execute_scan_job(
            service.durable_store.clone(),
            service.session_events.clone(),
            missing_source_id,
            scan_run_id,
            10,
            terminal_publication_complete,
            service.source_maintenance.clone(),
        );

        let events = read_after_events(&service, None, 64);
        assert!(
            events.events.iter().any(|e| {
                let LibraryBoundaryEvent::SourceScanEvent(event) = e else {
                    return false;
                };
                event.kind == library_boundary_protocol::SourceScanEventKind::SourceScanFailed
                    && event.root_id == missing_source_id
                    && event.scan_run_id == scan_run_id
            }),
            "failed scan must publish a failed terminal event"
        );
        assert!(
            service
                .source_maintenance
                .completed_runs_for_test()
                .iter()
                .all(|run| run.source_id != missing_source_id),
            "failed scans must not automatically request source maintenance"
        );
    }

    #[test]
    fn source_maintenance_reports_source_failures_without_empty_success() {
        let (tempdir, context, service) = open_service_with_context();
        let source_root = tempdir.path().join("blocked-maintenance-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        std::fs::write(source_root.join("track.flac"), b"track").expect("write track");

        let (_json, registered) =
            register_local_root(&service, source_root.to_string_lossy().into_owned());
        let _scan = start_root_scan(&service, registered.root_id);
        wait_for_scan_completed(&service, registered.root_id);
        clear_attachment_identity_rows(&context);

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

        assert!(
            service
                .source_maintenance
                .request_source(registered.root_id)
        );
        let runs = service
            .source_maintenance
            .run_queued(&service.durable_store, &service.session_events)
            .expect("run queued maintenance");

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].hash.hashed_count, 0);
        assert_eq!(runs[0].hash.remaining_candidates, 0);
        assert!(matches!(
            runs[0].source_failure,
            Some(library_boundary_protocol::SourceMaintenanceSourceFailure::SourceRootBlocked(_))
        ));
        assert_eq!(
            count_rows(&context, "source_file_attachment_links"),
            0,
            "blocked source maintenance must not create attachment links"
        );
        assert_eq!(
            count_rows(&context, "content_attachments"),
            0,
            "blocked source maintenance must not materialize attachments"
        );
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
