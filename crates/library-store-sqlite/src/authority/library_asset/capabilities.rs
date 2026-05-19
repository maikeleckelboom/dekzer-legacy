use rusqlite::params;

use crate::authority::artifact_rows::require_library_asset_capability_artifact;
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactId, CapabilityKind, CapabilityStabilityClass, CapabilityState, LibraryAssetId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceLibraryAssetCapabilityInput {
    pub library_asset_id: LibraryAssetId,
    pub capability_kind: CapabilityKind,
    pub profile_key: String,
    pub state: CapabilityState,
    pub stability_class: Option<CapabilityStabilityClass>,
    pub quality_current: Option<i64>,
    pub basis_fingerprint: Option<String>,
    pub selected_artifact_id: Option<ArtifactId>,
    pub updated_at: i64,
}

pub struct LibraryAssetCapabilitiesAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> LibraryAssetCapabilitiesAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn replace_library_asset_capability(
        &self,
        input: &ReplaceLibraryAssetCapabilityInput,
    ) -> LibrarySqliteResult<()> {
        if let Some(selected_artifact_id) = input.selected_artifact_id {
            let basis_fingerprint = input.basis_fingerprint.as_deref().ok_or_else(|| {
                LibrarySqliteError::WriteInvariant(
                    "selected capability artifacts require a basis_fingerprint".to_string(),
                )
            })?;
            require_library_asset_capability_artifact(
                self.tx,
                selected_artifact_id.get(),
                input.library_asset_id.get(),
                input.capability_kind.as_str(),
                &input.profile_key,
                basis_fingerprint,
            )?;
        }

        self.tx.execute(
            "INSERT INTO LibraryAssetCapabilities (
                 library_asset_id,
                 capability_kind,
                 profile_key,
                 state,
                 stability_class,
                 quality_current,
                 basis_fingerprint,
                 selected_artifact_id,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(library_asset_id, capability_kind, profile_key) DO UPDATE
             SET state = excluded.state,
                 stability_class = excluded.stability_class,
                 quality_current = excluded.quality_current,
                 basis_fingerprint = excluded.basis_fingerprint,
                 selected_artifact_id = excluded.selected_artifact_id,
                 updated_at = excluded.updated_at",
            params![
                input.library_asset_id.get(),
                input.capability_kind.as_str(),
                input.profile_key,
                input.state.as_str(),
                input
                    .stability_class
                    .map(|value| value.as_str().to_string()),
                input.quality_current,
                input.basis_fingerprint,
                input.selected_artifact_id.map(ArtifactId::get),
                input.updated_at,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{LibraryAssetCapabilitiesAuthorityTx, ReplaceLibraryAssetCapabilityInput};
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{CapabilityKind, CapabilityState, LibraryAssetId};
    use rusqlite::Connection;

    #[test]
    fn replace_library_asset_capability_preserves_typed_capability_kind() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline schema");

        admit_write(&mut connection, |write| {
            write.execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'eq:typed-capability', 'keep_metadata', 1, 1)",
                [],
            )?;
            LibraryAssetCapabilitiesAuthorityTx::new(write).replace_library_asset_capability(
                &ReplaceLibraryAssetCapabilityInput {
                    library_asset_id: LibraryAssetId::new(1).expect("positive library asset id"),
                    capability_kind: CapabilityKind::waveform(),
                    profile_key: "default".to_string(),
                    state: CapabilityState::Missing,
                    stability_class: None,
                    quality_current: None,
                    basis_fingerprint: None,
                    selected_artifact_id: None,
                    updated_at: 10,
                },
            )?;
            Ok(())
        })
        .expect("replace typed capability");

        let capability_kind: String = connection
            .query_row(
                "SELECT capability_kind
                 FROM LibraryAssetCapabilities
                 WHERE library_asset_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("read capability kind");
        assert_eq!(capability_kind, CapabilityKind::WAVEFORM);
    }
}
