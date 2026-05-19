use crate::LibrarySqliteResult;
use crate::authority::library_asset::{
    ApplyLibraryAssetMetadataCorrectionInput, LibraryAssetMetadataCorrectionsAuthorityTx,
    RetractLibraryAssetMetadataCorrectionInput,
};
use crate::authority::promotion::{
    AcceptSegmentationPromotionInput, AcceptSegmentationPromotionResult,
    AcceptSegmentationPromotionTx, ComputeCapabilityPromotionInput,
    ComputeCapabilityPromotionResult, ComputeCapabilityPromotionTx, InspectSourcePromotionInput,
    InspectSourcePromotionResult, InspectSourcePromotionTx, RebindSourcePromotionInput,
    RebindSourcePromotionResult, RebindSourcePromotionTx, ResolveLibraryAssetPromotionInput,
    ResolveLibraryAssetPromotionResult, ResolveLibraryAssetPromotionTx,
};
use crate::publication;
use library_domain::ProjectionDomain;

use super::SqliteDurableStore;

impl SqliteDurableStore {
    pub fn apply_library_asset_metadata_correction(
        &self,
        input: ApplyLibraryAssetMetadataCorrectionInput,
    ) -> LibrarySqliteResult<i64> {
        self.with_write(|write| {
            let correction_id = LibraryAssetMetadataCorrectionsAuthorityTx::new(write)
                .apply_metadata_correction(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(correction_id)
        })
    }

    pub fn retract_library_asset_metadata_correction(
        &self,
        input: RetractLibraryAssetMetadataCorrectionInput,
    ) -> LibrarySqliteResult<bool> {
        self.with_write(|write| {
            let changed = LibraryAssetMetadataCorrectionsAuthorityTx::new(write)
                .retract_metadata_correction(&input)?;
            if changed {
                publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            }
            Ok(changed)
        })
    }

    pub fn inspect_source(
        &self,
        input: InspectSourcePromotionInput,
    ) -> LibrarySqliteResult<InspectSourcePromotionResult> {
        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let result =
                InspectSourcePromotionTx::new(write, file_store_root).inspect_source(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }

    pub fn accept_segmentation(
        &self,
        input: AcceptSegmentationPromotionInput,
    ) -> LibrarySqliteResult<AcceptSegmentationPromotionResult> {
        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let result = AcceptSegmentationPromotionTx::new(write, file_store_root)
                .accept_segmentation(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }

    pub fn resolve_library_asset(
        &self,
        input: ResolveLibraryAssetPromotionInput,
    ) -> LibrarySqliteResult<ResolveLibraryAssetPromotionResult> {
        self.with_write(|write| {
            let result =
                ResolveLibraryAssetPromotionTx::new(write).resolve_library_asset(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }

    pub fn compute_capability(
        &self,
        input: ComputeCapabilityPromotionInput,
    ) -> LibrarySqliteResult<ComputeCapabilityPromotionResult> {
        let file_store_root = self.app_owned_state.artifact_file_store_root().clone();
        self.with_write(|write| {
            let result = ComputeCapabilityPromotionTx::new(write, file_store_root)
                .compute_capability(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }

    pub fn rebind_source(
        &self,
        input: RebindSourcePromotionInput,
    ) -> LibrarySqliteResult<RebindSourcePromotionResult> {
        self.with_write(|write| {
            let result = RebindSourcePromotionTx::new(write).rebind_source(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(result)
        })
    }
}
