use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params, params_from_iter};

use crate::browse_media::{
    BrowseMediaClass, canonical_media_class_from_file_kind, canonical_media_class_from_media_kind,
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
    Attached { track_id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct FilesystemPathLibraryAnnotation {
    pub requested_path: String,
    pub status: FilesystemPathLibraryStatus,
    pub media_class: Option<BrowseMediaClass>,
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
    pub parent_node_id: Option<i64>,
    pub track_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct TrackFilesystemPathTarget {
    pub directory_path: String,
    pub file_path: String,
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
                     (
                        SELECT pbr.library_asset_id
                        FROM source_files sf
                        JOIN LibraryBrowserRows pbr
                          ON pbr.primary_source_file_id = sf.source_file_id
                        WHERE sf.source_id = candidate_paths.root_id
                          AND sf.presence_state = 'present'
                          AND lower(sf.relative_path) = lower(candidate_paths.relative_path)
                        ORDER BY pbr.library_asset_id ASC
                        LIMIT 1
                    ) AS attached_track_id,
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
                        SELECT sfacts.media_kind
                        FROM source_files sf
                        JOIN SourceFacts sfacts
                          ON sfacts.source_file_id = sf.source_file_id
                        WHERE sf.source_id = candidate_paths.root_id
                          AND sf.presence_state = 'present'
                          AND lower(sf.relative_path) = lower(candidate_paths.relative_path)
                        ORDER BY sf.source_file_id ASC
                        LIMIT 1
                    ) AS media_kind
             FROM candidate_paths
             ORDER BY requested_path ASC,
                      CASE
                          WHEN attached_track_id IS NOT NULL THEN 0
                          WHEN has_file THEN 1
                          WHEN has_source_directory THEN 2
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
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut annotations = Vec::new();
        let mut seen_paths = HashSet::<String>::new();
        for (
            requested_path,
            root_id,
            relative_path,
            attached_track_id,
            has_file,
            has_source_directory,
            media_kind,
        ) in rows
        {
            if !seen_paths.insert(requested_path.clone()) {
                continue;
            }

            let status = if let Some(track_id) = attached_track_id {
                Some(FilesystemPathLibraryStatus::Attached { track_id })
            } else if has_file || has_source_directory {
                Some(FilesystemPathLibraryStatus::Indexed { root_id })
            } else {
                None
            };

            if let Some(status) = status {
                let media_class = canonical_media_class_from_file_kind(Some(
                    classify_relative_path_file_kind(&relative_path),
                ))
                .or_else(|| canonical_media_class_from_media_kind(media_kind.as_deref()));
                annotations.push(FilesystemPathLibraryAnnotation {
                    requested_path,
                    status,
                    media_class,
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
                parent_node_id: None,
                track_id: None,
            }));
        }

        let connection = open_connection(&self.path)?;
        connection
            .query_row(
                "SELECT sf.parent_source_directory_id,
                        (
                            SELECT pbr.library_asset_id
                            FROM LibraryBrowserRows pbr
                            WHERE pbr.primary_source_file_id = sf.source_file_id
                            ORDER BY pbr.library_asset_id ASC
                            LIMIT 1
                        ) AS track_id
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
                        parent_node_id: row.get(0)?,
                        track_id: row.get(1)?,
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
                parent_node_id: None,
                track_id: None,
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
                        parent_node_id: Some(row.get(0)?),
                        track_id: None,
                    })
                },
            )
            .optional()
            .map_err(LibrarySqliteError::from)
    }

    #[allow(dead_code)]
    pub(crate) fn resolve_track_filesystem_path_target(
        &self,
        track_id: i64,
    ) -> LibrarySqliteResult<Option<TrackFilesystemPathTarget>> {
        let connection = open_connection(&self.path)?;
        let Some((root_path, relative_path)) = connection
            .query_row(
                "SELECT COALESCE(lss.effective_path, sl.absolute_path) AS root_path,
                        sf.relative_path
                 FROM LibraryBrowserRows pbr
                 JOIN source_files sf
                   ON sf.source_file_id = pbr.primary_source_file_id
                 LEFT JOIN source_locators sl
                   ON sl.source_id = sf.source_id
                 LEFT JOIN source_state lss
                   ON lss.source_id = sf.source_id
                 WHERE pbr.library_asset_id = ?1
                   AND sf.presence_state = 'present'
                 ORDER BY pbr.library_asset_id ASC, sf.source_file_id ASC
                 LIMIT 1",
                [track_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        else {
            return Ok(None);
        };

        let Some(root_path) = root_path else {
            return Ok(None);
        };

        let file_path = join_root_relative_path(&root_path, &relative_path);
        let directory_path = file_path
            .parent()
            .map(normalize_system_path)
            .unwrap_or(root_path);

        Ok(Some(TrackFilesystemPathTarget {
            directory_path,
            file_path: normalize_system_path(&file_path),
        }))
    }
}

fn join_root_relative_path(root_path: &str, relative_path: &str) -> PathBuf {
    let mut path = PathBuf::from(root_path);
    for segment in relative_path.split('/') {
        if segment.is_empty() {
            continue;
        }
        path.push(segment);
    }
    path
}

fn normalize_system_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy().into_owned();
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
        value
    }

    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}
