use rusqlite::{OptionalExtension, params};

use crate::authority::artifact_rows::require_source_artifact;
use crate::authority::library_asset::{AcceptedSourceSegmentInput, SourceSegmentsAuthorityTx};
use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{ArtifactId, SourceFileId, SourceSegmentSetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceAcceptedSourceSegmentSetInput {
    pub source_segment_set_id: Option<SourceSegmentSetId>,
    pub source_file_id: SourceFileId,
    pub segment_set_kind: String,
    pub basis_fingerprint: String,
    pub accepted_artifact_id: ArtifactId,
    pub accepted_at: i64,
    pub updated_at: i64,
    pub segments: Vec<AcceptedSourceSegmentInput>,
}

pub struct SourceSegmentSetsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceSegmentSetsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn replace_accepted_source_segment_set(
        &self,
        input: &ReplaceAcceptedSourceSegmentSetInput,
    ) -> LibrarySqliteResult<SourceSegmentSetId> {
        if input.segments.is_empty() {
            return Err(LibrarySqliteError::WriteInvariant(
                "accepted source segment sets must contain at least one segment".to_string(),
            ));
        }

        require_source_artifact(
            self.tx,
            input.accepted_artifact_id.get(),
            input.source_file_id.get(),
            "segmentation_result",
            &input.basis_fingerprint,
        )?;

        let source_segment_set_id = self.resolve_segment_set_id(input)?;
        if self.segment_set_exists(source_segment_set_id)? {
            self.tx.execute(
                "UPDATE SourceSegmentSets
                 SET source_file_id = ?2,
                     segment_set_kind = ?3,
                     basis_fingerprint = ?4,
                     accepted_at = ?5,
                     accepted_artifact_id = ?6,
                     updated_at = ?7
                 WHERE source_segment_set_id = ?1",
                params![
                    source_segment_set_id.get(),
                    input.source_file_id.get(),
                    input.segment_set_kind,
                    input.basis_fingerprint,
                    input.accepted_at,
                    input.accepted_artifact_id.get(),
                    input.updated_at,
                ],
            )?;
        } else {
            self.tx.execute(
                "INSERT INTO SourceSegmentSets (
                     source_segment_set_id,
                     source_file_id,
                     segment_set_kind,
                     basis_fingerprint,
                     accepted_at,
                     accepted_artifact_id,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    source_segment_set_id.get(),
                    input.source_file_id.get(),
                    input.segment_set_kind,
                    input.basis_fingerprint,
                    input.accepted_at,
                    input.accepted_artifact_id.get(),
                    input.updated_at,
                ],
            )?;
        }

        SourceSegmentsAuthorityTx::new(self.tx).replace_segments_for_set(
            source_segment_set_id,
            input.accepted_at,
            &input.segments,
        )?;
        Ok(source_segment_set_id)
    }

    fn resolve_segment_set_id(
        &self,
        input: &ReplaceAcceptedSourceSegmentSetInput,
    ) -> LibrarySqliteResult<SourceSegmentSetId> {
        match input.source_segment_set_id {
            Some(source_segment_set_id) => Ok(source_segment_set_id),
            None => self
                .tx
                .query_row(
                    "SELECT source_segment_set_id
                     FROM SourceSegmentSets
                     WHERE source_file_id = ?1
                       AND segment_set_kind = ?2",
                    params![input.source_file_id.get(), input.segment_set_kind],
                    |row| row.get(0),
                )
                .optional()?
                .map_or_else(
                    || {
                        self.tx.execute(
                            "INSERT INTO SourceSegmentSets (
                                 source_file_id,
                                 segment_set_kind,
                                 basis_fingerprint,
                                 accepted_at,
                                 accepted_artifact_id,
                                 updated_at
                             )
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![
                                input.source_file_id.get(),
                                input.segment_set_kind,
                                input.basis_fingerprint,
                                input.accepted_at,
                                input.accepted_artifact_id.get(),
                                input.updated_at,
                            ],
                        )?;
                        parse_source_segment_set_id(self.tx.last_insert_rowid())
                    },
                    parse_source_segment_set_id,
                ),
        }
    }

    fn segment_set_exists(
        &self,
        source_segment_set_id: SourceSegmentSetId,
    ) -> LibrarySqliteResult<bool> {
        Ok(self.tx.query_row(
            "SELECT EXISTS(
                     SELECT 1
                     FROM SourceSegmentSets
                     WHERE source_segment_set_id = ?1
                 )",
            [source_segment_set_id.get()],
            |row| row.get::<_, i64>(0),
        )? != 0)
    }
}

fn parse_source_segment_set_id(value: i64) -> LibrarySqliteResult<SourceSegmentSetId> {
    SourceSegmentSetId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid SourceSegmentSets.source_segment_set_id value: {value}"
        ))
    })
}
