use rusqlite::{Connection, OptionalExtension, Row};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceFileAttachmentLink {
    pub attachment_id: i64,
    pub source_file_id: i64,
    pub source_id: i64,
    pub content_hash_algorithm: String,
    pub content_hash_value: String,
    pub file_kind: String,
    pub link_status: StoreSourceFileAttachmentLinkStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSourceFileAttachmentLinkStatus {
    Current,
    Stale,
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

const ATTACHMENT_LINK_SELECT_SQL: &str = "SELECT link.attachment_id,
       link.source_file_id,
       link.source_id,
       attachment.content_hash_algorithm,
       link.content_hash_value,
       link.file_kind,
       CASE
           WHEN facts.source_file_id IS NOT NULL
            AND facts.content_hash_algorithm = attachment.content_hash_algorithm
            AND facts.content_hash_value = link.content_hash_value
            AND file.source_id = facts.basis_source_id
            AND file.relative_path = facts.basis_relative_path
            AND file.size_bytes IS facts.basis_size_bytes
            AND file.mtime_ns IS facts.basis_mtime_ns
            AND file.presence_state = facts.basis_presence_state
           THEN 'current'
           ELSE 'stale'
       END AS link_status
FROM source_file_attachment_links link
JOIN content_attachments attachment
  ON attachment.attachment_id = link.attachment_id
JOIN source_files file
  ON file.source_file_id = link.source_file_id
LEFT JOIN SourceFacts facts
  ON facts.source_file_id = link.source_file_id";

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
    })
}
