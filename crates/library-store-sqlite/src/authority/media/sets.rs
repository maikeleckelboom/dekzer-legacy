use rusqlite::params;

use crate::authority::write_lane::AdmittedWrite;
use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSetUpsertRequest {
    pub identity_kind: String,
    pub identity_value: String,
    pub display_name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSetMembershipUpsertRequest {
    pub media_set_id: i64,
    pub asset_id: i64,
    pub ordinal: i64,
}

pub(crate) struct MediaSetTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> MediaSetTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn upsert_media_set(
        &mut self,
        request: &MediaSetUpsertRequest,
    ) -> LibrarySqliteResult<i64> {
        self.tx().execute(
            "INSERT INTO media_sets (
                 identity_kind,
                 identity_value,
                 display_name,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(identity_kind, identity_value) DO UPDATE
             SET display_name = excluded.display_name,
                 updated_at = excluded.updated_at",
            params![
                request.identity_kind.as_str(),
                request.identity_value.as_str(),
                request.display_name.as_str(),
                request.created_at_ms,
                request.updated_at_ms,
            ],
        )?;

        self.tx()
            .query_row(
                "SELECT media_set_id
                 FROM media_sets
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

    pub(crate) fn upsert_media_set_membership(
        &mut self,
        request: &MediaSetMembershipUpsertRequest,
    ) -> LibrarySqliteResult<()> {
        self.tx().execute(
            "INSERT INTO media_set_memberships (
                 media_set_id,
                 asset_id,
                 ordinal
             )
             VALUES (?1, ?2, ?3)
             ON CONFLICT(media_set_id, asset_id) DO UPDATE
             SET ordinal = excluded.ordinal",
            params![request.media_set_id, request.asset_id, request.ordinal],
        )?;

        Ok(())
    }

    fn tx(&self) -> &AdmittedWrite<'conn> {
        self.tx
    }
}
