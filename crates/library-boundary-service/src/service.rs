use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use library_boundary_protocol as protocol;
use library_domain::{LibraryAssetId, PlaylistId};
use library_store_sqlite::{
    AppendLibraryAssetToPlaylistInput, CreatePlaylistInput, DeletePlaylistInput,
    LibraryStoreContext, LocalRootAvailability, MovePlaylistEntryInput, ReadLocalRootsResult,
    RegisterLocalRootInput, RemoveLibraryAssetFromPlaylistInput, RenamePlaylistInput,
    RootScanObservation, SqliteDurableStore,
    UnregisterLocalRootInput,
};

use crate::session_events::LibraryBoundaryEventStream;
use crate::snapshot_read_protocol::{
    map_load_navigation_row_by_stable_key_reply, map_load_navigation_row_reply,
    map_maintained_read_model_revisions, map_read_contents_reply,
    map_read_library_asset_preparation_detail_reply,
    map_read_library_asset_waveform_overview_reply, map_read_library_tree_children_reply,
    map_read_navigation_node_library_browser_window_reply, map_read_navigation_rows_reply,
    map_search_navigation_node_library_browser_window_reply, store_contents_policy,
    store_contents_recursion, store_contents_scope, store_library_tree_entry_point,
};
use crate::storage_environment::resolve_library_storage_environment;

