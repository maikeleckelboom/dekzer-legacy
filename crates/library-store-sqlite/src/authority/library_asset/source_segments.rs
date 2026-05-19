use rusqlite::params;

use crate::authority::write_lane::AdmittedWrite;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{SourceSegmentId, SourceSegmentSetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedSourceSegmentInput {
    pub source_segment_id: Option<SourceSegmentId>,
    pub segment_kind: String,
    pub ordinal: i64,
    pub start_offset_ms: i64,
    pub end_offset_ms: Option<i64>,
    pub display_title: Option<String>,
    pub display_artist: Option<String>,
    pub display_album: Option<String>,
}

pub struct SourceSegmentsAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceSegmentsAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub(crate) fn replace_segments_for_set(
        &self,
        source_segment_set_id: SourceSegmentSetId,
        accepted_at: i64,
        segments: &[AcceptedSourceSegmentInput],
    ) -> LibrarySqliteResult<Vec<SourceSegmentId>> {
        self.tx.execute(
            "DELETE FROM SourceSegments
             WHERE source_segment_set_id = ?1",
            [source_segment_set_id.get()],
        )?;

        let mut inserted_ids = Vec::with_capacity(segments.len());
        for segment in segments {
            let inserted_id = match segment.source_segment_id {
                Some(source_segment_id) => {
                    self.tx.execute(
                        "INSERT INTO SourceSegments (
                             source_segment_id,
                             source_segment_set_id,
                             segment_kind,
                             ordinal,
                             start_offset_ms,
                             end_offset_ms,
                             display_title,
                             display_artist,
                             display_album,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                        params![
                            source_segment_id.get(),
                            source_segment_set_id.get(),
                            segment.segment_kind,
                            segment.ordinal,
                            segment.start_offset_ms,
                            segment.end_offset_ms,
                            segment.display_title,
                            segment.display_artist,
                            segment.display_album,
                            accepted_at,
                        ],
                    )?;
                    source_segment_id
                }
                None => {
                    self.tx.execute(
                        "INSERT INTO SourceSegments (
                             source_segment_set_id,
                             segment_kind,
                             ordinal,
                             start_offset_ms,
                             end_offset_ms,
                             display_title,
                             display_artist,
                             display_album,
                             created_at,
                             updated_at
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                        params![
                            source_segment_set_id.get(),
                            segment.segment_kind,
                            segment.ordinal,
                            segment.start_offset_ms,
                            segment.end_offset_ms,
                            segment.display_title,
                            segment.display_artist,
                            segment.display_album,
                            accepted_at,
                        ],
                    )?;
                    parse_source_segment_id(self.tx.last_insert_rowid())?
                }
            };
            inserted_ids.push(inserted_id);
        }

        Ok(inserted_ids)
    }
}

fn parse_source_segment_id(value: i64) -> LibrarySqliteResult<SourceSegmentId> {
    SourceSegmentId::new(value).ok_or_else(|| {
        LibrarySqliteError::WriteInvariant(format!(
            "invalid SourceSegments.source_segment_id value: {value}"
        ))
    })
}
