use rusqlite::{OptionalExtension, params};

use crate::LibrarySqliteResult;
use crate::authority::write_lane::AdmittedWrite;
use crate::browse_media::{
    SourceFileClassFilter, file_class_str_from_path, file_kind_str_from_path, is_image_file_class,
    is_primary_file_class, source_file_class_filter_predicate_sql_for_column,
};
use crate::browse_sort_key::{compute_name_sort_key, compute_path_sort_key};
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
        let file_kind = file_kind_str_from_path(&input.relative_path);
        let file_class = file_class_str_from_path(&input.relative_path);
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
                     name_sort_key = ?5,
                     path_sort_key = ?6,
                     relative_path = ?7,
                     size_bytes = ?8,
                     mtime_ns = ?9,
                     presence_state = ?10,
                     file_class = ?14,
                     file_kind = ?15,
                     last_observed_at = COALESCE(?11, last_observed_at),
                     last_presence_change_at = COALESCE(?12, last_presence_change_at),
                     updated_at = ?13
                 WHERE source_file_id = ?1",
                params![
                    existing.source_file_id,
                    input.source_id,
                    input.parent_source_directory_id,
                    input.name,
                    compute_name_sort_key(&input.name),
                    compute_path_sort_key(&input.relative_path),
                    input.relative_path,
                    input.size_bytes,
                    input.mtime_ns,
                    input.presence_state.as_str(),
                    observed_at,
                    last_presence_change_at,
                    input.updated_at,
                    file_class,
                    file_kind,
                ],
            )?;
            self.propagate_source_file_descendant_facts(existing.source_file_id, input.updated_at)?;
            return Ok(existing.source_file_id);
        }

        let first_discovered_at = input.first_discovered_at.unwrap_or(input.updated_at);
        let last_observed_at = input.observed_at.or(Some(input.updated_at));
        let source_file_id = match input.source_file_id {
            Some(source_file_id) => {
                self.tx.execute(
                    "INSERT INTO source_files (
                         source_file_id,
                         source_id,
                         parent_source_directory_id,
                         name,
                         name_sort_key,
                         path_sort_key,
                         relative_path,
                         size_bytes,
                         mtime_ns,
                         file_kind,
                         presence_state,
                         file_class,
                         first_discovered_at,
                         last_observed_at,
                         last_presence_change_at,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
                    params![
                        source_file_id,
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        compute_name_sort_key(&input.name),
                        compute_path_sort_key(&input.relative_path),
                        input.relative_path,
                        input.size_bytes,
                        input.mtime_ns,
                        file_kind,
                        input.presence_state.as_str(),
                        file_class,
                        first_discovered_at,
                        last_observed_at,
                        input.presence_changed_at,
                        input.updated_at,
                    ],
                )?;
                source_file_id
            }
            None => {
                self.tx.execute(
                    "INSERT INTO source_files (
                         source_id,
                         parent_source_directory_id,
                         name,
                         name_sort_key,
                         path_sort_key,
                         relative_path,
                         size_bytes,
                         mtime_ns,
                         file_kind,
                         presence_state,
                         file_class,
                         first_discovered_at,
                         last_observed_at,
                         last_presence_change_at,
                         created_at,
                         updated_at
                     )
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
                    params![
                        input.source_id,
                        input.parent_source_directory_id,
                        input.name,
                        compute_name_sort_key(&input.name),
                        compute_path_sort_key(&input.relative_path),
                        input.relative_path,
                        input.size_bytes,
                        input.mtime_ns,
                        file_kind,
                        input.presence_state.as_str(),
                        file_class,
                        first_discovered_at,
                        last_observed_at,
                        input.presence_changed_at,
                        input.updated_at,
                    ],
                )?;
                self.tx.last_insert_rowid()
            }
        };
        self.propagate_source_file_descendant_facts(source_file_id, input.updated_at)?;
        Ok(source_file_id)
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

    fn propagate_source_file_descendant_facts(
        &self,
        source_file_id: i64,
        updated_at: i64,
    ) -> LibrarySqliteResult<()> {
        let known_media_predicate = source_file_class_filter_predicate_sql_for_column(
            SourceFileClassFilter::PrimaryMediaAndImages,
            "file_class",
        );
        let source_file = self
            .tx
            .query_row(
                &format!(
                    "SELECT parent_source_directory_id,
                        file_class
                 FROM source_files
                 WHERE source_file_id = ?1
                   AND presence_state = 'present'
                   AND {known_media_predicate}"
                ),
                [source_file_id],
                |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .and_then(|(parent_source_directory_id, file_class)| {
                parent_source_directory_id
                    .map(|parent_source_directory_id| (parent_source_directory_id, file_class))
            });

        let Some((parent_source_directory_id, file_class)) = source_file else {
            return Ok(());
        };

        if is_primary_file_class(&file_class) {
            self.propagate_primary_media_descendant_fact(parent_source_directory_id, updated_at)?;
        }

        if is_image_file_class(&file_class) {
            self.propagate_image_media_descendant_fact(parent_source_directory_id, updated_at)?;
        }

        Ok(())
    }

    fn propagate_primary_media_descendant_fact(
        &self,
        parent_source_directory_id: i64,
        updated_at: i64,
    ) -> LibrarySqliteResult<()> {
        self.propagate_descendant_fact(
            parent_source_directory_id,
            updated_at,
            "has_primary_media_descendant",
        )
    }

    fn propagate_image_media_descendant_fact(
        &self,
        parent_source_directory_id: i64,
        updated_at: i64,
    ) -> LibrarySqliteResult<()> {
        self.propagate_descendant_fact(
            parent_source_directory_id,
            updated_at,
            "has_image_media_descendant",
        )
    }

    fn propagate_descendant_fact(
        &self,
        parent_source_directory_id: i64,
        updated_at: i64,
        fact_column: &'static str,
    ) -> LibrarySqliteResult<()> {
        let sql = match fact_column {
            "has_primary_media_descendant" => {
                "WITH RECURSIVE media_up(source_directory_id) AS (
                 SELECT ?1
                 WHERE EXISTS (
                     SELECT 1
                     FROM source_directories
                     WHERE source_directory_id = ?1
                       AND has_primary_media_descendant = 0
                 )
                 UNION ALL
                 SELECT parent.source_directory_id
                 FROM source_directories child
                 JOIN media_up current
                   ON current.source_directory_id = child.source_directory_id
                 JOIN source_directories parent
                   ON parent.source_directory_id = child.parent_source_directory_id
                 WHERE child.parent_source_directory_id IS NOT NULL
                   AND parent.has_primary_media_descendant = 0
             )
             UPDATE source_directories
             SET has_primary_media_descendant = 1,
                 dir_scan_updated_at = ?2,
                 updated_at = ?2
             WHERE source_directory_id IN (
                 SELECT source_directory_id
                 FROM media_up
             )"
            }
            "has_image_media_descendant" => {
                "WITH RECURSIVE media_up(source_directory_id) AS (
                 SELECT ?1
                 WHERE EXISTS (
                     SELECT 1
                     FROM source_directories
                     WHERE source_directory_id = ?1
                       AND has_image_media_descendant = 0
                 )
                 UNION ALL
                 SELECT parent.source_directory_id
                 FROM source_directories child
                 JOIN media_up current
                   ON current.source_directory_id = child.source_directory_id
                 JOIN source_directories parent
                   ON parent.source_directory_id = child.parent_source_directory_id
                 WHERE child.parent_source_directory_id IS NOT NULL
                   AND parent.has_image_media_descendant = 0
             )
             UPDATE source_directories
             SET has_image_media_descendant = 1,
                 dir_scan_updated_at = ?2,
                 updated_at = ?2
             WHERE source_directory_id IN (
                 SELECT source_directory_id
                 FROM media_up
             )"
            }
            _ => unreachable!("descendant fact column is an internal static value"),
        };

        self.tx
            .execute(sql, params![parent_source_directory_id, updated_at])?;
        Ok(())
    }
}
