use crate::authority::artifact_rows::require_source_artifact;
use crate::authority::library_asset::{
    LibraryAssetAttachmentsAuthorityTx, ReplaceLibraryAssetAttachmentsInput,
};
use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::authority::work::{
    CapabilityInvalidationAuthorityTx, MarkCapabilitiesStaleFromBasisInput, StaleCapabilityChange,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactId, LibraryAssetId, ProjectionDomain, SourceFileId, SourceSegmentId, WorkPriorityClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebindSourcePromotionInput {
    pub source_file_id: SourceFileId,
    pub accepted_artifact_id: ArtifactId,
    pub basis_fingerprint: String,
    pub library_asset_id: LibraryAssetId,
    pub source_segment_ids: Vec<SourceSegmentId>,
    pub accepted_at: i64,
    pub updated_at: i64,
    pub rebuild_projection_domains: Vec<ProjectionDomain>,
    pub rebuild_priority: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebindSourcePromotionResult {
    pub stale_capabilities: Vec<StaleCapabilityChange>,
    pub projection_rebuilds: Vec<RebuildProjectionPromotionResult>,
}

pub struct RebindSourcePromotionTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> RebindSourcePromotionTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn rebind_source(
        &self,
        input: &RebindSourcePromotionInput,
    ) -> LibrarySqliteResult<RebindSourcePromotionResult> {
        require_source_artifact(
            self.tx,
            input.accepted_artifact_id.get(),
            input.source_file_id.get(),
            "diagnostic_result",
            &input.basis_fingerprint,
        )?;

        let previous_attachment_ids = load_attachment_ids(self.tx, input.library_asset_id)?;
        LibraryAssetAttachmentsAuthorityTx::new(self.tx).replace_library_asset_attachments(
            &ReplaceLibraryAssetAttachmentsInput {
                library_asset_id: input.library_asset_id,
                source_segment_ids: input.source_segment_ids.clone(),
                accepted_at: input.accepted_at,
                updated_at: input.updated_at,
            },
        )?;

        let stale_capabilities = if previous_attachment_ids != input.source_segment_ids {
            CapabilityInvalidationAuthorityTx::new(self.tx)
                .mark_capabilities_stale_from_basis(&MarkCapabilitiesStaleFromBasisInput {
                    library_asset_ids: vec![input.library_asset_id],
                    invalidated_at: input.updated_at,
                })?
                .stale_capabilities
        } else {
            Vec::new()
        };

        let rebuild_projection = RebuildProjectionPromotionTx::new(self.tx);
        let mut projection_rebuilds = Vec::with_capacity(input.rebuild_projection_domains.len());
        for projection_domain in &input.rebuild_projection_domains {
            projection_rebuilds.push(rebuild_projection.rebuild_projection(
                &RebuildProjectionPromotionInput {
                    projection_domain: *projection_domain,
                    basis_fingerprint: input.basis_fingerprint.clone(),
                    priority_class: input.rebuild_priority,
                    queued_at: input.updated_at,
                },
            )?);
        }

        Ok(RebindSourcePromotionResult {
            stale_capabilities,
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
