use rusqlite::{OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{LibraryAssetId, LibraryAssetRetentionPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintOrReuseLibraryAssetInput {
    pub equivalence_fingerprint: String,
    pub retention_policy: LibraryAssetRetentionPolicy,
    pub changed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintOrReuseLibraryAssetResult {
    pub library_asset_id: LibraryAssetId,
    pub created: bool,
}

pub struct LibraryAssetsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> LibraryAssetsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn mint_or_reuse_library_asset(
        &self,
        input: &MintOrReuseLibraryAssetInput,
    ) -> LibrarySqliteResult<MintOrReuseLibraryAssetResult> {
        if let Some(library_asset_id) = self
            .tx
            .query_row(
                "SELECT library_asset_id
                 FROM LibraryAssets
                 WHERE equivalence_fingerprint = ?1",
                [input.equivalence_fingerprint.as_str()],
                |row| row.get(0),
            )
            .optional()?
        {
            self.tx.execute(
                "UPDATE LibraryAssets
                 SET retention_policy = ?2,
                     updated_at = ?3
                 WHERE library_asset_id = ?1",
                params![
                    library_asset_id,
                    input.retention_policy.as_str(),
                    input.changed_at,
                ],
            )?;
            return Ok(MintOrReuseLibraryAssetResult {
                library_asset_id: parse_library_asset_id(library_asset_id)?,
                created: false,
            });
        }

        self.tx.execute(
            "INSERT INTO LibraryAssets (
                 equivalence_fingerprint,
                 retention_policy,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?3)",
            params![
                input.equivalence_fingerprint,
                input.retention_policy.as_str(),
                input.changed_at,
            ],
        )?;

        Ok(MintOrReuseLibraryAssetResult {
            library_asset_id: parse_library_asset_id(self.tx.last_insert_rowid())?,
            created: true,
        })
    }
}

fn parse_library_asset_id(value: i64) -> LibrarySqliteResult<LibraryAssetId> {
    LibraryAssetId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid LibraryAssets.library_asset_id value: {value}"
        ))
    })
}
