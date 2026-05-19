use crate::LibrarySqliteResult;
use crate::authority::playlists::{
    AppendLibraryAssetToPlaylistInput, CreatePlaylistInput, DeletePlaylistInput,
    MovePlaylistEntryInput, PlaylistsAuthorityTx, RemoveLibraryAssetFromPlaylistInput,
    RenamePlaylistInput,
};
use crate::publication;
use library_domain::{PlaylistId, ProjectionDomain};

use super::SqliteDurableStore;

const PLAYLIST_BROWSER_INVALIDATION_REASON: &str = "playlist_entries";

impl SqliteDurableStore {
    pub fn create_playlist(&self, input: CreatePlaylistInput) -> LibrarySqliteResult<PlaylistId> {
        self.with_write(|write| {
            let playlist_id = PlaylistsAuthorityTx::new(write).create_playlist(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(playlist_id)
        })
    }

    pub fn rename_playlist(&self, input: RenamePlaylistInput) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let renamed = PlaylistsAuthorityTx::new(write).rename_playlist(&input)?;
            if renamed {
                publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            }
            Ok(renamed)
        })
    }

    pub fn delete_playlist(&self, input: DeletePlaylistInput) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let deleted = PlaylistsAuthorityTx::new(write).delete_playlist(&input)?;
            if deleted {
                publication::reseed_projection_domains(
                    write,
                    &[
                        ProjectionDomain::Navigation,
                        ProjectionDomain::LibraryBrowser,
                    ],
                )?;
                publication::invalidate_projection_domain(
                    write,
                    ProjectionDomain::LibraryBrowser,
                    PLAYLIST_BROWSER_INVALIDATION_REASON,
                )?;
            }
            Ok(deleted)
        })
    }

    pub fn append_library_asset_to_playlist(
        &self,
        input: AppendLibraryAssetToPlaylistInput,
    ) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            let playlist_entry_id =
                PlaylistsAuthorityTx::new(write).append_library_asset_to_playlist(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            publication::invalidate_projection_domain(
                write,
                ProjectionDomain::LibraryBrowser,
                PLAYLIST_BROWSER_INVALIDATION_REASON,
            )?;
            Ok(playlist_entry_id)
        })
    }

    pub fn remove_library_asset_from_playlist(
        &self,
        input: RemoveLibraryAssetFromPlaylistInput,
    ) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let removed =
                PlaylistsAuthorityTx::new(write).remove_library_asset_from_playlist(&input)?;
            if removed {
                publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
                publication::invalidate_projection_domain(
                    write,
                    ProjectionDomain::LibraryBrowser,
                    PLAYLIST_BROWSER_INVALIDATION_REASON,
                )?;
            }
            Ok(removed)
        })
    }

    pub fn move_playlist_entry(&self, input: MovePlaylistEntryInput) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let moved = PlaylistsAuthorityTx::new(write).move_playlist_entry(&input)?;
            if moved {
                publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
                publication::invalidate_projection_domain(
                    write,
                    ProjectionDomain::LibraryBrowser,
                    PLAYLIST_BROWSER_INVALIDATION_REASON,
                )?;
            }
            Ok(moved)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AppendLibraryAssetToPlaylistInput, ApplyLibraryAssetMetadataCorrectionInput,
        CreatePlaylistInput, DeletePlaylistInput, LibrarySqliteError, MaintainedReadModelScope,
        MovePlaylistEntryInput, RemoveLibraryAssetFromPlaylistInput, RenamePlaylistInput,
    };
    use library_domain::{LibraryAssetId, NavigationSelector, PlaylistId, encode_selector};
    use tempfile::TempDir;

    fn open_store() -> (TempDir, SqliteDurableStore) {
        let tempdir = TempDir::new().expect("create tempdir");
        let store = SqliteDurableStore::open(tempdir.path().join("library.sqlite3"))
            .expect("open durable store");
        (tempdir, store)
    }

    fn insert_library_asset(store: &SqliteDurableStore, library_asset_id: i64) {
        store
            .with_write(|write| {
                write.execute(
                    "INSERT INTO LibraryAssets (
                         library_asset_id,
                         equivalence_fingerprint,
                         retention_policy,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, 'keep_metadata', 1, 1)",
                    rusqlite::params![
                        library_asset_id,
                        format!("eq:playlist-test:{library_asset_id}"),
                    ],
                )?;
                Ok(())
            })
            .expect("insert library asset");
    }

    fn apply_title_correction(
        store: &SqliteDurableStore,
        library_asset_id: i64,
        title: &str,
        applied_at: i64,
    ) {
        store
            .apply_library_asset_metadata_correction(ApplyLibraryAssetMetadataCorrectionInput {
                library_asset_id: LibraryAssetId::new(library_asset_id).expect("positive id"),
                field_name: "title".to_string(),
                value_text: Some(title.to_string()),
                value_int: None,
                is_null_correction: false,
                source_kind: "test".to_string(),
                applied_at,
            })
            .expect("apply title correction");
    }

    fn maintained_revision(store: &SqliteDurableStore, scope: MaintainedReadModelScope) -> u64 {
        store
            .read_maintained_read_model_revisions()
            .expect("read maintained revisions")
            .into_iter()
            .find(|revision| revision.scope == scope)
            .expect("maintained revision scope exists")
            .revision
    }

    fn assert_revision_advanced(scope: MaintainedReadModelScope, before: u64, after: u64) {
        assert!(
            after > before,
            "expected {scope:?} revision to advance, before={before}, after={after}"
        );
    }

    fn assert_revision_unchanged(scope: MaintainedReadModelScope, before: u64, after: u64) {
        assert_eq!(
            after, before,
            "expected {scope:?} revision to remain unchanged"
        );
    }

    fn playlist_navigation_row_id(store: &SqliteDurableStore, playlist_id: PlaylistId) -> i64 {
        store
            .load_navigation_row_by_stable_key(&format!("playlist:{}", playlist_id.get()))
            .expect("load playlist navigation row")
            .expect("playlist navigation row exists")
            .navigation_row_id
    }

    fn read_playlist_browser_asset_ids(
        store: &SqliteDurableStore,
        playlist_id: PlaylistId,
    ) -> Vec<i64> {
        let navigation_row_id = playlist_navigation_row_id(store, playlist_id);
        store
            .read_navigation_node_library_browser_window(navigation_row_id, 0, 100)
            .expect("read playlist browser window")
            .expect("playlist navigation row resolves to browser scope")
            .rows
            .into_iter()
            .map(|row| row.library_asset_id)
            .collect()
    }

    fn search_playlist_browser_asset_ids(
        store: &SqliteDurableStore,
        playlist_id: PlaylistId,
        query: &str,
    ) -> Vec<i64> {
        let navigation_row_id = playlist_navigation_row_id(store, playlist_id);
        store
            .search_navigation_node_library_browser_window(navigation_row_id, query, 0, 100)
            .expect("search playlist browser window")
            .expect("playlist navigation row resolves to browser scope")
            .rows
            .into_iter()
            .map(|row| row.library_asset_id)
            .collect()
    }

    fn playlist_entries(
        store: &SqliteDurableStore,
        playlist_id: PlaylistId,
    ) -> Vec<(i64, i64, i64)> {
        let connection = store.open_read_connection().expect("open read connection");
        let mut statement = connection
            .prepare(
                "SELECT playlist_entry_id, library_asset_id, position
                 FROM PlaylistEntries
                 WHERE playlist_id = ?1
                 ORDER BY position ASC, playlist_entry_id ASC",
            )
            .expect("prepare playlist entries query");
        statement
            .query_map([playlist_id.get()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .expect("query playlist entries")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect playlist entries")
    }

    #[test]
    fn store_can_create_rename_and_delete_playlists_with_navigation_rows() {
        let (_tempdir, store) = open_store();
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: "Warmups".to_string(),
                created_at: 10,
            })
            .expect("create playlist");

        let playlists_family = store
            .load_navigation_row_by_stable_key("collection-group:playlists")
            .expect("load playlists group")
            .expect("playlists group exists");
        assert_eq!(playlists_family.family.as_deref(), Some("Collections"));
        assert_eq!(playlists_family.row_kind, "collection-group");
        let expected_group_selector = encode_selector(&NavigationSelector::PlaylistGroup);
        assert!(playlists_family.selectable);
        assert_eq!(
            playlists_family.selector_kind.as_deref(),
            Some(expected_group_selector.kind)
        );
        assert_eq!(
            playlists_family.selector_payload.as_deref(),
            Some(expected_group_selector.payload.as_str())
        );

        let children = store
            .read_navigation_rows(Some(playlists_family.navigation_row_id))
            .expect("read playlist navigation children");
        let playlist_row = children
            .iter()
            .find(|row| row.stable_key == format!("playlist:{}", playlist_id.get()))
            .expect("playlist child row exists");
        let expected_selector = encode_selector(&NavigationSelector::Playlist(playlist_id));
        assert!(playlist_row.selectable);
        assert_eq!(playlist_row.row_kind, "playlist");
        assert_eq!(playlist_row.display_name, "Warmups");
        assert_eq!(
            playlist_row.selector_kind.as_deref(),
            Some(expected_selector.kind)
        );
        assert_eq!(
            playlist_row.selector_payload.as_deref(),
            Some(expected_selector.payload.as_str())
        );

        assert!(
            store
                .rename_playlist(RenamePlaylistInput {
                    playlist_id,
                    display_name: "Peak Hour".to_string(),
                    renamed_at: 20,
                })
                .expect("rename playlist")
        );
        let renamed_row = store
            .load_navigation_row_by_stable_key(&format!("playlist:{}", playlist_id.get()))
            .expect("load renamed playlist row")
            .expect("renamed playlist row exists");
        assert_eq!(renamed_row.display_name, "Peak Hour");

        assert!(
            store
                .delete_playlist(DeletePlaylistInput { playlist_id })
                .expect("delete playlist")
        );
        assert!(
            store
                .load_navigation_row_by_stable_key(&format!("playlist:{}", playlist_id.get()))
                .expect("load deleted playlist row")
                .is_none()
        );
    }

    #[test]
    fn store_appends_removes_and_reorders_playlist_entries() {
        let (_tempdir, store) = open_store();
        for library_asset_id in [1, 2, 3] {
            insert_library_asset(&store, library_asset_id);
        }
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: "Set".to_string(),
                created_at: 10,
            })
            .expect("create playlist");

        let first_entry_id = store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(1).expect("positive id"),
                appended_at: 11,
            })
            .expect("append first asset");
        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                appended_at: 12,
            })
            .expect("append second asset");
        let third_entry_id = store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(3).expect("positive id"),
                appended_at: 13,
            })
            .expect("append third asset");

        assert_eq!(
            playlist_entries(&store, playlist_id)
                .iter()
                .map(|(_, asset_id, position)| (*asset_id, *position))
                .collect::<Vec<_>>(),
            vec![(1, 0), (2, 1), (3, 2)]
        );

        assert!(
            store
                .move_playlist_entry(MovePlaylistEntryInput {
                    playlist_id,
                    playlist_entry_id: third_entry_id,
                    new_position: 0,
                    moved_at: 20,
                })
                .expect("move third entry to front")
        );
        assert_eq!(
            playlist_entries(&store, playlist_id)
                .iter()
                .map(|(_, asset_id, position)| (*asset_id, *position))
                .collect::<Vec<_>>(),
            vec![(3, 0), (1, 1), (2, 2)]
        );

        assert!(
            store
                .move_playlist_entry(MovePlaylistEntryInput {
                    playlist_id,
                    playlist_entry_id: first_entry_id,
                    new_position: 9,
                    moved_at: 21,
                })
                .expect("move first entry past end")
        );
        assert_eq!(
            playlist_entries(&store, playlist_id)
                .iter()
                .map(|(_, asset_id, position)| (*asset_id, *position))
                .collect::<Vec<_>>(),
            vec![(3, 0), (2, 1), (1, 2)]
        );

        assert!(
            store
                .remove_library_asset_from_playlist(RemoveLibraryAssetFromPlaylistInput {
                    playlist_id,
                    library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                    removed_at: 30,
                })
                .expect("remove playlist asset")
        );
        assert_eq!(
            playlist_entries(&store, playlist_id)
                .iter()
                .map(|(_, asset_id, position)| (*asset_id, *position))
                .collect::<Vec<_>>(),
            vec![(3, 0), (1, 1)]
        );

        assert!(
            store
                .delete_playlist(DeletePlaylistInput { playlist_id })
                .expect("delete playlist")
        );
        assert!(playlist_entries(&store, playlist_id).is_empty());
    }

    #[test]
    fn playlist_writes_advance_exact_maintained_read_model_revisions() {
        let (_tempdir, store) = open_store();
        for library_asset_id in [1, 2, 3] {
            insert_library_asset(&store, library_asset_id);
        }
        store
            .reseed_current_projection_state()
            .expect("seed browser rows for library assets");

        let navigation_before_create =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_before_create =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: "Revision Set".to_string(),
                created_at: 10,
            })
            .expect("create playlist");
        let navigation_after_create =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_create =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_advanced(
            MaintainedReadModelScope::NavigationRows,
            navigation_before_create,
            navigation_after_create,
        );
        assert_revision_unchanged(
            MaintainedReadModelScope::LibraryBrowser,
            browser_before_create,
            browser_after_create,
        );

        assert!(
            store
                .rename_playlist(RenamePlaylistInput {
                    playlist_id,
                    display_name: "Renamed Revision Set".to_string(),
                    renamed_at: 20,
                })
                .expect("rename playlist")
        );
        let navigation_after_rename =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_rename =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_advanced(
            MaintainedReadModelScope::NavigationRows,
            navigation_after_create,
            navigation_after_rename,
        );
        assert_revision_unchanged(
            MaintainedReadModelScope::LibraryBrowser,
            browser_after_create,
            browser_after_rename,
        );

        let first_entry_id = store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(1).expect("positive id"),
                appended_at: 30,
            })
            .expect("append first asset");
        let navigation_after_append =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_append =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_unchanged(
            MaintainedReadModelScope::NavigationRows,
            navigation_after_rename,
            navigation_after_append,
        );
        assert_revision_advanced(
            MaintainedReadModelScope::LibraryBrowser,
            browser_after_rename,
            browser_after_append,
        );

        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                appended_at: 31,
            })
            .expect("append second asset");
        let third_entry_id = store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(3).expect("positive id"),
                appended_at: 32,
            })
            .expect("append third asset");
        let browser_before_move =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert!(
            store
                .move_playlist_entry(MovePlaylistEntryInput {
                    playlist_id,
                    playlist_entry_id: third_entry_id,
                    new_position: 0,
                    moved_at: 40,
                })
                .expect("move playlist entry")
        );
        let navigation_after_move =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_move =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_unchanged(
            MaintainedReadModelScope::NavigationRows,
            navigation_after_append,
            navigation_after_move,
        );
        assert_revision_advanced(
            MaintainedReadModelScope::LibraryBrowser,
            browser_before_move,
            browser_after_move,
        );

        assert!(
            store
                .remove_library_asset_from_playlist(RemoveLibraryAssetFromPlaylistInput {
                    playlist_id,
                    library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                    removed_at: 50,
                })
                .expect("remove playlist asset")
        );
        let navigation_after_remove =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_remove =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_unchanged(
            MaintainedReadModelScope::NavigationRows,
            navigation_after_move,
            navigation_after_remove,
        );
        assert_revision_advanced(
            MaintainedReadModelScope::LibraryBrowser,
            browser_after_move,
            browser_after_remove,
        );

        let browser_before_second_move =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert!(
            store
                .move_playlist_entry(MovePlaylistEntryInput {
                    playlist_id,
                    playlist_entry_id: first_entry_id,
                    new_position: 0,
                    moved_at: 60,
                })
                .expect("move first entry after removal")
        );
        let browser_after_second_move =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_advanced(
            MaintainedReadModelScope::LibraryBrowser,
            browser_before_second_move,
            browser_after_second_move,
        );

        let navigation_before_delete =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_before_delete =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert!(
            store
                .delete_playlist(DeletePlaylistInput { playlist_id })
                .expect("delete playlist")
        );
        let navigation_after_delete =
            maintained_revision(&store, MaintainedReadModelScope::NavigationRows);
        let browser_after_delete =
            maintained_revision(&store, MaintainedReadModelScope::LibraryBrowser);
        assert_revision_advanced(
            MaintainedReadModelScope::NavigationRows,
            navigation_before_delete,
            navigation_after_delete,
        );
        assert_revision_advanced(
            MaintainedReadModelScope::LibraryBrowser,
            browser_before_delete,
            browser_after_delete,
        );
    }

    #[test]
    fn playlist_browser_read_ordering_tracks_move_writes() {
        let (_tempdir, store) = open_store();
        for library_asset_id in [1, 2, 3] {
            insert_library_asset(&store, library_asset_id);
        }
        store
            .reseed_current_projection_state()
            .expect("seed browser rows for library assets");
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: "Move Read".to_string(),
                created_at: 10,
            })
            .expect("create playlist");

        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(1).expect("positive id"),
                appended_at: 11,
            })
            .expect("append first asset");
        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                appended_at: 12,
            })
            .expect("append second asset");
        let third_entry_id = store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(3).expect("positive id"),
                appended_at: 13,
            })
            .expect("append third asset");

        assert_eq!(
            read_playlist_browser_asset_ids(&store, playlist_id),
            vec![1, 2, 3]
        );

        assert!(
            store
                .move_playlist_entry(MovePlaylistEntryInput {
                    playlist_id,
                    playlist_entry_id: third_entry_id,
                    new_position: 0,
                    moved_at: 20,
                })
                .expect("move third entry to front")
        );

        assert_eq!(
            read_playlist_browser_asset_ids(&store, playlist_id),
            vec![3, 1, 2]
        );
    }

    #[test]
    fn playlist_scoped_search_tracks_membership_changes() {
        let (_tempdir, store) = open_store();
        for library_asset_id in [1, 2] {
            insert_library_asset(&store, library_asset_id);
        }
        apply_title_correction(&store, 1, "Alpha Break", 10);
        apply_title_correction(&store, 2, "Beta Roll", 11);
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: None,
                display_name: "Search Set".to_string(),
                created_at: 20,
            })
            .expect("create playlist");

        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(1).expect("positive id"),
                appended_at: 21,
            })
            .expect("append alpha asset");
        store
            .append_library_asset_to_playlist(AppendLibraryAssetToPlaylistInput {
                playlist_id,
                library_asset_id: LibraryAssetId::new(2).expect("positive id"),
                appended_at: 22,
            })
            .expect("append beta asset");

        assert_eq!(
            search_playlist_browser_asset_ids(&store, playlist_id, "Alpha"),
            vec![1]
        );
        assert_eq!(
            search_playlist_browser_asset_ids(&store, playlist_id, "Beta"),
            vec![2]
        );

        assert!(
            store
                .remove_library_asset_from_playlist(RemoveLibraryAssetFromPlaylistInput {
                    playlist_id,
                    library_asset_id: LibraryAssetId::new(1).expect("positive id"),
                    removed_at: 30,
                })
                .expect("remove alpha asset")
        );

        assert!(search_playlist_browser_asset_ids(&store, playlist_id, "Alpha").is_empty());
        assert_eq!(
            search_playlist_browser_asset_ids(&store, playlist_id, "Beta"),
            vec![2]
        );
    }

    #[test]
    fn store_rejects_duplicate_playlist_membership() {
        let (_tempdir, store) = open_store();
        insert_library_asset(&store, 1);
        let playlist_id = store
            .create_playlist(CreatePlaylistInput {
                playlist_id: Some(PlaylistId::new(10).expect("positive id")),
                display_name: "Duplicates".to_string(),
                created_at: 10,
            })
            .expect("create playlist");

        let input = AppendLibraryAssetToPlaylistInput {
            playlist_id,
            library_asset_id: LibraryAssetId::new(1).expect("positive id"),
            appended_at: 11,
        };
        store
            .append_library_asset_to_playlist(input)
            .expect("append playlist asset once");
        let error = store
            .append_library_asset_to_playlist(input)
            .expect_err("duplicate playlist membership is rejected");

        match error {
            LibrarySqliteError::WriteInvariant(detail) => {
                assert!(detail.contains("already contains library asset"));
            }
            other => panic!("expected WriteInvariant, got {other:?}"),
        }
    }
}
