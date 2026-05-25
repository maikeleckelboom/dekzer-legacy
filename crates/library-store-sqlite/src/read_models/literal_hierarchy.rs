use rusqlite::{Connection, OptionalExtension, params};

use crate::browse_media::{SourceFileVisibility, source_file_visibility_predicate_sql};
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
    pub recursive_scope_complete: bool,
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
    pub media_class: Option<String>,
    pub presence_state: String,
    pub size_bytes: Option<i64>,
    pub modified_at_ns: Option<i64>,
    pub updated_at: i64,
    pub has_child_directories: Option<bool>,
    pub has_primary_media_descendant: Option<bool>,
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
    source_file_visibility: SourceFileVisibility,
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
        source_file_visibility,
    )?;
    coverage.empty_result_authoritative =
        coverage.state == StoreLiteralHierarchyCoverageState::Complete && total_rows == 0;
    let rows = read_child_rows(
        connection,
        anchor.source_id,
        anchor.effective_parent_source_directory_id,
        offset,
        limit,
        source_file_visibility,
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
    recursive_scope_complete: bool,
    detail: &str,
) -> StoreLiteralHierarchyCoverage {
    StoreLiteralHierarchyCoverage {
        state,
        recursive_scope_complete,
        empty_result_authoritative: false,
        detail: Some(detail.to_string()),
    }
}

