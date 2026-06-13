use rusqlite::{Connection, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::baseline::CANONICAL_BASELINE_GENERATION;

pub(crate) fn install_seed_rows(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    install_library_metadata_row(connection, created_at_ms)?;
    install_search_filter_index_metadata_row(connection, created_at_ms)?;
    Ok(())
}

pub(crate) fn validate_seed_rows(connection: &Connection) -> LibrarySqliteResult<()> {
    validate_library_metadata_row(connection)?;
    validate_search_filter_index_metadata_row(connection)?;
    Ok(())
}

fn install_library_metadata_row(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO LibraryMetadata (
             library_id,
             schema_generation,
             created_at
         )
         VALUES (1, ?1, ?2)",
        params![CANONICAL_BASELINE_GENERATION, created_at_ms],
    )?;
    Ok(())
}

fn validate_library_metadata_row(connection: &Connection) -> LibrarySqliteResult<()> {
    let found = connection
        .prepare(
            "SELECT library_id, schema_generation, created_at
             FROM LibraryMetadata
             ORDER BY library_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    match found.as_slice() {
        [(1, schema_generation, _created_at)]
            if schema_generation == CANONICAL_BASELINE_GENERATION =>
        {
            Ok(())
        }
        _ => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "LibraryMetadata durable singleton row is malformed: found {found:?}"
        ))),
    }
}

fn install_search_filter_index_metadata_row(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_metadata (
             search_filter_index_id,
             indexer_version,
             generation,
             state,
             updated_at
         )
         VALUES (1, 'search_filter_v0', 0, 'ready', ?1)",
        [created_at_ms],
    )?;
    Ok(())
}

fn validate_search_filter_index_metadata_row(connection: &Connection) -> LibrarySqliteResult<()> {
    let found = connection
        .prepare(
            "SELECT search_filter_index_id, indexer_version, generation, state
             FROM search_filter_index_metadata
             ORDER BY search_filter_index_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    match found.as_slice() {
        [(1, indexer_version, generation, state)]
            if indexer_version == "search_filter_v0"
                && *generation >= 0
                && matches!(
                    state.as_str(),
                    "ready" | "rebuilding" | "partial" | "failed"
                ) =>
        {
            Ok(())
        }
        _ => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "search_filter_index_metadata durable singleton row is malformed: found {found:?}"
        ))),
    }
}
