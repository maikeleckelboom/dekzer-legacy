use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::browse_sort_key::compute_name_sort_key;
use library_domain::{SourceAccessIssueKind, SourcePresenceState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishRootChildDirectoryInput {
    pub source_id: i64,
    pub name: String,
    pub relative_path: String,
    pub mtime_ns: Option<i64>,
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertSourceDirectoryInput {
    pub source_directory_id: Option<i64>,
    pub source_id: i64,
    pub parent_source_directory_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    pub presence_state: SourcePresenceState,
    pub dir_scan_state: Option<String>,
    pub dir_scan_issue_kind: Option<SourceAccessIssueKind>,
    pub dir_scan_error_detail: Option<String>,
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
                     name_sort_key = ?5,
                     relative_path = ?6,
                     presence_state = ?7,
                     dir_scan_state = COALESCE(?9, dir_scan_state),
                     dir_scan_issue_kind = ?10,
                     dir_scan_error_detail = ?11,
                     dir_scan_updated_at = ?8,
                     scanned_at = COALESCE(?12, scanned_at),
                     mtime_ns = COALESCE(?13, mtime_ns),
                     updated_at = ?8
                 WHERE source_directory_id = ?1",
                params![
                    source_directory_id,
                    input.source_id,
                    input.parent_source_directory_id,
                    input.name,
                    compute_name_sort_key(&input.name),
                    input.relative_path,
                    input.presence_state.as_str(),
                    input.changed_at,
                    input.dir_scan_state.as_deref(),
                    input.dir_scan_issue_kind.map(SourceAccessIssueKind::as_str),
                    input.dir_scan_error_detail.as_deref(),
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
                         name_sort_key,
                         relative_path,
                         presence_state,
                         dir_scan_state,
                         dir_scan_issue_kind,
                         dir_scan_error_detail,
                         dir_scan_updated_at,
                         scanned_at,
                         mtime_ns,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        source_directory_id,
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        compute_name_sort_key(&input.name),
                        input.relative_path,
                        input.presence_state.as_str(),
                        dir_scan_state,
                        input.dir_scan_issue_kind.map(SourceAccessIssueKind::as_str),
                        input.dir_scan_error_detail.as_deref(),
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
                         name_sort_key,
                         relative_path,
                         presence_state,
                         dir_scan_state,
                         dir_scan_issue_kind,
                         dir_scan_error_detail,
                         dir_scan_updated_at,
                         scanned_at,
                         mtime_ns,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        compute_name_sort_key(&input.name),
                        input.relative_path,
                        input.presence_state.as_str(),
                        dir_scan_state,
                        input.dir_scan_issue_kind.map(SourceAccessIssueKind::as_str),
                        input.dir_scan_error_detail.as_deref(),
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

    pub fn establish_root_child_directory(
        &self,
        input: &EstablishRootChildDirectoryInput,
    ) -> LibrarySqliteResult<i64> {
        let existing_id: Option<i64> = self
            .tx
            .query_row(
                "SELECT source_directory_id
                 FROM source_directories
                 WHERE source_id = ?1
                   AND relative_path = ?2",
                params![input.source_id, input.relative_path],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(source_directory_id) = existing_id {
            self.tx.execute(
                "UPDATE source_directories
                 SET parent_source_directory_id = NULL,
                     name = ?3,
                     name_sort_key = ?4,
                     presence_state = 'present',
                     dir_scan_state = CASE
                         WHEN presence_state = 'present' THEN dir_scan_state
                         ELSE 'pending'
                     END,
                     dir_scan_issue_kind = CASE
                         WHEN presence_state = 'present' THEN dir_scan_issue_kind
                         ELSE NULL
                     END,
                     dir_scan_error_detail = CASE
                         WHEN presence_state = 'present' THEN dir_scan_error_detail
                         ELSE NULL
                     END,
                     dir_scan_updated_at = CASE
                         WHEN presence_state = 'present' THEN dir_scan_updated_at
                         ELSE ?5
                     END,
                     scanned_at = CASE
                         WHEN presence_state = 'present' THEN scanned_at
                         ELSE NULL
                     END,
                     mtime_ns = COALESCE(?6, mtime_ns),
                     updated_at = ?5
                 WHERE source_directory_id = ?1
                   AND source_id = ?2",
                params![
                    source_directory_id,
                    input.source_id,
                    input.name,
                    compute_name_sort_key(&input.name),
                    input.observed_at,
                    input.mtime_ns,
                ],
            )?;
            return Ok(source_directory_id);
        }

        self.tx.execute(
            "INSERT INTO source_directories (
                 source_id,
                 parent_source_directory_id,
                 name,
                 name_sort_key,
                 relative_path,
                 presence_state,
                 dir_scan_state,
                 dir_scan_issue_kind,
                 dir_scan_error_detail,
                 dir_scan_updated_at,
                 scanned_at,
                 mtime_ns,
                 created_at,
                 updated_at
             )
             VALUES (?1, NULL, ?2, ?3, ?4, 'present', 'pending', NULL, NULL, ?5, NULL, ?6, ?5, ?5)",
            params![
                input.source_id,
                input.name,
                compute_name_sort_key(&input.name),
                input.relative_path,
                input.observed_at,
                input.mtime_ns,
            ],
        )?;
        Ok(self.tx.last_insert_rowid())
    }

    pub fn mark_absent_root_child_directories_missing(
        &self,
        source_id: i64,
        observed_relative_paths: &std::collections::HashSet<String>,
        observed_at: i64,
    ) -> LibrarySqliteResult<usize> {
        let mut statement = self.tx.prepare(
            "SELECT source_directory_id,
                    relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND parent_source_directory_id IS NULL
               AND relative_path <> ''
               AND presence_state = 'present'",
        )?;
        let rows = statement
            .query_map([source_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut changed = 0_usize;
        for (source_directory_id, relative_path) in rows {
            if observed_relative_paths.contains(&relative_path) {
                continue;
            }
            let rows_changed = self.tx.execute(
                "UPDATE source_directories
                 SET presence_state = 'missing',
                     dir_scan_state = 'complete',
                     dir_scan_issue_kind = NULL,
                     dir_scan_error_detail = NULL,
                     dir_scan_updated_at = ?2,
                     scanned_at = ?2,
                     updated_at = ?2
                 WHERE source_directory_id = ?1",
                params![source_directory_id, observed_at],
            )?;
            changed += rows_changed;
        }

        Ok(changed)
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