fn read_child_count(
    connection: &Connection,
    source_id: i64,
    parent_source_directory_id: Option<i64>,
    source_file_visibility: SourceFileVisibility,
) -> LibrarySqliteResult<usize> {
    let directory_visibility_predicate = directory_visibility_predicate_sql(source_file_visibility);
    let file_visibility_predicate = source_file_visibility_predicate_sql(source_file_visibility);
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
    source_file_visibility: SourceFileVisibility,
) -> LibrarySqliteResult<Vec<StoreLiteralHierarchyNode>> {
    let offset = i64::try_from(offset).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy offset does not fit i64".to_string())
    })?;
    let limit = i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::WriteInvariant("literal hierarchy limit does not fit i64".to_string())
    })?;
    let directory_visibility_predicate = directory_visibility_predicate_sql(source_file_visibility);
    let file_visibility_predicate = source_file_visibility_predicate_sql(source_file_visibility);
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
                has_primary_media_descendant,
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
                    NULL AS media_class,
                    presence_state,
                    NULL AS size_bytes,
                    NULL AS modified_at_ns,
                    updated_at,
                    has_child_directories,
                    has_primary_media_descendant,
                    has_image_media_descendant,
                    dir_scan_state
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
                    media_class,
                    presence_state,
                    size_bytes,
                    mtime_ns AS modified_at_ns,
                    updated_at,
                    NULL AS has_child_directories,
                    NULL AS has_primary_media_descendant,
                    NULL AS has_image_media_descendant,
                    NULL AS dir_scan_state
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
                    has_primary_media_descendant: row.get(13)?,
                    has_image_media_descendant: row.get(14)?,
                    dir_scan_state: row.get(15)?,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn directory_visibility_predicate_sql(source_file_visibility: SourceFileVisibility) -> String {
    let revealable_descendant_predicate = match source_file_visibility {
        SourceFileVisibility::Performance => "has_primary_media_descendant = 1",
        SourceFileVisibility::PerformanceAndImages => {
            "(has_primary_media_descendant = 1 OR has_image_media_descendant = 1)"
        }
    };

    format!(
        "(dir_scan_state <> 'complete'
          OR {revealable_descendant_predicate}
          OR EXISTS (
              SELECT 1
              FROM source_directories descendant
              WHERE descendant.source_id = source_directories.source_id
                AND descendant.presence_state = 'present'
                AND descendant.relative_path COLLATE BINARY >= source_directories.relative_path || '/'
                AND descendant.relative_path COLLATE BINARY < source_directories.relative_path || {RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL}
                AND descendant.dir_scan_state <> 'complete'
          ))"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        StoreLiteralHierarchyCoverageState, StoreLiteralHierarchyEntryPoint, read_children,
    };
    use crate::SourceFileVisibility;
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

    struct DirectoryFacts {
        has_child_directories: bool,
        has_primary_media_descendant: bool,
        has_image_media_descendant: bool,
    }

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        facts: DirectoryFacts,
        dir_scan_state: &str,
        dir_scan_issue_kind: Option<&str>,
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
                     has_primary_media_descendant,
                     has_image_media_descendant,
                     dir_scan_state,
                     dir_scan_issue_kind,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 7, ?2, ?3, ?3, 'present', ?4, ?5, ?6, ?7, ?8, 1, 1, 1)",
                params![
                    source_directory_id,
                    parent_source_directory_id,
                    name,
                    facts.has_child_directories,
                    facts.has_primary_media_descendant,
                    facts.has_image_media_descendant,
                    dir_scan_state,
                    dir_scan_issue_kind,
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
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: true,
                has_primary_media_descendant: false,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked
        );
        assert!(!window.coverage.recursive_scope_complete);
        assert!(!window.coverage.empty_result_authoritative);
    }

    #[test]
    fn performance_file_rows_include_primary_media_only() {
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
            SourceFileVisibility::Performance,
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
            vec![("clip.mp4", Some("video")), ("track.flac", Some("audio"))]
        );
    }

    #[test]
    fn performance_and_images_file_rows_include_image_media() {
        let connection = test_connection();
        insert_source(&connection, 7);

        for (source_file_id, name, media_class) in [
            (11, "track.flac", "audio"),
            (12, "clip.mp4", "video"),
            (13, "cover.jpg", "image"),
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
            SourceFileVisibility::PerformanceAndImages,
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
            SourceFileVisibility::Performance,
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
            SourceFileVisibility::PerformanceAndImages,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(window.total_rows, 3);
        assert_eq!(window.rows.len(), 3);
    }

    #[test]
    fn performance_paginates_over_primary_media_files_only() {
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
            SourceFileVisibility::Performance,
        )
        .expect("read first literal hierarchy page")
        .expect("source window");
        let second_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            1,
            1,
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read literal hierarchy root")
        .expect("source window");
        let directory_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::PerformanceAndImages,
        )
        .expect("read literal hierarchy root")
        .expect("source window");
        let directory_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            Some(21),
            0,
            10,
            SourceFileVisibility::PerformanceAndImages,
        )
        .expect("read literal hierarchy directory")
        .expect("directory window");

        assert_eq!(root_window.total_rows, 1);
        assert_eq!(root_window.rows[0].display_name, "Covers");
        assert_eq!(directory_window.total_rows, 1);
        assert_eq!(
            directory_window.rows[0].media_class.as_deref(),
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert!(!window.coverage.empty_result_authoritative);
        assert!(!window.coverage.recursive_scope_complete);
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: true,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read literal hierarchy")
        .expect("source window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked
        );
        assert!(!window.coverage.recursive_scope_complete);
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
            DirectoryFacts {
                has_child_directories: true,
                has_primary_media_descendant: false,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read Music/Good hierarchy")
        .expect("Music/Good window");

        assert_eq!(
            good_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "clean sibling directory under partial source must have complete coverage"
        );
        assert!(
            good_window.coverage.recursive_scope_complete,
            "clean sibling directory under partial source must have recursiveScopeComplete = true"
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
            SourceFileVisibility::Performance,
        )
        .expect("read Music/Locked hierarchy")
        .expect("Music/Locked window");

        assert_eq!(
            locked_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Blocked,
            "blocked subtree must remain blocked"
        );
        assert!(!locked_window.coverage.recursive_scope_complete);
        assert!(!locked_window.coverage.empty_result_authoritative);

        let source_window = read_children(
            &connection,
            StoreLiteralHierarchyEntryPoint::Source { source_id: 7 },
            None,
            0,
            10,
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: true,
                has_primary_media_descendant: true,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: true,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read Music/Good hierarchy")
        .expect("Music/Good window");

        assert_eq!(
            good_window.coverage.state,
            StoreLiteralHierarchyCoverageState::Complete,
            "clean sibling directory with media must have complete coverage"
        );
        assert!(good_window.coverage.recursive_scope_complete);
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
            SourceFileVisibility::Performance,
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
            DirectoryFacts {
                has_child_directories: true,
                has_primary_media_descendant: false,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
        )
        .expect("read literal hierarchy")
        .expect("source location window");

        assert_eq!(
            window.coverage.state,
            StoreLiteralHierarchyCoverageState::LocationMissing,
            "Music/DeletedFolder must be locationMissing even under partial scan when parent is complete"
        );
        assert!(!window.coverage.recursive_scope_complete);
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
            DirectoryFacts {
                has_child_directories: true,
                has_primary_media_descendant: false,
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
            DirectoryFacts {
                has_child_directories: false,
                has_primary_media_descendant: false,
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
            SourceFileVisibility::Performance,
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
}
