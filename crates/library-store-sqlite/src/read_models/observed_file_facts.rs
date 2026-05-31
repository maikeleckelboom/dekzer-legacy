use rusqlite::{Connection, OptionalExtension};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreObservedFileFacts {
    pub source_file_id: i64,
    pub fact_kind: String,
    pub basis_fingerprint: String,
    pub basis_source_id: i64,
    pub basis_relative_path: String,
    pub basis_size_bytes: Option<i64>,
    pub basis_mtime_ns: Option<i64>,
    pub basis_presence_state: String,
    pub observed_at_ms: i64,
    pub content_hash: Option<StoreContentHashEvidence>,
    pub media_kind: String,
    pub accepted_artifact_id: i64,
    pub updated_at_ms: i64,
    pub status: StoreObservedFileFactStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreContentHashEvidence {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreObservedFileFactStatus {
    Current,
    Stale,
}

pub fn read_observed_file_facts_for_source_file(
    connection: &Connection,
    source_file_id: i64,
) -> LibrarySqliteResult<Option<StoreObservedFileFacts>> {
    connection
        .query_row(
            "SELECT facts.source_file_id,
                    facts.fact_kind,
                    facts.basis_fingerprint,
                    facts.basis_source_id,
                    facts.basis_relative_path,
                    facts.basis_size_bytes,
                    facts.basis_mtime_ns,
                    facts.basis_presence_state,
                    facts.observed_at_ms,
                    facts.content_hash_algorithm,
                    facts.content_hash_value,
                    facts.media_kind,
                    facts.accepted_artifact_id,
                    facts.updated_at,
                    CASE
                        WHEN file.source_id = facts.basis_source_id
                         AND file.relative_path = facts.basis_relative_path
                         AND file.size_bytes IS facts.basis_size_bytes
                         AND file.mtime_ns IS facts.basis_mtime_ns
                         AND file.presence_state = facts.basis_presence_state
                        THEN 'current'
                        ELSE 'stale'
                    END AS fact_status
             FROM SourceFacts facts
             JOIN source_files file
               ON file.source_file_id = facts.source_file_id
             WHERE facts.source_file_id = ?1",
            [source_file_id],
            |row| {
                let content_hash_algorithm = row.get::<_, Option<String>>(9)?;
                let content_hash_value = row.get::<_, Option<String>>(10)?;
                Ok(StoreObservedFileFacts {
                    source_file_id: row.get(0)?,
                    fact_kind: row.get(1)?,
                    basis_fingerprint: row.get(2)?,
                    basis_source_id: row.get(3)?,
                    basis_relative_path: row.get(4)?,
                    basis_size_bytes: row.get(5)?,
                    basis_mtime_ns: row.get(6)?,
                    basis_presence_state: row.get(7)?,
                    observed_at_ms: row.get(8)?,
                    content_hash: content_hash_algorithm
                        .zip(content_hash_value)
                        .map(|(algorithm, value)| StoreContentHashEvidence { algorithm, value }),
                    media_kind: row.get(11)?,
                    accepted_artifact_id: row.get(12)?,
                    updated_at_ms: row.get(13)?,
                    status: match row.get::<_, String>(14)?.as_str() {
                        "current" => StoreObservedFileFactStatus::Current,
                        _ => StoreObservedFileFactStatus::Stale,
                    },
                })
            },
        )
        .optional()
        .map_err(Into::into)
}
