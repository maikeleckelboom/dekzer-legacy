use rusqlite::OptionalExtension;

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactRow {
    pub(crate) artifact_id: i64,
    pub(crate) subject_kind: String,
    pub(crate) subject_id: String,
    pub(crate) artifact_kind: String,
    pub(crate) basis_fingerprint: String,
}

pub(crate) fn load_artifact_row(
    tx: &AdmittedWrite<'_>,
    artifact_id: i64,
) -> LibrarySqliteResult<ArtifactRow> {
    tx.query_row(
        "SELECT artifact_id,
                subject_kind,
                subject_id,
                artifact_kind,
                basis_fingerprint
         FROM work_artifacts
         WHERE artifact_id = ?1",
        [artifact_id],
        |row| {
            Ok(ArtifactRow {
                artifact_id: row.get(0)?,
                subject_kind: row.get(1)?,
                subject_id: row.get(2)?,
                artifact_kind: row.get(3)?,
                basis_fingerprint: row.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!("artifact {artifact_id} does not exist"))
    })
}

pub(crate) fn require_source_artifact(
    tx: &AdmittedWrite<'_>,
    artifact_id: i64,
    source_file_id: i64,
    expected_artifact_kind: &str,
    expected_basis_fingerprint: &str,
) -> LibrarySqliteResult<ArtifactRow> {
    let artifact = load_artifact_row(tx, artifact_id)?;
    let expected_subject_id = source_file_id.to_string();
    require(
        artifact.subject_kind == "source_file",
        format!(
            "artifact {artifact_id} must target subject_kind=source_file, found {}",
            artifact.subject_kind
        ),
    )?;
    require(
        artifact.subject_id == expected_subject_id,
        format!(
            "artifact {artifact_id} must target source_file {} but points at {}",
            source_file_id, artifact.subject_id
        ),
    )?;
    require(
        artifact.artifact_kind == expected_artifact_kind,
        format!(
            "artifact {artifact_id} must have artifact_kind={expected_artifact_kind}, found {}",
            artifact.artifact_kind
        ),
    )?;
    require(
        artifact.basis_fingerprint == expected_basis_fingerprint,
        format!(
            "artifact {artifact_id} basis_fingerprint mismatch: expected {expected_basis_fingerprint}, found {}",
            artifact.basis_fingerprint
        ),
    )?;
    Ok(artifact)
}

fn require(condition: bool, message: String) -> LibrarySqliteResult<()> {
    if condition {
        Ok(())
    } else {
        Err(LibrarySqliteError::WriteInvariant(message))
    }
}
