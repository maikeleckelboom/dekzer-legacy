use crate::authority::promotion::{
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult, RebuildProjectionPromotionTx,
};
use crate::authority::sources::{
    CommitAcceptedSourceFactsInput, CommitAcceptedSourceFactsMergePolicy, SourceFactsAuthorityTx,
};
use crate::authority::work::{
    ArtifactFileStoreRoot, retire_artifact_if_unreferenced_and_unclaimed,
};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{ArtifactId, ProjectionDomain, SourceFileId, WorkPriorityClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSourcePromotionInput {
    pub source_facts: CommitAcceptedSourceFactsInput,
    pub source_facts_merge_policy: CommitAcceptedSourceFactsMergePolicy,
    pub rebuild_projection_domains: Vec<ProjectionDomain>,
    pub rebuild_priority: WorkPriorityClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectSourcePromotionResult {
    pub projection_rebuilds: Vec<RebuildProjectionPromotionResult>,
}

pub struct InspectSourcePromotionTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
    file_store_root: ArtifactFileStoreRoot,
}

impl<'write, 'conn> InspectSourcePromotionTx<'write, 'conn> {
    pub(crate) fn new(
        tx: &'write mut AdmittedWrite<'conn>,
        file_store_root: ArtifactFileStoreRoot,
    ) -> Self {
        Self {
            tx,
            file_store_root,
        }
    }

    pub fn inspect_source(
        &mut self,
        input: &InspectSourcePromotionInput,
    ) -> LibrarySqliteResult<InspectSourcePromotionResult> {
        let previous_source_facts =
            load_source_facts_state(&*self.tx, input.source_facts.source_file_id)?;
        SourceFactsAuthorityTx::new(&*self.tx).commit_accepted_source_facts_with_merge(
            &input.source_facts,
            input.source_facts_merge_policy,
        )?;

        if let Some((_, previous_accepted_artifact_id)) = previous_source_facts.as_ref()
            && *previous_accepted_artifact_id != input.source_facts.accepted_artifact_id
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
                    basis_fingerprint: input.source_facts.basis_fingerprint.clone(),
                    priority_class: input.rebuild_priority,
                    queued_at: input.source_facts.updated_at,
                },
            )?);
        }

        Ok(InspectSourcePromotionResult {
            projection_rebuilds,
        })
    }
}

fn load_source_facts_state(
    tx: &AdmittedWrite<'_>,
    source_file_id: SourceFileId,
) -> LibrarySqliteResult<Option<(String, ArtifactId)>> {
    use rusqlite::OptionalExtension;

    let state = tx
        .query_row(
            "SELECT basis_fingerprint,
                accepted_artifact_id
         FROM SourceFacts
         WHERE source_file_id = ?1",
            [source_file_id.get()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    state
        .map(|(basis_fingerprint, artifact_id)| {
            Ok((basis_fingerprint, parse_artifact_id(artifact_id)?))
        })
        .transpose()
}

fn parse_artifact_id(value: i64) -> LibrarySqliteResult<ArtifactId> {
    ArtifactId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("invalid Artifacts.artifact_id value: {value}"))
    })
}
