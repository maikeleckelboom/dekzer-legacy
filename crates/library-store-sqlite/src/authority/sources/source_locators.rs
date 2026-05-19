use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLocatorInput {
    AbsolutePath {
        absolute_path: String,
    },
    RemovableVolume {
        device_identity_kind: String,
        device_identity_value: String,
        relative_suffix: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceLocatorInput {
    pub source_id: i64,
    pub locator: SourceLocatorInput,
}

pub struct SourceLocatorsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceLocatorsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source_locator(
        &self,
        input: &UpsertSourceLocatorInput,
    ) -> LibrarySqliteResult<()> {
        match &input.locator {
            SourceLocatorInput::AbsolutePath { absolute_path } => {
                self.tx.execute(
                    "INSERT INTO source_locators (
                         source_id,
                         locator_kind,
                         absolute_path,
                         device_identity_kind,
                         device_identity_value,
                         relative_suffix
                     )
                     VALUES (?1, 'absolute_path', ?2, NULL, NULL, '')
                     ON CONFLICT(source_id) DO UPDATE
                     SET locator_kind = excluded.locator_kind,
                         absolute_path = excluded.absolute_path,
                         device_identity_kind = excluded.device_identity_kind,
                         device_identity_value = excluded.device_identity_value,
                         relative_suffix = excluded.relative_suffix",
                    params![input.source_id, absolute_path],
                )?;
            }
            SourceLocatorInput::RemovableVolume {
                device_identity_kind,
                device_identity_value,
                relative_suffix,
            } => {
                self.tx.execute(
                    "INSERT INTO source_locators (
                         source_id,
                         locator_kind,
                         absolute_path,
                         device_identity_kind,
                         device_identity_value,
                         relative_suffix
                     )
                     VALUES (?1, 'removable_volume', NULL, ?2, ?3, ?4)
                     ON CONFLICT(source_id) DO UPDATE
                     SET locator_kind = excluded.locator_kind,
                         absolute_path = excluded.absolute_path,
                         device_identity_kind = excluded.device_identity_kind,
                         device_identity_value = excluded.device_identity_value,
                         relative_suffix = excluded.relative_suffix",
                    params![
                        input.source_id,
                        device_identity_kind,
                        device_identity_value,
                        relative_suffix,
                    ],
                )?;
            }
        }
        Ok(())
    }
}
