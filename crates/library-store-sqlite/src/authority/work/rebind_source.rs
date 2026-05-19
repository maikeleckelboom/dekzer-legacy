use crate::LibrarySqliteResult;
use crate::authority::work::{
    MachineWorkKey, QueueMachineWorkInput, QueueMachineWorkResult, WorkItemsAuthorityTx,
};
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{MachineWorkKind, SourceFileId, WorkPriorityClass, WorkSubject};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRebindSourceWorkInput {
    pub source_file_id: SourceFileId,
    pub basis_fingerprint: String,
    pub priority_class: WorkPriorityClass,
    pub queued_at: i64,
}

pub struct RebindSourceWorkAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> RebindSourceWorkAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn queue_rebind_source_work(
        &self,
        input: &QueueRebindSourceWorkInput,
    ) -> LibrarySqliteResult<QueueMachineWorkResult> {
        WorkItemsAuthorityTx::new(self.tx).queue_machine_work(&QueueMachineWorkInput {
            key: MachineWorkKey {
                subject: WorkSubject::SourceFile(input.source_file_id),
                work_kind: MachineWorkKind::RebindSource,
                capability_kind: None,
                target_profile_key: None,
                target_quality: None,
                basis_fingerprint: input.basis_fingerprint.clone(),
            },
            priority_class: input.priority_class,
            queued_at: input.queued_at,
        })
    }
}
