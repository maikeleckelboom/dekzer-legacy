use rusqlite::{Connection, OptionalExtension, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLiteralHierarchyEntryPoint {
    Source { source_id: i64 },
    SourceLocation { source_location_id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLiteralHierarchyWindow {
    pub entry_point: StoreLiteralHierarchyEntryPoint,
    pub parent_source_directory_id: Option<i64>,
    pub offset: usize,
    pub limit: usize,
    pub total_rows: usize,
    pub rows: Vec<StoreLiteralHierarchyNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLiteralHierarchyNode {
    pub node_kind: String,
    pub source_id: i64,
    pub source_directory_id: Option<i64>,
    pub source_file_id: Option<i64>,
    pub parent_source_directory_id: Option<i64>,
    pub relative_path: String,
    pub display_name: String,
    pub media_class: Option<String>,
    pub presence_state: String,
    pub size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub updated_at: i64,
    pub has_child_directories: Option<bool>,
    pub has_media_descendant: Option<bool>,
    pub dir_scan_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadAnchor {
    source_id: i64,
    effective_parent_source_directory_id: Option<i64>,
    empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLocationAnchor {
    source_id: i64,
    relative_path: String,
}

const DEFAULT_BROWSE_FILE_PREDICATE_SQL: &str = "media_class IN ('audio', 'video', 'image')";

pub(crate) fn read_children(
    connection: &Connection,
    entry_point: StoreLiteralHierarchyEntryPoint,
    parent_source_directory_id: Option<i64>,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<Option<StoreLiteralHierarchyWindow>> {
    let Some(anchor) = resolve_read_anchor(connection, entry_point, parent_source_directory_id)?
    else {
        return Ok(None);
    };

    if anchor.empty {
        return Ok(Some(StoreLiteralHierarchyWindow {
            entry_point,
            parent_source_directory_id,
            offset,
            limit,
            total_rows: 0,
            rows: Vec::new(),
        }));
    }

    let total_rows = read_child_count(
        connection,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
    )?;
    let rows = read_child_rows(
        connection,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
        offset,
        limit,
    )?;

    Ok(Some(StoreLiteralHierarchyWindow {
        entry_point,
        parent_source_directory_id,
        offset,
        limit,
        total_rows,
        rows,
    }))
}

fn resolve_read_anchor(
    connection: &Connection,
    entry_point: StoreLiteralHierarchyEntryPoint,
    parent_source_directory_id: Option<i64>,
) -> LibrarySqliteResult<Option<ReadAnchor>> {
    match entry_point {
        StoreLiteralHierarchyEntryPoint::Source { source_id } => {
            if !source_exists(connection, source_id)? {
                return Ok(None);
            }
            if let Some(parent_source_directory_id) = parent_source_directory_id
                && !directory_belongs_to_source(connection, source_id, parent_source_directory_id)?
            {
                return Ok(None);
            }

            Ok(Some(ReadAnchor {
                source_id,
                effective_parent_source_directory_id: parent_source_directory_id,
                empty: false,
            }))
        }
        StoreLiteralHierarchyEntryPoint::SourceLocation { source_location_id } => {
            let Some(source_location) = load_source_location(connection, source_location_id)?
            else {
                return Ok(None);
            };

            let Some(base_directory_id) = directory_id_for_relative_path(
                connection,
                source_location.source_id,
                &source_location.relative_path,
            )?
            else {
                return Ok(Some(ReadAnchor {
                    source_id: source_location.source_id,
                    effective_parent_source_directory_id: None,
                    empty: true,
                }));
            };

            let effective_parent_source_directory_id = match parent_source_directory_id {
                Some(parent_source_directory_id) => {
                    if !directory_is_within_source_location(
                        connection,
                        source_location.source_id,
                        parent_source_directory_id,
                        &source_location.relative_path,
                    )? {
                        return Ok(None);
                    }
                    Some(parent_source_directory_id)
                }
                None => Some(base_directory_id),
            };

            Ok(Some(ReadAnchor {
                source_id: source_location.source_id,
                effective_parent_source_directory_id,
                empty: false,
            }))
        }
    }
}

fn source_exists(connection: &Connection, source_id: i64) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM sources
                 WHERE source_id = ?1
             )",
            [source_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
        .map_err(Into::into)
}

fn load_source_location(
    connection: &Connection,
    source_location_id: i64,
) -> LibrarySqliteResult<Option<SourceLocationAnchor>> {
    connection
        .query_row(
            "SELECT source_id,
                    relative_path
             FROM source_locations
             WHERE source_location_id = ?1
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1",
            [source_location_id],
            |row| {
                Ok(SourceLocationAnchor {
                    source_id: row.get(0)?,
                    relative_path: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn directory_id_for_relative_path(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<Option<i64>> {
    connection
        .query_row(
            "SELECT source_directory_id
             FROM source_directories
             WHERE source_id = ?1
               AND relative_path = ?2
               AND presence_state = 'present'
             ORDER BY source_directory_id ASC
             LIMIT 1",
            params![source_id, relative_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn directory_belongs_to_source(
    connection: &Connection,
    source_id: i64,
    source_directory_id: i64,
) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM source_directories
                 WHERE source_id = ?1
                   AND source_directory_id = ?2
                   AND presence_state = 'present'
             )",
            params![source_id, source_directory_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
        .map_err(Into::into)
}

fn directory_is_within_source_location(
    connection: &Connection,
    source_id: i64,
    source_directory_id: i64,
    source_location_relative_path: &str,
) -> LibrarySqliteResult<bool> {
    let Some(relative_path) = connection
        .query_row(
            "SELECT relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND source_directory_id = ?2
               AND presence_state = 'present'",
            params![source_id, source_directory_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    else {
        return Ok(false);
    };

    Ok(relative_path == source_location_relative_path
        || relative_path
            .strip_prefix(source_location_relative_path)
            .is_some_and(|suffix| suffix.starts_with('/')))
}

fn read_child_count(
    connection: &Connection,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
) -> LibrarySqliteResult<usize> {
    let count = connection.query_row(
        &format!(
            "SELECT (
             SELECT COUNT(*)
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND (
                    (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
         ) + (
             SELECT COUNT(*)
             FROM source_files
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND {DEFAULT_BROWSE_FILE_PREDICATE_SQL}
               AND (
                    (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
         )"
        ),
        params![source_id, parent_source_directory_id],
        |row| row.get::<_, i64>(0),
    )?;
    usize::try_from(count).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "literal hierarchy child count does not fit usize: {count}"
        ))
    })
}

fn read_child_rows(
    connection: &Connection,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<Vec<StoreLiteralHierarchyNode>> {
    let offset = i64::try_from(offset).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy offset does not fit i64".to_string())
    })?;
    let limit = i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy limit does not fit i64".to_string())
    })?;
    let mut statement = connection.prepare(&format!(
        "SELECT node_kind,
                source_id,
                source_directory_id,
                source_file_id,
                parent_source_directory_id,
                relative_path,
                display_name,
                media_class,
                presence_state,
                size_bytes,
                modified_at_ns,
                updated_at,
                has_child_directories,
                has_media_descendant,
                dir_scan_state
         FROM (
             SELECT 0 AS sort_kind,
                    'directory' AS node_kind,
                    source_id,
                    source_directory_id,
                    NULL AS source_file_id,
                    parent_source_directory_id,
                    relative_path,
                    name AS display_name,
                    NULL AS media_class,
                    presence_state,
                    NULL AS size_bytes,
                    NULL AS modified_at_ns,
                    updated_at,
                    has_child_directories,
                    has_media_descendant,
                    dir_scan_state
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND (
                    (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
             UNION ALL
             SELECT 1 AS sort_kind,
                    'file' AS node_kind,
                    source_id,
                    NULL AS source_directory_id,
                    source_file_id,
                    parent_source_directory_id,
                    relative_path,
                    name AS display_name,
                    media_class,
                    presence_state,
                    size_bytes,
                    mtime_ns AS modified_at_ns,
                    updated_at,
                    NULL AS has_child_directories,
                    NULL AS has_media_descendant,
                    NULL AS dir_scan_state
             FROM source_files
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND {DEFAULT_BROWSE_FILE_PREDICATE_SQL}
               AND (
                    (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
         )
         ORDER BY sort_kind ASC,
                  lower(display_name) ASC,
                  display_name ASC,
                  relative_path ASC
         LIMIT ?3
         OFFSET ?4"
    ))?;
    let rows = statement
        .query_map(
            params![source_id, parent_source_directory_id, limit, offset],
            |row| {
                Ok(StoreLiteralHierarchyNode {
                    node_kind: row.get(0)?,
                    source_id: row.get(1)?,
                    source_directory_id: row.get(2)?,
                    source_file_id: row.get(3)?,
                    parent_source_directory_id: row.get(4)?,
                    relative_path: row.get(5)?,
                    display_name: row.get(6)?,
                    media_class: row.get(7)?,
                    presence_state: row.get(8)?,
                    size_bytes: row.get(9)?,
                    modified_at_ns: row.get(10)?,
                    updated_at: row.get(11)?,
                    has_child_directories: row.get(12)?,
                    has_media_descendant: row.get(13)?,
                    dir_scan_state: row.get(14)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::{StoreLiteralHierarchyEntryPoint, read_children};
    use crate::schema::install_baseline_schema_for_test;
    use rusqlite::{Connection, params};

    fn test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        connection
    }

    fn insert_source(connection: &Connection, source_id: i64) {
        connection
            .execute(
                "INSERT INTO sources (
                     source_id,
                     source_class,
                     authority,
                     identity_key,
                     display_name,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'external_mounted', 'device', ?2, ?3, 1, 1)",
                params![source_id, format!("source:{source_id}"), "Fixture"],
            )
            .expect("insert source");
    }

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        has_child_directories: bool,
        has_media_descendant: bool,
        dir_scan_state: &str,
    ) {
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     relative_path,
                     presence_state,
                     has_child_directories,
                     has_media_descendant,
                     dir_scan_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 7, ?2, ?3, ?3, 'present', ?4, ?5, ?6, 1, 1, 1)",
                params![
                    source_directory_id,
                    parent_source_directory_id,
                    name,
                    has_child_directories,
                    has_media_descendant,
                    dir_scan_state
                ],
            )
            .expect("insert source directory");
    }

    fn insert_file(connection: &Connection, source_file_id: i64, name: &str, media_class: &str) {
        insert_file_in_directory(connection, source_file_id, None, name, media_class);
    }

    fn insert_file_in_directory(
        connection: &Connection,
        source_file_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        media_class: &str,
    ) {
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     relative_path,
                     media_class,
                     presence_state,
                     first_discovered_at,
                     last_observed_at,
                     last_presence_change_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 7, ?2, ?3, ?3, ?4, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    parent_source_directory_id,
                    name,
                    media_class
                ],
            )
            .expect("insert source file");
    }

    #[test]
    fn default_browse_file_rows_include_product_visible_media_classes() {
        let connection = test_connection();
        insert_source(&connection, 7);

        for (source_file_id, name, media_class) in [
            (11, "track.flac", "audio"),
            (12, "clip.mp4", "video"),
            (13, "cover.mp3", "image"),
            (14, "notes.txt", "unsupported"),
            (15, "mystery", "none"),
        ] {
            insert_file(&connection, source_file_id, name, media_class);
        }

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        let media_classes = window
            .rows
            .iter()
            .map(|row| (row.display_name.as_str(), row.media_class.as_deref()))
            .collect::<Vec<_>>();
        assert_eq!(
            media_classes,
            vec![
                ("clip.mp4", Some("video")),
                ("cover.mp3", Some("image")),
                ("track.flac", Some("audio")),
            ]
        );
    }

    #[test]
    fn default_browse_total_rows_excludes_unsupported_and_none_files() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_file(&connection, 11, "track.flac", "audio");
        insert_file(&connection, 12, "clip.mp4", "video");
        insert_file(&connection, 13, "cover.jpg", "image");
        insert_file(&connection, 14, "notes.txt", "unsupported");
        insert_file(&connection, 15, "mystery", "none");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 3);
        assert_eq!(window.rows.len(), 3);
    }

    #[test]
    fn default_browse_paginates_over_product_visible_files_only() {
        let connection = test_connection();
        insert_source(&connection, 7);

        for index in 0..10 {
            insert_file(
                &connection,
                100 + index,
                &format!("00-hidden-{index:02}.txt"),
                "unsupported",
            );
        }
        insert_file(&connection, 11, "visible-a.wav", "audio");
        insert_file(&connection, 12, "visible-b.mp4", "video");
        insert_file(&connection, 13, "zz-hidden", "none");

        let first_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            1,
        )
        .expect("read first literal hierarchy page")
        .expect("source window");
        let second_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            1,
            1,
        )
        .expect("read second literal hierarchy page")
        .expect("source window");

        assert_eq!(first_window.total_rows, 2);
        assert_eq!(
            first_window
                .rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["visible-a.wav"]
        );
        assert_eq!(second_window.total_rows, 2);
        assert_eq!(
            second_window
                .rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["visible-b.mp4"]
        );
    }

    #[test]
    fn default_browse_directories_remain_visible_when_file_rows_are_hidden() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(&connection, 21, None, "Documents", false, false, "complete");
        insert_file_in_directory(
            &connection,
            31,
            Some(21),
            "Documents/readme.txt",
            "unsupported",
        );

        let root_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
        )
        .expect("read literal hierarchy root")
        .expect("source window");
        let directory_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
        )
        .expect("read literal hierarchy directory")
        .expect("directory window");

        assert_eq!(root_window.total_rows, 1);
        assert_eq!(
            root_window
                .rows
                .iter()
                .map(|row| (row.node_kind.as_str(), row.display_name.as_str()))
                .collect::<Vec<_>>(),
            vec![("directory", "Documents")]
        );
        assert_eq!(directory_window.total_rows, 0);
        assert!(directory_window.rows.is_empty());
    }
}
