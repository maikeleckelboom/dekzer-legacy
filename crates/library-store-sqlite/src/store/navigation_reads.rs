use std::collections::HashSet;

use rusqlite::{OptionalExtension, params, params_from_iter};

use crate::browse_media::{
    SourceFileClass, canonical_file_class_from_file_kind, canonical_file_class_from_media_kind,
    classify_relative_path_file_kind,
};
use crate::read_models::navigation::{
    NavigationRow, load_row as load_navigation_row_query,
    load_row_by_stable_key as load_navigation_row_by_key_query,
    read_rows as read_navigation_rows_query,
};
use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::{SqliteDurableStore, bootstrap::open_connection};

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum FilesystemPathLibraryStatus {
    Indexed { root_id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct FilesystemPathLibraryAnnotation {
    pub requested_path: String,
    pub status: FilesystemPathLibraryStatus,
    pub file_class: Option<SourceFileClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct FilesystemPathRootResolution {
    pub requested_path: String,
    pub root_id: i64,
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct LibraryNavigationPathTarget {
    pub root_id: i64,
    pub parent_item_key: Option<i64>,
}

impl SqliteDurableStore {
    pub fn read_navigation_rows(
        &self,
        parent_navigation_row_id: Option<i64>,
    ) -> LibrarySqliteResult<Vec<NavigationRow>> {
        let connection = open_connection(&self.path)?;
        read_navigation_rows_query(&connection, parent_navigation_row_id)
    }

    pub fn load_navigation_row(
        &self,
        navigation_row_id: i64,
    ) -> LibrarySqliteResult<Option<NavigationRow>> {
        let connection = open_connection(&self.path)?;
        load_navigation_row_query(&connection, navigation_row_id)
    }

    pub fn load_navigation_row_by_stable_key(
        &self,
        stable_key: &str,
    ) -> LibrarySqliteResult<Option<NavigationRow>> {
        let connection = open_connection(&self.path)?;
        load_navigation_row_by_key_query(&connection, stable_key)
    }

    #[allow(dead_code)]
    pub(crate) fn annotate_filesystem_paths(
        &self,
        absolute_paths: &[String],
    ) -> LibrarySqliteResult<Vec<FilesystemPathLibraryAnnotation>> {
        if absolute_paths.is_empty() {
            return Ok(Vec::new());
        }

        let connection = open_connection(&self.path)?;
        let native_separator = std::path::MAIN_SEPARATOR.to_string();

        let mut requested_values = String::new();
        for index in 0..absolute_paths.len() {
            if index > 0 {
                requested_values.push_str(", ");
            }
            requested_values.push_str("(?)");
        }

        let sql = format!(
            "WITH requested(path) AS (
                 VALUES {requested_values}
             ),
             candidate_paths AS (
                 SELECT requested.path AS requested_path,
                        ls.source_id AS root_id,
                        COALESCE(lss.effective_path, sl.absolute_path) AS root_path,
                        CASE
                            WHEN lower(requested.path) = lower(COALESCE(lss.effective_path, sl.absolute_path)) THEN ''
                            ELSE replace(
                                substr(
                                    requested.path,
                                    length(COALESCE(lss.effective_path, sl.absolute_path))
                                        + CASE
                                            WHEN substr(COALESCE(lss.effective_path, sl.absolute_path), -1, 1) = ? THEN 1
                                            ELSE 2
                                          END
                                ),
                                ?,
                                '/'
                            )
                        END AS relative_path,
                        length(COALESCE(lss.effective_path, sl.absolute_path)) AS root_path_length
                 FROM requested
                 JOIN sources ls
                 JOIN source_locators sl
                   ON sl.source_id = ls.source_id
                 LEFT JOIN source_state lss
                   ON lss.source_id = ls.source_id
                 WHERE COALESCE(lss.effective_path, sl.absolute_path) IS NOT NULL
                   AND (
                        lower(requested.path) = lower(COALESCE(lss.effective_path, sl.absolute_path))
                        OR lower(requested.path) LIKE lower(
                            CASE
                                WHEN substr(COALESCE(lss.effective_path, sl.absolute_path), -1, 1) = ? THEN COALESCE(lss.effective_path, sl.absolute_path)
                                ELSE COALESCE(lss.effective_path, sl.absolute_path) || ?
                            END
                        ) || '%'
                   )
             )
              SELECT requested_path,
                     root_id,
                     relative_path,
                     root_path_length,
                    EXISTS(
                        SELECT 1
                        FROM source_files sf
                        WHERE sf.source_id = candidate_paths.root_id
                          AND sf.presence_state = 'present'
                          AND lower(sf.relative_path) = lower(candidate_paths.relative_path)
                    ) AS has_file,
                    EXISTS(
                        SELECT 1
                        FROM source_directories sd
                        WHERE sd.source_id = candidate_paths.root_id
                          AND sd.presence_state = 'present'
                          AND lower(sd.relative_path) = lower(candidate_paths.relative_path)
                    ) AS has_source_directory,
                    (
                        SELECT observations.media_kind
                        FROM source_files sf
                        JOIN source_file_observations observations
                          ON observations.source_file_id = sf.source_file_id
                        WHERE sf.source_id = candidate_paths.root_id
                          AND sf.presence_state = 'present'
                          AND lower(sf.relative_path) = lower(candidate_paths.relative_path)
                        ORDER BY sf.source_file_id ASC
                        LIMIT 1
                    ) AS media_kind
             FROM candidate_paths
             ORDER BY requested_path ASC,
                      CASE
                          WHEN has_file THEN 0
                          WHEN has_source_directory THEN 1
                          ELSE 3
                      END ASC,
                      root_path_length DESC,
                      root_id ASC"
        );

        let mut params = absolute_paths.to_vec();
        params.push(native_separator.clone());
        params.push(native_separator.clone());
        params.push(native_separator.clone());
        params.push(native_separator);

        let mut statement = connection.prepare(&sql)?;
        let rows = statement
            .query_map(params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut annotations = Vec::new();
        let mut seen_paths = HashSet::<String>::new();
        for (requested_path, root_id, relative_path, has_file, has_source_directory, media_kind) in
            rows
        {
            if !seen_paths.insert(requested_path.clone()) {
                continue;
            }

            let status = if has_file || has_source_directory {
                Some(FilesystemPathLibraryStatus::Indexed { root_id })
            } else {
                None
            };

            if let Some(status) = status {
                let file_class = canonical_file_class_from_file_kind(Some(
                    classify_relative_path_file_kind(&relative_path),
                ))
                .or_else(|| canonical_file_class_from_media_kind(media_kind.as_deref()));
                annotations.push(FilesystemPathLibraryAnnotation {
                    requested_path,
                    status,
                    file_class,
                });
            }
        }

        Ok(annotations)
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_filesystem_paths_to_library_roots(
        &self,
        absolute_paths: &[String],
    ) -> LibrarySqliteResult<Vec<FilesystemPathRootResolution>> {
        if absolute_paths.is_empty() {
            return Ok(Vec::new());
        }

        let connection = open_connection(&self.path)?;
        let native_separator = std::path::MAIN_SEPARATOR.to_string();

        let mut requested_values = String::new();
        for index in 0..absolute_paths.len() {
            if index > 0 {
                requested_values.push_str(", ");
            }
            requested_values.push_str("(?)");
        }

        let sql = format!(
            "WITH requested(path) AS (
                 VALUES {requested_values}
             ),
             candidate_paths AS (
                 SELECT requested.path AS requested_path,
                        ls.source_id AS root_id,
                        CASE
                            WHEN lower(requested.path) = lower(COALESCE(lss.effective_path, sl.absolute_path)) THEN ''
                            ELSE replace(
                                substr(
                                    requested.path,
                                    length(COALESCE(lss.effective_path, sl.absolute_path))
                                        + CASE
                                            WHEN substr(COALESCE(lss.effective_path, sl.absolute_path), -1, 1) = ? THEN 1
                                            ELSE 2
                                          END
                                ),
                                ?,
                                '/'
                            )
                        END AS relative_path,
                        length(COALESCE(lss.effective_path, sl.absolute_path)) AS root_path_length
                 FROM requested
                 JOIN sources ls
                 JOIN source_locators sl
                   ON sl.source_id = ls.source_id
                 LEFT JOIN source_state lss
                   ON lss.source_id = ls.source_id
                 WHERE COALESCE(lss.effective_path, sl.absolute_path) IS NOT NULL
                   AND (
                        lower(requested.path) = lower(COALESCE(lss.effective_path, sl.absolute_path))
                        OR lower(requested.path) LIKE lower(
                            CASE
                                WHEN substr(COALESCE(lss.effective_path, sl.absolute_path), -1, 1) = ? THEN COALESCE(lss.effective_path, sl.absolute_path)
                                ELSE COALESCE(lss.effective_path, sl.absolute_path) || ?
                            END
                        ) || '%'
                   )
             )
             SELECT requested_path, root_id, relative_path
             FROM candidate_paths
             ORDER BY requested_path ASC, root_path_length DESC, root_id ASC"
        );

        let mut params = absolute_paths.to_vec();
        params.push(native_separator.clone());
        params.push(native_separator.clone());
        params.push(native_separator.clone());
        params.push(native_separator);

        let mut statement = connection.prepare(&sql)?;
        let rows = statement
            .query_map(params_from_iter(params.iter()), |row| {
                Ok(FilesystemPathRootResolution {
                    requested_path: row.get(0)?,
                    root_id: row.get(1)?,
                    relative_path: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut resolutions = Vec::new();
        let mut seen_paths = HashSet::<String>::new();
        for row in rows {
            if !seen_paths.insert(row.requested_path.clone()) {
                continue;
            }
            resolutions.push(row);
        }

        Ok(resolutions)
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_library_navigation_target_for_file_path(
        &self,
        root_id: i64,
        relative_path: &str,
    ) -> LibrarySqliteResult<Option<LibraryNavigationPathTarget>> {
        if relative_path.is_empty() {
            return Ok(Some(LibraryNavigationPathTarget {
                root_id,
                parent_item_key: None,
            }));
        }

        let connection = open_connection(&self.path)?;
        connection
            .query_row(
                "SELECT sf.parent_source_directory_id
                 FROM source_files sf
                 WHERE sf.source_id = ?1
                   AND sf.presence_state = 'present'
                   AND lower(sf.relative_path) = lower(?2)
                 ORDER BY sf.source_file_id ASC
                 LIMIT 1",
                params![root_id, relative_path],
                |row| {
                    Ok(LibraryNavigationPathTarget {
                        root_id,
                        parent_item_key: row.get(0)?,
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_library_navigation_target_for_folder_path(
        &self,
        root_id: i64,
        relative_path: &str,
    ) -> LibrarySqliteResult<Option<LibraryNavigationPathTarget>> {
        if relative_path.is_empty() {
            return Ok(Some(LibraryNavigationPathTarget {
                root_id,
                parent_item_key: None,
            }));
        }

        let connection = open_connection(&self.path)?;
        connection
            .query_row(
                "SELECT sd.source_directory_id
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.presence_state = 'present'
                   AND lower(sd.relative_path) = lower(?2)
                 ORDER BY sd.source_directory_id ASC
                 LIMIT 1",
                params![root_id, relative_path],
                |row| {
                    Ok(LibraryNavigationPathTarget {
                        root_id,
                        parent_item_key: Some(row.get(0)?),
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)
    }
}
