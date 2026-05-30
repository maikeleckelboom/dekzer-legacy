use crate::LibrarySqliteResult;
use crate::authority::work::{
    BlockMachineWorkInput, ClaimMachineWorkBatchInput, ClaimedMachineWorkItem,
    CompleteMachineWorkInput, FailMachineWorkInput, FinishWorkRunInput, PrepAssignmentsAuthorityTx,
    PrepPoliciesAuthorityTx, QueueAcceptSegmentationWorkInput, QueueComputeCapabilityWorkInput,
    QueueInspectSourceWorkInput, QueueMachineWorkResult, QueueRebindSourceWorkInput,
    QueueRebuildProjectionWorkInput, RebindSourceWorkAuthorityTx, ReplacePrepAssignmentsInput,
    ReplaceResolvedLibraryAssetPrepTargetsInput, ResolvedTargetsAuthorityTx, StartWorkRunInput,
    StartedWorkRun, UpsertPrepPolicyInput, WorkItemsAuthorityTx, WorkRunsAuthorityTx,
};
use crate::publication;
use library_domain::{PrepPolicyId, ProjectionDomain};

use super::SqliteDurableStore;

impl SqliteDurableStore {
    #[allow(dead_code)]
    pub(crate) fn freeze_root_work(&self, root_ids: &[i64]) {
        self.source_admission_gate.freeze_sources(root_ids.iter());
    }

    pub fn cancel_root_work(&self, root_ids: &[i64]) {
        self.source_admission_gate.cancel_sources(root_ids.iter());
    }

    #[allow(dead_code)]
    pub(crate) fn thaw_root_work(&self, root_ids: &[i64]) {
        self.source_admission_gate.thaw_sources(root_ids.iter());
    }

    pub fn upsert_prep_policy(
        &self,
        input: UpsertPrepPolicyInput,
    ) -> LibrarySqliteResult<PrepPolicyId> {
        self.with_write(|write| {
            let prep_policy_id = PrepPoliciesAuthorityTx::new(write).upsert_prep_policy(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::Navigation])?;
            Ok(prep_policy_id)
        })
    }

    pub fn replace_prep_assignments(
        &self,
        input: ReplacePrepAssignmentsInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            PrepAssignmentsAuthorityTx::new(write).replace_prep_assignments(&input)
        })
    }

    pub fn replace_resolved_library_asset_prep_targets(
        &self,
        input: ReplaceResolvedLibraryAssetPrepTargetsInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| {
            ResolvedTargetsAuthorityTx::new(write)
                .replace_resolved_library_asset_prep_targets(&input)?;
            publication::reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])?;
            Ok(())
        })
    }

    pub fn queue_inspect_source_work(
        &self,
        input: QueueInspectSourceWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).queue_inspect_source_work(&input))
    }

    pub fn queue_compute_capability_work(
        &self,
        input: QueueComputeCapabilityWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| {
            WorkItemsAuthorityTx::new(write).queue_compute_capability_work(&input)
        })
    }

    pub fn queue_accept_segmentation_work(
        &self,
        input: QueueAcceptSegmentationWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| {
            WorkItemsAuthorityTx::new(write).queue_accept_segmentation_work(&input)
        })
    }

    pub fn queue_rebind_source_work(
        &self,
        input: QueueRebindSourceWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| {
            RebindSourceWorkAuthorityTx::new(write).queue_rebind_source_work(&input)
        })
    }

    pub fn queue_rebuild_projection_work(
        &self,
        input: QueueRebuildProjectionWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| {
            WorkItemsAuthorityTx::new(write).queue_rebuild_projection_work(&input)
        })
    }

    pub fn claim_machine_work_batch(
        &self,
        input: ClaimMachineWorkBatchInput,
    ) -> LibrarySqliteResult<Vec<ClaimedMachineWorkItem>> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).claim_machine_work_batch(&input))
    }

    pub fn start_work_run(&self, input: StartWorkRunInput) -> LibrarySqliteResult<StartedWorkRun> {
        self.with_write(|write| WorkRunsAuthorityTx::new(write).start_work_run(&input))
    }

    pub fn finish_work_run(&self, input: FinishWorkRunInput) -> LibrarySqliteResult<()> {
        self.with_write(|write| WorkRunsAuthorityTx::new(write).finish_work_run(&input))
    }

    pub fn complete_machine_work_item(
        &self,
        input: CompleteMachineWorkInput,
    ) -> LibrarySqliteResult<()> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).complete_machine_work_item(&input))
    }

    pub fn fail_machine_work_item(&self, input: FailMachineWorkInput) -> LibrarySqliteResult<()> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).fail_machine_work_item(&input))
    }

    pub fn block_machine_work_item(&self, input: BlockMachineWorkInput) -> LibrarySqliteResult<()> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).block_machine_work_item(&input))
    }
}
