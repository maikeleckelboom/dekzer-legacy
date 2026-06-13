use rusqlite::{Connection, OptionalExtension};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceFileObservation {
    pub source_file_id: i64,
    pub basis_fingerprint: String,
    pub basis_source_id: i64,
    pub basis_relative_path: String,
    pub basis_size_bytes: Option<i64>,
    pub basis_mtime_ns: Option<i64>,
    pub basis_presence_state: String,
    pub observed_at_ms: i64,
    pub content_hash: Option<StoreContentHashEvidence>,
    pub media_kind: String,
    pub mime_type: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub codec: Option<String>,
    pub accepted_artifact_id: i64,
    pub updated_at_ms: i64,
    pub status: StoreSourceFileObservationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreContentHashEvidence {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSourceFileObservationStatus {
    Current,
    Stale,
}

pub fn read_source_file_observation(
    connection: &Connection,
    source_file_id: i64,
) -> LibrarySqliteResult<Option<StoreSourceFileObservation>> {
    connection
        .query_row(
            "SELECT observations.source_file_id,
                    observations.basis_fingerprint,
                    observations.basis_source_id,
                    observations.basis_relative_path,
                    observations.basis_size_bytes,
                    observations.basis_mtime_ns,
                    observations.basis_presence_state,
                    observations.observed_at_ms,
                    observations.content_hash_algorithm,
                    observations.content_hash_value,
                    observations.media_kind,
                    observations.mime_type,
                    observations.duration_ms,
                    observations.sample_rate_hz,
                    observations.channels,
                    observations.bit_depth,
                    observations.codec,
                    observations.accepted_artifact_id,
                    observations.updated_at,
                    CASE
                        WHEN file.source_id = observations.basis_source_id
                         AND file.relative_path = observations.basis_relative_path
                         AND file.size_bytes IS observations.basis_size_bytes
                         AND file.mtime_ns IS observations.basis_mtime_ns
                         AND file.presence_state = observations.basis_presence_state
                        THEN 'current'
                        ELSE 'stale'
                    END AS observation_status
             FROM source_file_observations observations
             JOIN source_files file
               ON file.source_file_id = observations.source_file_id
             WHERE observations.source_file_id = ?1",
            [source_file_id],
            |row| {
                let content_hash_algorithm = row.get::<_, Option<String>>(8)?;
                let content_hash_value = row.get::<_, Option<String>>(9)?;
                Ok(StoreSourceFileObservation {
                    source_file_id: row.get(0)?,
                    basis_fingerprint: row.get(1)?,
                    basis_source_id: row.get(2)?,
                    basis_relative_path: row.get(3)?,
                    basis_size_bytes: row.get(4)?,
                    basis_mtime_ns: row.get(5)?,
                    basis_presence_state: row.get(6)?,
                    observed_at_ms: row.get(7)?,
                    content_hash: content_hash_algorithm
                        .zip(content_hash_value)
                        .map(|(algorithm, value)| StoreContentHashEvidence { algorithm, value }),
                    media_kind: row.get(10)?,
                    mime_type: row.get(11)?,
                    duration_ms: row.get(12)?,
                    sample_rate_hz: row.get(13)?,
                    channels: row.get(14)?,
                    bit_depth: row.get(15)?,
                    codec: row.get(16)?,
                    accepted_artifact_id: row.get(17)?,
                    updated_at_ms: row.get(18)?,
                    status: match row.get::<_, String>(19)?.as_str() {
                        "current" => StoreSourceFileObservationStatus::Current,
                        _ => StoreSourceFileObservationStatus::Stale,
                    },
                })
            },
        )
        .optional()
        .map_err(Into::into)
}
