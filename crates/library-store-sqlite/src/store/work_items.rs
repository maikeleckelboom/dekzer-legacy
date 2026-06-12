use super::SqliteDurableStore;
use crate::LibrarySqliteResult;
use crate::authority::work::{
    BlockMachineWorkInput, ClaimMachineWorkBatchInput, ClaimedMachineWorkItem,
    CompleteMachineWorkInput, FailMachineWorkInput, FinishWorkRunInput,
    QueueInspectSourceWorkInput, QueueMachineWorkResult, QueueRebuildProjectionWorkInput,
    StartWorkRunInput, StartedWorkRun, WorkItemsAuthorityTx, WorkRunsAuthorityTx,
};

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

    pub fn queue_inspect_source_work(
        &self,
        input: QueueInspectSourceWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        self.with_write(|write| WorkItemsAuthorityTx::new(write).queue_inspect_source_work(&input))
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
