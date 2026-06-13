use rusqlite::params;

use crate::authority::work::file_store::{
    ArtifactFileStorePath, ArtifactFileStoreRoot, persist_artifact_file_payload,
};
use crate::authority::work::work_items::load_work_item_row;
use crate::authority::work::work_runs::{PersistedWorkRun, WorkRunAuthorityTx};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    ArtifactId, ArtifactKind, ArtifactStorageKind, MachineWorkKind, WorkItemId, WorkRunId,
    WorkSubject,
};

#[allow(dead_code)]
const INLINE_PAYLOAD_MAX_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordArtifactInput {
    pub work_run_id: WorkRunId,
    pub artifact_kind: ArtifactKind,
    pub media_type: String,
    pub basis_fingerprint: String,
    pub payload_hash: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordInlineArtifactInput {
    pub artifact: RecordArtifactInput,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFileStoreArtifactInput {
    pub artifact: RecordArtifactInput,
    pub root_kind: String,
    pub relative_path: String,
    pub payload_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PersistArtifactPayloadInput {
    pub artifact: RecordArtifactInput,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PersistedArtifactStorage {
    InlinePayload,
    FileStore(ArtifactFileStorePath),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PersistedArtifactPayload {
    pub artifact: RecordedArtifact,
    pub storage: PersistedArtifactStorage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedArtifact {
    pub artifact_id: ArtifactId,
    pub work_run_id: WorkRunId,
    pub subject: WorkSubject,
    pub artifact_kind: ArtifactKind,
    pub basis_fingerprint: String,
    pub storage_kind: ArtifactStorageKind,
    pub created_at: i64,
}

pub struct WorkArtifactAuthorityTx<'write, 'conn> {
    tx: &'write mut AdmittedWrite<'conn>,
}

struct ArtifactContext {
    run: PersistedWorkRun,
    work_item: crate::authority::work::ClaimedMachineWorkItem,
}

impl<'write, 'conn> WorkArtifactAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write mut AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn record_inline_artifact(
        &self,
        input: &RecordInlineArtifactInput,
    ) -> LibrarySqliteResult<RecordedArtifact> {
        let context = self.load_context(input.artifact.work_run_id)?;
        validate_artifact_input(&context, &input.artifact)?;
        let artifact = self.insert_artifact(
            &context,
            &input.artifact,
            ArtifactStorageKind::InlinePayload,
        )?;
        self.tx.execute(
            "INSERT INTO work_artifact_inline_payloads (
                 artifact_id,
                 payload,
                 created_at
             )
             VALUES (?1, ?2, ?3)",
            params![
                artifact.artifact_id.get(),
                &input.payload,
                artifact.created_at
            ],
        )?;
        Ok(artifact)
    }

    pub fn record_file_store_artifact(
        &self,
        input: &RecordFileStoreArtifactInput,
    ) -> LibrarySqliteResult<RecordedArtifact> {
        let context = self.load_context(input.artifact.work_run_id)?;
        validate_artifact_input(&context, &input.artifact)?;
        if input.root_kind.trim().is_empty() {
            return Err(LibrarySqliteError::WriteInvariant(
                "artifact file-store root_kind must not be empty".to_string(),
            ));
        }
        if input.relative_path.trim().is_empty() {
            return Err(LibrarySqliteError::WriteInvariant(
                "artifact file-store relative_path must not be empty".to_string(),
            ));
        }
        let artifact =
            self.insert_artifact(&context, &input.artifact, ArtifactStorageKind::FileStore)?;
        self.tx.execute(
            "INSERT INTO work_artifact_file_store_entries (
                 artifact_id,
                 root_kind,
                 relative_path,
                 payload_bytes,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                artifact.artifact_id.get(),
                input.root_kind,
                input.relative_path,
                input.payload_bytes,
                artifact.created_at,
            ],
        )?;
        Ok(artifact)
    }

    #[allow(dead_code)]
    pub fn persist_artifact_payload(
        &mut self,
        file_store_root: &ArtifactFileStoreRoot,
        input: &PersistArtifactPayloadInput,
    ) -> LibrarySqliteResult<PersistedArtifactPayload> {
        let context = self.load_context(input.artifact.work_run_id)?;
        validate_artifact_input(&context, &input.artifact)?;

        if input.payload.len() <= INLINE_PAYLOAD_MAX_BYTES {
            let artifact = self.insert_artifact(
                &context,
                &input.artifact,
                ArtifactStorageKind::InlinePayload,
            )?;
            self.tx.execute(
                "INSERT INTO work_artifact_inline_payloads (
                     artifact_id,
                     payload,
                     created_at
                )
                 VALUES (?1, ?2, ?3)",
                params![
                    artifact.artifact_id.get(),
                    &input.payload,
                    artifact.created_at
                ],
            )?;
            return Ok(PersistedArtifactPayload {
                artifact,
                storage: PersistedArtifactStorage::InlinePayload,
            });
        }

        let artifact =
            self.insert_artifact(&context, &input.artifact, ArtifactStorageKind::FileStore)?;
        let file_store_path = persist_artifact_file_payload(
            self.tx,
            file_store_root,
            artifact.artifact_id.get(),
            artifact.created_at,
            &input.payload,
        )?;
        self.tx.execute(
            "INSERT INTO work_artifact_file_store_entries (
                 artifact_id,
                 root_kind,
                 relative_path,
                 payload_bytes,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                artifact.artifact_id.get(),
                file_store_path.root_kind.as_str(),
                file_store_path.relative_path.as_str(),
                input.payload.len() as i64,
                artifact.created_at,
            ],
        )?;
        Ok(PersistedArtifactPayload {
            artifact,
            storage: PersistedArtifactStorage::FileStore(file_store_path),
        })
    }

    fn load_context(&self, work_run_id: WorkRunId) -> LibrarySqliteResult<ArtifactContext> {
        let runs = WorkRunAuthorityTx::new(&*self.tx);
        let run = runs.load_work_run(work_run_id)?;
        if run.finished_at.is_some() {
            return Err(LibrarySqliteError::WriteInvariant(format!(
                "artifacts must be recorded before work run {} is finished",
                work_run_id.get()
            )));
        }
        let work_item_id = WorkItemId::new(run.work_item_id).ok_or_else(|| {
            LibrarySqliteError::WriteInvariant(format!(
                "invalid work_runs.work_item_id value: {}",
                run.work_item_id
            ))
        })?;
        Ok(ArtifactContext {
            work_item: load_work_item_row(&*self.tx, work_item_id)?,
            run,
        })
    }

    fn insert_artifact(
        &self,
        context: &ArtifactContext,
        input: &RecordArtifactInput,
        storage_kind: ArtifactStorageKind,
    ) -> LibrarySqliteResult<RecordedArtifact> {
        let subject_id = context.work_item.subject.storage_id();
        self.tx.execute(
            "INSERT INTO work_artifacts (
                 work_run_id,
                 subject_kind,
                 subject_id,
                 artifact_kind,
                 adapter_key,
                 adapter_version,
                 basis_fingerprint,
                 media_type,
                 storage_kind,
                 payload_hash,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                context.run.work_run_id,
                context.work_item.subject.kind().as_str(),
                subject_id.as_str(),
                input.artifact_kind.as_str(),
                context.run.adapter_key.as_str(),
                context.run.adapter_version.as_str(),
                input.basis_fingerprint.as_str(),
                input.media_type.as_str(),
                storage_kind.as_str(),
                input.payload_hash.as_str(),
                input.created_at,
            ],
        )?;

        Ok(RecordedArtifact {
            artifact_id: parse_artifact_id(self.tx.last_insert_rowid())?,
            work_run_id: input.work_run_id,
            subject: context.work_item.subject,
            artifact_kind: input.artifact_kind,
            basis_fingerprint: input.basis_fingerprint.clone(),
            storage_kind,
            created_at: input.created_at,
        })
    }
}

