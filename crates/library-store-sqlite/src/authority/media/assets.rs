use rusqlite::{params, OptionalExtension};

use crate::authority::write_lane::AdmittedWrite;
use crate::source_media::SourceMediaReferenceKind;
use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseUpsertRequest {
    pub identity_kind: String,
    pub identity_value: String,
    pub title: Option<String>,
    pub artist_credit: Option<String>,
    pub release_kind: String,
    pub year: Option<i64>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTrackMembershipUpsertRequest {
    pub release_id: i64,
    pub track_id: i64,
    pub disc_number: Option<i64>,
    pub sequence_number: Option<i64>,
    pub side_label: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionItemCreateRequest {
    pub item_kind: String,
    pub release_id: Option<i64>,
    pub display_name: Option<String>,
    pub acquired_at: Option<i64>,
    pub source_kind: Option<String>,
    pub notes: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAssetMaterialization {
    pub asset_id: i64,
    pub file_id: i64,
    pub source_root_id: i64,
    pub source_relative_path: String,
    pub media_kind: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRenditionUpsertRequest {
    pub asset_id: i64,
    pub rendition_kind: String,
    pub profile_key: String,
    pub mime_type: String,
    pub source_root_id: i64,
    pub source_relative_path: String,
    pub width_px: i64,
    pub height_px: i64,
    pub byte_size: Option<i64>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone)]
struct FileMediaMaterializationRow {
    file_id: i64,
    source_root_id: i64,
    source_relative_path: String,
    media_kind: String,
    mime_type: String,
    width_px: Option<i64>,
    height_px: Option<i64>,
    duration_ms: Option<i64>,
    orientation_degrees: Option<i64>,
    content_hash: Option<String>,
}

pub(crate) struct MediaAssetTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> MediaAssetTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn upsert_release(
        &mut self,
        request: &ReleaseUpsertRequest,
    ) -> LibrarySqliteResult<i64> {
        self.tx().execute(
            "INSERT INTO releases (
                 identity_kind,
                 identity_value,
                 title,
                 artist_credit,
                 release_kind,
                 year,
                 label,
                 catalog_number,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(identity_kind, identity_value) DO UPDATE
             SET title = excluded.title,
                 artist_credit = excluded.artist_credit,
                 release_kind = excluded.release_kind,
                 year = excluded.year,
                 label = excluded.label,
                 catalog_number = excluded.catalog_number,
                 updated_at = excluded.updated_at",
            params![
                request.identity_kind.as_str(),
                request.identity_value.as_str(),
                request.title.as_deref(),
                request.artist_credit.as_deref(),
                request.release_kind.as_str(),
                request.year,
                request.label.as_deref(),
                request.catalog_number.as_deref(),
                request.created_at_ms,
                request.updated_at_ms,
            ],
        )?;

        self.tx()
            .query_row(
                "SELECT release_id
                 FROM releases
                 WHERE identity_kind = ?1
                   AND identity_value = ?2",
                params![
                    request.identity_kind.as_str(),
                    request.identity_value.as_str()
                ],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn upsert_release_track_membership(
        &mut self,
        request: &ReleaseTrackMembershipUpsertRequest,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO release_track_memberships (
                 release_id,
                 track_id,
                 disc_number,
                 sequence_number,
                 side_label,
                 is_primary
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(release_id, track_id) DO UPDATE
             SET disc_number = excluded.disc_number,
                 sequence_number = excluded.sequence_number,
                 side_label = excluded.side_label,
                 is_primary = excluded.is_primary",
            params![
                request.release_id,
                request.track_id,
                request.disc_number,
                request.sequence_number,
                request.side_label.as_deref(),
                if request.is_primary { 1 } else { 0 },
            ],
        )?;

        Ok(())
    }

    pub(crate) fn create_collection_item(
        &mut self,
        request: &CollectionItemCreateRequest,
    ) -> LibrarySqliteResult<i64> {
        self.tx().execute(
            "INSERT INTO collection_items (
                 item_kind,
                 release_id,
                 display_name,
                 acquired_at,
                 source_kind,
                 notes,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.item_kind.as_str(),
                request.release_id,
                request.display_name.as_deref(),
                request.acquired_at,
                request.source_kind.as_deref(),
                request.notes.as_deref(),
                request.created_at_ms,
                request.updated_at_ms,
            ],
        )?;

        Ok(self.tx().last_insert_rowid())
    }

    pub(crate) fn materialize_media_asset_for_file(
        &mut self,
        file_id: i64,
        materialized_at_ms: i64,
    ) -> LibrarySqliteResult<MediaAssetMaterialization> {
        let row = self.load_file_media_materialization_row(file_id)?;
        let content_hash = row
            .content_hash
            .as_deref()
            .ok_or(LibrarySqliteError::MissingFileMediaContentHash(file_id))?;

        let asset_id_by_source_reference = self
            .tx()
            .query_row(
                "SELECT asset_id
                 FROM media_assets
                 WHERE source_root_id = ?1
                   AND source_relative_path = ?2",
                params![row.source_root_id, row.source_relative_path.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let asset_id_by_content_hash = self
            .tx()
            .query_row(
                "SELECT asset_id
                 FROM media_assets
                 WHERE content_hash = ?1",
                [content_hash],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        let asset_id = match (asset_id_by_source_reference, asset_id_by_content_hash) {
            (Some(asset_id), Some(content_hash_asset_id)) if asset_id != content_hash_asset_id => {
                content_hash_asset_id
            }
            (Some(asset_id), _) => {
                self.update_media_asset(asset_id, &row, content_hash, materialized_at_ms)?;
                asset_id
            }
            (None, Some(asset_id)) => {
                self.tx().execute(
                    "UPDATE media_assets
                     SET media_kind = ?2,
                         mime_type = ?3,
                         source_reference_kind = ?4,
                         source_root_id = ?5,
                         source_relative_path = ?6,
                         width_px = ?7,
                         height_px = ?8,
                         duration_ms = ?9,
                         orientation_degrees = ?10,
                         updated_at = ?11
                     WHERE asset_id = ?1",
                    params![
                        asset_id,
                        row.media_kind.as_str(),
                        row.mime_type.as_str(),
                        SourceMediaReferenceKind::SourceRootRelativeFile.as_str(),
                        row.source_root_id,
                        row.source_relative_path.as_str(),
                        row.width_px,
                        row.height_px,
                        row.duration_ms,
                        row.orientation_degrees,
                        materialized_at_ms,
                    ],
                )?;
                asset_id
            }
            (None, None) => self.insert_media_asset(&row, content_hash, materialized_at_ms)?,
        };

        Ok(MediaAssetMaterialization {
            asset_id,
            file_id: row.file_id,
            source_root_id: row.source_root_id,
            source_relative_path: row.source_relative_path,
            media_kind: row.media_kind,
            mime_type: row.mime_type,
        })
    }

    pub(crate) fn materialize_asset_renditions_for_file(
        &mut self,
        file_id: i64,
    ) -> LibrarySqliteResult<()> {
        let _existing_asset: Option<i64> = self
            .tx()
            .query_row(
                "SELECT asset_id
                 FROM media_assets
                 WHERE source_root_id = (
                        SELECT source_id
                        FROM source_files
                        WHERE source_file_id = ?1
                      )
                   AND source_relative_path = (
                         SELECT relative_path
                        FROM source_files
                        WHERE source_file_id = ?1
                     )",
                [file_id],
                |row| row.get(0),
            )
            .optional()?;

        Ok(())
    }

    pub(crate) fn upsert_asset_rendition(
        &mut self,
        request: &AssetRenditionUpsertRequest,
    ) -> LibrarySqliteResult<i64> {
        self.tx().execute(
            "INSERT INTO asset_renditions (
                 asset_id,
                 rendition_kind,
                 profile_key,
                 mime_type,
                 source_reference_kind,
                 source_root_id,
                 source_relative_path,
                 width_px,
                 height_px,
                 byte_size,
                 created_at
              )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(asset_id, profile_key) DO UPDATE
             SET rendition_kind = excluded.rendition_kind,
                  mime_type = excluded.mime_type,
                  source_reference_kind = excluded.source_reference_kind,
                  source_root_id = excluded.source_root_id,
                  source_relative_path = excluded.source_relative_path,
                  width_px = excluded.width_px,
                 height_px = excluded.height_px,
                 byte_size = excluded.byte_size",
            params![
                request.asset_id,
                request.rendition_kind.as_str(),
                request.profile_key.as_str(),
                request.mime_type.as_str(),
                SourceMediaReferenceKind::SourceRootRelativeFile.as_str(),
                request.source_root_id,
                request.source_relative_path.as_str(),
                request.width_px,
                request.height_px,
                request.byte_size,
                request.created_at_ms,
            ],
        )?;

        self.tx()
            .query_row(
                "SELECT rendition_id
                 FROM asset_renditions
                 WHERE asset_id = ?1
                   AND profile_key = ?2",
                params![request.asset_id, request.profile_key.as_str()],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }

    fn load_file_media_materialization_row(
        &self,
        file_id: i64,
    ) -> LibrarySqliteResult<FileMediaMaterializationRow> {
        self.tx()
            .query_row(
                "SELECT sf.source_file_id,
                        sf.source_id,
                        sf.relative_path,
                        latest.media_kind,
                        latest.mime_type,
                        latest.width_px,
                        latest.height_px,
                        latest.duration_ms,
                        latest.orientation_degrees,
                        latest.content_hash
                 FROM source_files sf
                 JOIN file_media_latest latest ON latest.file_id = sf.source_file_id
                 WHERE sf.source_file_id = ?1",
                [file_id],
                |row| {
                    Ok(FileMediaMaterializationRow {
                        file_id: row.get(0)?,
                        source_root_id: row.get(1)?,
                        source_relative_path: row.get(2)?,
                        media_kind: row.get(3)?,
                        mime_type: row.get(4)?,
                        width_px: row.get(5)?,
                        height_px: row.get(6)?,
                        duration_ms: row.get(7)?,
                        orientation_degrees: row.get(8)?,
                        content_hash: row.get(9)?,
                    })
                },
            )
            .optional()?
            .ok_or(LibrarySqliteError::MissingFileMediaLatest(file_id))
    }

    fn insert_media_asset(
        &mut self,
        row: &FileMediaMaterializationRow,
        content_hash: &str,
        materialized_at_ms: i64,
    ) -> LibrarySqliteResult<i64> {
        self.tx().execute(
            "INSERT INTO media_assets (
                 content_hash,
                 media_kind,
                 mime_type,
                 source_reference_kind,
                 source_root_id,
                 source_relative_path,
                 width_px,
                 height_px,
                 duration_ms,
                 orientation_degrees,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                content_hash,
                row.media_kind.as_str(),
                row.mime_type.as_str(),
                SourceMediaReferenceKind::SourceRootRelativeFile.as_str(),
                row.source_root_id,
                row.source_relative_path.as_str(),
                row.width_px,
                row.height_px,
                row.duration_ms,
                row.orientation_degrees,
                materialized_at_ms,
            ],
        )?;

        Ok(self.tx().last_insert_rowid())
    }

    fn update_media_asset(
        &mut self,
        asset_id: i64,
        row: &FileMediaMaterializationRow,
        content_hash: &str,
        materialized_at_ms: i64,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "UPDATE media_assets
             SET content_hash = ?2,
                 media_kind = ?3,
                 mime_type = ?4,
                 source_reference_kind = ?5,
                 source_root_id = ?6,
                 source_relative_path = ?7,
                 width_px = ?8,
                 height_px = ?9,
                 duration_ms = ?10,
                 orientation_degrees = ?11,
                 updated_at = ?12
             WHERE asset_id = ?1",
            params![
                asset_id,
                content_hash,
                row.media_kind.as_str(),
                row.mime_type.as_str(),
                SourceMediaReferenceKind::SourceRootRelativeFile.as_str(),
                row.source_root_id,
                row.source_relative_path.as_str(),
                row.width_px,
                row.height_px,
                row.duration_ms,
                row.orientation_degrees,
                materialized_at_ms,
            ],
        )?;

        Ok(())
    }
}
