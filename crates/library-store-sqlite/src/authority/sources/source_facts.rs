use rusqlite::params;

use crate::LibrarySqliteResult;
use crate::authority::artifact_rows::require_source_artifact;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::{ArtifactId, SourceFileId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitAcceptedSourceFactsInput {
    pub source_file_id: SourceFileId,
    pub accepted_artifact_id: ArtifactId,
    pub basis_fingerprint: String,
    pub content_hash: Option<String>,
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
    pub updated_at: i64,
}

pub struct SourceFactsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceFactsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn commit_accepted_source_facts(
        &self,
        input: &CommitAcceptedSourceFactsInput,
    ) -> LibrarySqliteResult<()> {
        require_source_artifact(
            self.tx,
            input.accepted_artifact_id.get(),
            input.source_file_id.get(),
            "inspection_result",
            &input.basis_fingerprint,
        )?;

        self.tx.execute(
            "INSERT INTO SourceFacts (
                 source_file_id,
                 basis_fingerprint,
                 content_hash,
                 media_kind,
                 mime_type,
                 duration_ms,
                 sample_rate_hz,
                 channels,
                 bit_depth,
                 codec,
                 updated_at,
                 accepted_artifact_id
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(source_file_id) DO UPDATE
             SET basis_fingerprint = excluded.basis_fingerprint,
                 content_hash = excluded.content_hash,
                 media_kind = excluded.media_kind,
                 mime_type = excluded.mime_type,
                 duration_ms = excluded.duration_ms,
                 sample_rate_hz = excluded.sample_rate_hz,
                 channels = excluded.channels,
                 bit_depth = excluded.bit_depth,
                 codec = excluded.codec,
                 updated_at = excluded.updated_at,
                 accepted_artifact_id = excluded.accepted_artifact_id",
            params![
                input.source_file_id.get(),
                input.basis_fingerprint,
                input.content_hash,
                input.media_kind,
                input.mime_type,
                input.duration_ms,
                input.sample_rate_hz,
                input.channels,
                input.bit_depth,
                input.codec,
                input.updated_at,
                input.accepted_artifact_id.get(),
            ],
        )?;
        Ok(())
    }
}
