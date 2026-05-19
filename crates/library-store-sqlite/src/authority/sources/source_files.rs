use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use library_domain::SourcePresenceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSourceFileObservationInput {
    pub source_file_id: Option<i64>,
    pub source_id: i64,
    pub parent_source_directory_id: Option<i64>,
    pub name: String,
    pub relative_path: String,
    pub size_bytes: Option<i64>,
    pub mtime_ns: Option<i64>,
    pub presence_state: SourcePresenceState,
    pub first_discovered_at: Option<i64>,
    pub observed_at: Option<i64>,
    pub presence_changed_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
struct ExistingSourceFileRow {
    source_file_id: i64,
    presence_state: String,
}

pub struct SourceFilesAuthorityTx<'write, 'conn> {
    tx: &'write AdmittedWrite<'conn>,
}

impl<'write, 'conn> SourceFilesAuthorityTx<'write, 'conn> {
    pub(crate) fn new(tx: &'write AdmittedWrite<'conn>) -> Self {
        Self { tx }
    }

    pub fn record_source_file_observation(
        &self,
        input: &RecordSourceFileObservationInput,
    ) -> LibrarySqliteResult<i64> {
        let existing = self.load_existing_row(input)?;
        if let Some(existing) = existing {
            let observed_at = input.observed_at.or(Some(input.updated_at));
            let last_presence_change_at =
                if existing.presence_state == input.presence_state.as_str() {
                    None
                } else {
                    Some(input.presence_changed_at)
                };
            self.tx.execute(
                "UPDATE source_files
                 SET source_id = ?2,
                     parent_source_directory_id = ?3,
                     name = ?4,
                     relative_path = ?5,
                     size_bytes = ?6,
                     mtime_ns = ?7,
                     presence_state = ?8,
                     last_observed_at = COALESCE(?9, last_observed_at),
                     last_presence_change_at = COALESCE(?10, last_presence_change_at),
                     updated_at = ?11
                 WHERE source_file_id = ?1",
                params![
                    existing.source_file_id,
                    input.source_id,
                    input.parent_source_directory_id,
                    input.name,
                    input.relative_path,
                    input.size_bytes,
                    input.mtime_ns,
                    input.presence_state.as_str(),
                    observed_at,
                    last_presence_change_at,
                    input.updated_at,
                ],
            )?;
            return Ok(existing.source_file_id);
        }

        let first_discovered_at = input.first_discovered_at.unwrap_or(input.updated_at);
        let last_observed_at = input.observed_at.or(Some(input.updated_at));
        match input.source_file_id {
            Some(source_file_id) => {
                self.tx.execute(
                    "INSERT INTO source_files (
                         source_file_id,
                         source_id,
                         parent_source_directory_id,
                         name,
                         relative_path,
                         size_bytes,
                         mtime_ns,
                         presence_state,
                         first_discovered_at,
                         last_observed_at,
                         last_presence_change_at,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                    params![
                        source_file_id,
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        input.relative_path,
                        input.size_bytes,
                        input.mtime_ns,
                        input.presence_state.as_str(),
                        first_discovered_at,
                        last_observed_at,
                        input.presence_changed_at,
                        input.updated_at,
                    ],
                )?;
                Ok(source_file_id)
            }
            None => {
                self.tx.execute(
                    "INSERT INTO source_files (
                         source_id,
                         parent_source_directory_id,
                         name,
                         relative_path,
                         size_bytes,
                         mtime_ns,
                         presence_state,
                         first_discovered_at,
                         last_observed_at,
                         last_presence_change_at,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                    params![
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        input.relative_path,
                        input.size_bytes,
                        input.mtime_ns,
                        input.presence_state.as_str(),
                        first_discovered_at,
                        last_observed_at,
                        input.presence_changed_at,
                        input.updated_at,
                    ],
                )?;
                Ok(self.tx.last_insert_rowid())
            }
        }
    }

    fn load_existing_row(
        &self,
        input: &RecordSourceFileObservationInput,
    ) -> LibrarySqliteResult<Option<ExistingSourceFileRow>> {
        let query = match input.source_file_id {
            Some(source_file_id) => self
                .tx
                .query_row(
                    "SELECT source_file_id, presence_state
                     FROM source_files
                     WHERE source_file_id = ?1",
                    [source_file_id],
                    |row| {
                        Ok(ExistingSourceFileRow {
                            source_file_id: row.get(0)?,
                            presence_state: row.get(1)?,
                        })
                    },
                )
                .optional()?,
            None => self
                .tx
                .query_row(
                    "SELECT source_file_id, presence_state
                     FROM source_files
                     WHERE source_id = ?1
                       AND relative_path = ?2",
                    params![input.source_id, input.relative_path],
                    |row| {
                        Ok(ExistingSourceFileRow {
                            source_file_id: row.get(0)?,
                            presence_state: row.get(1)?,
                        })
                    },
                )
                .optional()?,
        };
        Ok(query)
    }
}
