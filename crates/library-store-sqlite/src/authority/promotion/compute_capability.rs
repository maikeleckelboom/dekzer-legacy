use crate::authority::library_asset::{
    LibraryAssetCapabilitiesAuthorityTx, ReplaceLibraryAssetCapabilityInput,
};
use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::authority::work::{
    ArtifactFileStoreRoot, CapabilityInvalidationAuthorityTx,
    MarkCapabilityStaleFromDependencyInput, QueueMachineWorkResult,
    retire_artifact_if_unreferenced_and_unclaimed,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactId, CapabilityKind, LibraryAssetId, ProjectionDomain, WorkPriorityClass,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCapabilityPromotionInput {
    pub capability: ReplaceLibraryAssetCapabilityInput,
    pub rebuild_projection_domains: Vec<ProjectionDomain>,
    pub rebuild_priority: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCapabilityPromotionResult {
    pub projection_rebuilds: Vec<RebuildProjectionPromotionResult>,
}

pub struct ComputeCapabilityPromotionTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
    file_store_root: ArtifactFileStoreRoot,
}

impl<'write, 'conn> ComputeCapabilityPromotionTx<'write, 'conn> {
    pub(crate) fn new(
        tx: &'write mut AdmittedWrite<'conn>,
        file_store_root: ArtifactFileStoreRoot,
    ) -> Self {
        Self {
            tx,
            file_store_root,
        }
    }

    pub fn compute_capability(
        &mut self,
        input: &ComputeCapabilityPromotionInput,
    ) -> LibrarySqliteResult<ComputeCapabilityPromotionResult> {
        let previous_capability = load_capability_basis(
            &*self.tx,
            input.capability.library_asset_id,
            &input.capability.capability_kind,
            &input.capability.profile_key,
        )?;
        LibraryAssetCapabilitiesAuthorityTx::new(&*self.tx)
            .replace_library_asset_capability(&input.capability)?;

        if let Some((_, Some(previous_selected_artifact_id))) = &previous_capability
            && Some(*previous_selected_artifact_id) != input.capability.selected_artifact_id
        {
            retire_artifact_if_unreferenced_and_unclaimed(
                self.tx,
                &self.file_store_root,
                previous_selected_artifact_id.get(),
            )?;
        }

        let rebuild_projection = RebuildProjectionPromotionTx::new(&*self.tx);
        let basis_fingerprint = input
            .capability
            .basis_fingerprint
            .clone()
            .unwrap_or_else(|| {
                format!(
                    "library_asset:{}:{}",
                    input.capability.library_asset_id.get(),
                    input.capability.capability_kind.as_str()
                )
            });
        let mut projection_rebuilds = Vec::with_capacity(input.rebuild_projection_domains.len());
        for projection_domain in &input.rebuild_projection_domains {
            projection_rebuilds.push(rebuild_projection.rebuild_projection(
                &RebuildProjectionPromotionInput {
                    projection_domain: *projection_domain,
                    basis_fingerprint: basis_fingerprint.clone(),
                    priority_class: input.rebuild_priority,
                    queued_at: input.capability.updated_at,
                },
            )?);
        }

        if let Some((previous_basis_fingerprint, previous_selected_artifact_id)) =
            previous_capability
            && (previous_basis_fingerprint.as_deref()
                != input.capability.basis_fingerprint.as_deref()
                || previous_selected_artifact_id != input.capability.selected_artifact_id)
        {
            let invalidation = CapabilityInvalidationAuthorityTx::new(&*self.tx)
                .mark_capability_stale_from_dependency(&MarkCapabilityStaleFromDependencyInput {
                    library_asset_id: input.capability.library_asset_id,
                    upstream_capability_kind: input.capability.capability_kind.clone(),
                    upstream_basis_fingerprint: basis_fingerprint.clone(),
                    invalidated_at: input.capability.updated_at,
                })?;
            append_projection_rebuilds(&mut projection_rebuilds, invalidation.projection_rebuilds);
        }

        Ok(ComputeCapabilityPromotionResult {
            projection_rebuilds,
        })
    }
}

fn load_capability_basis(
    tx: &AdmittedWrite<'_>,
    library_asset_id: LibraryAssetId,
    capability_kind: &CapabilityKind,
    profile_key: &str,
) -> LibrarySqliteResult<Option<(Option<String>, Option<ArtifactId>)>> {
    use rusqlite::OptionalExtension;

    let basis = tx
        .query_row(
            "SELECT basis_fingerprint, selected_artifact_id
         FROM LibraryAssetCapabilities
         WHERE library_asset_id = ?1
           AND capability_kind = ?2
           AND profile_key = ?3",
            rusqlite::params![
                library_asset_id.get(),
                capability_kind.as_str(),
                profile_key
            ],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                ))
            },
        )
        .optional()
        .map_err(LibrarySqliteError::from)?;

    basis
        .map(|(basis_fingerprint, artifact_id)| {
            Ok((
                basis_fingerprint,
                artifact_id.map(parse_artifact_id).transpose()?,
            ))
        })
        .transpose()
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

fn parse_artifact_id(value: i64) -> LibrarySqliteResult<ArtifactId> {
    ArtifactId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid LibraryAssetCapabilities.selected_artifact_id value: {value}"
        ))
    })
}
