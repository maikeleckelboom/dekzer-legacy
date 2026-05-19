use crate::authority::library_asset::{
    ReplaceAcceptedSourceSegmentSetInput, SourceSegmentSetsAuthorityTx,
};
use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::authority::work::{
    ArtifactFileStoreRoot, CapabilityInvalidationAuthorityTx, MarkCapabilitiesStaleFromBasisInput,
    QueueMachineWorkResult, load_attached_library_asset_ids_for_source_file,
    retire_artifact_if_unreferenced_and_unclaimed,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{ArtifactId, ProjectionDomain, SourceSegmentSetId, WorkPriorityClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptSegmentationPromotionInput {
    pub segment_set: ReplaceAcceptedSourceSegmentSetInput,
    pub rebuild_projection_domains: Vec<ProjectionDomain>,
    pub rebuild_priority: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptSegmentationPromotionResult {
    pub source_segment_set_id: SourceSegmentSetId,
    pub projection_rebuilds: Vec<RebuildProjectionPromotionResult>,
}

pub struct AcceptSegmentationPromotionTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
    file_store_root: ArtifactFileStoreRoot,
}

impl<'write, 'conn> AcceptSegmentationPromotionTx<'write, 'conn> {
    pub(crate) fn new(
        tx: &'write mut AdmittedWrite<'conn>,
        file_store_root: ArtifactFileStoreRoot,
    ) -> Self {
        Self {
            tx,
            file_store_root,
        }
    }

    pub fn accept_segmentation(
        &mut self,
        input: &AcceptSegmentationPromotionInput,
    ) -> LibrarySqliteResult<AcceptSegmentationPromotionResult> {
        let attached_library_asset_ids = load_attached_library_asset_ids_for_source_file(
            &*self.tx,
            input.segment_set.source_file_id.get(),
        )?;
        let previous_accepted_artifact_id =
            load_segment_set_accepted_artifact_id(&*self.tx, &input.segment_set)?;
        let source_segment_set_id = SourceSegmentSetsAuthorityTx::new(&*self.tx)
            .replace_accepted_source_segment_set(&input.segment_set)?;

        if let Some(previous_accepted_artifact_id) = previous_accepted_artifact_id
            && previous_accepted_artifact_id != input.segment_set.accepted_artifact_id
        {
            retire_artifact_if_unreferenced_and_unclaimed(
                self.tx,
                &self.file_store_root,
                previous_accepted_artifact_id.get(),
            )?;
        }

        let rebuild_projection = RebuildProjectionPromotionTx::new(&*self.tx);
        let mut projection_rebuilds = Vec::with_capacity(input.rebuild_projection_domains.len());
        for projection_domain in &input.rebuild_projection_domains {
            projection_rebuilds.push(rebuild_projection.rebuild_projection(
                &RebuildProjectionPromotionInput {
                    projection_domain: *projection_domain,
                    basis_fingerprint: input.segment_set.basis_fingerprint.clone(),
                    priority_class: input.rebuild_priority,
                    queued_at: input.segment_set.updated_at,
                },
            )?);
        }

        if !attached_library_asset_ids.is_empty() {
            let invalidation = CapabilityInvalidationAuthorityTx::new(&*self.tx)
                .mark_capabilities_stale_from_basis(&MarkCapabilitiesStaleFromBasisInput {
                    library_asset_ids: attached_library_asset_ids,
                    invalidated_at: input.segment_set.updated_at,
                })?;
            append_projection_rebuilds(&mut projection_rebuilds, invalidation.projection_rebuilds);
        }

        Ok(AcceptSegmentationPromotionResult {
            source_segment_set_id,
            projection_rebuilds,
        })
    }
}

fn load_segment_set_accepted_artifact_id(
    tx: &AdmittedWrite<'_>,
    input: &ReplaceAcceptedSourceSegmentSetInput,
) -> LibrarySqliteResult<Option<ArtifactId>> {
    use rusqlite::{OptionalExtension, params};

    let artifact_id = match input.source_segment_set_id {
        Some(source_segment_set_id) => tx
            .query_row(
                "SELECT accepted_artifact_id
                 FROM SourceSegmentSets
                 WHERE source_segment_set_id = ?1",
                [source_segment_set_id.get()],
                |row| row.get(0),
            )
            .optional()?,
        None => tx
            .query_row(
                "SELECT accepted_artifact_id
                 FROM SourceSegmentSets
                 WHERE source_file_id = ?1
                   AND segment_set_kind = ?2",
                params![input.source_file_id.get(), input.segment_set_kind],
                |row| row.get(0),
            )
            .optional()?,
    };

    artifact_id.map(parse_artifact_id).transpose()
}

fn parse_artifact_id(value: i64) -> LibrarySqliteResult<ArtifactId> {
    ArtifactId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("invalid Artifacts.artifact_id value: {value}"))
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
