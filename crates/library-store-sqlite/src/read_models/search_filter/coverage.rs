use rusqlite::{Connection, OptionalExtension, params};

use super::SEARCH_INDEXER_VERSION;
use super::types::{StoreSearchIndexState, StoreSearchScope};
use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchIndexMetadata {
    pub(crate) generation: i64,
    pub(crate) state: StoreSearchIndexState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchScopeCoverage {
    pub(crate) index_generation: i64,
    pub(crate) index_state: StoreSearchIndexState,
    pub(crate) fully_ready: bool,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceCoverageRow {
    state: StoreSearchIndexState,
}

pub(crate) fn read_index_metadata(
    connection: &Connection,
) -> LibrarySqliteResult<SearchIndexMetadata> {
    connection
        .query_row(
            "SELECT generation, state
             FROM search_filter_index_metadata
             WHERE search_filter_index_id = 1
               AND indexer_version = ?1",
            [SEARCH_INDEXER_VERSION],
            |row| {
                let state = row.get::<_, String>(1)?;
                let state = StoreSearchIndexState::from_storage_value(&state).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(SearchIndexMetadata {
                    generation: row.get(0)?,
                    state,
                })
            },
        )
        .map_err(Into::into)
}

pub(crate) fn resolve_scope_coverage(
    connection: &Connection,
    metadata: &SearchIndexMetadata,
    scope: &StoreSearchScope,
) -> LibrarySqliteResult<SearchScopeCoverage> {
    if metadata.state != StoreSearchIndexState::Ready {
        return Ok(SearchScopeCoverage {
            index_generation: metadata.generation,
            index_state: metadata.state,
            fully_ready: false,
            detail: Some("Search/filter index metadata is not ready.".to_string()),
        });
    }

    let Some(source_ids) = scope_source_ids(connection, scope)? else {
        return Ok(SearchScopeCoverage {
            index_generation: metadata.generation,
            index_state: StoreSearchIndexState::Missing,
            fully_ready: false,
            detail: Some(
                "Search/filter scope coverage could not resolve an owning source.".to_string(),
            ),
        });
    };

    if source_ids.is_empty() {
        return Ok(SearchScopeCoverage {
            index_generation: metadata.generation,
            index_state: StoreSearchIndexState::Ready,
            fully_ready: true,
            detail: None,
        });
    }

    let mut missing_count = 0usize;
    let mut rebuilding_count = 0usize;
    let mut partial_count = 0usize;
    let mut failed_count = 0usize;

    for source_id in source_ids {
        match read_source_coverage(connection, source_id)? {
            None => missing_count += 1,
            Some(SourceCoverageRow {
                state: StoreSearchIndexState::Ready,
            }) => {}
            Some(SourceCoverageRow {
                state: StoreSearchIndexState::Rebuilding,
            }) => rebuilding_count += 1,
            Some(SourceCoverageRow {
                state: StoreSearchIndexState::Partial,
            }) => partial_count += 1,
            Some(SourceCoverageRow {
                state: StoreSearchIndexState::Failed,
            }) => failed_count += 1,
            Some(SourceCoverageRow {
                state: StoreSearchIndexState::Missing,
            }) => missing_count += 1,
        }
    }

    let (index_state, detail) = if failed_count > 0 {
        (
            StoreSearchIndexState::Failed,
            format!("{failed_count} source coverage partition(s) failed."),
        )
    } else if rebuilding_count > 0 {
        (
            StoreSearchIndexState::Rebuilding,
            format!("{rebuilding_count} source coverage partition(s) are rebuilding."),
        )
    } else if partial_count > 0 {
        (
            StoreSearchIndexState::Partial,
            format!("{partial_count} source coverage partition(s) are partial."),
        )
    } else if missing_count > 0 {
        (
            StoreSearchIndexState::Missing,
            format!("{missing_count} source coverage partition(s) are missing."),
        )
    } else {
        return Ok(SearchScopeCoverage {
            index_generation: metadata.generation,
            index_state: StoreSearchIndexState::Ready,
            fully_ready: true,
            detail: None,
        });
    };

    Ok(SearchScopeCoverage {
        index_generation: metadata.generation,
        index_state,
        fully_ready: false,
        detail: Some(detail),
    })
}

pub(crate) fn mark_source_coverage_rebuilding(
    connection: &Connection,
    source_id: i64,
    updated_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_source_coverage (
             source_id, indexer_version, generation, state, rebuilt_at, updated_at, detail
         )
         VALUES (?1, ?2, 0, 'rebuilding', NULL, ?3, NULL)
         ON CONFLICT(source_id) DO UPDATE SET
             indexer_version = excluded.indexer_version,
             state = 'rebuilding',
             updated_at = excluded.updated_at,
             detail = NULL",
        params![source_id, SEARCH_INDEXER_VERSION, updated_at_ms],
    )?;
    Ok(())
}

pub(crate) fn mark_source_coverage_ready(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "UPDATE search_filter_index_source_coverage
         SET indexer_version = ?2,
             generation = ?3,
             state = 'ready',
             rebuilt_at = ?4,
             updated_at = ?4,
             detail = NULL
         WHERE source_id = ?1",
        params![source_id, SEARCH_INDEXER_VERSION, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn scope_source_ids(
    connection: &Connection,
    scope: &StoreSearchScope,
) -> LibrarySqliteResult<Option<Vec<i64>>> {
    match scope {
        StoreSearchScope::Library => connection
            .prepare(
                "SELECT source_id
                 FROM sources
                 WHERE is_user_visible = 1
                 ORDER BY source_id",
            )?
            .query_map([], |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
            .map_err(Into::into),
        StoreSearchScope::Source { source_id } => {
            if source_is_user_visible(connection, *source_id)? {
                Ok(Some(vec![*source_id]))
            } else {
                Ok(None)
            }
        }
        StoreSearchScope::SourceLocation { source_location_id } => {
            let source_id = connection
                .query_row(
                    "SELECT location.source_id
                     FROM source_locations location
                     JOIN sources source ON source.source_id = location.source_id
                     WHERE location.source_location_id = ?1
                       AND location.is_user_visible = 1
                       AND source.is_user_visible = 1",
                    [source_location_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            Ok(source_id.map(|source_id| vec![source_id]))
        }
        StoreSearchScope::Directory {
            source_id,
            source_directory_id,
        } => connection
            .query_row(
                "SELECT directory.source_id
                 FROM source_directories directory
                 JOIN sources source ON source.source_id = directory.source_id
                 WHERE directory.source_id = ?1
                   AND directory.source_directory_id = ?2
                   AND source.is_user_visible = 1",
                [source_id, source_directory_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|source_id| source_id.map(|source_id| vec![source_id]))
            .map_err(Into::into),
    }
}

pub(crate) fn source_exists(connection: &Connection, source_id: i64) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sources WHERE source_id = ?1)",
            [source_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .map_err(Into::into)
}

pub(crate) fn source_is_user_visible(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM sources
                 WHERE source_id = ?1
                   AND is_user_visible = 1
             )",
            [source_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .map_err(Into::into)
}

fn read_source_coverage(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<SourceCoverageRow>> {
    connection
        .query_row(
            "SELECT state
             FROM search_filter_index_source_coverage
             WHERE source_id = ?1
               AND indexer_version = ?2",
            params![source_id, SEARCH_INDEXER_VERSION],
            |row| {
                let state = row.get::<_, String>(0)?;
                let state = StoreSearchIndexState::from_storage_value(&state).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(SourceCoverageRow { state })
            },
        )
        .optional()
        .map_err(Into::into)
}
