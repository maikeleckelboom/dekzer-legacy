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
    pub effective_limit: usize,
    pub remaining_source_file_links: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceFileAttachmentLink {
    pub attachment_id: i64,
    pub source_file_id: i64,
    pub source_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub file_kind: String,
    pub link_status: StoreSourceFileAttachmentLinkStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSourceFileAttachmentLinkStatus {
    Current,
    Stale,
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
             ORDER BY link.source_id ASC,
                      link.source_file_id ASC"
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
    let total_links = connection.query_row(
        "SELECT COUNT(*)
         FROM source_file_attachment_links
         WHERE attachment_id = ?1",
        [attachment_id],
        |row| read_count(row, 0),
    )?;

    let source_file_links = connection
        .prepare(&format!(
            "{ATTACHMENT_LINK_SELECT_SQL}
             WHERE link.attachment_id = ?1
             ORDER BY link.source_id ASC,
                      link.source_file_id ASC
             LIMIT ?2"
        ))?
        .query_map(
            rusqlite::params![attachment_id, limit_i64],
            map_attachment_link_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let remaining_source_file_links = total_links.saturating_sub(source_file_links.len());

    Ok(Some(StoreAttachmentSourceFiles {
        attachment,
        source_file_links,
        effective_limit: limit,
        remaining_source_file_links,
    }))
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
                 LEFT JOIN SourceFacts facts
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
                 LEFT JOIN SourceFacts facts
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

const ATTACHMENT_LINK_SELECT_SQL: &str = "SELECT link.attachment_id,
       link.source_file_id,
       link.source_id,
       attachment.content_hash_algorithm,
       attachment.content_hash_value,
       link.file_kind,
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
       link.created_at,
       link.updated_at
FROM source_file_attachment_links link
JOIN content_attachments attachment
  ON attachment.attachment_id = link.attachment_id
JOIN source_files file
  ON file.source_file_id = link.source_file_id
LEFT JOIN SourceFacts facts
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
        attachment_id: row.get(0)?,
        source_file_id: row.get(1)?,
        source_id: row.get(2)?,
        content_hash_algorithm: row.get(3)?,
        content_hash_value: row.get(4)?,
        file_kind: row.get(5)?,
        link_status: match row.get::<_, String>(6)?.as_str() {
            "current" => StoreSourceFileAttachmentLinkStatus::Current,
            _ => StoreSourceFileAttachmentLinkStatus::Stale,
        },
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
