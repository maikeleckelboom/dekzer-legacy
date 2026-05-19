use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{LibraryAssetId, SourceSegmentId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceLibraryAssetAttachmentsInput {
    pub library_asset_id: LibraryAssetId,
    pub source_segment_ids: Vec<SourceSegmentId>,
    pub accepted_at: i64,
    pub updated_at: i64,
}

pub struct LibraryAssetAttachmentsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> LibraryAssetAttachmentsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn replace_library_asset_attachments(
        &self,
        input: &ReplaceLibraryAssetAttachmentsInput,
    ) -> LibrarySqliteResult<()> {
        if input.source_segment_ids.is_empty() {
            self.tx.execute(
                "DELETE FROM LibraryAssetAttachments
                 WHERE library_asset_id = ?1",
                [input.library_asset_id.get()],
            )?;
            return Ok(());
        }

        let placeholders = std::iter::repeat_n("?", input.source_segment_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let delete_sql = format!(
            "DELETE FROM LibraryAssetAttachments
             WHERE library_asset_id = ?1
               AND source_segment_id NOT IN ({placeholders})"
        );
        let delete_params = std::iter::once(rusqlite::types::Value::Integer(
            input.library_asset_id.get(),
        ))
        .chain(
            input
                .source_segment_ids
                .iter()
                .copied()
                .map(|id| rusqlite::types::Value::Integer(id.get())),
        )
        .collect::<Vec<_>>();
        self.tx
            .execute(&delete_sql, rusqlite::params_from_iter(delete_params))?;

        for source_segment_id in &input.source_segment_ids {
            self.tx.execute(
                "INSERT INTO LibraryAssetAttachments (
                     library_asset_id,
                     source_segment_id,
                     accepted_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(source_segment_id) DO UPDATE
                 SET library_asset_id = excluded.library_asset_id,
                     accepted_at = excluded.accepted_at,
                     updated_at = excluded.updated_at",
                params![
                    input.library_asset_id.get(),
                    source_segment_id.get(),
                    input.accepted_at,
                    input.updated_at,
                ],
            )?;
        }

        Ok(())
    }
}
