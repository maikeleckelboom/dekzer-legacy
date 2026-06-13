use crate::LibrarySqliteResult;
use crate::authority::work::{
    QueueMachineWorkResult, QueueRebuildProjectionWorkInput, WorkItemAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{ProjectionDomain, WorkItemId, WorkPriorityClass};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildProjectionPromotionInput {
    pub projection_domain: ProjectionDomain,
    pub basis_fingerprint: String,
    pub priority_class: WorkPriorityClass,
    pub queued_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildProjectionPromotionResult {
    pub work_item_id: WorkItemId,
    pub created: bool,
}

pub struct RebuildProjectionPromotionTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> RebuildProjectionPromotionTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn rebuild_projection(
        &self,
        input: &RebuildProjectionPromotionInput,
    ) -> LibrarySqliteResult<RebuildProjectionPromotionResult> {
        let queued = WorkItemAuthorityTx::new(self.tx).queue_rebuild_projection_work(
            &QueueRebuildProjectionWorkInput {
                projection_domain: input.projection_domain,
                basis_fingerprint: input.basis_fingerprint.clone(),
                priority_class: input.priority_class,
                queued_at: input.queued_at,
            },
        )?;
        Ok(map_queue_result(queued))
    }
}

fn map_queue_result(result: QueueMachineWorkResult) -> RebuildProjectionPromotionResult {
    RebuildProjectionPromotionResult {
        work_item_id: result.work_item_id,
        created: result.created,
    }
}
