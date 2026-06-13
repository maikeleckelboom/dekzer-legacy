use rusqlite::{Connection, OptionalExtension, Row};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreAttachmentIdentity {
    pub attachment_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreAttachmentSourceFiles {
    pub attachment: StoreAttachmentIdentity,
    pub source_file_links: Vec<StoreSourceFileAttachmentLink>,
    pub summary: StoreAttachmentSourceFilesSummary,
    pub effective_limit: usize,
    pub remaining_source_file_links: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreAttachmentSourceFilesSummary {
    pub total_occurrence_count: usize,
    pub available_occurrence_count: usize,
    pub unavailable_occurrence_count: usize,
    pub current_link_occurrence_count: usize,
    pub stale_link_occurrence_count: usize,
    pub distinct_source_count: usize,
    pub has_multiple_occurrences: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceFileAttachmentLink {
    pub source_file_attachment_link_id: i64,
    pub attachment_id: i64,
    pub source_file_id: i64,
    pub source_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub source_display_name: String,
    pub source_class: String,
    pub parent_source_directory_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    pub size_bytes: Option<i64>,
    pub mtime_ns: Option<i64>,
    pub file_kind: String,
    pub file_class: String,
    pub presence_state: String,
    pub has_current_blake3_fact: bool,
    pub link_status: StoreSourceFileAttachmentLinkStatus,
    pub source_mount_status: String,
    pub source_access_state: String,
    pub source_access_issue_kind: Option<String>,
    pub source_scan_phase: String,
    pub occurrence_status: StoreAttachmentOccurrenceStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub source_file_updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSourceFileAttachmentLinkStatus {
    Current,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreAttachmentOccurrenceStatus {
    Available,
    SourceUnavailable,
    SourceMissing,
    SourceBlocked,
    FileMissing,
    FileRemoved,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceAttachmentSummary {
    pub source_id: i64,
    pub current_links_count: usize,
    pub stale_links_count: usize,
    pub source_files_with_current_blake3_facts_count: usize,
    pub source_files_with_attachment_links_count: usize,
    pub source_files_missing_attachment_links_count: usize,
}

pub fn get_attachment_identity(
    connection: &Connection,
    attachment_id: i64,
) -> LibrarySqliteResult<Option<StoreAttachmentIdentity>> {
    connection
        .query_row(
            "SELECT attachment_id,
                    content_hash_algorithm,
                    content_hash_value
             FROM content_attachments
             WHERE attachment_id = ?1",
            [attachment_id],
            map_attachment_identity_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn get_attachment_for_source_file(
    connection: &Connection,
    source_file_id: i64,
) -> LibrarySqliteResult<Option<StoreSourceFileAttachmentLink>> {
    connection
        .query_row(
            &format!(
                "{ATTACHMENT_LINK_SELECT_SQL}
                 WHERE link.source_file_id = ?1
                 ORDER BY link.updated_at DESC,
                          link.source_file_attachment_link_id DESC
                 LIMIT 1"
            ),
            [source_file_id],
            map_attachment_link_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn get_source_files_for_attachment(
    connection: &Connection,
    attachment_id: i64,
) -> LibrarySqliteResult<Vec<StoreSourceFileAttachmentLink>> {
    connection
        .prepare(&format!(
            "{ATTACHMENT_LINK_SELECT_SQL}
             WHERE link.attachment_id = ?1
             ORDER BY file.source_id ASC,
                      file.source_file_id ASC"
        ))?
        .query_map([attachment_id], map_attachment_link_row)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn get_source_files_for_attachment_limited(
    connection: &Connection,
    attachment_id: i64,
    limit: usize,
) -> LibrarySqliteResult<Option<StoreAttachmentSourceFiles>> {
    let Some(attachment) = get_attachment_identity(connection, attachment_id)? else {
        return Ok(None);
    };

    let limit_i64 =
        i64::try_from(limit).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MAX))?;
    let summary = read_attachment_source_files_summary(connection, attachment_id)?;

    let source_file_links = connection
        .prepare(&format!(
            "{ATTACHMENT_LINK_SELECT_SQL}
             WHERE link.attachment_id = ?1
             ORDER BY file.source_id ASC,
                      file.source_file_id ASC
             LIMIT ?2"
        ))?
        .query_map(
            rusqlite::params![attachment_id, limit_i64],
            map_attachment_link_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let remaining_source_file_links = summary
        .total_occurrence_count
        .saturating_sub(source_file_links.len());

    Ok(Some(StoreAttachmentSourceFiles {
        attachment,
        source_file_links,
        summary,
        effective_limit: limit,
        remaining_source_file_links,
    }))
}

fn read_attachment_source_files_summary(
    connection: &Connection,
    attachment_id: i64,
) -> LibrarySqliteResult<StoreAttachmentSourceFilesSummary> {
    connection
        .query_row(
            &format!(
                "WITH occurrence AS (
                     {ATTACHMENT_LINK_SELECT_SQL}
                     WHERE link.attachment_id = ?1
                 )
                 SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN occurrence_status = 'available' THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN occurrence_status <> 'available' THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN link_status = 'current' THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN link_status = 'stale' THEN 1 ELSE 0 END), 0),
                        COUNT(DISTINCT source_id)
                 FROM occurrence"
            ),
            [attachment_id],
            |row| {
                let total_occurrence_count = read_count(row, 0)?;
                Ok(StoreAttachmentSourceFilesSummary {
                    total_occurrence_count,
                    available_occurrence_count: read_count(row, 1)?,
                    unavailable_occurrence_count: read_count(row, 2)?,
                    current_link_occurrence_count: read_count(row, 3)?,
                    stale_link_occurrence_count: read_count(row, 4)?,
                    distinct_source_count: read_count(row, 5)?,
                    has_multiple_occurrences: total_occurrence_count > 1,
                })
            },
        )
        .map_err(Into::into)
}

pub fn get_source_attachment_summary(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<StoreSourceAttachmentSummary>> {
    connection
        .query_row(
            "WITH source_file_current_blake3 AS (
                 SELECT file.source_file_id,
                        CASE
                            WHEN facts.source_file_id IS NOT NULL
                             AND facts.content_hash_algorithm = 'blake3'
                             AND facts.content_hash_value IS NOT NULL
                             AND file.source_id = facts.basis_source_id
                             AND file.relative_path = facts.basis_relative_path
                             AND file.size_bytes IS facts.basis_size_bytes
                             AND file.mtime_ns IS facts.basis_mtime_ns
                             AND file.presence_state = facts.basis_presence_state
                            THEN 1
                            ELSE 0
                        END AS has_current_blake3
                 FROM source_files file
                 LEFT JOIN source_file_facts facts
                   ON facts.source_file_id = file.source_file_id
                 WHERE file.source_id = ?1
             ),
             link_status AS (
                 SELECT link.source_file_id,
                        CASE
                            WHEN facts.source_file_id IS NOT NULL
                             AND facts.content_hash_algorithm = attachment.content_hash_algorithm
                             AND facts.content_hash_value = attachment.content_hash_value
                             AND file.source_id = facts.basis_source_id
                             AND file.relative_path = facts.basis_relative_path
                             AND file.size_bytes IS facts.basis_size_bytes
                             AND file.mtime_ns IS facts.basis_mtime_ns
                             AND file.presence_state = facts.basis_presence_state
                            THEN 1
                            ELSE 0
                        END AS is_current
                 FROM source_file_attachment_links link
                 JOIN content_attachments attachment
                   ON attachment.attachment_id = link.attachment_id
                 JOIN source_files file
                   ON file.source_file_id = link.source_file_id
                 LEFT JOIN source_file_facts facts
                   ON facts.source_file_id = link.source_file_id
                 WHERE link.source_id = ?1
             )
             SELECT source.source_id,
                    (SELECT COUNT(*) FROM link_status WHERE is_current = 1),
                    (SELECT COUNT(*) FROM link_status WHERE is_current = 0),
                    (SELECT COUNT(*)
                     FROM source_file_current_blake3
                     WHERE has_current_blake3 = 1),
                    (SELECT COUNT(*)
                     FROM source_file_attachment_links link
                     WHERE link.source_id = ?1),
                    (SELECT COUNT(*)
                     FROM source_file_current_blake3 facts
                     LEFT JOIN source_file_attachment_links link
                       ON link.source_file_id = facts.source_file_id
                     WHERE facts.has_current_blake3 = 1
                       AND link.source_file_id IS NULL)
             FROM sources source
             WHERE source.source_id = ?1",
            [source_id],
            |row| {
                Ok(StoreSourceAttachmentSummary {
                    source_id: row.get(0)?,
                    current_links_count: read_count(row, 1)?,
                    stale_links_count: read_count(row, 2)?,
                    source_files_with_current_blake3_facts_count: read_count(row, 3)?,
                    source_files_with_attachment_links_count: read_count(row, 4)?,
                    source_files_missing_attachment_links_count: read_count(row, 5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

// Occurrence status is a derived source/path availability status over persisted
// source lifecycle and source-file inventory facts. It intentionally stays
// separate from link_status, which reports content-evidence freshness, and this
// read never probes the filesystem or decides relocation/cleanup semantics.
const ATTACHMENT_LINK_SELECT_SQL: &str = "SELECT link.source_file_attachment_link_id,
       link.attachment_id,
       link.source_file_id,
       file.source_id,
       attachment.content_hash_algorithm,
       attachment.content_hash_value,
       source.display_name,
       source.source_class,
       file.parent_source_directory_id,
       file.name,
       file.relative_path,
       file.size_bytes,
       file.mtime_ns,
       link.file_kind,
       file.file_class,
       file.presence_state,
       CASE
           WHEN facts.source_file_id IS NOT NULL
            AND facts.content_hash_algorithm = 'blake3'
            AND facts.content_hash_value IS NOT NULL
            AND file.source_id = facts.basis_source_id
            AND file.relative_path = facts.basis_relative_path
            AND file.size_bytes IS facts.basis_size_bytes
            AND file.mtime_ns IS facts.basis_mtime_ns
            AND file.presence_state = facts.basis_presence_state
           THEN 1
           ELSE 0
       END AS has_current_blake3_fact,
       CASE
           WHEN facts.source_file_id IS NOT NULL
            AND facts.content_hash_algorithm = attachment.content_hash_algorithm
            AND facts.content_hash_value = attachment.content_hash_value
            AND file.source_id = facts.basis_source_id
            AND file.relative_path = facts.basis_relative_path
            AND file.size_bytes IS facts.basis_size_bytes
            AND file.mtime_ns IS facts.basis_mtime_ns
            AND file.presence_state = facts.basis_presence_state
           THEN 'current'
           ELSE 'stale'
       END AS link_status,
       COALESCE(state.mount_status, 'unknown'),
       COALESCE(state.access_state, 'unknown'),
       state.access_issue_kind,
       COALESCE(scan_state.scan_phase, 'idle'),
       CASE
           WHEN COALESCE(state.access_state, 'unknown') = 'missing' THEN 'source_missing'
           WHEN COALESCE(state.access_state, 'unknown') = 'blocked' THEN 'source_blocked'
           WHEN COALESCE(state.mount_status, 'unknown') <> 'mounted' THEN 'source_unavailable'
           WHEN file.presence_state = 'missing' THEN 'file_missing'
           WHEN file.presence_state = 'removed' THEN 'file_removed'
           WHEN file.presence_state = 'present'
            AND COALESCE(state.mount_status, 'unknown') = 'mounted'
            AND COALESCE(state.access_state, 'unknown') = 'accessible'
           THEN 'available'
           ELSE 'unknown'
       END AS occurrence_status,
       link.created_at,
       link.updated_at,
       file.updated_at
FROM source_file_attachment_links link
JOIN content_attachments attachment
  ON attachment.attachment_id = link.attachment_id
JOIN source_files file
  ON file.source_file_id = link.source_file_id
JOIN sources source
  ON source.source_id = file.source_id
LEFT JOIN source_state state
  ON state.source_id = file.source_id
LEFT JOIN source_scan_state scan_state
  ON scan_state.source_id = file.source_id
LEFT JOIN source_file_facts facts
  ON facts.source_file_id = link.source_file_id";

fn map_attachment_identity_row(row: &Row<'_>) -> rusqlite::Result<StoreAttachmentIdentity> {
    Ok(StoreAttachmentIdentity {
        attachment_id: row.get(0)?,
        content_hash_algorithm: row.get(1)?,
        content_hash_value: row.get(2)?,
    })
}

fn read_count(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

fn map_attachment_link_row(row: &Row<'_>) -> rusqlite::Result<StoreSourceFileAttachmentLink> {
    Ok(StoreSourceFileAttachmentLink {
        source_file_attachment_link_id: row.get(0)?,
        attachment_id: row.get(1)?,
        source_file_id: row.get(2)?,
        source_id: row.get(3)?,
        content_hash_algorithm: row.get(4)?,
        content_hash_value: row.get(5)?,
        source_display_name: row.get(6)?,
        source_class: row.get(7)?,
        parent_source_directory_id: row.get(8)?,
        name: row.get(9)?,
        relative_path: row.get(10)?,
        size_bytes: row.get(11)?,
        mtime_ns: row.get(12)?,
        file_kind: row.get(13)?,
        file_class: row.get(14)?,
        presence_state: row.get(15)?,
        has_current_blake3_fact: row.get::<_, i64>(16)? != 0,
        link_status: match row.get::<_, String>(17)?.as_str() {
            "current" => StoreSourceFileAttachmentLinkStatus::Current,
            _ => StoreSourceFileAttachmentLinkStatus::Stale,
        },
        source_mount_status: row.get(18)?,
        source_access_state: row.get(19)?,
        source_access_issue_kind: row.get(20)?,
        source_scan_phase: row.get(21)?,
        occurrence_status: match row.get::<_, String>(22)?.as_str() {
            "available" => StoreAttachmentOccurrenceStatus::Available,
            "source_unavailable" => StoreAttachmentOccurrenceStatus::SourceUnavailable,
            "source_missing" => StoreAttachmentOccurrenceStatus::SourceMissing,
            "source_blocked" => StoreAttachmentOccurrenceStatus::SourceBlocked,
            "file_missing" => StoreAttachmentOccurrenceStatus::FileMissing,
            "file_removed" => StoreAttachmentOccurrenceStatus::FileRemoved,
            _ => StoreAttachmentOccurrenceStatus::Unknown,
        },
        created_at: row.get(23)?,
        updated_at: row.get(24)?,
        source_file_updated_at: row.get(25)?,
    })
}
