use rusqlite::{params, OptionalExtension};

use crate::authority::write_lane::AdmittedWrite;
use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityAssetLinkTarget {
    Track(i64),
    Release(i64),
    CollectionItem(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityAssetLinkUpsertRequest {
    pub target: EntityAssetLinkTarget,
    pub asset_id: i64,
    pub role: String,
    pub ordinal: i64,
    pub is_primary: bool,
    pub link_source_kind: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub(crate) struct EntityAssetLinkTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> EntityAssetLinkTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn upsert_entity_asset_link(
        &mut self,
        request: &EntityAssetLinkUpsertRequest,
    ) -> LibrarySqliteResult<i64> {
        let existing_link_id = self.lookup_existing_link_id(request)?;

        if let Some(link_id) = existing_link_id {
            self.tx().execute(
                "UPDATE entity_asset_links
                 SET is_primary = ?2,
                     link_source_kind = ?3,
                     updated_at = ?4
                 WHERE link_id = ?1",
                params![
                    link_id,
                    if request.is_primary { 1 } else { 0 },
                    request.link_source_kind.as_str(),
                    request.updated_at_ms,
                ],
            )?;
            return Ok(link_id);
        }

        let (track_id, release_id, collection_item_id) = match request.target {
            EntityAssetLinkTarget::Track(track_id) => (Some(track_id), None, None),
            EntityAssetLinkTarget::Release(release_id) => (None, Some(release_id), None),
            EntityAssetLinkTarget::CollectionItem(collection_item_id) => {
                (None, None, Some(collection_item_id))
            }
        };

        self.tx().execute(
            "INSERT INTO entity_asset_links (
                 track_id,
                 release_id,
                 collection_item_id,
                 asset_id,
                 role,
                 ordinal,
                 is_primary,
                 link_source_kind,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                track_id,
                release_id,
                collection_item_id,
                request.asset_id,
                request.role.as_str(),
                request.ordinal,
                if request.is_primary { 1 } else { 0 },
                request.link_source_kind.as_str(),
                request.created_at_ms,
                request.updated_at_ms,
            ],
        )?;

        Ok(self.tx().last_insert_rowid())
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }

    fn lookup_existing_link_id(
        &self,
        request: &EntityAssetLinkUpsertRequest,
    ) -> LibrarySqliteResult<Option<i64>> {
        match request.target {
            EntityAssetLinkTarget::Track(track_id) => self
                .tx()
                .query_row(
                    "SELECT link_id
                     FROM entity_asset_links
                     WHERE track_id = ?1
                       AND release_id IS NULL
                       AND collection_item_id IS NULL
                       AND asset_id = ?2
                       AND role = ?3
                       AND ordinal = ?4",
                    params![
                        track_id,
                        request.asset_id,
                        request.role.as_str(),
                        request.ordinal,
                    ],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Into::into),
            EntityAssetLinkTarget::Release(release_id) => self
                .tx()
                .query_row(
                    "SELECT link_id
                     FROM entity_asset_links
                     WHERE track_id IS NULL
                       AND release_id = ?1
                       AND collection_item_id IS NULL
                       AND asset_id = ?2
                       AND role = ?3
                       AND ordinal = ?4",
                    params![
                        release_id,
                        request.asset_id,
                        request.role.as_str(),
                        request.ordinal,
                    ],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Into::into),
            EntityAssetLinkTarget::CollectionItem(collection_item_id) => self
                .tx()
                .query_row(
                    "SELECT link_id
                     FROM entity_asset_links
                     WHERE track_id IS NULL
                       AND release_id IS NULL
                       AND collection_item_id = ?1
                       AND asset_id = ?2
                       AND role = ?3
                       AND ordinal = ?4",
                    params![
                        collection_item_id,
                        request.asset_id,
                        request.role.as_str(),
                        request.ordinal,
                    ],
                    |row| row.get(0),
                )
                .optional()
                .map_err(Into::into),
        }
    }
}
