use rusqlite::{OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{LibraryAssetId, PlaylistId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePlaylistInput {
    pub playlist_id: Option<PlaylistId>,
    pub display_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlaylistInput {
    pub playlist_id: PlaylistId,
    pub display_name: String,
    pub renamed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletePlaylistInput {
    pub playlist_id: PlaylistId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendLibraryAssetToPlaylistInput {
    pub playlist_id: PlaylistId,
    pub library_asset_id: LibraryAssetId,
    pub appended_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveLibraryAssetFromPlaylistInput {
    pub playlist_id: PlaylistId,
    pub library_asset_id: LibraryAssetId,
    pub removed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovePlaylistEntryInput {
    pub playlist_id: PlaylistId,
    pub playlist_entry_id: i64,
    pub new_position: i64,
    pub moved_at: i64,
}

pub struct PlaylistsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> PlaylistsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn create_playlist(&self, input: &CreatePlaylistInput) -> LibrarySqliteResult<PlaylistId> {
        validate_display_name(&input.display_name)?;

        match input.playlist_id {
            Some(playlist_id) => {
                self.tx.execute(
                    "INSERT INTO Playlists (
                         playlist_id,
                         display_name,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?3)",
                    params![playlist_id.get(), input.display_name, input.created_at],
                )?;
                Ok(playlist_id)
            }
            None => {
                self.tx.execute(
                    "INSERT INTO Playlists (
                         display_name,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?2)",
                    params![input.display_name, input.created_at],
                )?;
                PlaylistId::new(self.tx.last_insert_rowid()).ok_or_else(|| {
                    LibrarySqliteError::WriteInvariant(format!(
                        "inserted Playlists.playlist_id is not a domain id: {}",
                        self.tx.last_insert_rowid()
                    ))
                })
            }
        }
    }

    pub fn rename_playlist(&self, input: &RenamePlaylistInput) -> LibrarySqliteResult<bool> {
        validate_display_name(&input.display_name)?;

        let updated = self.tx.execute(
            "UPDATE Playlists
             SET display_name = ?2,
                 updated_at = ?3
             WHERE playlist_id = ?1",
            params![
                input.playlist_id.get(),
                input.display_name,
                input.renamed_at,
            ],
        )?;
        Ok(updated != 0)
    }

    pub fn delete_playlist(&self, input: &DeletePlaylistInput) -> LibrarySqliteResult<bool> {
        let deleted = self.tx.execute(
            "DELETE FROM Playlists
             WHERE playlist_id = ?1",
            [input.playlist_id.get()],
        )?;
        Ok(deleted != 0)
    }

    pub fn append_library_asset_to_playlist(
        &self,
        input: &AppendLibraryAssetToPlaylistInput,
    ) -> LibrarySqliteResult<i64> {
        reject_duplicate_membership(self.tx, input.playlist_id, input.library_asset_id)?;
        let next_position = next_playlist_position(self.tx, input.playlist_id)?;
        self.tx.execute(
            "INSERT INTO PlaylistEntries (
                 playlist_id,
                 library_asset_id,
                 position,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                input.playlist_id.get(),
                input.library_asset_id.get(),
                next_position,
                input.appended_at,
            ],
        )?;
        Ok(self.tx.last_insert_rowid())
    }

    pub fn remove_library_asset_from_playlist(
        &self,
        input: &RemoveLibraryAssetFromPlaylistInput,
    ) -> LibrarySqliteResult<bool> {
        let Some((playlist_entry_id, removed_position, max_position)) = self
            .tx
            .query_row(
                "SELECT pe.playlist_entry_id,
                        pe.position,
                        (
                            SELECT MAX(position)
                            FROM PlaylistEntries
                            WHERE playlist_id = pe.playlist_id
                        )
                 FROM PlaylistEntries pe
                 WHERE pe.playlist_id = ?1
                   AND pe.library_asset_id = ?2",
                params![input.playlist_id.get(), input.library_asset_id.get()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(false);
        };

        self.tx.execute(
            "DELETE FROM PlaylistEntries
             WHERE playlist_entry_id = ?1",
            [playlist_entry_id],
        )?;
        shift_entries_after_removal(
            self.tx,
            input.playlist_id,
            removed_position,
            max_position,
            input.removed_at,
        )?;
        Ok(true)
    }

    pub fn move_playlist_entry(&self, input: &MovePlaylistEntryInput) -> LibrarySqliteResult<bool> {
        if input.new_position < 0 {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "playlist entry target position must be non-negative, found {}",
                input.new_position
            )));
        }

        let Some((current_position, entry_count, max_position)) = self
            .tx
            .query_row(
                "SELECT pe.position,
                        (
                            SELECT COUNT(*)
                            FROM PlaylistEntries
                            WHERE playlist_id = pe.playlist_id
                        ),
                        (
                            SELECT MAX(position)
                            FROM PlaylistEntries
                            WHERE playlist_id = pe.playlist_id
                        )
                 FROM PlaylistEntries pe
                 WHERE pe.playlist_id = ?1
                   AND pe.playlist_entry_id = ?2",
                params![input.playlist_id.get(), input.playlist_entry_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
        else {
            return Ok(false);
        };

        let target_position = input.new_position.min(entry_count - 1);
        if current_position == target_position {
            self.tx.execute(
                "UPDATE PlaylistEntries
                 SET updated_at = ?3
                 WHERE playlist_id = ?1
                   AND playlist_entry_id = ?2",
                params![
                    input.playlist_id.get(),
                    input.playlist_entry_id,
                    input.moved_at
                ],
            )?;
            return Ok(true);
        }

        let parking_position = max_position + 1;
        let shift_offset = max_position + entry_count + 2;
        self.tx.execute(
            "UPDATE PlaylistEntries
             SET position = ?3,
                 updated_at = ?4
             WHERE playlist_id = ?1
               AND playlist_entry_id = ?2",
            params![
                input.playlist_id.get(),
                input.playlist_entry_id,
                parking_position,
                input.moved_at,
            ],
        )?;

        if current_position < target_position {
            park_entries_in_position_range(
                self.tx,
                input.playlist_id,
                current_position + 1,
                target_position,
                shift_offset,
                input.moved_at,
            )?;
            self.tx.execute(
                "UPDATE PlaylistEntries
                 SET position = position - ?4 - 1,
                     updated_at = ?5
                 WHERE playlist_id = ?1
                   AND position >= ?2 + ?4
                   AND position <= ?3 + ?4",
                params![
                    input.playlist_id.get(),
                    current_position + 1,
                    target_position,
                    shift_offset,
                    input.moved_at,
                ],
            )?;
        } else {
            park_entries_in_position_range(
                self.tx,
                input.playlist_id,
                target_position,
                current_position - 1,
                shift_offset,
                input.moved_at,
            )?;
            self.tx.execute(
                "UPDATE PlaylistEntries
                 SET position = position - ?4 + 1,
                     updated_at = ?5
                 WHERE playlist_id = ?1
                   AND position >= ?2 + ?4
                   AND position <= ?3 + ?4",
                params![
                    input.playlist_id.get(),
                    target_position,
                    current_position - 1,
                    shift_offset,
                    input.moved_at,
                ],
            )?;
        }

        self.tx.execute(
            "UPDATE PlaylistEntries
             SET position = ?3,
                 updated_at = ?4
             WHERE playlist_id = ?1
               AND playlist_entry_id = ?2",
            params![
                input.playlist_id.get(),
                input.playlist_entry_id,
                target_position,
                input.moved_at,
            ],
        )?;
        Ok(true)
    }
}

fn validate_display_name(display_name: &str) -> LibrarySqliteResult<()> {
    if display_name.trim().is_empty() {
        Err(LibrarySqliteError::WriteInvariant(
            "playlist display_name must not be empty".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn reject_duplicate_membership(
    tx: &AdmittedWrite<'_>,
    playlist_id: PlaylistId,
    library_asset_id: LibraryAssetId,
) -> LibrarySqliteResult<()> {
    let exists = tx.query_row(
        "SELECT EXISTS(
             SELECT 1
             FROM PlaylistEntries
             WHERE playlist_id = ?1
               AND library_asset_id = ?2
         )",
        params![playlist_id.get(), library_asset_id.get()],
        |row| row.get::<_, i64>(0),
    )? != 0;

    if exists {
        Err(LibrarySqliteError::WriteInvariant(format!(
            "playlist {} already contains library asset {}",
            playlist_id.get(),
            library_asset_id.get()
        )))
    } else {
        Ok(())
    }
}

fn next_playlist_position(
    tx: &AdmittedWrite<'_>,
    playlist_id: PlaylistId,
) -> LibrarySqliteResult<i64> {
    tx.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0)
         FROM PlaylistEntries
         WHERE playlist_id = ?1",
        [playlist_id.get()],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn shift_entries_after_removal(
    tx: &AdmittedWrite<'_>,
    playlist_id: PlaylistId,
    removed_position: i64,
    max_position: i64,
    changed_at: i64,
) -> LibrarySqliteResult<()> {
    if removed_position >= max_position {
        return Ok(());
    }

    let shift_offset = max_position + 2;
    tx.execute(
        "UPDATE PlaylistEntries
         SET position = position + ?3,
             updated_at = ?4
         WHERE playlist_id = ?1
           AND position > ?2",
        params![
            playlist_id.get(),
            removed_position,
            shift_offset,
            changed_at
        ],
    )?;
    tx.execute(
        "UPDATE PlaylistEntries
         SET position = position - ?3 - 1,
             updated_at = ?4
         WHERE playlist_id = ?1
           AND position > ?2 + ?3",
        params![
            playlist_id.get(),
            removed_position,
            shift_offset,
            changed_at
        ],
    )?;
    Ok(())
}

fn park_entries_in_position_range(
    tx: &AdmittedWrite<'_>,
    playlist_id: PlaylistId,
    start_position: i64,
    end_position: i64,
    shift_offset: i64,
    changed_at: i64,
) -> LibrarySqliteResult<()> {
    tx.execute(
        "UPDATE PlaylistEntries
         SET position = position + ?4,
             updated_at = ?5
         WHERE playlist_id = ?1
           AND position >= ?2
           AND position <= ?3",
        params![
            playlist_id.get(),
            start_position,
            end_position,
            shift_offset,
            changed_at,
        ],
    )?;
    Ok(())
}
