use rusqlite::{Connection, Row, params};

use super::SEARCH_INDEXER_VERSION;
use super::coverage::{
    mark_source_coverage_ready, mark_source_coverage_rebuilding, source_exists,
    source_is_user_visible,
};
use super::types::RebuildSearchFilterIndexForSourceResult;
use crate::{LibrarySqliteError, LibrarySqliteResult};

pub(crate) fn rebuild_search_filter_index_for_source(
    connection: &Connection,
    source_id: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<RebuildSearchFilterIndexForSourceResult> {
    if !source_exists(connection, source_id)? {
        return Err(LibrarySqliteError::Canonical(crate::CanonicalError::new(
            crate::CanonicalErrorCode::NotFound,
            format!("source {source_id} does not exist"),
        )));
    }

    if !source_is_user_visible(connection, source_id)? {
        let generation =
            purge_source_index_rows_and_advance_generation(connection, source_id, rebuilt_at_ms)?;
        connection.execute(
            "DELETE FROM search_filter_index_source_coverage
             WHERE source_id = ?1",
            [source_id],
        )?;
        connection.execute(
            "UPDATE search_filter_index_metadata
             SET state = 'ready',
                 updated_at = ?1
             WHERE search_filter_index_id = 1",
            [rebuilt_at_ms],
        )?;
        return Ok(RebuildSearchFilterIndexForSourceResult {
            source_id,
            generation,
            rows_indexed: 0,
        });
    }

    mark_source_coverage_rebuilding(connection, source_id, rebuilt_at_ms)?;
    connection.execute(
        "UPDATE search_filter_index_metadata
         SET state = 'rebuilding',
             updated_at = ?1
         WHERE search_filter_index_id = 1",
        [rebuilt_at_ms],
    )?;

    purge_source_index_rows(connection, source_id)?;

    let generation = connection.query_row(
        "UPDATE search_filter_index_metadata
         SET generation = generation + 1,
             updated_at = ?1
         WHERE search_filter_index_id = 1
           AND indexer_version = ?2
         RETURNING generation",
        params![rebuilt_at_ms, SEARCH_INDEXER_VERSION],
        |row| row.get::<_, i64>(0),
    )?;

    insert_source_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_source_location_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_directory_rows(connection, source_id, generation, rebuilt_at_ms)?;
    insert_source_file_rows(connection, source_id, generation, rebuilt_at_ms)?;

    connection.execute(
        "INSERT INTO search_filter_index_fts(rowid, display_label, display_path)
         SELECT row_id, display_label, COALESCE(display_path, '')
         FROM search_filter_index_rows
         WHERE source_id = ?1",
        [source_id],
    )?;

    let rows_indexed = connection.query_row(
        "SELECT COUNT(*)
         FROM search_filter_index_rows
         WHERE source_id = ?1",
        [source_id],
        |row| read_usize(row, 0),
    )?;

    mark_source_coverage_ready(connection, source_id, generation, rebuilt_at_ms)?;
    connection.execute(
        "UPDATE search_filter_index_metadata
         SET state = 'ready',
             updated_at = ?1
         WHERE search_filter_index_id = 1",
        [rebuilt_at_ms],
    )?;

    Ok(RebuildSearchFilterIndexForSourceResult {
        source_id,
        generation,
        rows_indexed,
    })
}

fn purge_source_index_rows_and_advance_generation(
    connection: &Connection,
    source_id: i64,
    updated_at_ms: i64,
) -> LibrarySqliteResult<i64> {
    purge_source_index_rows(connection, source_id)?;
    connection
        .query_row(
            "UPDATE search_filter_index_metadata
             SET generation = generation + 1,
                 updated_at = ?1
             WHERE search_filter_index_id = 1
               AND indexer_version = ?2
             RETURNING generation",
            params![updated_at_ms, SEARCH_INDEXER_VERSION],
            |row| row.get::<_, i64>(0),
        )
        .map_err(Into::into)
}

fn purge_source_index_rows(connection: &Connection, source_id: i64) -> LibrarySqliteResult<()> {
    let old_row_ids = connection
        .prepare(
            "SELECT row_id
             FROM search_filter_index_rows
             WHERE source_id = ?1",
        )?
        .query_map([source_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for row_id in old_row_ids {
        connection.execute(
            "DELETE FROM search_filter_index_fts WHERE rowid = ?1",
            [row_id],
        )?;
    }
    connection.execute(
        "DELETE FROM search_filter_index_rows WHERE source_id = ?1",
        [source_id],
    )?;
    Ok(())
}

fn insert_source_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             display_label, display_path, sort_key, source_access_state,
             source_scan_phase, evidence_coverage_state, updated_at
         )
         SELECT ?2, 'source', 'source', 'source:' || source.source_id, source.source_id,
                source.display_name,
                COALESCE(state.effective_path, locator.absolute_path),
                lower(source.display_name),
                state.access_state,
                scan.scan_phase,
                'not_applicable',
                ?3
         FROM sources source
         LEFT JOIN source_state state ON state.source_id = source.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = source.source_id
         LEFT JOIN source_locators locator ON locator.source_id = source.source_id
         WHERE source.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_source_location_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             source_location_id, display_label, display_path, relative_path, sort_key,
             source_access_state, source_scan_phase, presence_state, evidence_coverage_state,
             updated_at
         )
         SELECT ?2, 'source_location', 'source_location',
                'sourceLocation:' || location.source_location_id,
                location.source_id,
                location.source_location_id,
                COALESCE(location.display_name, location.relative_path),
                location.relative_path,
                location.relative_path,
                lower(location.relative_path),
                state.access_state,
                scan.scan_phase,
                CASE WHEN directory.source_directory_id IS NULL THEN 'missing' ELSE directory.presence_state END,
                'not_applicable',
                ?3
         FROM source_locations location
         LEFT JOIN source_directories directory
           ON directory.source_id = location.source_id
          AND directory.relative_path = location.relative_path
         LEFT JOIN source_state state ON state.source_id = location.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = location.source_id
         WHERE location.source_id = ?1
           AND location.is_user_visible = 1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_directory_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             source_directory_id, parent_source_directory_id, display_label, display_path,
             relative_path, sort_key, presence_state, source_access_state, source_scan_phase,
             evidence_coverage_state, updated_at
         )
         SELECT ?2, 'directory', 'source_hierarchy',
                'directory:' || directory.source_directory_id,
                directory.source_id,
                directory.source_directory_id,
                directory.parent_source_directory_id,
                directory.name,
                directory.relative_path,
                directory.relative_path,
                directory.name_browse_sort_key || '/' || directory.relative_path,
                directory.presence_state,
                state.access_state,
                scan.scan_phase,
                'not_applicable',
                ?3
         FROM source_directories directory
         LEFT JOIN source_state state ON state.source_id = directory.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = directory.source_id
         WHERE directory.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn insert_source_file_rows(
    connection: &Connection,
    source_id: i64,
    generation: i64,
    rebuilt_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_rows (
             generation, result_kind, authority_layer, stable_key, source_id,
             parent_source_directory_id, source_file_id, display_label, display_path,
             relative_path, sort_key, file_class, file_kind, media_relevance,
             presence_state, source_access_state, source_scan_phase, has_current_blake3,
             has_current_probe, attachment_link_state, attachment_id, content_hash_algorithm,
             content_hash_value, evidence_coverage_state, updated_at
         )
         SELECT ?2,
                'source_file',
                'source_file_inventory',
                'sourceFile:' || file.source_file_id,
                file.source_id,
                file.parent_source_directory_id,
                file.source_file_id,
                file.name,
                file.relative_path,
                file.relative_path,
                file.relative_path_browse_sort_key || '/' || file.relative_path,
                file.file_class,
                file.file_kind,
                CASE
                    WHEN file.file_class IN ('audio', 'video') THEN 'playable_media'
                    WHEN file.file_kind = 'cue_sheet' THEN 'audio_workflow'
                    WHEN file.file_class IN ('image', 'unsupported') THEN 'explicit_inventory'
                    WHEN file.file_kind IN ('log_doc', 'text_doc') THEN 'companion_file'
                    ELSE 'not_media_relevant'
                END,
                file.presence_state,
                state.access_state,
                scan.scan_phase,
                CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND facts.content_hash_algorithm = 'blake3'
                     AND facts.content_hash_value IS NOT NULL
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                    THEN 1 ELSE 0
                END,
                CASE
                    WHEN facts.source_file_id IS NOT NULL
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                     AND (
                         facts.mime_type IS NOT NULL
                         OR facts.duration_ms IS NOT NULL
                         OR facts.sample_rate_hz IS NOT NULL
                         OR facts.channels IS NOT NULL
                         OR facts.bit_depth IS NOT NULL
                         OR facts.codec IS NOT NULL
                     )
                    THEN 1 ELSE 0
                END,
                CASE
                    WHEN link.source_file_attachment_link_id IS NULL THEN 'missing'
                    WHEN facts.source_file_id IS NOT NULL
                     AND facts.content_hash_algorithm = attachment.content_hash_algorithm
                     AND facts.content_hash_value = attachment.content_hash_value
                     AND file.source_id = facts.basis_source_id
                     AND file.relative_path = facts.basis_relative_path
                     AND file.size_bytes IS facts.basis_size_bytes
                     AND file.mtime_ns IS facts.basis_mtime_ns
                     AND file.presence_state = facts.basis_presence_state
                    THEN 'current'
                    ELSE 'stale'
                END,
                attachment.attachment_id,
                attachment.content_hash_algorithm,
                attachment.content_hash_value,
                'indexed',
                ?3
         FROM source_files file
         LEFT JOIN source_state state ON state.source_id = file.source_id
         LEFT JOIN source_scan_state scan ON scan.source_id = file.source_id
         LEFT JOIN SourceFacts facts ON facts.source_file_id = file.source_file_id
         LEFT JOIN source_file_attachment_links link ON link.source_file_id = file.source_file_id
         LEFT JOIN content_attachments attachment ON attachment.attachment_id = link.attachment_id
         WHERE file.source_id = ?1",
        params![source_id, generation, rebuilt_at_ms],
    )?;
    Ok(())
}

fn read_usize(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}
