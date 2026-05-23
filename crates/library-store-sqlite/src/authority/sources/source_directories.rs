use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::SourcePresenceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceDirectoryInput {
    pub source_directory_id: Option<i64>,
    pub source_id: i64,
    pub parent_source_directory_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    pub presence_state: SourcePresenceState,
    pub dir_scan_state: Option<String>,
    pub scanned_at: Option<i64>,
    pub mtime_ns: Option<i64>,
    pub first_created_at: Option<i64>,
    pub changed_at: i64,
}

pub struct SourceDirectoriesAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceDirectoriesAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn upsert_source_directory(
        &self,
        input: &UpsertSourceDirectoryInput,
    ) -> LibrarySqliteResult<i64> {
        let existing_id = self.resolve_existing_id(input)?;

        if let Some(source_directory_id) = existing_id {
            self.tx.execute(
                "UPDATE source_directories
                 SET source_id = ?2,
                     parent_source_directory_id = ?3,
                     name = ?4,
                     relative_path = ?5,
                     presence_state = ?6,
                     dir_scan_state = COALESCE(?8, dir_scan_state),
                     dir_scan_error_kind = CASE
                         WHEN ?8 IS NULL THEN dir_scan_error_kind
                         ELSE NULL
                     END,
                     dir_scan_error_detail = CASE
                         WHEN ?8 IS NULL THEN dir_scan_error_detail
                         ELSE NULL
                     END,
                     dir_scan_updated_at = ?7,
                     scanned_at = COALESCE(?9, scanned_at),
                     mtime_ns = COALESCE(?10, mtime_ns),
                     updated_at = ?7
                 WHERE source_directory_id = ?1",
                params![
                    source_directory_id,
                    input.source_id,
                    input.parent_source_directory_id,
                    input.name,
                    input.relative_path,
                    input.presence_state.as_str(),
                    input.changed_at,
                    input.dir_scan_state.as_deref(),
                    input.scanned_at,
                    input.mtime_ns,
                ],
            )?;
            self.mark_parent_has_child_directories(input)?;
            return Ok(source_directory_id);
        }

        let created_at = input.first_created_at.unwrap_or(input.changed_at);
        let dir_scan_state = input.dir_scan_state.as_deref().unwrap_or("pending");
        match input.source_directory_id {
            Some(source_directory_id) => {
                self.tx.execute(
                    "INSERT INTO source_directories (
                         source_directory_id,
                         source_id,
                         parent_source_directory_id,
                         name,
                         relative_path,
                         presence_state,
                         dir_scan_state,
                         dir_scan_updated_at,
                         scanned_at,
                         mtime_ns,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        source_directory_id,
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        input.relative_path,
                        input.presence_state.as_str(),
                        dir_scan_state,
                        input.changed_at,
                        input.scanned_at,
                        input.mtime_ns,
                        created_at,
                        input.changed_at,
                    ],
                )?;
                self.mark_parent_has_child_directories(input)?;
                Ok(source_directory_id)
            }
            None => {
                self.tx.execute(
                    "INSERT INTO source_directories (
                         source_id,
                         parent_source_directory_id,
                         name,
                         relative_path,
                         presence_state,
                         dir_scan_state,
                         dir_scan_updated_at,
                         scanned_at,
                         mtime_ns,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        input.relative_path,
                        input.presence_state.as_str(),
                        dir_scan_state,
                        input.changed_at,
                        input.scanned_at,
                        input.mtime_ns,
                        created_at,
                        input.changed_at,
                    ],
                )?;
                let source_directory_id = self.tx.last_insert_rowid();
                self.mark_parent_has_child_directories(input)?;
                Ok(source_directory_id)
            }
        }
    }

    fn mark_parent_has_child_directories(
        &self,
        input: &UpsertSourceDirectoryInput,
    ) -> LibrarySqliteResult<()> {
        let Some(parent_source_directory_id) = input.parent_source_directory_id else {
            return Ok(());
        };

        self.tx.execute(
            "UPDATE source_directories
             SET has_child_directories = 1,
                 dir_scan_updated_at = ?2,
                 updated_at = ?2
             WHERE source_directory_id = ?1",
            params![parent_source_directory_id, input.changed_at],
        )?;
        Ok(())
    }

    fn resolve_existing_id(
        &self,
        input: &UpsertSourceDirectoryInput,
    ) -> LibrarySqliteResult<Option<i64>> {
        if let Some(source_directory_id) = input.source_directory_id
            && self.tx.query_row(
                "SELECT EXISTS(
                         SELECT 1
                         FROM source_directories
                         WHERE source_directory_id = ?1
                     )",
                [source_directory_id],
                |row| row.get::<_, i64>(0),
            )? != 0
        {
            return Ok(Some(source_directory_id));
        }

        self.tx
            .query_row(
                "SELECT source_directory_id
                 FROM source_directories
                 WHERE source_id = ?1
                   AND relative_path = ?2",
                params![input.source_id, input.relative_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
}
