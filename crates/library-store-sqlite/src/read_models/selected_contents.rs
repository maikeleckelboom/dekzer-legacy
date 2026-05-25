use rusqlite::{Connection, OptionalExtension, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreSelectedContentsScope {
    Source {
        source_id: i64,
    },
    SourceLocation {
        source_location_id: i64,
    },
    Directory {
        source_id: i64,
        source_directory_id: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSelectedContentsState {
    Ready,
    Empty,
    Partial,
    SourceUnavailable,
    LocationMissing,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSelectedContentsCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSelectedContentsCoverage {
    pub state: StoreSelectedContentsCoverageState,
    pub recursive_scope_complete: bool,
    pub empty_result_authoritative: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreSelectedContentsResult {
    pub state: StoreSelectedContentsState,
    pub scope: StoreSelectedContentsScope,
    pub rows: Vec<StoreSelectedContentsRow>,
    pub coverage: StoreSelectedContentsCoverage,
    pub next_cursor: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSelectedContentsRowOrigin {
    LibraryAsset,
    SourceFile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreSelectedContentsRow {
    pub stable_id: String,
    pub label: String,
    pub origin: StoreSelectedContentsRowOrigin,
    pub library_asset_id: Option<i64>,
    pub row_version: Option<i64>,
    pub primary_source_file_id: Option<i64>,
    pub scoped_source_file_id: i64,
    pub source_id: i64,
    pub relative_path: String,
    pub file_name: String,
    pub media_class: String,
    pub availability_state: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub musical_key: Option<String>,
    pub tempo_bpm: Option<f64>,
    pub waveform_quality_current: Option<i64>,
    pub waveform_quality_target: Option<i64>,
    pub stems_state_summary: Option<String>,
    pub prep_readiness_summary: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolvedSelectedContentsScope {
    WholeSource {
        source_id: i64,
    },
    AcceptedSourceLocations {
        source_id: i64,
    },
    Prefix {
        source_id: i64,
        relative_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceReadiness {
    source_class: String,
    mount_status: Option<String>,
    resolution_status: Option<String>,
    scan_phase: Option<String>,
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
const SELECTED_CONTENTS_ORDER_SQL: &str = "CASE availability_state
        WHEN 'available' THEN 0
        WHEN 'degraded' THEN 1
        WHEN 'unavailable' THEN 2
        ELSE 3
    END ASC,
    lower(COALESCE(title, relative_path, '')) ASC,
    lower(COALESCE(artist, '')) ASC,
    lower(COALESCE(album, '')) ASC,
    lower(COALESCE(relative_path, '')) ASC,
    scoped_source_file_id ASC";

pub(crate) fn read_selected_contents(
    connection: &Connection,
    scope: StoreSelectedContentsScope,
    limit: usize,
    cursor: Option<&str>,
) -> LibrarySqliteResult<StoreSelectedContentsResult> {
    if cursor.is_some() {
        return Ok(non_ready_result(
            scope,
            StoreSelectedContentsState::Failed,
            StoreSelectedContentsCoverageState::Failed,
            "Selected contents cursor paging is not available in this first slice.",
        ));
    }

    let Some((source_readiness, resolved_scope)) = resolve_scope(connection, &scope)? else {
        return Ok(non_ready_result(
            scope,
            StoreSelectedContentsState::LocationMissing,
            StoreSelectedContentsCoverageState::LocationMissing,
            "The selected library contents target is not available.",
        ));
    };

    if let Some((state, coverage_state, detail)) = source_unavailable_state(&source_readiness) {
        let rows = read_rows(connection, &resolved_scope, limit)?;
        return Ok(StoreSelectedContentsResult {
            state,
            scope,
            rows,
            coverage: StoreSelectedContentsCoverage {
                state: coverage_state,
                recursive_scope_complete: false,
                empty_result_authoritative: false,
                detail: Some(detail.to_string()),
            },
            next_cursor: None,
            detail: Some(detail.to_string()),
        });
    }

    let coverage = read_coverage(connection, &source_readiness, &resolved_scope)?;
    let rows = read_rows(connection, &resolved_scope, limit)?;
    let state = selected_contents_state(&coverage, rows.is_empty());
    let detail = selected_contents_detail(state, coverage.state);

    Ok(StoreSelectedContentsResult {
        state,
        scope,
        rows,
        coverage: StoreSelectedContentsCoverage {
            empty_result_authoritative: state == StoreSelectedContentsState::Empty,
            ..coverage
        },
        next_cursor: None,
        detail: detail.map(str::to_string),
    })
}

fn resolve_scope(
    connection: &Connection,
    scope: &StoreSelectedContentsScope,
) -> LibrarySqliteResult<Option<(SourceReadiness, ResolvedSelectedContentsScope)>> {
    match scope {
        StoreSelectedContentsScope::Source { source_id } => {
            let Some(source_readiness) = load_source_readiness(connection, *source_id)? else {
                return Ok(None);
            };

            let accepted_count = accepted_source_location_count(connection, *source_id)?;
            let resolved_scope = if accepted_count == 0 {
                ResolvedSelectedContentsScope::WholeSource {
                    source_id: *source_id,
                }
            } else if accepted_source_location_missing_count(connection, *source_id)? > 0 {
                return Ok(Some((
                    source_readiness,
                    ResolvedSelectedContentsScope::AcceptedSourceLocations {
                        source_id: *source_id,
                    },
                )));
            } else {
                ResolvedSelectedContentsScope::AcceptedSourceLocations {
                    source_id: *source_id,
                }
            };

            Ok(Some((source_readiness, resolved_scope)))
        }
        StoreSelectedContentsScope::SourceLocation { source_location_id } => {
            let Some((source_id, relative_path)) =
                load_accepted_source_location(connection, *source_location_id)?
            else {
                return Ok(None);
            };
            let Some(source_readiness) = load_source_readiness(connection, source_id)? else {
                return Ok(None);
            };

            if !present_directory_exists(connection, source_id, &relative_path)? {
                return Ok(Some((
                    source_readiness,
                    ResolvedSelectedContentsScope::Prefix {
                        source_id,
                        relative_path,
                    },
                )));
            }

            Ok(Some((
                source_readiness,
                ResolvedSelectedContentsScope::Prefix {
                    source_id,
                    relative_path,
                },
            )))
        }
        StoreSelectedContentsScope::Directory {
            source_id,
            source_directory_id,
        } => {
            let Some(source_readiness) = load_source_readiness(connection, *source_id)? else {
                return Ok(None);
            };
            let Some(relative_path) =
                load_present_directory_path(connection, *source_id, *source_directory_id)?
            else {
                return Ok(Some((
                    source_readiness,
                    ResolvedSelectedContentsScope::Prefix {
                        source_id: *source_id,
                        relative_path: String::new(),
                    },
                )));
            };

            Ok(Some((
                source_readiness,
                ResolvedSelectedContentsScope::Prefix {
                    source_id: *source_id,
                    relative_path,
                },
            )))
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
                    ss.resolution_status,
                    sss.scan_phase
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
                    resolution_status: row.get(2)?,
                    scan_phase: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_accepted_source_location(
    connection: &Connection,
    source_location_id: i64,
) -> LibrarySqliteResult<Option<(i64, String)>> {
    connection
        .query_row(
            "SELECT source_id, relative_path
             FROM source_locations
             WHERE source_location_id = ?1
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1",
            [source_location_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn accepted_source_location_count(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<i64> {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM source_locations
             WHERE source_id = ?1
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1",
            [source_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn accepted_source_location_missing_count(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<i64> {
    connection
        .query_row(
            "SELECT COUNT(*)
             FROM source_locations sl
             WHERE sl.source_id = ?1
               AND sl.authority = 'user'
               AND sl.location_kind = 'registered_subpath'
               AND sl.is_user_visible = 1
               AND NOT EXISTS (
                   SELECT 1
                   FROM source_directories sd
                   WHERE sd.source_id = sl.source_id
                     AND sd.relative_path COLLATE BINARY = sl.relative_path COLLATE BINARY
                     AND sd.presence_state = 'present'
               )",
            [source_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn present_directory_exists(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM source_directories
                 WHERE source_id = ?1
                   AND relative_path COLLATE BINARY = ?2 COLLATE BINARY
                   AND presence_state = 'present'
             )",
            params![source_id, relative_path],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
        .map_err(Into::into)
}

fn load_present_directory_path(
    connection: &Connection,
    source_id: i64,
    source_directory_id: i64,
) -> LibrarySqliteResult<Option<String>> {
    connection
        .query_row(
            "SELECT relative_path
             FROM source_directories
             WHERE source_id = ?1
               AND source_directory_id = ?2
               AND presence_state = 'present'",
            params![source_id, source_directory_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn source_unavailable_state(
    source: &SourceReadiness,
) -> Option<(
    StoreSelectedContentsState,
    StoreSelectedContentsCoverageState,
    &'static str,
)> {
    if source.resolution_status.as_deref() == Some("inaccessible") {
        return Some((
            StoreSelectedContentsState::Blocked,
            StoreSelectedContentsCoverageState::Blocked,
            "The selected source is blocked or inaccessible.",
        ));
    }

    if source.source_class != "internal"
        && !matches!(source.mount_status.as_deref(), Some("mounted"))
    {
        return Some((
            StoreSelectedContentsState::SourceUnavailable,
            StoreSelectedContentsCoverageState::SourceUnavailable,
            "The selected source is unavailable.",
        ));
    }

    match source.scan_phase.as_deref() {
        Some("blocked") => Some((
            StoreSelectedContentsState::Blocked,
            StoreSelectedContentsCoverageState::Blocked,
            "The selected source scan is blocked.",
        )),
        Some("failed") => Some((
            StoreSelectedContentsState::Failed,
            StoreSelectedContentsCoverageState::Failed,
            "The selected source scan failed.",
        )),
        _ => None,
    }
}

fn read_coverage(
    connection: &Connection,
    source: &SourceReadiness,
    scope: &ResolvedSelectedContentsScope,
) -> LibrarySqliteResult<StoreSelectedContentsCoverage> {
    let counts = match scope {
        ResolvedSelectedContentsScope::WholeSource { source_id } => {
            read_whole_source_coverage_counts(connection, *source_id)?
        }
        ResolvedSelectedContentsScope::AcceptedSourceLocations { source_id } => {
            if accepted_source_location_missing_count(connection, *source_id)? > 0 {
                return Ok(coverage(
                    StoreSelectedContentsCoverageState::LocationMissing,
                    false,
                    "One or more accepted source locations are missing.",
                ));
            }
            read_accepted_source_locations_coverage_counts(connection, *source_id)?
        }
        ResolvedSelectedContentsScope::Prefix {
            source_id,
            relative_path,
        } => {
            if !present_directory_exists(connection, *source_id, relative_path)? {
                return Ok(coverage(
                    StoreSelectedContentsCoverageState::LocationMissing,
                    false,
                    "The selected folder is missing.",
                ));
            }
            read_prefix_coverage_counts(connection, *source_id, relative_path)?
        }
    };

    Ok(coverage_from_counts(source, scope, counts))
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

fn read_accepted_source_locations_coverage_counts(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<CoverageCounts> {
    connection
        .query_row(
            &format!(
                "WITH accepted_locations(relative_path) AS (
                     SELECT relative_path
                     FROM source_locations
                     WHERE source_id = ?1
                       AND authority = 'user'
                       AND location_kind = 'registered_subpath'
                       AND is_user_visible = 1
                 )
                 SELECT COUNT(*),
                        SUM(CASE WHEN sd.dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN sd.dir_scan_state = 'failed' THEN 1 ELSE 0 END)
                 FROM source_directories sd
                 WHERE sd.source_id = ?1
                   AND sd.presence_state = 'present'
                   AND EXISTS (
                       SELECT 1
                       FROM accepted_locations al
                       WHERE {}
                   )",
                relative_path_scope_predicate("sd", "al.relative_path")
            ),
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
                   AND {}",
                relative_path_scope_predicate("sd", "?2")
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

fn coverage_from_counts(
    source: &SourceReadiness,
    scope: &ResolvedSelectedContentsScope,
    counts: CoverageCounts,
) -> StoreSelectedContentsCoverage {
    if counts.total_directories == 0 {
        if matches!(scope, ResolvedSelectedContentsScope::WholeSource { .. })
            && source.scan_phase.as_deref() == Some("complete")
        {
            return coverage(
                StoreSelectedContentsCoverageState::Complete,
                true,
                "The selected source has complete scan coverage.",
            );
        }

        if matches!(scope, ResolvedSelectedContentsScope::WholeSource { .. }) {
            return coverage(
                StoreSelectedContentsCoverageState::Pending,
                false,
                "The selected source has no completed directory coverage yet.",
            );
        }

        return coverage(
            StoreSelectedContentsCoverageState::LocationMissing,
            false,
            "The selected contents scope has no present directory coverage.",
        );
    }

    if counts.blocked_directories > 0 {
        return coverage(
            StoreSelectedContentsCoverageState::Blocked,
            false,
            "Part of the selected contents scope is blocked.",
        );
    }

    if counts.failed_directories > 0 {
        return coverage(
            StoreSelectedContentsCoverageState::Failed,
            false,
            "Part of the selected contents scope failed to scan.",
        );
    }

    if counts.scanning_directories > 0 {
        return coverage(
            StoreSelectedContentsCoverageState::Scanning,
            false,
            "The selected contents scope is still scanning.",
        );
    }

    if counts.pending_directories > 0 {
        return coverage(
            StoreSelectedContentsCoverageState::Pending,
            false,
            "The selected contents scope has pending scan coverage.",
        );
    }

    if matches!(scope, ResolvedSelectedContentsScope::WholeSource { .. })
        && source.scan_phase.as_deref() != Some("complete")
    {
        return coverage(
            StoreSelectedContentsCoverageState::Pending,
            false,
            "The selected source has not completed a full recursive scan.",
        );
    }

    coverage(
        StoreSelectedContentsCoverageState::Complete,
        true,
        "The selected contents scope has complete scan coverage.",
    )
}

fn coverage(
    state: StoreSelectedContentsCoverageState,
    recursive_scope_complete: bool,
    detail: &str,
) -> StoreSelectedContentsCoverage {
    StoreSelectedContentsCoverage {
        state,
        recursive_scope_complete,
        empty_result_authoritative: false,
        detail: Some(detail.to_string()),
    }
}

fn selected_contents_state(
    coverage: &StoreSelectedContentsCoverage,
    rows_empty: bool,
) -> StoreSelectedContentsState {
    match coverage.state {
        StoreSelectedContentsCoverageState::Complete => {
            if rows_empty {
                StoreSelectedContentsState::Empty
            } else {
                StoreSelectedContentsState::Ready
            }
        }
        StoreSelectedContentsCoverageState::Pending
        | StoreSelectedContentsCoverageState::Scanning => StoreSelectedContentsState::Partial,
        StoreSelectedContentsCoverageState::Blocked => StoreSelectedContentsState::Blocked,
        StoreSelectedContentsCoverageState::Failed => StoreSelectedContentsState::Failed,
        StoreSelectedContentsCoverageState::SourceUnavailable => {
            StoreSelectedContentsState::SourceUnavailable
        }
        StoreSelectedContentsCoverageState::LocationMissing => {
            StoreSelectedContentsState::LocationMissing
        }
    }
}

fn selected_contents_detail(
    state: StoreSelectedContentsState,
    coverage_state: StoreSelectedContentsCoverageState,
) -> Option<&'static str> {
    match state {
        StoreSelectedContentsState::Ready => None,
        StoreSelectedContentsState::Empty => Some("No media contents were found in this scope."),
        StoreSelectedContentsState::Partial => Some(match coverage_state {
            StoreSelectedContentsCoverageState::Scanning => {
                "Still indexing. Results may be incomplete."
            }
            _ => "Indexing is incomplete. Results may be incomplete.",
        }),
        StoreSelectedContentsState::SourceUnavailable => {
            Some("The selected source is unavailable.")
        }
        StoreSelectedContentsState::LocationMissing => Some("The selected folder is missing."),
        StoreSelectedContentsState::Blocked => Some("The selected contents scope is blocked."),
        StoreSelectedContentsState::Failed => Some("The selected contents scope failed to scan."),
    }
}

fn read_rows(
    connection: &Connection,
    scope: &ResolvedSelectedContentsScope,
    limit: usize,
) -> LibrarySqliteResult<Vec<StoreSelectedContentsRow>> {
    let limit = i64::try_from(limit).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "selected contents limit {limit} exceeds i64 range"
        ))
    })?;

    match scope {
        ResolvedSelectedContentsScope::WholeSource { source_id } => {
            read_rows_with_source_predicate(
                connection,
                "sf.source_id = ?1",
                *source_id,
                None,
                limit,
            )
        }
        ResolvedSelectedContentsScope::AcceptedSourceLocations { source_id } => {
            let predicate = format!(
                "sf.source_id = ?1
                 AND EXISTS (
                     SELECT 1
                     FROM accepted_locations al
                     WHERE {}
                 )",
                relative_path_scope_predicate("sf", "al.relative_path")
            );
            read_rows_for_accepted_locations(connection, &predicate, *source_id, limit)
        }
        ResolvedSelectedContentsScope::Prefix {
            source_id,
            relative_path,
        } => read_rows_with_source_predicate(
            connection,
            &format!(
                "sf.source_id = ?1
                 AND {}",
                source_file_descendant_predicate("sf", "?2")
            ),
            *source_id,
            Some(relative_path),
            limit,
        ),
    }
}

fn read_rows_for_accepted_locations(
    connection: &Connection,
    source_predicate: &str,
    source_id: i64,
    limit: i64,
) -> LibrarySqliteResult<Vec<StoreSelectedContentsRow>> {
    let sql = selected_rows_sql(Some(accepted_locations_cte()), source_predicate);
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(params![source_id, limit], selected_contents_row_from_row)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn read_rows_with_source_predicate(
    connection: &Connection,
    source_predicate: &str,
    source_id: i64,
    relative_path: Option<&str>,
    limit: i64,
) -> LibrarySqliteResult<Vec<StoreSelectedContentsRow>> {
    let sql = selected_rows_sql(None, source_predicate);
    let mut statement = connection.prepare(&sql)?;
    let rows = if let Some(relative_path) = relative_path {
        statement
            .query_map(
                params![source_id, relative_path, limit],
                selected_contents_row_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    } else {
        statement
            .query_map(params![source_id, limit], selected_contents_row_from_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(LibrarySqliteError::from)?
    };
    Ok(rows)
}

fn selected_rows_sql(prefix_cte: Option<&str>, source_predicate: &str) -> String {
    let cte_prefix = prefix_cte
        .map(|cte| format!("WITH {cte},"))
        .unwrap_or_else(|| "WITH".to_string());

    format!(
        "{cte_prefix}
         scope_files AS (
             SELECT sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
                    sf.media_class,
                    sf.updated_at
             FROM source_files sf
             WHERE sf.presence_state = 'present'
               AND sf.media_class IN ('audio', 'video')
               AND {source_predicate}
         ),
         promoted_scope AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    pbr.availability_state,
                    pbr.title,
                    pbr.artist,
                    pbr.album,
                    pbr.duration_ms,
                    pbr.musical_key,
                    pbr.tempo_bpm,
                    pbr.waveform_quality_current,
                    pbr.waveform_quality_target,
                    pbr.stems_state_summary,
                    pbr.prep_readiness_summary,
                    pbr.updated_at,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
                    sf.media_class,
                    pia.accepted_at,
                    ss.ordinal,
                    ROW_NUMBER() OVER (
                        PARTITION BY pbr.library_asset_id
                        ORDER BY CASE
                                     WHEN sf.source_file_id = pbr.primary_source_file_id THEN 0
                                     ELSE 1
                                 END ASC,
                                 pia.accepted_at ASC,
                                 ss.ordinal ASC,
                                 sf.source_file_id ASC
                    ) AS attachment_rank
             FROM scope_files sf
             JOIN SourceSegmentSets sss
               ON sss.source_file_id = sf.source_file_id
             JOIN SourceSegments ss
               ON ss.source_segment_set_id = sss.source_segment_set_id
             JOIN LibraryAssetAttachments pia
               ON pia.source_segment_id = ss.source_segment_id
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = pia.library_asset_id
         ),
         promoted AS (
             SELECT library_asset_id,
                    row_version,
                    primary_source_file_id,
                    source_file_id AS scoped_source_file_id,
                    source_id,
                    relative_path,
                    name AS file_name,
                    media_class,
                    availability_state,
                    title,
                    artist,
                    album,
                    duration_ms,
                    musical_key,
                    tempo_bpm,
                    waveform_quality_current,
                    waveform_quality_target,
                    stems_state_summary,
                    prep_readiness_summary,
                    updated_at
             FROM promoted_scope
             WHERE attachment_rank = 1
         ),
         source_file_rows AS (
             SELECT NULL AS library_asset_id,
                    NULL AS row_version,
                    NULL AS primary_source_file_id,
                    sf.source_file_id AS scoped_source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name AS file_name,
                    sf.media_class,
                    'available' AS availability_state,
                    NULL AS title,
                    NULL AS artist,
                    NULL AS album,
                    NULL AS duration_ms,
                    NULL AS musical_key,
                    NULL AS tempo_bpm,
                    NULL AS waveform_quality_current,
                    NULL AS waveform_quality_target,
                    NULL AS stems_state_summary,
                    'underprepared' AS prep_readiness_summary,
                    sf.updated_at
             FROM scope_files sf
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM promoted_scope ps
                 WHERE ps.source_file_id = sf.source_file_id
             )
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                scoped_source_file_id,
                source_id,
                relative_path,
                file_name,
                media_class,
                availability_state,
                title,
                artist,
                album,
                duration_ms,
                musical_key,
                tempo_bpm,
                waveform_quality_current,
                waveform_quality_target,
                stems_state_summary,
                prep_readiness_summary,
                updated_at
         FROM (
             SELECT * FROM promoted
             UNION ALL
             SELECT * FROM source_file_rows
         )
         ORDER BY {SELECTED_CONTENTS_ORDER_SQL}
         LIMIT ?{}",
        if prefix_cte.is_some() || source_predicate == "sf.source_id = ?1" {
            "2"
        } else {
            "3"
        }
    )
}

fn accepted_locations_cte() -> &'static str {
    "accepted_locations(relative_path) AS (
         SELECT relative_path
         FROM source_locations
         WHERE source_id = ?1
           AND authority = 'user'
           AND location_kind = 'registered_subpath'
           AND is_user_visible = 1
     )"
}

fn relative_path_scope_predicate(alias: &str, prefix_sql: &str) -> String {
    format!(
        "{alias}.relative_path COLLATE BINARY = {prefix_sql} COLLATE BINARY
         OR {}",
        source_file_descendant_predicate(alias, prefix_sql)
    )
}

fn source_file_descendant_predicate(alias: &str, prefix_sql: &str) -> String {
    format!(
        "({alias}.relative_path COLLATE BINARY >= {prefix_sql} || '/'
          AND {alias}.relative_path COLLATE BINARY < {prefix_sql} || {RELATIVE_PATH_PREFIX_UPPER_BOUND_SENTINEL_SQL})"
    )
}

fn selected_contents_row_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoreSelectedContentsRow> {
    let library_asset_id: Option<i64> = row.get(0)?;
    let row_version: Option<i64> = row.get(1)?;
    let primary_source_file_id: Option<i64> = row.get(2)?;
    let scoped_source_file_id: i64 = row.get(3)?;
    let source_id: i64 = row.get(4)?;
    let relative_path: String = row.get(5)?;
    let file_name: String = row.get(6)?;
    let media_class: String = row.get(7)?;
    let availability_state: String = row.get(8)?;
    let title: Option<String> = row.get(9)?;
    let artist: Option<String> = row.get(10)?;
    let album: Option<String> = row.get(11)?;
    let duration_ms: Option<i64> = row.get(12)?;
    let musical_key: Option<String> = row.get(13)?;
    let tempo_bpm: Option<f64> = row.get(14)?;
    let waveform_quality_current: Option<i64> = row.get(15)?;
    let waveform_quality_target: Option<i64> = row.get(16)?;
    let stems_state_summary: Option<String> = row.get(17)?;
    let prep_readiness_summary: String = row.get(18)?;
    let updated_at: i64 = row.get(19)?;

    let origin = if library_asset_id.is_some() {
        StoreSelectedContentsRowOrigin::LibraryAsset
    } else {
        StoreSelectedContentsRowOrigin::SourceFile
    };

    let stable_id = match &origin {
        StoreSelectedContentsRowOrigin::LibraryAsset => {
            let id =
                library_asset_id.expect("library_asset_id must be present for LibraryAsset origin");
            format!("library-asset:{id}")
        }
        StoreSelectedContentsRowOrigin::SourceFile => {
            format!("source-file:{scoped_source_file_id}")
        }
    };

    let label = selected_contents_label(title.as_deref(), &file_name, &relative_path);

    Ok(StoreSelectedContentsRow {
        stable_id,
        label,
        origin,
        library_asset_id,
        row_version,
        primary_source_file_id,
        scoped_source_file_id,
        source_id,
        relative_path,
        file_name,
        media_class,
        availability_state,
        title,
        artist,
        album,
        duration_ms,
        musical_key,
        tempo_bpm,
        waveform_quality_current,
        waveform_quality_target,
        stems_state_summary,
        prep_readiness_summary,
        updated_at,
    })
}

fn selected_contents_label(title: Option<&str>, file_name: &str, relative_path: &str) -> String {
    title
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if !file_name.trim().is_empty() {
                Some(file_name)
            } else {
                None
            }
        })
        .or_else(|| {
            if !relative_path.trim().is_empty() {
                Some(relative_path)
            } else {
                None
            }
        })
        .unwrap_or("Untitled")
        .to_string()
}

fn non_ready_result(
    scope: StoreSelectedContentsScope,
    state: StoreSelectedContentsState,
    coverage_state: StoreSelectedContentsCoverageState,
    detail: &str,
) -> StoreSelectedContentsResult {
    StoreSelectedContentsResult {
        state,
        scope,
        rows: Vec::new(),
        coverage: StoreSelectedContentsCoverage {
            state: coverage_state,
            recursive_scope_complete: false,
            empty_result_authoritative: false,
            detail: Some(detail.to_string()),
        },
        next_cursor: None,
        detail: Some(detail.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};

    use super::{
        StoreSelectedContentsCoverageState, StoreSelectedContentsRowOrigin,
        StoreSelectedContentsScope, StoreSelectedContentsState, read_selected_contents,
    };
    use crate::schema::install_baseline_schema_for_test;

    fn open_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open test database");
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
                params![
                    source_id,
                    format!("source:{source_id}"),
                    format!("Source {source_id}")
                ],
            )
            .expect("insert source");
        connection
            .execute(
                "INSERT INTO source_state (
                     source_id,
                     mount_status,
                     mount_epoch,
                     resolution_status,
                     mount_root,
                     effective_path,
                     updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'resolved', 'root', 'root', 1)",
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

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        source_id: i64,
        relative_path: &str,
        dir_scan_state: &str,
    ) {
        let name = relative_path.rsplit('/').next().unwrap_or(relative_path);
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     relative_path,
                     presence_state,
                     dir_scan_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, NULL, ?3, ?4, 'present', ?5, 1, 1, 1)",
                params![
                    source_directory_id,
                    source_id,
                    name,
                    relative_path,
                    dir_scan_state
                ],
            )
            .expect("insert source directory");
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
                params![
                    source_location_id,
                    source_id,
                    authority,
                    location_kind,
                    relative_path
                ],
            )
            .expect("insert source location");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_asset_file(
        connection: &Connection,
        library_asset_id: i64,
        source_file_id: i64,
        source_id: i64,
        parent_directory_id: i64,
        relative_path: &str,
        media_class: &str,
        title: &str,
    ) {
        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    source_id,
                    parent_directory_id,
                    file_name,
                    relative_path,
                    media_class
                ],
            )
            .expect("insert source file");
        connection
            .execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 'keep_metadata', 1, 1)",
                params![library_asset_id, format!("asset:{library_asset_id}")],
            )
            .expect("insert library asset");
        connection
            .execute(
                "INSERT INTO Artifacts (
                     artifact_id,
                     work_run_id,
                     subject_kind,
                     subject_id,
                     artifact_kind,
                     artifact_role,
                     adapter_key,
                     adapter_version,
                     basis_fingerprint,
                     media_type,
                     storage_kind,
                     payload_hash,
                     created_at
                 )
                 VALUES (?1, 1, 'source_file', ?2, 'inspection_result', 'primary_result', 'test', '1', ?3, 'application/json', 'inline_payload', ?4, 1)",
                params![
                    10_000 + source_file_id,
                    source_file_id.to_string(),
                    format!("basis:{source_file_id}"),
                    format!("hash:{source_file_id}")
                ],
            )
            .expect("insert artifact");
        connection
            .execute(
                "INSERT INTO SourceSegmentSets (
                     source_segment_set_id,
                     source_file_id,
                     segment_set_kind,
                     basis_fingerprint,
                     accepted_at,
                     accepted_artifact_id,
                     updated_at
                 )
                 VALUES (?1, ?2, 'whole_file', ?3, 1, ?4, 1)",
                params![
                    20_000 + source_file_id,
                    source_file_id,
                    format!("basis:segments:{source_file_id}"),
                    10_000 + source_file_id
                ],
            )
            .expect("insert segment set");
        connection
            .execute(
                "INSERT INTO SourceSegments (
                     source_segment_id,
                     source_segment_set_id,
                     segment_kind,
                     ordinal,
                     start_offset_ms,
                     display_title,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 'whole_file', 0, 0, ?3, 1, 1)",
                params![30_000 + source_file_id, 20_000 + source_file_id, title],
            )
            .expect("insert segment");
        connection
            .execute(
                "INSERT INTO LibraryAssetAttachments (
                     library_asset_attachment_id,
                     library_asset_id,
                     source_segment_id,
                     accepted_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, 1, 1)",
                params![
                    40_000 + source_file_id,
                    library_asset_id,
                    30_000 + source_file_id
                ],
            )
            .expect("insert attachment");
        connection
            .execute(
                "INSERT INTO LibraryBrowserRows (
                     library_asset_id,
                     row_version,
                     primary_source_file_id,
                     availability_state,
                     title,
                     prep_readiness_summary,
                     updated_at
                 )
                 VALUES (?1, 1, ?2, 'available', ?3, 'not_required', 1)",
                params![library_asset_id, source_file_id, title],
            )
            .expect("insert browser row");
    }

    fn seed_assets(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO WorkItems (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     basis_fingerprint,
                     state,
                     priority_class,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'projection_domain', 'library_browser', 'rebuild_projection', 'basis:test', 'completed', 'interactive', 1, 1)",
                [],
            )
            .expect("insert work item");
        connection
            .execute(
                "INSERT INTO WorkRuns (
                     work_run_id,
                     work_item_id,
                     adapter_key,
                     adapter_version,
                     started_at,
                     outcome
                 )
                 VALUES (1, 1, 'test', '1', 1, 'ok')",
                [],
            )
            .expect("insert work run");
    }

    #[test]
    fn source_scope_uses_only_accepted_locations_when_present() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Other", "complete");
        insert_location(&connection, 100, 1, "Music", "user", "registered_subpath");
        insert_location(&connection, 101, 1, "Other", "device", "observed_path");
        insert_asset_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/track.wav",
            "audio",
            "Track",
        );
        insert_asset_file(
            &connection,
            2,
            1001,
            1,
            11,
            "Other/clip.mp4",
            "video",
            "Clip",
        );

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Source { source_id: 1 },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| (row.origin, row.library_asset_id))
                .collect::<Vec<_>>(),
            vec![(StoreSelectedContentsRowOrigin::LibraryAsset, Some(1))]
        );
        assert_eq!(
            result.coverage.state,
            StoreSelectedContentsCoverageState::Complete
        );
    }

    #[test]
    fn source_location_scope_requires_accepted_user_visible_location() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Observed", "complete");
        insert_location(&connection, 100, 1, "Observed", "device", "observed_path");
        insert_asset_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Observed/track.wav",
            "audio",
            "Track",
        );

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::SourceLocation {
                source_location_id: 100,
            },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::LocationMissing);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn directory_scope_reads_recursive_media_and_excludes_prefix_siblings() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_directory(&connection, 11, 1, "Music/Nested", "complete");
        insert_directory(&connection, 12, 1, "Music2", "complete");
        insert_asset_file(
            &connection,
            1,
            1000,
            1,
            11,
            "Music/Nested/alpha.wav",
            "audio",
            "Alpha",
        );
        insert_asset_file(
            &connection,
            2,
            1001,
            1,
            12,
            "Music2/beta.wav",
            "audio",
            "Beta",
        );
        insert_asset_file(
            &connection,
            3,
            1002,
            1,
            10,
            "Music/cover.png",
            "unsupported",
            "Cover",
        );

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| (row.origin, row.library_asset_id, row.media_class.as_str()))
                .collect::<Vec<_>>(),
            vec![(
                StoreSelectedContentsRowOrigin::LibraryAsset,
                Some(1),
                "audio"
            )]
        );
    }

    #[test]
    fn incomplete_empty_scope_is_partial_not_authoritative_empty() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Pending", "pending");

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Partial);
        assert_eq!(
            result.coverage.state,
            StoreSelectedContentsCoverageState::Pending
        );
        assert!(!result.coverage.empty_result_authoritative);
    }

    fn insert_scanned_file(
        connection: &Connection,
        source_file_id: i64,
        source_id: i64,
        parent_directory_id: i64,
        relative_path: &str,
        media_class: &str,
    ) {
        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    source_id,
                    parent_directory_id,
                    file_name,
                    relative_path,
                    media_class
                ],
            )
            .expect("insert scanned source file");
    }

    #[test]
    fn source_scope_returns_source_file_rows_without_promotion() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/track.wav", "audio");
        insert_scanned_file(&connection, 1001, 1, 10, "Music/clip.mp4", "video");

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Source { source_id: 1 },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(result.rows.len(), 2);

        for row in &result.rows {
            assert_eq!(row.origin, StoreSelectedContentsRowOrigin::SourceFile);
            assert!(row.library_asset_id.is_none());
            assert!(row.row_version.is_none());
            assert!(row.primary_source_file_id.is_none());
            assert!(row.title.is_none());
            assert!(row.artist.is_none());
            assert!(row.album.is_none());
            assert!(row.duration_ms.is_none());
            assert_eq!(row.availability_state, "available");
            assert_eq!(row.prep_readiness_summary, "underprepared");
            assert!(
                row.stable_id.starts_with("source-file:"),
                "source-file row stable_id must start with 'source-file:', got: {}",
                row.stable_id
            );
        }

        let media_classes: Vec<&str> = result
            .rows
            .iter()
            .map(|row| row.media_class.as_str())
            .collect();
        assert!(media_classes.contains(&"audio"));
        assert!(media_classes.contains(&"video"));
    }

    #[test]
    fn promoted_asset_rows_still_work_when_promotion_rows_exist() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_asset_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/track.wav",
            "audio",
            "Track",
        );

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Source { source_id: 1 },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0].origin,
            StoreSelectedContentsRowOrigin::LibraryAsset
        );
        assert_eq!(result.rows[0].library_asset_id, Some(1));
        assert!(
            result.rows[0].stable_id.starts_with("library-asset:"),
            "promoted row stable_id must start with 'library-asset:', got: {}",
            result.rows[0].stable_id
        );
    }

    #[test]
    fn complete_scope_with_media_files_without_promotion_returns_ready_not_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_scanned_file(&connection, 1000, 1, 10, "Music/track.wav", "audio");

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0].origin,
            StoreSelectedContentsRowOrigin::SourceFile
        );
        assert_eq!(
            result.coverage.state,
            StoreSelectedContentsCoverageState::Complete
        );
    }

    #[test]
    fn complete_scope_with_no_media_files_returns_authoritative_empty() {
        let connection = open_connection();
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Documents", "complete");
        insert_scanned_file(
            &connection,
            1000,
            1,
            10,
            "Documents/readme.txt",
            "unsupported",
        );

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Directory {
                source_id: 1,
                source_directory_id: 10,
            },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Empty);
        assert!(result.rows.is_empty());
        assert!(result.coverage.empty_result_authoritative);
    }

    #[test]
    fn mixed_promoted_and_source_file_rows_coexist() {
        let connection = open_connection();
        seed_assets(&connection);
        insert_source(&connection, 1);
        insert_directory(&connection, 10, 1, "Music", "complete");
        insert_asset_file(
            &connection,
            1,
            1000,
            1,
            10,
            "Music/promoted.wav",
            "audio",
            "Promoted",
        );
        insert_scanned_file(&connection, 1001, 1, 10, "Music/scanned.wav", "audio");

        let result = read_selected_contents(
            &connection,
            StoreSelectedContentsScope::Source { source_id: 1 },
            10,
            None,
        )
        .expect("read selected contents");

        assert_eq!(result.state, StoreSelectedContentsState::Ready);
        assert_eq!(result.rows.len(), 2);

        let origins: Vec<_> = result.rows.iter().map(|r| r.origin).collect();
        assert!(origins.contains(&StoreSelectedContentsRowOrigin::LibraryAsset));
        assert!(origins.contains(&StoreSelectedContentsRowOrigin::SourceFile));
    }
}