fn validate_artifact_input(
    context: &ArtifactContext,
    input: &RecordArtifactInput,
) -> LibrarySqliteResult<()> {
    if input.media_type.trim().is_empty() {
        return Err(LibrarySqliteError::WriteInvariant(
            "artifact media_type must not be empty".to_string(),
        ));
    }
    if input.payload_hash.trim().is_empty() {
        return Err(LibrarySqliteError::WriteInvariant(
            "artifact payload_hash must not be empty".to_string(),
        ));
    }
    if input.basis_fingerprint != context.work_item.basis_fingerprint {
        return Err(LibrarySqliteError::WriteInvariant(format!(
            "artifact basis_fingerprint mismatch for work item {}: expected {}, found {}",
            context.work_item.work_item_id.get(),
            context.work_item.basis_fingerprint,
            input.basis_fingerprint
        )));
    }

    let expected_kind = expected_artifact_kind(context.work_item.work_kind);
    if input.artifact_kind != expected_kind {
        return Err(LibrarySqliteError::WriteInvariant(format!(
            "{} artifacts must use artifact_kind={}, found {}",
            context.work_item.work_kind.as_str(),
            expected_kind.as_str(),
            input.artifact_kind.as_str()
        )));
    }

    Ok(())
}

fn parse_artifact_id(value: i64) -> LibrarySqliteResult<ArtifactId> {
    ArtifactId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid work_artifacts.artifact_id value: {value}"
        ))
    })
}

fn expected_artifact_kind(work_kind: MachineWorkKind) -> ArtifactKind {
    match work_kind {
        MachineWorkKind::InspectSourceFile => ArtifactKind::InspectionResult,
        MachineWorkKind::RebuildProjection => ArtifactKind::ProjectionSnapshot,
    }
}
