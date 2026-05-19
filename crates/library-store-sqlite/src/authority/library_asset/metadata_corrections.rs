use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::LibraryAssetId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyLibraryAssetMetadataCorrectionInput {
    pub library_asset_id: LibraryAssetId,
    pub field_name: String,
    pub value_text: Option<String>,
    pub value_int: Option<i64>,
    pub is_null_correction: bool,
    pub source_kind: String,
    pub applied_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetractLibraryAssetMetadataCorrectionInput {
    pub library_asset_id: LibraryAssetId,
    pub field_name: String,
    pub retracted_at: i64,
}

pub struct LibraryAssetMetadataCorrectionsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> LibraryAssetMetadataCorrectionsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn apply_metadata_correction(
        &self,
        input: &ApplyLibraryAssetMetadataCorrectionInput,
    ) -> LibrarySqliteResult<i64> {
        self.tx.execute(
            "UPDATE LibraryAssetMetadataCorrections
             SET retracted_at = ?3
             WHERE library_asset_id = ?1
               AND field_name = ?2
               AND retracted_at IS NULL",
            params![
                input.library_asset_id.get(),
                input.field_name,
                input.applied_at
            ],
        )?;

        self.tx.execute(
            "INSERT INTO LibraryAssetMetadataCorrections (
                 library_asset_id,
                 field_name,
                 value_text,
                 value_int,
                 is_null_correction,
                 source_kind,
                 applied_at,
                 retracted_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![
                input.library_asset_id.get(),
                input.field_name,
                input.value_text,
                input.value_int,
                if input.is_null_correction { 1 } else { 0 },
                input.source_kind,
                input.applied_at,
            ],
        )?;
        Ok(self.tx.last_insert_rowid())
    }

    pub fn retract_metadata_correction(
        &self,
        input: &RetractLibraryAssetMetadataCorrectionInput,
    ) -> LibrarySqliteResult<bool> {
        let changed = self.tx.execute(
            "UPDATE LibraryAssetMetadataCorrections
             SET retracted_at = ?3
             WHERE library_asset_id = ?1
               AND field_name = ?2
               AND retracted_at IS NULL",
            params![
                input.library_asset_id.get(),
                input.field_name,
                input.retracted_at
            ],
        )?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplyLibraryAssetMetadataCorrectionInput, LibraryAssetMetadataCorrectionsAuthorityTx,
        RetractLibraryAssetMetadataCorrectionInput,
    };
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::LibraryAssetId;
    use rusqlite::Connection;

    #[test]
    fn metadata_correction_inputs_preserve_typed_library_asset_id() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let library_asset_id = library_asset_id(42);

        admit_write(&mut connection, |write| {
            write.execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'eq:typed-metadata-correction', 'keep_metadata', 1, 1)",
                [library_asset_id.get()],
            )?;

            let correction_id = LibraryAssetMetadataCorrectionsAuthorityTx::new(write)
                .apply_metadata_correction(&ApplyLibraryAssetMetadataCorrectionInput {
                    library_asset_id,
                    field_name: "title".to_string(),
                    value_text: Some("Typed Title".to_string()),
                    value_int: None,
                    is_null_correction: false,
                    source_kind: "user_edit".to_string(),
                    applied_at: 10,
                })?;
            assert!(correction_id > 0);
            Ok(())
        })
        .expect("apply typed metadata correction");

        let persisted_row: (i64, String, Option<String>) = connection
            .query_row(
                "SELECT library_asset_id, field_name, value_text
                 FROM LibraryAssetMetadataCorrections
                 WHERE library_asset_id = ?1",
                [library_asset_id.get()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read persisted correction");

        assert_eq!(
            persisted_row,
            (
                library_asset_id.get(),
                "title".to_string(),
                Some("Typed Title".to_string())
            )
        );
    }

    #[test]
    fn retraction_input_writes_typed_library_asset_id_through_to_raw_persistence() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");
        let library_asset_id = library_asset_id(43);

        admit_write(&mut connection, |write| {
            write.execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'eq:typed-metadata-retraction', 'keep_metadata', 1, 1)",
                [library_asset_id.get()],
            )?;

            let authority = LibraryAssetMetadataCorrectionsAuthorityTx::new(write);
            authority.apply_metadata_correction(&ApplyLibraryAssetMetadataCorrectionInput {
                library_asset_id,
                field_name: "title".to_string(),
                value_text: Some("Old Title".to_string()),
                value_int: None,
                is_null_correction: false,
                source_kind: "user_edit".to_string(),
                applied_at: 10,
            })?;

            let changed = authority.retract_metadata_correction(
                &RetractLibraryAssetMetadataCorrectionInput {
                    library_asset_id,
                    field_name: "title".to_string(),
                    retracted_at: 20,
                },
            )?;
            assert!(changed);
            Ok(())
        })
        .expect("retract typed metadata correction");

        let retracted_at: Option<i64> = connection
            .query_row(
                "SELECT retracted_at
                 FROM LibraryAssetMetadataCorrections
                 WHERE library_asset_id = ?1
                   AND field_name = 'title'",
                [library_asset_id.get()],
                |row| row.get(0),
            )
            .expect("read retracted correction");

        assert_eq!(retracted_at, Some(20));
    }

    fn library_asset_id(value: i64) -> LibraryAssetId {
        LibraryAssetId::new(value).expect("positive library asset id")
    }
}
