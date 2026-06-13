use rusqlite::{Connection, OptionalExtension, params};

use crate::browse_media::{SourceFileClassFilter, source_file_class_filter_predicate_sql};
use crate::read_models::source_location_coverage::{
    SourceLocationCoverage, classify_source_location_coverage,
};
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
    pub coverage: StoreLiteralHierarchyCoverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreLiteralHierarchyCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLiteralHierarchyCoverage {
    pub state: StoreLiteralHierarchyCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
    pub detail: Option<String>,
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
    pub file_class: Option<String>,
    pub presence_state: String,
    pub size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub updated_at: i64,
    pub has_child_directories: Option<bool>,
    pub has_navigable_child_directories: Option<bool>,
    pub has_playable_media_descendant: Option<bool>,
    pub has_image_media_descendant: Option<bool>,
    pub dir_scan_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadAnchor {
    source_id: i64,
    effective_parent_source_directory_id: Option<i64>,
    source_readiness: SourceReadiness,
    location_unavailable_coverage: Option<SourceLocationCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLocationAnchor {
    source_id: i64,
    relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceReadiness {
    source_class: String,
    mount_status: Option<String>,
    access_state: Option<String>,
    access_issue_kind: Option<String>,
    scan_phase: Option<String>,
    scan_issue_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoverageCounts {
    total_directories: i64,
    pending_directories: i64,
    scanning_directories: i64,
    blocked_directories: i64,
    failed_directories: i64,
}

const RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL: &str = "char(48)";

pub(crate) fn read_children(
    connection: &Connection,
    entry_point: StoreLiteralHierarchyEntryPoint,
    parent_source_directory_id: Option<i64>,
    offset: usize,
    limit: usize,
    row_admission: SourceFileClassFilter,
) -> LibrarySqliteResult<Option<StoreLiteralHierarchyWindow>> {
    let Some(anchor) = resolve_read_anchor(connection, entry_point, parent_source_directory_id)?
    else {
        return Ok(None);
    };

    let mut coverage = read_hierarchy_coverage(
        connection,
        &anchor.source_readiness,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
        anchor.location_unavailable_coverage.as_ref(),
    )?;

    if !matches!(coverage.state, StoreLiteralHierarchyCoverageState::Complete)
        && matches!(
            coverage.state,
            StoreLiteralHierarchyCoverageState::SourceUnavailable
                | StoreLiteralHierarchyCoverageState::LocationMissing
                | StoreLiteralHierarchyCoverageState::Blocked
                | StoreLiteralHierarchyCoverageState::Failed
        )
        && anchor.source_readiness.scan_phase.as_deref() != Some("partial")
    {
        return Ok(Some(StoreLiteralHierarchyWindow {
            entry_point,
            parent_source_directory_id,
            offset,
            limit,
            total_rows: 0,
            rows: Vec::new(),
            coverage,
        }));
    }

    let total_rows = read_child_count(
        connection,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
        row_admission,
    )?;
    coverage.empty_result_authoritative =
        coverage.state == StoreLiteralHierarchyCoverageState::Complete && total_rows == 0;
    let rows = read_child_rows(
        connection,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
        offset,
        limit,
        row_admission,
    )?;

    Ok(Some(StoreLiteralHierarchyWindow {
        entry_point,
        parent_source_directory_id,
        offset,
        limit,
        total_rows,
        rows,
        coverage,
    }))
}

fn resolve_read_anchor(
    connection: &Connection,
    entry_point: StoreLiteralHierarchyEntryPoint,
    parent_source_directory_id: Option<i64>,
) -> LibrarySqliteResult<Option<ReadAnchor>> {
    match entry_point {
        StoreLiteralHierarchyEntryPoint::Source { source_id } => {
            let Some(source_readiness) = load_source_readiness(connection, source_id)? else {
                return Ok(None);
            };
            if let Some(parent_source_directory_id) = parent_source_directory_id
                && !directory_belongs_to_source(connection, source_id, parent_source_directory_id)?
            {
                return Ok(None);
            }

            Ok(Some(ReadAnchor {
                source_id,
                effective_parent_source_directory_id: parent_source_directory_id,
                source_readiness,
                location_unavailable_coverage: None,
            }))
        }
        StoreLiteralHierarchyEntryPoint::SourceLocation { source_location_id } => {
            let Some(source_location) = load_source_location(connection, source_location_id)?
            else {
                return Ok(None);
            };
            let Some(source_readiness) =
                load_source_readiness(connection, source_location.source_id)?
            else {
                return Ok(None);
            };

            let location_coverage = classify_source_location_coverage(
                connection,
                source_location.source_id,
                &source_location.relative_path,
                source_readiness.scan_phase.as_deref(),
            )?;

            if !matches!(location_coverage, SourceLocationCoverage::Present) {
                return Ok(Some(ReadAnchor {
                    source_id: source_location.source_id,
                    effective_parent_source_directory_id: None,
                    source_readiness,
                    location_unavailable_coverage: Some(location_coverage),
                }));
            }

            let Some(base_directory_id) = directory_id_for_relative_path(
                connection,
                source_location.source_id,
                &source_location.relative_path,
            )?
            else {
                return Ok(Some(ReadAnchor {
                    source_id: source_location.source_id,
                    effective_parent_source_directory_id: None,
                    source_readiness,
                    location_unavailable_coverage: Some(SourceLocationCoverage::Missing),
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
                source_readiness,
                location_unavailable_coverage: None,
            }))
        }
    }
}

fn load_source_readiness(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<SourceReadiness>> {
    connection
        .query_row(
            "SELECT s.source_class,
                    ss.mount_status,
                    ss.access_state,
                    ss.access_issue_kind,
                    sss.scan_phase,
                    sss.scan_issue_kind
             FROM sources s
             LEFT JOIN source_state ss
               ON ss.source_id = s.source_id
             LEFT JOIN source_scan_state sss
               ON sss.source_id = s.source_id
             WHERE s.source_id = ?1",
            [source_id],
            |row| {
                Ok(SourceReadiness {
                    source_class: row.get(0)?,
                    mount_status: row.get(1)?,
                    access_state: row.get(2)?,
                    access_issue_kind: row.get(3)?,
                    scan_phase: row.get(4)?,
                    scan_issue_kind: row.get(5)?,
                })
            },
        )
        .optional()
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

fn read_hierarchy_coverage(
    connection: &Connection,
    source: &SourceReadiness,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
    location_unavailable_coverage: Option<&SourceLocationCoverage>,
) -> LibrarySqliteResult<StoreLiteralHierarchyCoverage> {
    if let Some(coverage) = source_readiness_coverage(source) {
        return Ok(coverage);
    }
    if let Some(location_coverage) = location_unavailable_coverage {
        return Ok(match location_coverage {
            SourceLocationCoverage::Missing => literal_coverage(
                StoreLiteralHierarchyCoverageState::LocationMissing,
                false,
                "The selected source location is missing.",
            ),
            SourceLocationCoverage::Blocked => literal_coverage(
                StoreLiteralHierarchyCoverageState::Blocked,
                false,
                "The selected source location is under a blocked subtree.",
            ),
            SourceLocationCoverage::Failed => literal_coverage(
                StoreLiteralHierarchyCoverageState::Failed,
                false,
                "The selected source location is under a failed subtree.",
            ),
            SourceLocationCoverage::Pending => literal_coverage(
                StoreLiteralHierarchyCoverageState::Pending,
                false,
                "The selected source location has pending coverage.",
            ),
            SourceLocationCoverage::Scanning => literal_coverage(
                StoreLiteralHierarchyCoverageState::Scanning,
                false,
                "The selected source location is being scanned.",
            ),
            SourceLocationCoverage::Unknown => literal_coverage(
                pending_or_scanning_coverage_state(source),
                false,
                "The selected source location has unproven coverage.",
            ),
            SourceLocationCoverage::Present => {
                unreachable!("Present is not an unavailable coverage")
            }
        });
    }

    let is_whole_source = parent_source_directory_id.is_none();
    let counts = if let Some(parent_source_directory_id) = parent_source_directory_id {
        let Some((relative_path, presence_state)) =
            load_directory_path_and_presence(connection, source_id, parent_source_directory_id)?
        else {
            return Ok(literal_coverage(
                pending_or_scanning_coverage_state(source),
                false,
                "The selected folder has not been proven present or missing yet.",
            ));
        };
        if presence_state == "missing" {
            return Ok(literal_coverage(
                StoreLiteralHierarchyCoverageState::LocationMissing,
                false,
                "The selected folder is missing.",
            ));
        }
        read_prefix_coverage_counts(connection, source_id, &relative_path)?
    } else {
        read_whole_source_coverage_counts(connection, source_id)?
    };

    Ok(literal_coverage_from_counts(
        source,
        counts,
        is_whole_source,
    ))
}

fn source_readiness_coverage(source: &SourceReadiness) -> Option<StoreLiteralHierarchyCoverage> {
    if source.source_class != "internal"
        && !matches!(source.mount_status.as_deref(), Some("mounted"))
    {
        return Some(literal_coverage(
            StoreLiteralHierarchyCoverageState::SourceUnavailable,
            false,
            "The selected source is unavailable.",
        ));
    }
    match source.access_state.as_deref() {
        Some("missing") => {
            return Some(literal_coverage(
                StoreLiteralHierarchyCoverageState::LocationMissing,
                false,
                "The selected source root is missing.",
            ));
        }
        Some("blocked") => {
            return Some(literal_coverage(
                if source.access_issue_kind.as_deref() == Some("unavailable_mount") {
                    StoreLiteralHierarchyCoverageState::SourceUnavailable
                } else {
                    StoreLiteralHierarchyCoverageState::Blocked
                },
                false,
                "The selected source root is blocked.",
            ));
        }
        _ => {}
    }
    match source.scan_phase.as_deref() {
        Some("blocked") => Some(literal_coverage(
            if source.scan_issue_kind.as_deref() == Some("unavailable_mount") {
                StoreLiteralHierarchyCoverageState::SourceUnavailable
            } else {
                StoreLiteralHierarchyCoverageState::Blocked
            },
            false,
            "The selected source scan is blocked.",
        )),
        Some("failed") => Some(literal_coverage(
            StoreLiteralHierarchyCoverageState::Failed,
            false,
            "The selected source scan failed.",
        )),
        Some("partial") => None,
        _ => None,
    }
}

fn load_directory_path_and_presence(
    connection: &Connection,
    source_id: i64,
    source_directory_id: i64,
) -> LibrarySqliteResult<Option<(String, String)>> {
    connection
        .query_row(
            "SELECT relative_path, presence_state
             FROM source_directories
             WHERE source_id = ?1
               AND source_directory_id = ?2",
            params![source_id, source_directory_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn read_whole_source_coverage_counts(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<CoverageCounts> {
    connection
        .query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN dir_scan_state = 'failed' THEN 1 ELSE 0 END)
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'",
            [source_id],
            coverage_counts_from_row,
        )
        .map_err(Into::into)
}

fn read_prefix_coverage_counts(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<CoverageCounts> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        SUM(CASE WHEN dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN dir_scan_state = 'failed' THEN 1 ELSE 0 END)
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.presence_state = 'present'
                   AND (
                       sd.relative_path COLLATE BINARY = ?2 COLLATE BINARY
                       OR (
                           sd.relative_path COLLATE BINARY >= ?2 || '/'
                           AND sd.relative_path COLLATE BINARY < ?2 || {RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL}
                       )
                   )"
            ),
            params![source_id, relative_path],
            coverage_counts_from_row,
        )
        .map_err(Into::into)
}

fn coverage_counts_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CoverageCounts> {
    Ok(CoverageCounts {
        total_directories: row.get(0)?,
        pending_directories: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
        scanning_directories: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
        blocked_directories: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
        failed_directories: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
    })
}

fn literal_coverage_from_counts(
    source: &SourceReadiness,
    counts: CoverageCounts,
    is_whole_source: bool,
) -> StoreLiteralHierarchyCoverage {
    if counts.blocked_directories > 0 {
        return literal_coverage(
            StoreLiteralHierarchyCoverageState::Blocked,
            false,
            "Part of the selected hierarchy scope is blocked.",
        );
    }
    if counts.failed_directories > 0 {
        return literal_coverage(
            StoreLiteralHierarchyCoverageState::Failed,
            false,
            "Part of the selected hierarchy scope failed to scan.",
        );
    }
    if counts.scanning_directories > 0 {
        return literal_coverage(
            StoreLiteralHierarchyCoverageState::Scanning,
            false,
            "The selected hierarchy scope is still scanning.",
        );
    }
    let scan_finalized = matches!(
        source.scan_phase.as_deref(),
        Some("complete") | Some("partial")
    );
    if counts.pending_directories > 0 || !scan_finalized {
        return literal_coverage(
            pending_or_scanning_coverage_state(source),
            false,
            "The selected hierarchy scope has incomplete scan coverage.",
        );
    }
    if counts.total_directories == 0 && !scan_finalized {
        return literal_coverage(
            pending_or_scanning_coverage_state(source),
            false,
            "The selected hierarchy scope has not completed scan coverage.",
        );
    }
    if is_whole_source && source.scan_phase.as_deref() == Some("partial") {
        return literal_coverage(
            StoreLiteralHierarchyCoverageState::Pending,
            false,
            "The selected source scan has incomplete descendant coverage.",
        );
    }
    literal_coverage(
        StoreLiteralHierarchyCoverageState::Complete,
        true,
        "The selected hierarchy scope has complete scan coverage.",
    )
}

fn pending_or_scanning_coverage_state(
    source: &SourceReadiness,
) -> StoreLiteralHierarchyCoverageState {
    if source.scan_phase.as_deref() == Some("scanning") {
        StoreLiteralHierarchyCoverageState::Scanning
    } else {
        StoreLiteralHierarchyCoverageState::Pending
    }
}

fn literal_coverage(
    state: StoreLiteralHierarchyCoverageState,
    subtree_coverage_complete: bool,
    detail: &str,
) -> StoreLiteralHierarchyCoverage {
    StoreLiteralHierarchyCoverage {
        state,
        subtree_coverage_complete,
        empty_result_authoritative: false,
        detail: Some(detail.to_string()),
    }
}

fn read_child_count(
    connection: &Connection,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
    row_admission: SourceFileClassFilter,
) -> LibrarySqliteResult<usize> {
    let directory_visibility_predicate =
        directory_visibility_predicate_sql(row_admission, "source_directories");
    let file_visibility_predicate = hierarchy_file_visibility_predicate_sql(row_admission);
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
               AND {directory_visibility_predicate}
         ) + (
             SELECT COUNT(*)
             FROM source_files
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND {file_visibility_predicate}
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
    row_admission: SourceFileClassFilter,
) -> LibrarySqliteResult<Vec<StoreLiteralHierarchyNode>> {
    let offset = i64::try_from(offset).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy offset does not fit i64".to_string())
    })?;
    let limit = i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy limit does not fit i64".to_string())
    })?;
    let directory_visibility_predicate =
        directory_visibility_predicate_sql(row_admission, "source_directories");
    let navigable_child_directories_sql =
        navigable_child_directories_sql(row_admission, "source_directories");
    let file_visibility_predicate = hierarchy_file_visibility_predicate_sql(row_admission);
    let mut statement = connection.prepare(&format!(
        "SELECT node_kind,
                source_id,
                source_directory_id,
                source_file_id,
                parent_source_directory_id,
                relative_path,
                display_name,
                file_class,
                presence_state,
                size_bytes,
                modified_at_ns,
                updated_at,
                has_child_directories,
                has_navigable_child_directories,
                has_playable_media_descendant,
                has_image_media_descendant,
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
                    NULL AS file_class,
                    presence_state,
                    NULL AS size_bytes,
                    NULL AS modified_at_ns,
                    updated_at,
                    has_child_directories,
                    {navigable_child_directories_sql} AS has_navigable_child_directories,
                    has_playable_media_descendant,
                    has_image_media_descendant,
                    dir_scan_state,
                    name_sort_key
             FROM source_directories
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND (
                   (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
               AND {directory_visibility_predicate}
             UNION ALL
             SELECT 1 AS sort_kind,
                    'file' AS node_kind,
                    source_id,
                    NULL AS source_directory_id,
                    source_file_id,
                    parent_source_directory_id,
                    relative_path,
                    name AS display_name,
                    file_class,
                    presence_state,
                    size_bytes,
                    mtime_ns AS modified_at_ns,
                    updated_at,
                    NULL AS has_child_directories,
                    NULL AS has_navigable_child_directories,
                    NULL AS has_playable_media_descendant,
                    NULL AS has_image_media_descendant,
                    NULL AS dir_scan_state,
                    name_sort_key
             FROM source_files
             WHERE source_id = ?1
               AND presence_state = 'present'
               AND {file_visibility_predicate}
               AND (
                    (?2 IS NULL AND parent_source_directory_id IS NULL)
                    OR parent_source_directory_id = ?2
               )
         )
         ORDER BY sort_kind ASC,
                  name_sort_key ASC,
                  display_name ASC,
                  COALESCE(source_directory_id, source_file_id) ASC
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
                    file_class: row.get(7)?,
                    presence_state: row.get(8)?,
                    size_bytes: row.get(9)?,
                    modified_at_ns: row.get(10)?,
                    updated_at: row.get(11)?,
                    has_child_directories: row.get(12)?,
                    has_navigable_child_directories: row.get(13)?,
                    has_playable_media_descendant: row.get(14)?,
                    has_image_media_descendant: row.get(15)?,
                    dir_scan_state: row.get(16)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn hierarchy_file_visibility_predicate_sql(row_admission: SourceFileClassFilter) -> &'static str {
    match row_admission {
        SourceFileClassFilter::AudioDirectories
        | SourceFileClassFilter::NavigationOnly
        | SourceFileClassFilter::PlayableMediaDirectories => "0 = 1",
        SourceFileClassFilter::Audio
        | SourceFileClassFilter::PlayableMedia
        | SourceFileClassFilter::PlayableMediaAndImages => {
            source_file_class_filter_predicate_sql(row_admission)
        }
        SourceFileClassFilter::AllSourceFiles => {
            source_file_class_filter_predicate_sql(row_admission)
        }
    }
}

fn directory_visibility_predicate_sql(row_admission: SourceFileClassFilter, alias: &str) -> String {
    let revealable_descendant_predicate = match row_admission {
        SourceFileClassFilter::Audio | SourceFileClassFilter::AudioDirectories => {
            format!("EXISTS (
                SELECT 1
                FROM source_files descendant_file
                WHERE descendant_file.source_id = {alias}.source_id
                  AND descendant_file.presence_state = 'present'
                  AND descendant_file.file_class = 'audio'
                  AND descendant_file.relative_path COLLATE BINARY >= {alias}.relative_path || '/'
                  AND descendant_file.relative_path COLLATE BINARY < {alias}.relative_path || char(48)
            )")
        }
        SourceFileClassFilter::NavigationOnly
        | SourceFileClassFilter::PlayableMedia
        | SourceFileClassFilter::PlayableMediaDirectories => {
            format!("{alias}.has_playable_media_descendant = 1")
        }
        SourceFileClassFilter::PlayableMediaAndImages => {
            format!(
                "({alias}.has_playable_media_descendant = 1 OR {alias}.has_image_media_descendant = 1)"
            )
        }
        SourceFileClassFilter::AllSourceFiles => {
            return "1 = 1".to_string();
        }
    };

    format!(
        "({alias}.dir_scan_state <> 'complete'
          OR {revealable_descendant_predicate}
          OR EXISTS (
              SELECT 1
              FROM source_directories descendant
              WHERE descendant.source_id = {alias}.source_id
                AND descendant.presence_state = 'present'
                AND descendant.relative_path COLLATE BINARY >= {alias}.relative_path || '/'
                AND descendant.relative_path COLLATE BINARY < {alias}.relative_path || {RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL}
                AND descendant.dir_scan_state <> 'complete'
          ))"
    )
}

fn navigable_child_directories_sql(row_admission: SourceFileClassFilter, alias: &str) -> String {
    if matches!(row_admission, SourceFileClassFilter::AllSourceFiles) {
        return format!(
            "CASE
                 WHEN {alias}.has_child_directories = 1 THEN 1
                 WHEN {alias}.dir_scan_state = 'complete' THEN 0
                 ELSE NULL
             END"
        );
    }

    let child_visibility_predicate =
        directory_visibility_predicate_sql(row_admission, "child_directory");

    format!(
        "CASE
             WHEN {alias}.dir_scan_state <> 'complete' THEN NULL
             WHEN EXISTS (
                 SELECT 1
                 FROM source_directories child_directory
                 WHERE child_directory.source_id = {alias}.source_id
                   AND child_directory.presence_state = 'present'
                   AND child_directory.parent_source_directory_id = {alias}.source_directory_id
                   AND child_directory.dir_scan_state <> 'complete'
             ) THEN NULL
             WHEN EXISTS (
                 SELECT 1
                 FROM source_directories child_directory
                 WHERE child_directory.source_id = {alias}.source_id
                   AND child_directory.presence_state = 'present'
                   AND child_directory.parent_source_directory_id = {alias}.source_directory_id
                   AND child_directory.dir_scan_state = 'complete'
                   AND {child_visibility_predicate}
             ) THEN 1
             ELSE 0
         END"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        StoreLiteralHierarchyCoverageState, StoreLiteralHierarchyEntryPoint, read_children,
    };
    use crate::SourceFileClassFilter;
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
        connection
            .execute(
                "INSERT INTO source_state (
                     source_id,
                     mount_status,
                     mount_epoch,
                     access_state,
                     access_checked_at,
                     effective_path,
                     updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 1)",
                [source_id],
            )
            .expect("insert source state");
        connection
            .execute(
                "INSERT INTO source_scan_state (
                     source_id,
                     scan_phase,
                     last_scan_started_at,
                     last_scan_finished_at,
                     last_successful_scan_at,
                     updated_at
                 )
                 VALUES (?1, 'complete', 1, 2, 2, 2)",
                [source_id],
            )
            .expect("insert source scan state");
    }

    fn set_source_access(
        connection: &Connection,
        source_id: i64,
        access_state: &str,
        issue_kind: Option<&str>,
    ) {
        connection
            .execute(
                "UPDATE source_state
                 SET access_state = ?2,
                     access_issue_kind = ?3,
                     access_checked_at = 10,
                     updated_at = 10
                 WHERE source_id = ?1",
                params![source_id, access_state, issue_kind],
            )
            .expect("update source access");
    }

    fn set_directory_scan_issue(
        connection: &Connection,
        source_directory_id: i64,
        dir_scan_state: &str,
        issue_kind: &str,
    ) {
        connection
            .execute(
                "UPDATE source_directories
                 SET dir_scan_state = ?2,
                     dir_scan_issue_kind = ?3,
                     dir_scan_updated_at = 10,
                     updated_at = 10
                 WHERE source_directory_id = ?1",
                params![source_directory_id, dir_scan_state, issue_kind],
            )
            .expect("update directory scan issue");
    }

    struct DirectoryObservations {
        has_child_directories: bool,
        has_playable_media_descendant: bool,
        has_image_media_descendant: bool,
    }

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        observations: DirectoryObservations,
        dir_scan_state: &str,
        dir_scan_issue_kind: Option<&str>,
    ) {
        let name_sort_key = crate::browse_sort_key::compute_name_sort_key(name);
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     relative_path,
                     presence_state,
                     has_child_directories,
                     has_playable_media_descendant,
                     has_image_media_descendant,
                     dir_scan_state,
                     dir_scan_issue_kind,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 7, ?2, ?3, ?4, ?3, 'present', ?5, ?6, ?7, ?8, ?9, 1, 1, 1)",
                params![
                    source_directory_id,
                    parent_source_directory_id,
                    name,
                    name_sort_key,
                    observations.has_child_directories,
                    observations.has_playable_media_descendant,
                    observations.has_image_media_descendant,
                    dir_scan_state,
                    dir_scan_issue_kind,
                ],
            )
            .expect("insert source directory");
    }

    fn insert_file(connection: &Connection, source_file_id: i64, name: &str, file_class: &str) {
        insert_file_in_directory(connection, source_file_id, None, name, file_class);
    }

    fn insert_media_directories(connection: &Connection, directories: &[(i64, &str)]) {
        for (source_directory_id, name) in directories {
            insert_directory(
                connection,
                *source_directory_id,
                None,
                name,
                DirectoryObservations {
                    has_child_directories: false,
                    has_playable_media_descendant: true,
                    has_image_media_descendant: false,
                },
                "complete",
                None,
            );
        }
    }

    fn insert_audio_files(connection: &Connection, files: &[(i64, &str)]) {
        for (source_file_id, name) in files {
            insert_file(connection, *source_file_id, name, "audio");
        }
    }

    fn assert_display_names(rows: &[super::StoreLiteralHierarchyNode], expected: &[&str]) {
        assert_eq!(
            rows.iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    fn insert_file_in_directory(
        connection: &Connection,
        source_file_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        file_class: &str,
    ) {
        let name_sort_key = crate::browse_sort_key::compute_name_sort_key(name);
        let path_sort_key = crate::browse_sort_key::compute_path_sort_key(name);
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_sort_key,
                     path_sort_key,
                     relative_path,
                     file_class,
                     presence_state,
                     first_discovered_at,
                     last_observed_at,
                     last_presence_change_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 7, ?2, ?3, ?4, ?5, ?3, ?6, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    parent_source_directory_id,
                    name,
                    name_sort_key,
                    path_sort_key,
                    file_class
                ],
            )
            .expect("insert source file");
    }

    #[test]
    fn source_access_block_returns_blocked_hierarchy_window_not_authoritative_empty() {
        let connection = test_connection();
        insert_source(&connection, 7);
        set_source_access(&connection, 7, "blocked", Some("permission_denied"));

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked
        );
        assert_eq!(window.total_rows, 0);
        assert!(window.rows.is_empty());
        assert!(!window.coverage.empty_result_authoritative);
    }

    #[test]
    fn blocked_descendant_prevents_complete_hierarchy_coverage() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 21, "blocked", "permission_denied");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(20),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked
        );
        assert!(!window.coverage.subtree_coverage_complete);
        assert!(!window.coverage.empty_result_authoritative);
    }

    #[test]
    fn performance_file_rows_include_playable_media_only() {
        let connection = test_connection();
        insert_source(&connection, 7);

        for (source_file_id, name, file_class) in [
            (11, "track.flac", "audio"),
            (12, "clip.mp4", "video"),
            (13, "cover.mp3", "image"),
            (14, "notes.txt", "unsupported"),
            (15, "mystery", "none"),
        ] {
            insert_file(&connection, source_file_id, name, file_class);
        }

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        let file_classes = window
            .rows
            .iter()
            .map(|row| (row.display_name.as_str(), row.file_class.as_deref()))
            .collect::<Vec<_>>();
        assert_eq!(
            file_classes,
            vec![("clip.mp4", Some("video")), ("track.flac", Some("audio"))]
        );
    }

    #[test]
    fn audio_tree_hides_complete_video_only_directories() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Audio Album",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 21, Some(20), "Audio Album/track.flac", "audio");
        insert_directory(
            &connection,
            30,
            None,
            "Video Album",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(30), "Video Album/clip.mp4", "video");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::Audio,
        )
        .expect("read audio hierarchy")
        .expect("source window");

        assert_display_names(&window.rows, &["Audio Album"]);
    }

    #[test]
    fn audio_tree_keeps_incomplete_unmatched_directories_pending() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Scanning Videos",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "scanning",
            None,
        );
        insert_file_in_directory(
            &connection,
            21,
            Some(20),
            "Scanning Videos/clip.mp4",
            "video",
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::Audio,
        )
        .expect("read audio hierarchy")
        .expect("source window");

        assert_display_names(&window.rows, &["Scanning Videos"]);
        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Scanning
        );
        assert!(!window.coverage.empty_result_authoritative);
    }

    #[test]
    fn performance_and_images_file_rows_include_image_media() {
        let connection = test_connection();
        insert_source(&connection, 7);

        for (source_file_id, name, file_class) in [
            (11, "track.flac", "audio"),
            (12, "clip.mp4", "video"),
            (13, "cover.jpg", "image"),
            (14, "notes.txt", "unsupported"),
            (15, "mystery", "none"),
        ] {
            insert_file(&connection, source_file_id, name, file_class);
        }

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        let file_classes = window
            .rows
            .iter()
            .map(|row| (row.display_name.as_str(), row.file_class.as_deref()))
            .collect::<Vec<_>>();
        assert_eq!(
            file_classes,
            vec![
                ("clip.mp4", Some("video")),
                ("cover.jpg", Some("image")),
                ("track.flac", Some("audio")),
            ]
        );
    }

    #[test]
    fn performance_total_rows_excludes_image_unsupported_and_none_files() {
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
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 2);
        assert_eq!(window.rows.len(), 2);
    }

    #[test]
    fn performance_and_images_total_rows_excludes_unsupported_and_none_files() {
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
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 3);
        assert_eq!(window.rows.len(), 3);
    }

    #[test]
    fn audio_hierarchy_hides_complete_video_only_child_folders() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Audio",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            22,
            Some(20),
            "Music/Videos",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(21), "Music/Audio/track.flac", "audio");
        insert_file_in_directory(&connection, 32, Some(22), "Music/Videos/clip.mp4", "video");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(20),
            0,
            10,
            SourceFileClassFilter::Audio,
        )
        .expect("read audio hierarchy")
        .expect("Music window");

        assert_eq!(window.total_rows, 1);
        assert_display_names(&window.rows, &["Music/Audio"]);
    }

    #[test]
    fn direct_audio_directory_without_visible_child_folders_is_terminal_scope() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Album",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(20), "Album/track.flac", "audio");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::Audio,
        )
        .expect("read audio hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 1);
        assert_eq!(window.rows[0].display_name, "Album");
        assert_eq!(window.rows[0].has_child_directories, Some(false));
        assert_eq!(window.rows[0].has_navigable_child_directories, Some(false));
    }

    #[test]
    fn audio_hierarchy_keeps_incomplete_child_readiness_unknown() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Pending",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "scanning",
            None,
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::Audio,
        )
        .expect("read audio hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 1);
        assert_eq!(window.rows[0].display_name, "Music");
        assert_eq!(window.rows[0].has_child_directories, Some(true));
        assert_eq!(window.rows[0].has_navigable_child_directories, None);
    }

    #[test]
    fn source_inventory_includes_all_source_file_classes() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_file(&connection, 11, "track.flac", "audio");
        insert_file(&connection, 12, "clip.mp4", "video");
        insert_file(&connection, 13, "cover.jpg", "image");
        insert_file(&connection, 14, "notes.txt", "unsupported");
        insert_file(&connection, 15, "archive.zip", "unsupported");
        insert_file(&connection, 16, "mystery", "none");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::AllSourceFiles,
        )
        .expect("read source inventory hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 5);
        assert_eq!(
            window
                .rows
                .iter()
                .map(|row| (row.display_name.as_str(), row.file_class.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("archive.zip", Some("unsupported")),
                ("clip.mp4", Some("video")),
                ("cover.jpg", Some("image")),
                ("notes.txt", Some("unsupported")),
                ("track.flac", Some("audio")),
            ]
        );
    }

    #[test]
    fn navigation_only_returns_directory_rows_and_counts_navigation_children_only() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            20,
            None,
            "Albums",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            None,
            "Singles",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file(&connection, 11, "track.flac", "audio");
        insert_file(&connection, 12, "clip.mp4", "video");
        insert_file(&connection, 13, "cover.jpg", "image");
        insert_file(&connection, 14, "album.cue", "unsupported");
        insert_file(&connection, 15, "mystery", "none");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::NavigationOnly,
        )
        .expect("read navigation-only hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 2);
        assert_eq!(
            window
                .rows
                .iter()
                .map(|row| (row.node_kind.as_str(), row.display_name.as_str()))
                .collect::<Vec<_>>(),
            vec![("directory", "Albums"), ("directory", "Singles")]
        );
        assert!(window.rows.iter().all(|row| row.source_file_id.is_none()));
        assert!(window.rows.iter().all(|row| row.file_class.is_none()));
    }

    #[test]
    fn performance_paginates_over_playable_media_files_only() {
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
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read first literal hierarchy page")
        .expect("source window");
        let second_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            1,
            1,
            SourceFileClassFilter::PlayableMedia,
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
    fn performance_keeps_incomplete_directories_visible_when_file_rows_are_hidden() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            21,
            None,
            "Documents",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "pending",
            None,
        );
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
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy root")
        .expect("source window");
        let directory_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
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

    #[test]
    fn image_only_complete_folder_is_hidden_in_performance_mode() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            21,
            None,
            "Covers",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: true,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(21), "Covers/front.jpg", "image");

        let root_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy root")
        .expect("source window");

        assert_eq!(root_window.total_rows, 0);
        assert!(root_window.rows.is_empty());
    }

    #[test]
    fn image_only_complete_folder_is_visible_in_performance_and_images_mode() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            21,
            None,
            "Covers",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: true,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(21), "Covers/front.jpg", "image");

        let root_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy root")
        .expect("source window");
        let directory_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy directory")
        .expect("directory window");

        assert_eq!(root_window.total_rows, 1);
        assert_eq!(root_window.rows[0].display_name, "Covers");
        assert_eq!(directory_window.total_rows, 1);
        assert_eq!(
            directory_window.rows[0].file_class.as_deref(),
            Some("image")
        );
    }

    #[test]
    fn partial_scan_phase_does_not_produce_authoritative_empty() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert!(!window.coverage.empty_result_authoritative);
        assert!(!window.coverage.subtree_coverage_complete);
    }

    #[test]
    fn partial_scan_phase_returns_visible_rows_and_blocked_coverage() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(20), "Music/track.flac", "audio");
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 21, "blocked", "permission_denied");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(20),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked
        );
        assert!(!window.coverage.subtree_coverage_complete);
        assert!(!window.coverage.empty_result_authoritative);
        assert!(
            !window.rows.is_empty(),
            "partial scan must return visible rows outside blocked subtree"
        );
    }

    #[test]
    fn clean_sibling_directory_under_partial_source_is_complete() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Good",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            22,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 22, "blocked", "permission_denied");

        let good_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read Music/Good hierarchy")
        .expect("Music/Good window");

        assert_eq!(
            good_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "clean sibling directory under partial source must have complete coverage"
        );
        assert!(
            good_window.coverage.subtree_coverage_complete,
            "clean sibling directory under partial source must have subtreeCoverageComplete = true"
        );
        assert!(
            good_window.coverage.empty_result_authoritative,
            "clean empty sibling directory under partial source may be authoritative empty"
        );

        let locked_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(22),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read Music/Locked hierarchy")
        .expect("Music/Locked window");

        assert_eq!(
            locked_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked,
            "blocked subtree must remain blocked"
        );
        assert!(!locked_window.coverage.subtree_coverage_complete);
        assert!(!locked_window.coverage.empty_result_authoritative);

        let source_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read whole source hierarchy")
        .expect("whole source window");

        assert_ne!(
            source_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "whole source must remain non-complete when partial"
        );
        assert!(!source_window.coverage.empty_result_authoritative);
    }

    #[test]
    fn complete_directory_under_partial_source_with_media_files_returns_rows() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            21,
            Some(20),
            "Music/Good",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file_in_directory(&connection, 31, Some(21), "Music/Good/track.flac", "audio");
        insert_directory(
            &connection,
            22,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 22, "blocked", "permission_denied");

        let good_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read Music/Good hierarchy")
        .expect("Music/Good window");

        assert_eq!(
            good_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "clean sibling directory with media must have complete coverage"
        );
        assert!(good_window.coverage.subtree_coverage_complete);
        assert!(
            !good_window.rows.is_empty(),
            "clean sibling directory must return media rows"
        );

        let source_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read whole source hierarchy")
        .expect("whole source window");

        assert_ne!(
            source_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "whole source must remain non-complete when partial"
        );
    }

    fn insert_location(
        connection: &Connection,
        source_location_id: i64,
        source_id: i64,
        relative_path: &str,
        authority: &str,
        location_kind: &str,
    ) {
        connection
            .execute(
                "INSERT INTO source_locations (
                     source_location_id,
                     source_id,
                     authority,
                     location_kind,
                     relative_path,
                     display_name,
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1, 1, 1)",
                rusqlite::params![
                    source_location_id,
                    source_id,
                    authority,
                    location_kind,
                    relative_path
                ],
            )
            .expect("insert source location");
    }

    #[test]
    fn source_location_hierarchy_missing_proven_under_partial_scan() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            22,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 22, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            7,
            "Music/DeletedFolder",
            "user",
            "registered_subpath",
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::SourceLocation {
                source_location_id: 100,
            },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source location window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::LocationMissing,
            "Music/DeletedFolder must be locationMissing even under partial scan when parent is complete"
        );
        assert!(!window.coverage.subtree_coverage_complete);
    }

    #[test]
    fn source_location_hierarchy_under_blocked_parent_is_blocked_not_missing() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = 'partial', scan_issue_kind = 'permission_denied' WHERE source_id = 7",
                [],
            )
            .expect("set partial scan phase");
        insert_directory(
            &connection,
            20,
            None,
            "Music",
            DirectoryObservations {
                has_child_directories: true,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_directory(
            &connection,
            22,
            Some(20),
            "Music/Locked",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: false,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        set_directory_scan_issue(&connection, 22, "blocked", "permission_denied");
        insert_location(
            &connection,
            100,
            7,
            "Music/Locked/SubFolder",
            "user",
            "registered_subpath",
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::SourceLocation {
                source_location_id: 100,
            },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source location window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked,
            "source location under blocked parent must be blocked, not missing"
        );
        assert!(!window.coverage.empty_result_authoritative);
    }

    #[test]
    fn natural_order_directories_sort_numerically_within_brackets() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_media_directories(&connection, &[(100, "[10]"), (101, "[1]"), (102, "[2]")]);

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_display_names(&window.rows, &["[1]", "[2]", "[10]"]);
    }

    #[test]
    fn natural_order_files_sort_numerically_within_track_names() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_audio_files(
            &connection,
            &[
                (200, "Track 10.wav"),
                (201, "Track 1.wav"),
                (202, "Track 2.wav"),
            ],
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_display_names(
            &window.rows,
            &["Track 1.wav", "Track 2.wav", "Track 10.wav"],
        );
    }

    #[test]
    fn natural_order_mixed_directories_and_files() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_directory(
            &connection,
            100,
            None,
            "[2]",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file(&connection, 200, "[10].wav", "audio");
        insert_directory(
            &connection,
            101,
            None,
            "[1]",
            DirectoryObservations {
                has_child_directories: false,
                has_playable_media_descendant: true,
                has_image_media_descendant: false,
            },
            "complete",
            None,
        );
        insert_file(&connection, 201, "[1].wav", "audio");
        insert_file(&connection, 202, "[2].wav", "audio");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        let kinds_and_names: Vec<(&str, &str)> = window
            .rows
            .iter()
            .map(|row| (row.node_kind.as_str(), row.display_name.as_str()))
            .collect();
        assert_eq!(
            kinds_and_names,
            vec![
                ("directory", "[1]"),
                ("directory", "[2]"),
                ("file", "[1].wav"),
                ("file", "[2].wav"),
                ("file", "[10].wav"),
            ],
            "directories must precede files, each group in natural order"
        );
    }

    #[test]
    fn natural_order_stable_pagination() {
        let connection = test_connection();
        insert_source(&connection, 7);
        for i in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            insert_file(&connection, 100 + i, &format!("Track {}.wav", i), "audio");
        }

        let first = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            5,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read first page")
        .expect("source window");

        let second = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            5,
            5,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read second page")
        .expect("source window");

        let third = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            10,
            5,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read third page")
        .expect("source window");

        assert_eq!(first.total_rows, 12);
        assert_eq!(first.rows.len(), 5);
        assert_eq!(second.rows.len(), 5);
        assert_eq!(third.rows.len(), 2);

        assert_eq!(
            first
                .rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Track 1.wav",
                "Track 2.wav",
                "Track 3.wav",
                "Track 4.wav",
                "Track 5.wav"
            ]
        );
        assert_eq!(
            second
                .rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Track 6.wav",
                "Track 7.wav",
                "Track 8.wav",
                "Track 9.wav",
                "Track 10.wav"
            ]
        );
        assert_eq!(
            third
                .rows
                .iter()
                .map(|row| row.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Track 11.wav", "Track 12.wav"]
        );
    }

    #[test]
    fn natural_order_case_folding_and_tie_stability() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_file(&connection, 201, "track 1.wav", "audio");
        insert_file(&connection, 200, "Track 1.wav", "audio");
        insert_file(&connection, 202, "track 01.wav", "audio");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 3);
        let names: Vec<&str> = window
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect();
        assert_eq!(
            names[0], "Track 1.wav",
            "shorter digit string sorts first, display_name tie-break"
        );
        assert!(
            names.contains(&"track 1.wav"),
            "track 1.wav must be in results"
        );
        assert_eq!(
            names[2], "track 01.wav",
            "longer digit string sorts last (leading zero tie-break)"
        );
    }

    #[test]
    fn natural_order_directories_leading_zero_order() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_media_directories(
            &connection,
            &[
                (100, "[10]"),
                (101, "[01]"),
                (102, "[2]"),
                (103, "[001]"),
                (104, "[1]"),
            ],
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMediaAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_display_names(&window.rows, &["[1]", "[01]", "[001]", "[2]", "[10]"]);
    }

    #[test]
    fn natural_order_files_leading_zero_order() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_audio_files(
            &connection,
            &[
                (200, "Track 001.wav"),
                (201, "Track 01.wav"),
                (202, "Track 10.wav"),
                (203, "Track 2.wav"),
                (204, "Track 1.wav"),
            ],
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_display_names(
            &window.rows,
            &[
                "Track 1.wav",
                "Track 01.wav",
                "Track 001.wav",
                "Track 2.wav",
                "Track 10.wav",
            ],
        );
    }

    #[test]
    fn natural_order_large_numbers() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_audio_files(
            &connection,
            &[
                (200, "Track 999999999999999999999999999999.wav"),
                (201, "Track 10.wav"),
                (202, "Track 9.wav"),
            ],
        );

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_display_names(
            &window.rows,
            &[
                "Track 9.wav",
                "Track 10.wav",
                "Track 999999999999999999999999999999.wav",
            ],
        );
    }

    #[test]
    fn natural_order_padded_zero_sort_by_numeric_value() {
        let connection = test_connection();
        insert_source(&connection, 7);
        insert_file(
            &connection,
            200,
            "Track 000000000000000000000000000009.wav",
            "audio",
        );
        insert_file(&connection, 201, "Track 10.wav", "audio");
        insert_file(&connection, 202, "Track 9.wav", "audio");

        let window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileClassFilter::PlayableMedia,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        let names: Vec<&str> = window
            .rows
            .iter()
            .map(|row| row.display_name.as_str())
            .collect();
        assert_eq!(
            names[0], "Track 9.wav",
            "Track 9 must sort first (shorter original digit length)"
        );
        assert_eq!(
            names[1], "Track 000000000000000000000000000009.wav",
            "padded zeros have numeric value 9, sort after Track 9 (longer original length)"
        );
        assert_eq!(
            names[2], "Track 10.wav",
            "Track 10 must sort after the 9-valued runs"
        );
    }
}
