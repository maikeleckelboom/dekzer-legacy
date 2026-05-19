use crate::authority::library_asset::{
    LibraryAssetAttachmentsAuthorityTx, LibraryAssetsAuthorityTx, MintOrReuseLibraryAssetInput,
    MintOrReuseLibraryAssetResult, ReplaceLibraryAssetAttachmentsInput,
};
use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::authority::work::{
    CapabilityInvalidationAuthorityTx, MarkCapabilitiesStaleFromBasisInput, QueueMachineWorkResult,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    LibraryAssetId, LibraryAssetRetentionPolicy, ProjectionDomain, SourceSegmentId,
    WorkPriorityClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveLibraryAssetPromotionInput {
    pub equivalence_fingerprint: String,
    pub retention_policy: LibraryAssetRetentionPolicy,
    pub source_segment_ids: Vec<SourceSegmentId>,
    pub accepted_at: i64,
    pub updated_at: i64,
    pub rebuild_projection_domains: Vec<ProjectionDomain>,
    pub rebuild_priority: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveLibraryAssetPromotionResult {
    pub library_asset: MintOrReuseLibraryAssetResult,
    pub projection_rebuilds: Vec<RebuildProjectionPromotionResult>,
}

pub struct ResolveLibraryAssetPromotionTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> ResolveLibraryAssetPromotionTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn resolve_library_asset(
        &self,
        input: &ResolveLibraryAssetPromotionInput,
    ) -> LibrarySqliteResult<ResolveLibraryAssetPromotionResult> {
        let library_asset = LibraryAssetsAuthorityTx::new(self.tx).mint_or_reuse_library_asset(
            &MintOrReuseLibraryAssetInput {
                equivalence_fingerprint: input.equivalence_fingerprint.clone(),
                retention_policy: input.retention_policy,
                changed_at: input.updated_at,
            },
        )?;
        let previous_attachment_ids = load_attachment_ids(self.tx, library_asset.library_asset_id)?;
        LibraryAssetAttachmentsAuthorityTx::new(self.tx).replace_library_asset_attachments(
            &ReplaceLibraryAssetAttachmentsInput {
                library_asset_id: library_asset.library_asset_id,
                source_segment_ids: input.source_segment_ids.clone(),
                accepted_at: input.accepted_at,
                updated_at: input.updated_at,
            },
        )?;

        let rebuild_projection = RebuildProjectionPromotionTx::new(self.tx);
        let mut projection_rebuilds = Vec::with_capacity(input.rebuild_projection_domains.len());
        for projection_domain in &input.rebuild_projection_domains {
            projection_rebuilds.push(rebuild_projection.rebuild_projection(
                &RebuildProjectionPromotionInput {
                    projection_domain: *projection_domain,
                    basis_fingerprint: input.equivalence_fingerprint.clone(),
                    priority_class: input.rebuild_priority,
                    queued_at: input.updated_at,
                },
            )?);
        }

        if previous_attachment_ids != input.source_segment_ids {
            let invalidation = CapabilityInvalidationAuthorityTx::new(self.tx)
                .mark_capabilities_stale_from_basis(&MarkCapabilitiesStaleFromBasisInput {
                    library_asset_ids: vec![library_asset.library_asset_id],
                    invalidated_at: input.updated_at,
                })?;
            append_projection_rebuilds(&mut projection_rebuilds, invalidation.projection_rebuilds);
        }

        Ok(ResolveLibraryAssetPromotionResult {
            library_asset,
            projection_rebuilds,
        })
    }
}

fn load_attachment_ids(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
) -> LibrarySqliteResult<Vec<SourceSegmentId>> {
    tx.prepare(
        "SELECT source_segment_id
         FROM LibraryAssetAttachments
         WHERE library_asset_id = ?1
         ORDER BY source_segment_id ASC",
    )?
    .query_map([library_asset_id.get()], |row| row.get(0))?
    .map(|result| result.map_err(Into::into).and_then(parse_source_segment_id))
    .collect()
}

fn parse_source_segment_id(value: i64) -> LibrarySqliteResult<SourceSegmentId> {
    SourceSegmentId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid SourceSegments.source_segment_id value: {value}"
        ))
    })
}

fn append_projection_rebuilds(
    projection_rebuilds: &mut Vec<RebuildProjectionPromotionResult>,
    queued_results: Vec<QueueMachineWorkResult>,
) {
    projection_rebuilds.extend(queued_results.into_iter().map(|result| {
        RebuildProjectionPromotionResult {
            work_item_id: result.work_item_id,
            created: result.created,
        }
    }));
}