pub struct LibraryBoundaryService {
    durable_store: SqliteDurableStore,
    session_events: LibraryBoundaryEventStream,
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
            protocol::CommandRequest::SnapshotRead(command) => self
                .handle_snapshot_read_command(command)
                .map(protocol::CommandReply::SnapshotRead),
        }
    }

    pub fn read_library_boundary_events(
        &self,
        request: protocol::ReadLibraryBoundaryEventsRequest,
    ) -> protocol::ProtocolResult<protocol::ReadLibraryBoundaryEventsReply> {
        if request.max_events == 0 {
            return Err(protocol::ProtocolError::InvalidRequest {
                detail: "libraryBoundaryEvents.readPending maxEvents must be greater than zero"
                    .to_string(),
            });
        }

        self.publish_maintained_snapshot_invalidations()?;
        Ok(protocol::ReadLibraryBoundaryEventsReply {
            events: self.session_events.drain(request.max_events),
        })
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
        let (events, latest_event_sequence) = self
            .session_events
            .read_after(request.last_seen_event_sequence, request.max_events);
        Ok(protocol::ReadLibraryBoundaryEventsAfterReply {
            events,
            latest_event_sequence,
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

    pub fn run_root_scan(
        &self,
        request: protocol::RunRootScanRequest,
    ) -> protocol::ProtocolResult<protocol::RunRootScanReply> {
        let root_id = require_positive_i64(request.root_id, "rootId")?;
        let scan_started_at_ms = unix_time_ms()?;

        self.publish_scan_started(root_id, scan_started_at_ms);

        let event_stream = &self.session_events;
        let result = self.durable_store.run_root_scan_with_observer(
            root_id,
            scan_started_at_ms,
            |observation| match observation {
                RootScanObservation::HierarchyPublished {
                    root_id: obs_root_id,
                    scan_run_id,
                    reason: _reason,
                } => {
                    event_stream.publish_scan_event(
                        library_boundary_protocol::SourceScanEventKind::SourceScanProgressed,
                        obs_root_id,
                        scan_run_id,
                        library_boundary_protocol::ScanRunPhase::Scanning,
                        0,
                        0,
                        0,
                        0,
                        0,
                        None,
                    );
                }
                RootScanObservation::SourceWorkQueued { .. } => {}
            },
        );

        match result {
            Ok(scan) => {
                self.publish_scan_completed(
                    root_id,
                    scan.scan_run_id,
                    scan.discovered_file_count,
                    scan.queued_source_work_items,
                );
                self.publish_maintained_snapshot_invalidations()?;
                Ok(protocol::RunRootScanReply {
                    root_id,
                    scan_run_id: scan.scan_run_id,
                    discovered_file_count: scan.discovered_file_count,
                    queued_source_work_items: scan.queued_source_work_items,
                })
            }
            Err(error) => {
                self.publish_scan_failed_or_blocked(root_id, &error);
                Err(map_store_error(error))
            }
        }
    }

    fn publish_scan_started(&self, root_id: i64, _scan_started_at_ms: i64) {
        self.session_events.publish_scan_event(
            library_boundary_protocol::SourceScanEventKind::SourceScanStarted,
            root_id,
            0,
            library_boundary_protocol::ScanRunPhase::Scanning,
            0,
            0,
            0,
            0,
            0,
            None,
        );
    }

    fn publish_scan_completed(
        &self,
        root_id: i64,
        scan_run_id: i64,
        discovered_file_count: usize,
        queued_source_work_items: usize,
    ) {
        self.session_events.publish_scan_event(
            library_boundary_protocol::SourceScanEventKind::SourceScanCompleted,
            root_id,
            scan_run_id,
            library_boundary_protocol::ScanRunPhase::Scanning,
            0,
            discovered_file_count,
            discovered_file_count,
            discovered_file_count,
            queued_source_work_items,
            None,
        );
    }

    fn publish_scan_failed_or_blocked(
        &self,
        root_id: i64,
        error: &library_store_sqlite::LibrarySqliteError,
    ) {
        use library_store_sqlite::{CanonicalErrorCode, LibrarySqliteError};

        let detail = error.to_string();
        let is_blocked = match error {
            LibrarySqliteError::Canonical(canonical) => {
                matches!(canonical.code, CanonicalErrorCode::NotFound)
            }
            LibrarySqliteError::RootWorkCancelled { .. } => true,
            _ => false,
        };

        if is_blocked {
            self.session_events.publish_scan_event(
                library_boundary_protocol::SourceScanEventKind::SourceScanBlocked,
                root_id,
                0,
                library_boundary_protocol::ScanRunPhase::Blocked,
                0,
                0,
                0,
                0,
                0,
                Some(detail),
            );
        } else {
            self.session_events.publish_scan_event(
                library_boundary_protocol::SourceScanEventKind::SourceScanFailed,
                root_id,
                0,
                library_boundary_protocol::ScanRunPhase::Scanning,
                0,
                0,
                0,
                0,
                0,
                Some(detail),
            );
        }
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

    fn handle_library_boundary_event_command(
        &self,
        command: protocol::LibraryBoundaryEventStreamCommand,
    ) -> protocol::ProtocolResult<protocol::LibraryBoundaryEventStreamReply> {
        match command {
            protocol::LibraryBoundaryEventStreamCommand::ReadPending(request) => self
                .read_library_boundary_events(request)
                .map(protocol::LibraryBoundaryEventStreamReply::ReadPending),
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
            protocol::LibraryRootCommand::RunRootScan(request) => self
                .run_root_scan(request)
                .map(protocol::LibraryRootReply::RunRootScan),
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
        CommandOutcome, CommandReply, CommandRequest, ContentsReadPolicy, ContentsRowProfile,
        CreatePlaylistReply, CreatePlaylistRequest, DeletePlaylistReply, DeletePlaylistRequest,
        DirectoryImageMediaState, DirectoryPrimaryMediaState, DirectoryScanState,
        LibraryBoundaryEvent, LibraryBoundaryEventStreamCommand, LibraryBoundaryEventStreamReply,
        LibraryRootCommand, LibraryRootReply, LibraryTreeEntryPoint, LibraryTreeNodeKind,
        LibraryTreePresenceState, LoadNavigationRowByStableKeyReply,
        LoadNavigationRowByStableKeyRequest, MaintainedSnapshotScope, PlaylistWriteCommand,
        PlaylistWriteReply, ProtocolError, ReadLibraryBoundaryEventsReply,
        ReadLibraryBoundaryEventsRequest, ReadLibraryTreeChildrenRequest,
        RegisterLocalRootReply, RegisterLocalRootRequest, RenamePlaylistReply,
        RenamePlaylistRequest, RunRootScanReply, RunRootScanRequest, SnapshotReadCommand,
        SnapshotReadReply, UnregisterLocalRootReply, UnregisterLocalRootRequest,
    };
    use serde_json::json;
    use tempfile::TempDir;

    use library_store_sqlite::{LibraryStoreContext, StoreEnvironment, durable_store_path};

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

    fn expect_run_root_scan_reply(reply: CommandReply) -> RunRootScanReply {
        match reply {
            CommandReply::LibraryRoots(LibraryRootReply::RunRootScan(reply)) => reply,
            other => panic!("expected run root scan reply, got {other:?}"),
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

    fn expect_event_stream_read_pending_reply(
        reply: CommandReply,
    ) -> ReadLibraryBoundaryEventsReply {
        match reply {
            CommandReply::LibraryBoundaryEvents(LibraryBoundaryEventStreamReply::ReadPending(
                reply,
            )) => reply,
            other => panic!("expected event stream readPending reply, got {other:?}"),
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

    fn run_root_scan(service: &LibraryBoundaryService, root_id: i64) -> RunRootScanReply {
        expect_run_root_scan_reply(expect_success(service.handle_command(
            CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(RunRootScanRequest {
                root_id,
            })),
        )))
    }

    fn read_pending_events(
        service: &LibraryBoundaryService,
        max_events: usize,
    ) -> ReadLibraryBoundaryEventsReply {
        expect_event_stream_read_pending_reply(expect_success(service.handle_command(
            CommandRequest::LibraryBoundaryEvents(LibraryBoundaryEventStreamCommand::ReadPending(
                ReadLibraryBoundaryEventsRequest { max_events },
            )),
        )))
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

    fn assert_maintained_invalidation_for_scope(
        event: &LibraryBoundaryEvent,
        expected_scope: MaintainedSnapshotScope,
    ) {
        let LibraryBoundaryEvent::MaintainedSnapshotInvalidated(payload) = event else {
            panic!("expected maintained snapshot invalidated event");
        };
        assert!(payload.event_sequence >= 0);
        assert!(payload.occurred_at_ms > 0);
        assert_eq!(payload.invalidation.scope, expected_scope);
        assert!(
            payload
                .invalidation
                .revision
                .map(|revision| revision.value() > 0)
                .unwrap_or(false),
            "expected positive maintained snapshot revision, got {:?}",
            payload.invalidation.revision
        );
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

        let scan = run_root_scan(&service, registered.root_id);
        assert_eq!(scan.root_id, registered.root_id);
        assert!(scan.scan_run_id > 0);
        assert_eq!(scan.discovered_file_count, 1);
        assert_eq!(scan.queued_source_work_items, 1);

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
        assert_eq!(
            crate_row.presence_state,
            LibraryTreePresenceState::Present
        );
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
    fn protocol_commands_read_and_drain_real_snapshot_invalidation_events() {
        let (_tempdir, _context, service) = open_service_with_context();
        let initially_empty = read_pending_events(&service, 1);
        assert!(initially_empty.events.is_empty());

        let (_create_json, created) = create_playlist(&service, "Event Test");
        let deleted = delete_playlist(&service, created.playlist_id);
        assert!(deleted.deleted);

        let first_drain = read_pending_events(&service, 1);
        assert_eq!(first_drain.events.len(), 1);
        assert_maintained_invalidation_for_scope(
            &first_drain.events[0],
            MaintainedSnapshotScope::NavigationRows,
        );

        let second_drain = read_pending_events(&service, 1);
        assert_eq!(second_drain.events.len(), 1);
        assert_maintained_invalidation_for_scope(
            &second_drain.events[0],
            MaintainedSnapshotScope::LibraryBrowser,
        );

        let drained_again = read_pending_events(&service, 1);
        assert!(
            drained_again.events.is_empty(),
            "drained events must not replay forever"
        );

        let error = service
            .try_handle_command(CommandRequest::LibraryBoundaryEvents(
                LibraryBoundaryEventStreamCommand::ReadPending(ReadLibraryBoundaryEventsRequest {
                    max_events: 0,
                }),
            ))
            .expect_err("zero maxEvents is invalid");
        assert!(matches!(error, ProtocolError::InvalidRequest { .. }));
        assert_eq!(error.code(), "INVALID_REQUEST");
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
            CommandRequest::LibraryRoots(LibraryRootCommand::RunRootScan(RunRootScanRequest {
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
}
