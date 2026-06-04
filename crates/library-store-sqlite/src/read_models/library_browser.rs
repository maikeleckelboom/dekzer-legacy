use rusqlite::{Connection, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::LibraryBrowseScope;

#[derive(Debug, Clone, PartialEq)]
pub struct ScopedLibraryAssetBrowserRow {
    pub library_asset_id: i64,
    pub row_version: i64,
    pub primary_source_file_id: Option<i64>,
    pub scoped_source_file_id: Option<i64>,
    pub source_id: Option<i64>,
    pub relative_path: Option<String>,
    pub file_name: Option<String>,
    // Finite projection vocabulary enforced by the baseline schema and mapped
    // to protocol DTOs by the future boundary service.
    pub availability_state: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub musical_key: Option<String>,
    pub tempo_bpm: Option<f64>,
    // Browse-facing summaries derived by projection rebuild logic from
    // capability/artifact state; they are not canonical authority.
    pub waveform_quality_current: Option<i64>,
    pub waveform_quality_target: Option<i64>,
    // Finite browse vocabulary mapped to a typed protocol enum by the future
    // boundary service: missing, queued, leased, ready, stale, blocked, failed.
    pub stems_state_summary: Option<String>,
    // Finite browse projection over effective resolved prep intent plus
    // matching capability state.
    pub prep_readiness_summary: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreLibraryBrowserWindow {
    pub offset: usize,
    pub limit: usize,
    pub total_rows: usize,
    pub rows: Vec<ScopedLibraryAssetBrowserRow>,
}

pub fn read_window(
    connection: &Connection,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    read_window_internal(connection, BrowserReadScope::AllMedia, offset, limit)
}

pub fn read_window_for_scope(
    connection: &Connection,
    scope: LibraryBrowseScope,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    read_window_internal(connection, BrowserReadScope::from(scope), offset, limit)
}

pub fn read_window_for_source(
    connection: &Connection,
    source_id: i64,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    read_window_internal(
        connection,
        BrowserReadScope::Source(source_id),
        offset,
        limit,
    )
}

pub fn search_window(
    connection: &Connection,
    query: &str,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    let Some(match_query) = compile_match_query(query) else {
        return read_window_internal(connection, BrowserReadScope::AllMedia, offset, limit);
    };
    search_window_internal(
        connection,
        BrowserReadScope::AllMedia,
        &match_query,
        offset,
        limit,
    )
}

pub fn search_window_for_scope(
    connection: &Connection,
    scope: LibraryBrowseScope,
    query: &str,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    let scope = BrowserReadScope::from(scope);
    let Some(match_query) = compile_match_query(query) else {
        return read_window_internal(connection, scope, offset, limit);
    };
    search_window_internal(connection, scope, &match_query, offset, limit)
}

fn read_window_internal(
    connection: &Connection,
    scope: BrowserReadScope,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    let total_rows = load_total_rows(connection, scope)?;

    if limit == 0 {
        return Ok(StoreLibraryBrowserWindow {
            offset,
            limit,
            total_rows,
            rows: Vec::new(),
        });
    }

    if offset >= total_rows {
        return Ok(StoreLibraryBrowserWindow {
            offset,
            limit,
            total_rows,
            rows: Vec::new(),
        });
    }

    let offset_i64 = usize_to_i64(offset, "offset")?;
    let upper_bound = offset.checked_add(limit).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "library browser window {offset} + {limit} overflows usize range"
        ))
    })?;
    let upper_bound_i64 = usize_to_i64(upper_bound, "upper bound")?;

    let rows = match scope {
        BrowserReadScope::AllMedia => read_aggregate_window_rows(
            connection,
            None,
            AGGREGATE_DEFAULT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::AllAudio => read_aggregate_window_rows(
            connection,
            Some("sfacts.media_kind = 'audio'"),
            AGGREGATE_DEFAULT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::AllVideos => read_aggregate_window_rows(
            connection,
            Some("sfacts.media_kind = 'video'"),
            AGGREGATE_DEFAULT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::RecentlyAdded => read_aggregate_window_rows(
            connection,
            None,
            AGGREGATE_RECENT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::NeedsPreparation => read_aggregate_window_rows(
            connection,
            Some(NEEDS_PREPARATION_PREDICATE),
            AGGREGATE_DEFAULT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::PlaylistGroup => read_aggregate_window_rows(
            connection,
            Some(
                "EXISTS (
                     SELECT 1
                     FROM PlaylistEntries pe
                     WHERE pe.library_asset_id = pbr.library_asset_id
                 )",
            ),
            AGGREGATE_DEFAULT_ORDER,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::Source(source_id) => {
            let location_scope_cte = accepted_source_locations_cte("?1");
            let source_scope_predicate = source_aggregate_scope_predicate("?1");
            read_scoped_window_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                &source_scope_predicate,
                source_id,
                offset_i64,
                upper_bound_i64,
            )?
        }
        BrowserReadScope::SourceLocation(source_location_id) => {
            let location_scope_cte = accepted_source_location_cte("?1");
            read_scoped_window_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                SOURCE_LOCATION_FILE_SCOPE_PREDICATE,
                source_location_id,
                offset_i64,
                upper_bound_i64,
            )?
        }
        BrowserReadScope::Playlist(playlist_id) => {
            read_playlist_window_rows(connection, playlist_id, offset_i64, upper_bound_i64)?
        }
        BrowserReadScope::PrepPolicy(prep_policy_id) => {
            read_prep_policy_window_rows(connection, prep_policy_id, offset_i64, upper_bound_i64)?
        }
    };

    Ok(StoreLibraryBrowserWindow {
        offset,
        limit,
        total_rows,
        rows: rows.into_iter().map(|row| row.0).collect(),
    })
}

const AGGREGATE_DEFAULT_ORDER: &str = "lower(COALESCE(pbr.title, sf.relative_path, '')) ASC,
                                     lower(COALESCE(pbr.artist, '')) ASC,
                                     lower(COALESCE(pbr.album, '')) ASC,
                                     lower(COALESCE(sf.relative_path, '')) ASC,
                                     pbr.library_asset_id ASC";
const AGGREGATE_RECENT_ORDER: &str = "pbr.updated_at DESC,
                                    pbr.library_asset_id DESC";
const NEEDS_PREPARATION_PREDICATE: &str =
    "pbr.prep_readiness_summary IN ('preparing', 'underprepared', 'blocked', 'failed')";
const SOURCE_LOCATION_FILE_SCOPE_PREDICATE: &str = "EXISTS (
    SELECT 1
    FROM location_scope
    WHERE sf.source_id = location_scope.source_id
      AND (
          sf.relative_path COLLATE BINARY = location_scope.relative_path COLLATE BINARY
          OR (
              sf.relative_path COLLATE BINARY >= location_scope.relative_path || '/'
              AND sf.relative_path COLLATE BINARY < location_scope.relative_path || char(48)
          )
      )
)";

fn read_aggregate_window_rows(
    connection: &Connection,
    where_clause: Option<&str>,
    order_clause: &str,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let where_sql = where_clause
        .map(|where_clause| format!("WHERE {where_clause}"))
        .unwrap_or_default();
    let sql = format!(
        "WITH numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY {order_clause}
                    ) AS row_number
             FROM LibraryBrowserRows pbr
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             LEFT JOIN SourceFacts sfacts
               ON sfacts.source_file_id = sf.source_file_id
             {where_sql}
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?1
           AND row_number <= ?2
         ORDER BY row_number ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(
            params![offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn read_scoped_window_rows(
    connection: &Connection,
    cte_prefix: String,
    scope_predicate: &str,
    scope_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let sql = format!(
        "{cte_prefix}
             source_scope AS (
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
                        pia.accepted_at,
                        ss.ordinal,
                        ROW_NUMBER() OVER (
                            PARTITION BY pbr.library_asset_id
                            ORDER BY pia.accepted_at ASC, ss.ordinal ASC, sf.source_file_id ASC
                        ) AS attachment_rank
                 FROM LibraryBrowserRows pbr
                 JOIN LibraryAssetAttachments pia
                   ON pia.library_asset_id = pbr.library_asset_id
                 JOIN SourceSegments ss
                   ON ss.source_segment_id = pia.source_segment_id
                 JOIN SourceSegmentSets sss
                   ON sss.source_segment_set_id = ss.source_segment_set_id
                 JOIN source_files sf
                   ON sf.source_file_id = sss.source_file_id
                 WHERE {scope_predicate}
             ),
             dedup AS (
                 SELECT library_asset_id,
                        row_version,
                        primary_source_file_id,
                        source_file_id,
                        source_id,
                        relative_path,
                        name,
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
                 FROM source_scope
                 WHERE attachment_rank = 1
             ),
             numbered AS (
                 SELECT library_asset_id,
                        row_version,
                        primary_source_file_id,
                        source_file_id,
                        source_id,
                        relative_path,
                        name,
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
                        COUNT(*) OVER () AS total_rows,
                        ROW_NUMBER() OVER (
                            ORDER BY lower(COALESCE(title, relative_path, '')),
                                     lower(COALESCE(artist, '')),
                                     lower(COALESCE(album, '')),
                                     lower(COALESCE(relative_path, '')),
                                     library_asset_id ASC
                        ) AS row_number
                 FROM dedup
             )
             SELECT library_asset_id,
                    row_version,
                    primary_source_file_id,
                    source_file_id,
                    source_id,
                    relative_path,
                    name,
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
                    total_rows
             FROM numbered
             WHERE row_number > ?2
               AND row_number <= ?3
             ORDER BY row_number ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(
            params![scope_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn read_playlist_window_rows(
    connection: &Connection,
    playlist_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let mut statement = connection.prepare(
        "WITH numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY pe.position ASC,
                                 pe.playlist_entry_id ASC,
                                 pbr.library_asset_id ASC
                    ) AS row_number
             FROM PlaylistEntries pe
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = pe.library_asset_id
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             WHERE pe.playlist_id = ?1
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?2
           AND row_number <= ?3
         ORDER BY row_number ASC",
    )?;
    let rows = statement
        .query_map(
            params![playlist_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn read_prep_policy_window_rows(
    connection: &Connection,
    prep_policy_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let mut statement = connection.prepare(
        "WITH numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY lower(COALESCE(pbr.title, sf.relative_path, '')) ASC,
                                 lower(COALESCE(pbr.artist, '')) ASC,
                                 lower(COALESCE(pbr.album, '')) ASC,
                                 lower(COALESCE(sf.relative_path, '')) ASC,
                                 pbr.library_asset_id ASC
                    ) AS row_number
             FROM LibraryBrowserRows pbr
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             WHERE EXISTS (
                 SELECT 1
                 FROM ResolvedLibraryAssetPrepTargets rpt
                 WHERE rpt.library_asset_id = pbr.library_asset_id
                   AND rpt.resolved_from_policy_id = ?1
             )
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?2
           AND row_number <= ?3
         ORDER BY row_number ASC",
    )?;
    let rows = statement
        .query_map(
            params![prep_policy_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn search_window_internal(
    connection: &Connection,
    scope: BrowserReadScope,
    match_query: &str,
    offset: usize,
    limit: usize,
) -> LibrarySqliteResult<StoreLibraryBrowserWindow> {
    let total_rows = load_total_search_rows(connection, scope, match_query)?;

    if limit == 0 {
        return Ok(StoreLibraryBrowserWindow {
            offset,
            limit,
            total_rows,
            rows: Vec::new(),
        });
    }

    if offset >= total_rows {
        return Ok(StoreLibraryBrowserWindow {
            offset,
            limit,
            total_rows,
            rows: Vec::new(),
        });
    }

    let offset_i64 = usize_to_i64(offset, "offset")?;
    let upper_bound = offset.checked_add(limit).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "library browser window {offset} + {limit} overflows usize range"
        ))
    })?;
    let upper_bound_i64 = usize_to_i64(upper_bound, "upper bound")?;

    let rows = match scope {
        BrowserReadScope::AllMedia => search_aggregate_window_rows(
            connection,
            None,
            AGGREGATE_SEARCH_DEFAULT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::AllAudio => search_aggregate_window_rows(
            connection,
            Some("sfacts.media_kind = 'audio'"),
            AGGREGATE_SEARCH_DEFAULT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::AllVideos => search_aggregate_window_rows(
            connection,
            Some("sfacts.media_kind = 'video'"),
            AGGREGATE_SEARCH_DEFAULT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::RecentlyAdded => search_aggregate_window_rows(
            connection,
            None,
            AGGREGATE_SEARCH_RECENT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::NeedsPreparation => search_aggregate_window_rows(
            connection,
            Some(NEEDS_PREPARATION_PREDICATE),
            AGGREGATE_SEARCH_DEFAULT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::PlaylistGroup => search_aggregate_window_rows(
            connection,
            Some(
                "EXISTS (
                     SELECT 1
                     FROM PlaylistEntries pe
                     WHERE pe.library_asset_id = pbr.library_asset_id
                 )",
            ),
            AGGREGATE_SEARCH_DEFAULT_ORDER,
            match_query,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::Source(source_id) => {
            let location_scope_cte = accepted_source_locations_cte("?2");
            let source_scope_predicate = source_aggregate_scope_predicate("?2");
            search_scoped_window_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                &source_scope_predicate,
                match_query,
                source_id,
                offset_i64,
                upper_bound_i64,
            )?
        }
        BrowserReadScope::SourceLocation(source_location_id) => {
            let location_scope_cte = accepted_source_location_cte("?2");
            search_scoped_window_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                SOURCE_LOCATION_FILE_SCOPE_PREDICATE,
                match_query,
                source_location_id,
                offset_i64,
                upper_bound_i64,
            )?
        }
        BrowserReadScope::Playlist(playlist_id) => search_playlist_window_rows(
            connection,
            match_query,
            playlist_id,
            offset_i64,
            upper_bound_i64,
        )?,
        BrowserReadScope::PrepPolicy(prep_policy_id) => search_prep_policy_window_rows(
            connection,
            match_query,
            prep_policy_id,
            offset_i64,
            upper_bound_i64,
        )?,
    };

    Ok(StoreLibraryBrowserWindow {
        offset,
        limit,
        total_rows,
        rows: rows.into_iter().map(|row| row.0).collect(),
    })
}

const AGGREGATE_SEARCH_DEFAULT_ORDER: &str = "matched.rank ASC,
                                            lower(COALESCE(pbr.title, sf.relative_path, '')) ASC,
                                            lower(COALESCE(pbr.artist, '')) ASC,
                                            lower(COALESCE(pbr.album, '')) ASC,
                                            lower(COALESCE(sf.relative_path, '')) ASC,
                                            pbr.library_asset_id ASC";
const AGGREGATE_SEARCH_RECENT_ORDER: &str = "matched.rank ASC,
                                           pbr.updated_at DESC,
                                           pbr.library_asset_id DESC";

fn search_aggregate_window_rows(
    connection: &Connection,
    where_clause: Option<&str>,
    order_clause: &str,
    match_query: &str,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let where_sql = where_clause
        .map(|where_clause| format!("WHERE {where_clause}"))
        .unwrap_or_default();
    let sql = format!(
        "WITH matched AS (
             SELECT rowid AS library_asset_id,
                    bm25(LibraryBrowserRows_fts) AS rank
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         ),
         numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY {order_clause}
                    ) AS row_number
             FROM matched
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             LEFT JOIN SourceFacts sfacts
               ON sfacts.source_file_id = sf.source_file_id
             {where_sql}
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?2
           AND row_number <= ?3
         ORDER BY row_number ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(
            params![match_query, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn search_playlist_window_rows(
    connection: &Connection,
    match_query: &str,
    playlist_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let mut statement = connection.prepare(
        "WITH matched AS (
             SELECT rowid AS library_asset_id,
                    bm25(LibraryBrowserRows_fts) AS rank
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         ),
         numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY matched.rank ASC,
                                 pe.position ASC,
                                 pe.playlist_entry_id ASC,
                                 lower(COALESCE(pbr.title, sf.relative_path, '')),
                                 pbr.library_asset_id ASC
                    ) AS row_number
             FROM matched
             JOIN PlaylistEntries pe
               ON pe.library_asset_id = matched.library_asset_id
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             WHERE pe.playlist_id = ?2
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?3
           AND row_number <= ?4
         ORDER BY row_number ASC",
    )?;
    let rows = statement
        .query_map(
            params![match_query, playlist_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn search_scoped_window_rows(
    connection: &Connection,
    cte_prefix: String,
    scope_predicate: &str,
    match_query: &str,
    scope_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let sql = format!(
        "{cte_prefix}
         matched AS (
             SELECT rowid AS library_asset_id,
                    bm25(LibraryBrowserRows_fts) AS rank
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         ),
         source_scope AS (
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
                    matched.rank,
                    pia.accepted_at,
                    ss.ordinal,
                    ROW_NUMBER() OVER (
                        PARTITION BY pbr.library_asset_id
                        ORDER BY pia.accepted_at ASC, ss.ordinal ASC, sf.source_file_id ASC
                    ) AS attachment_rank
             FROM matched
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             JOIN LibraryAssetAttachments pia
               ON pia.library_asset_id = pbr.library_asset_id
             JOIN SourceSegments ss
               ON ss.source_segment_id = pia.source_segment_id
             JOIN SourceSegmentSets sss
               ON sss.source_segment_set_id = ss.source_segment_set_id
             JOIN source_files sf
               ON sf.source_file_id = sss.source_file_id
             WHERE {scope_predicate}
         ),
         dedup AS (
             SELECT library_asset_id,
                    row_version,
                    primary_source_file_id,
                    source_file_id,
                    source_id,
                    relative_path,
                    name,
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
                    rank
             FROM source_scope
             WHERE attachment_rank = 1
         ),
         numbered AS (
             SELECT library_asset_id,
                    row_version,
                    primary_source_file_id,
                    source_file_id,
                    source_id,
                    relative_path,
                    name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY rank ASC,
                                 lower(COALESCE(title, relative_path, '')),
                                 lower(COALESCE(artist, '')),
                                 lower(COALESCE(album, '')),
                                 lower(COALESCE(relative_path, '')),
                                 library_asset_id ASC
                    ) AS row_number
             FROM dedup
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?3
           AND row_number <= ?4
         ORDER BY row_number ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(
            params![match_query, scope_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn search_prep_policy_window_rows(
    connection: &Connection,
    match_query: &str,
    prep_policy_id: i64,
    offset_i64: i64,
    upper_bound_i64: i64,
) -> LibrarySqliteResult<Vec<(ScopedLibraryAssetBrowserRow, i64)>> {
    let mut statement = connection.prepare(
        "WITH matched AS (
             SELECT rowid AS library_asset_id,
                    bm25(LibraryBrowserRows_fts) AS rank
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         ),
         numbered AS (
             SELECT pbr.library_asset_id,
                    pbr.row_version,
                    pbr.primary_source_file_id,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
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
                    COUNT(*) OVER () AS total_rows,
                    ROW_NUMBER() OVER (
                        ORDER BY matched.rank ASC,
                                 lower(COALESCE(pbr.title, sf.relative_path, '')) ASC,
                                 lower(COALESCE(pbr.artist, '')) ASC,
                                 lower(COALESCE(pbr.album, '')) ASC,
                                 lower(COALESCE(sf.relative_path, '')) ASC,
                                 pbr.library_asset_id ASC
                    ) AS row_number
             FROM matched
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             LEFT JOIN source_files sf
               ON sf.source_file_id = pbr.primary_source_file_id
             WHERE EXISTS (
                 SELECT 1
                 FROM ResolvedLibraryAssetPrepTargets rpt
                 WHERE rpt.library_asset_id = pbr.library_asset_id
                   AND rpt.resolved_from_policy_id = ?2
             )
         )
         SELECT library_asset_id,
                row_version,
                primary_source_file_id,
                source_file_id,
                source_id,
                relative_path,
                name,
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
                total_rows
         FROM numbered
         WHERE row_number > ?3
           AND row_number <= ?4
         ORDER BY row_number ASC",
    )?;
    let rows = statement
        .query_map(
            params![match_query, prep_policy_id, offset_i64, upper_bound_i64],
            library_browser_row_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()
        .map_err(LibrarySqliteError::from)?;
    Ok(rows)
}

fn load_total_rows(connection: &Connection, scope: BrowserReadScope) -> LibrarySqliteResult<usize> {
    let total_rows = match scope {
        BrowserReadScope::AllMedia => load_total_aggregate_rows(connection, None)?,
        BrowserReadScope::AllAudio => {
            load_total_aggregate_rows(connection, Some("sfacts.media_kind = 'audio'"))?
        }
        BrowserReadScope::AllVideos => {
            load_total_aggregate_rows(connection, Some("sfacts.media_kind = 'video'"))?
        }
        BrowserReadScope::RecentlyAdded => load_total_aggregate_rows(connection, None)?,
        BrowserReadScope::NeedsPreparation => {
            load_total_aggregate_rows(connection, Some(NEEDS_PREPARATION_PREDICATE))?
        }
        BrowserReadScope::PlaylistGroup => load_total_aggregate_rows(
            connection,
            Some(
                "EXISTS (
                     SELECT 1
                     FROM PlaylistEntries pe
                     WHERE pe.library_asset_id = pbr.library_asset_id
                 )",
            ),
        )?,
        BrowserReadScope::Source(source_id) => {
            let location_scope_cte = accepted_source_locations_cte("?1");
            let source_scope_predicate = source_aggregate_scope_predicate("?1");
            load_total_scoped_rows(
                connection,
                scoped_single_cte_prefix(&location_scope_cte),
                &source_scope_predicate,
                source_id,
            )?
        }
        BrowserReadScope::SourceLocation(source_location_id) => {
            let location_scope_cte = accepted_source_location_cte("?1");
            load_total_scoped_rows(
                connection,
                scoped_single_cte_prefix(&location_scope_cte),
                SOURCE_LOCATION_FILE_SCOPE_PREDICATE,
                source_location_id,
            )?
        }
        BrowserReadScope::Playlist(playlist_id) => connection.query_row(
            "SELECT COUNT(*)
             FROM PlaylistEntries pe
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = pe.library_asset_id
             WHERE pe.playlist_id = ?1",
            [playlist_id],
            |row| row.get::<_, i64>(0),
        )?,
        BrowserReadScope::PrepPolicy(prep_policy_id) => connection.query_row(
            "SELECT COUNT(*)
             FROM LibraryBrowserRows pbr
             WHERE EXISTS (
                 SELECT 1
                 FROM ResolvedLibraryAssetPrepTargets rpt
                 WHERE rpt.library_asset_id = pbr.library_asset_id
                   AND rpt.resolved_from_policy_id = ?1
             )",
            [prep_policy_id],
            |row| row.get::<_, i64>(0),
        )?,
    };

    usize::try_from(total_rows).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "library browser total row count {total_rows} exceeds usize range"
        ))
    })
}

fn load_total_search_rows(
    connection: &Connection,
    scope: BrowserReadScope,
    match_query: &str,
) -> LibrarySqliteResult<usize> {
    let total_rows = match scope {
        BrowserReadScope::AllMedia => {
            load_total_aggregate_search_rows(connection, None, match_query)?
        }
        BrowserReadScope::AllAudio => load_total_aggregate_search_rows(
            connection,
            Some("sfacts.media_kind = 'audio'"),
            match_query,
        )?,
        BrowserReadScope::AllVideos => load_total_aggregate_search_rows(
            connection,
            Some("sfacts.media_kind = 'video'"),
            match_query,
        )?,
        BrowserReadScope::RecentlyAdded => {
            load_total_aggregate_search_rows(connection, None, match_query)?
        }
        BrowserReadScope::NeedsPreparation => load_total_aggregate_search_rows(
            connection,
            Some(NEEDS_PREPARATION_PREDICATE),
            match_query,
        )?,
        BrowserReadScope::PlaylistGroup => load_total_aggregate_search_rows(
            connection,
            Some(
                "EXISTS (
                     SELECT 1
                     FROM PlaylistEntries pe
                     WHERE pe.library_asset_id = pbr.library_asset_id
                 )",
            ),
            match_query,
        )?,
        BrowserReadScope::Source(source_id) => {
            let location_scope_cte = accepted_source_locations_cte("?2");
            let source_scope_predicate = source_aggregate_scope_predicate("?2");
            load_total_scoped_search_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                &source_scope_predicate,
                match_query,
                source_id,
            )?
        }
        BrowserReadScope::SourceLocation(source_location_id) => {
            let location_scope_cte = accepted_source_location_cte("?2");
            load_total_scoped_search_rows(
                connection,
                scoped_read_cte_prefix(Some(&location_scope_cte)),
                SOURCE_LOCATION_FILE_SCOPE_PREDICATE,
                match_query,
                source_location_id,
            )?
        }
        BrowserReadScope::Playlist(playlist_id) => connection.query_row(
            "WITH matched AS (
                 SELECT rowid AS library_asset_id
                 FROM LibraryBrowserRows_fts
                 WHERE LibraryBrowserRows_fts MATCH ?1
             )
             SELECT COUNT(*)
             FROM matched
             JOIN PlaylistEntries pe
               ON pe.library_asset_id = matched.library_asset_id
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             WHERE pe.playlist_id = ?2",
            params![match_query, playlist_id],
            |row| row.get::<_, i64>(0),
        )?,
        BrowserReadScope::PrepPolicy(prep_policy_id) => connection.query_row(
            "WITH matched AS (
                 SELECT rowid AS library_asset_id
                 FROM LibraryBrowserRows_fts
                 WHERE LibraryBrowserRows_fts MATCH ?1
             )
             SELECT COUNT(*)
             FROM matched
             JOIN LibraryBrowserRows pbr
               ON pbr.library_asset_id = matched.library_asset_id
             WHERE EXISTS (
                 SELECT 1
                 FROM ResolvedLibraryAssetPrepTargets rpt
                 WHERE rpt.library_asset_id = pbr.library_asset_id
                   AND rpt.resolved_from_policy_id = ?2
             )",
            params![match_query, prep_policy_id],
            |row| row.get::<_, i64>(0),
        )?,
    };

    usize::try_from(total_rows).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "library browser total row count {total_rows} exceeds usize range"
        ))
    })
}

fn load_total_aggregate_rows(
    connection: &Connection,
    where_clause: Option<&str>,
) -> LibrarySqliteResult<i64> {
    let where_sql = where_clause
        .map(|where_clause| format!("WHERE {where_clause}"))
        .unwrap_or_default();
    let sql = format!(
        "SELECT COUNT(*)
         FROM LibraryBrowserRows pbr
         LEFT JOIN source_files sf
           ON sf.source_file_id = pbr.primary_source_file_id
         LEFT JOIN SourceFacts sfacts
           ON sfacts.source_file_id = sf.source_file_id
         {where_sql}"
    );
    connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn load_total_aggregate_search_rows(
    connection: &Connection,
    where_clause: Option<&str>,
    match_query: &str,
) -> LibrarySqliteResult<i64> {
    let where_sql = where_clause
        .map(|where_clause| format!("WHERE {where_clause}"))
        .unwrap_or_default();
    let sql = format!(
        "WITH matched AS (
             SELECT rowid AS library_asset_id
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         )
         SELECT COUNT(*)
         FROM matched
         JOIN LibraryBrowserRows pbr
           ON pbr.library_asset_id = matched.library_asset_id
         LEFT JOIN source_files sf
           ON sf.source_file_id = pbr.primary_source_file_id
         LEFT JOIN SourceFacts sfacts
           ON sfacts.source_file_id = sf.source_file_id
         {where_sql}"
    );
    connection
        .query_row(&sql, [match_query], |row| row.get(0))
        .map_err(Into::into)
}

fn load_total_scoped_rows(
    connection: &Connection,
    cte_prefix: String,
    scope_predicate: &str,
    scope_id: i64,
) -> LibrarySqliteResult<i64> {
    let sql = format!(
        "{cte_prefix}
         SELECT COUNT(DISTINCT pbr.library_asset_id)
         FROM LibraryBrowserRows pbr
         JOIN LibraryAssetAttachments pia
           ON pia.library_asset_id = pbr.library_asset_id
         JOIN SourceSegments ss
           ON ss.source_segment_id = pia.source_segment_id
         JOIN SourceSegmentSets sss
           ON sss.source_segment_set_id = ss.source_segment_set_id
         JOIN source_files sf
           ON sf.source_file_id = sss.source_file_id
         WHERE {scope_predicate}"
    );
    connection
        .query_row(&sql, [scope_id], |row| row.get(0))
        .map_err(Into::into)
}

fn load_total_scoped_search_rows(
    connection: &Connection,
    cte_prefix: String,
    scope_predicate: &str,
    match_query: &str,
    scope_id: i64,
) -> LibrarySqliteResult<i64> {
    let sql = format!(
        "{cte_prefix}
         matched AS (
             SELECT rowid AS library_asset_id
             FROM LibraryBrowserRows_fts
             WHERE LibraryBrowserRows_fts MATCH ?1
         )
         SELECT COUNT(DISTINCT pbr.library_asset_id)
         FROM matched
         JOIN LibraryBrowserRows pbr
           ON pbr.library_asset_id = matched.library_asset_id
         JOIN LibraryAssetAttachments pia
           ON pia.library_asset_id = pbr.library_asset_id
         JOIN SourceSegments ss
           ON ss.source_segment_id = pia.source_segment_id
         JOIN SourceSegmentSets sss
           ON sss.source_segment_set_id = ss.source_segment_set_id
         JOIN source_files sf
           ON sf.source_file_id = sss.source_file_id
         WHERE {scope_predicate}"
    );
    connection
        .query_row(&sql, params![match_query, scope_id], |row| row.get(0))
        .map_err(Into::into)
}

fn accepted_source_locations_cte(scope_id_parameter: &str) -> String {
    format!(
        "location_scope(source_id, relative_path) AS (
             SELECT source_id, relative_path
             FROM source_locations
             WHERE source_id = {scope_id_parameter}
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1
         )"
    )
}

fn accepted_source_location_cte(scope_id_parameter: &str) -> String {
    format!(
        "location_scope(source_id, relative_path) AS (
             SELECT source_id, relative_path
             FROM source_locations
             WHERE source_location_id = {scope_id_parameter}
               AND authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1
         )"
    )
}

fn source_aggregate_scope_predicate(scope_id_parameter: &str) -> String {
    format!(
        "sf.source_id = {scope_id_parameter}
         AND (
             NOT EXISTS (SELECT 1 FROM location_scope)
             OR {SOURCE_LOCATION_FILE_SCOPE_PREDICATE}
         )"
    )
}

fn scoped_read_cte_prefix(recursive_scope: Option<&str>) -> String {
    match recursive_scope {
        Some(recursive_scope) => format!("WITH RECURSIVE {recursive_scope},"),
        None => "WITH".to_string(),
    }
}

fn scoped_single_cte_prefix(scope: &str) -> String {
    format!("WITH {scope}")
}

fn usize_to_i64(value: usize, label: &str) -> LibrarySqliteResult<i64> {
    i64::try_from(value).map_err(|_| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "library browser {label} {value} exceeds i64 range"
        ))
    })
}

fn compile_match_query(query: &str) -> Option<String> {
    let terms = query
        .split_whitespace()
        .map(|term| term.trim_matches('"'))
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", escape_fts_term(term)))
        .collect::<Vec<_>>();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn escape_fts_term(term: &str) -> String {
    term.replace('"', "\"\"")
}

fn library_browser_row_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(ScopedLibraryAssetBrowserRow, i64)> {
    Ok((
        ScopedLibraryAssetBrowserRow {
            library_asset_id: row.get(0)?,
            row_version: row.get(1)?,
            primary_source_file_id: row.get(2)?,
            scoped_source_file_id: row.get(3)?,
            source_id: row.get(4)?,
            relative_path: row.get(5)?,
            file_name: row.get(6)?,
            availability_state: row.get(7)?,
            title: row.get(8)?,
            artist: row.get(9)?,
            album: row.get(10)?,
            duration_ms: row.get(11)?,
            musical_key: row.get(12)?,
            tempo_bpm: row.get(13)?,
            waveform_quality_current: row.get(14)?,
            waveform_quality_target: row.get(15)?,
            stems_state_summary: row.get(16)?,
            prep_readiness_summary: row.get(17)?,
            updated_at: row.get(18)?,
        },
        row.get(19)?,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserReadScope {
    AllMedia,
    AllAudio,
    AllVideos,
    RecentlyAdded,
    NeedsPreparation,
    PlaylistGroup,
    Source(i64),
    SourceLocation(i64),
    Playlist(i64),
    PrepPolicy(i64),
}

impl From<LibraryBrowseScope> for BrowserReadScope {
    fn from(scope: LibraryBrowseScope) -> Self {
        match scope {
            LibraryBrowseScope::AllMedia => Self::AllMedia,
            LibraryBrowseScope::AllAudio => Self::AllAudio,
            LibraryBrowseScope::AllVideos => Self::AllVideos,
            LibraryBrowseScope::RecentlyAdded => Self::RecentlyAdded,
            LibraryBrowseScope::NeedsPreparation => Self::NeedsPreparation,
            LibraryBrowseScope::PlaylistGroup => Self::PlaylistGroup,
            LibraryBrowseScope::Source(id) => Self::Source(id.get()),
            LibraryBrowseScope::SourceLocation(id) => Self::SourceLocation(id.get()),
            LibraryBrowseScope::Playlist(id) => Self::Playlist(id.get()),
            LibraryBrowseScope::PrepPolicyScope(id) => Self::PrepPolicy(id.get()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{read_window_for_scope, search_window_for_scope};
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::{
        LibraryBrowseScope, PlaylistId, PrepPolicyId, SourceId, SourceLocationId,
    };
    use rusqlite::{Connection, params};

    fn open_browser_scope_test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install canonical baseline");
        connection
    }

    fn insert_source(connection: &Connection, source_id: i64, label: &str) {
        connection
            .execute(
                "INSERT INTO sources (
                     source_id,
                     source_class,
                     authority,
                     identity_key,
                     display_name,
                     medium_label,
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'internal', 'system', ?2, ?3, NULL, 1, 1, 1)",
                params![source_id, format!("test:{label}"), label],
            )
            .expect("insert source");
        connection
            .execute(
                "INSERT INTO browser_user_order (
                     node_domain,
                     node_id,
                     parent_scope,
                     ordinal,
                     created_at,
                     updated_at
                 )
                 VALUES ('source', ?1, NULL, ?2, 1, 1)",
                params![source_id.to_string(), source_id],
            )
            .expect("insert source order");
    }

    fn insert_directory(
        connection: &Connection,
        source_directory_id: i64,
        source_id: i64,
        parent_source_directory_id: Option<i64>,
        name: &str,
        relative_path: &str,
    ) {
        let name_browse_sort_key =
            crate::browse_sort_key::compute_name_browse_sort_key(name);
        let relative_path_browse_sort_key =
            crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path);
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_browse_sort_key,
                     relative_path_browse_sort_key,
                     relative_path,
                     presence_state,
                     dir_scan_updated_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'present', 1, 1, 1)",
                params![
                    source_directory_id,
                    source_id,
                    parent_source_directory_id,
                    name,
                    name_browse_sort_key,
                    relative_path_browse_sort_key,
                    relative_path,
                ],
            )
            .expect("insert source directory");
    }

    fn insert_source_location(
        connection: &Connection,
        source_location_id: i64,
        source_id: i64,
        display_name: &str,
        relative_path: &str,
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
                 VALUES (?1, ?2, 'user', 'registered_subpath', ?3, ?4, 1, 1, 1)",
                params![source_location_id, source_id, relative_path, display_name],
            )
            .expect("insert source location");
    }

    fn insert_source_fact(connection: &Connection, source_file_id: i64, media_kind: &str) {
        connection
            .execute(
                "INSERT INTO SourceFacts (
                     source_file_id,
                     fact_kind,
                     basis_fingerprint,
                     basis_source_id,
                     basis_relative_path,
                     basis_size_bytes,
                     basis_mtime_ns,
                     basis_presence_state,
                     observed_at_ms,
                     content_hash_algorithm,
                     content_hash_value,
                     media_kind,
                     mime_type,
                     duration_ms,
                     sample_rate_hz,
                     channels,
                     bit_depth,
                     codec,
                     updated_at,
                     accepted_artifact_id
                 )
                 SELECT source_file_id,
                        'source_inspection',
                        ?2,
                        source_id,
                        relative_path,
                        size_bytes,
                        mtime_ns,
                        presence_state,
                        1,
                        NULL,
                        NULL,
                        ?3,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        NULL,
                        1,
                        1
                 FROM source_files
                 WHERE source_file_id = ?1",
                params![
                    source_file_id,
                    format!("basis:facts:{source_file_id}"),
                    media_kind
                ],
            )
            .expect("insert source facts");
    }

    fn insert_library_asset_in_file(
        connection: &Connection,
        library_asset_id: i64,
        source_file_id: i64,
        source_id: i64,
        parent_source_directory_id: i64,
        relative_path: &str,
        title: &str,
    ) {
        let source_segment_set_id = source_file_id + 1_000;
        let source_segment_id = source_file_id + 2_000;
        let file_name = relative_path.rsplit('/').next().unwrap_or(relative_path);
        let name_browse_sort_key =
            crate::browse_sort_key::compute_name_browse_sort_key(file_name);
        let relative_path_browse_sort_key =
            crate::browse_sort_key::compute_relative_path_browse_sort_key(relative_path);
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_browse_sort_key,
                     relative_path_browse_sort_key,
                     relative_path,
                     size_bytes,
                     mtime_ns,
                     presence_state,
                     first_discovered_at,
                     last_observed_at,
                     last_presence_change_at,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, 'present', 1, 1, 1, 1, 1)",
                params![
                    source_file_id,
                    source_id,
                    parent_source_directory_id,
                    file_name,
                    name_browse_sort_key,
                    relative_path_browse_sort_key,
                    relative_path,
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
                params![library_asset_id, format!("eq:{library_asset_id}")],
            )
            .expect("insert library asset");
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
                 VALUES (?1, ?2, 'track', ?3, 1, 1, 1)",
                params![
                    source_segment_set_id,
                    source_file_id,
                    format!("basis:set:{source_file_id}"),
                ],
            )
            .expect("insert source segment set");
        connection
            .execute(
                "INSERT INTO SourceSegments (
                     source_segment_id,
                     source_segment_set_id,
                     segment_kind,
                     ordinal,
                     start_offset_ms,
                     end_offset_ms,
                     display_title,
                     display_artist,
                     display_album,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 'track_span', 0, 0, 1000, ?3, NULL, NULL, 1, 1)",
                params![source_segment_id, source_segment_set_id, title],
            )
            .expect("insert source segment");
        connection
            .execute(
                "INSERT INTO LibraryAssetAttachments (
                     library_asset_id,
                     source_segment_id,
                     accepted_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 1, 1)",
                params![library_asset_id, source_segment_id],
            )
            .expect("insert library asset attachment");
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
            .expect("insert library browser row");
        connection
            .execute(
                "INSERT INTO LibraryBrowserRows_fts (rowid, title, artist, album)
                 VALUES (?1, ?2, '', '')",
                params![library_asset_id, title],
            )
            .expect("insert library browser search row");
    }

    fn set_prep_readiness_summary(
        connection: &Connection,
        library_asset_id: i64,
        prep_readiness_summary: &str,
    ) {
        connection
            .execute(
                "UPDATE LibraryBrowserRows
                 SET prep_readiness_summary = ?2
                 WHERE library_asset_id = ?1",
                params![library_asset_id, prep_readiness_summary],
            )
            .expect("update prep readiness summary");
    }

    fn populate_scope_fixture(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO WorkItems (
                     work_item_id,
                     subject_kind,
                     subject_id,
                     work_kind,
                     priority_class,
                     basis_fingerprint,
                     state,
                     created_at,
                     updated_at
                 )
                 VALUES (1, 'source_file', '1', 'inspect_source', 'interactive', 'basis:work', 'completed', 1, 1)",
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
                 VALUES (1, 1, 'test', '1', 1, 'succeeded')",
                [],
            )
            .expect("insert work run");
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
                 VALUES (
                     1,
                     1,
                     'source_file',
                     '1',
                     'segmentation_result',
                     'primary_result',
                     'test',
                     '1',
                     'basis:artifact',
                     'application/json',
                     'inline_payload',
                     'hash:artifact',
                     1
                 )",
                [],
            )
            .expect("insert artifact");
        insert_source(connection, 1, "Source One");
        insert_source(connection, 2, "Source Two");
        insert_directory(connection, 10, 1, None, "Album", "Album");
        insert_directory(connection, 11, 1, Some(10), "Side A", "Album/Side A");
        insert_directory(connection, 12, 1, None, "Singles", "Singles");
        insert_directory(connection, 20, 2, None, "Elsewhere", "Elsewhere");
        insert_source_location(connection, 30, 1, "Album", "Album");
        insert_library_asset_in_file(
            connection,
            1,
            100,
            1,
            11,
            "Album/Side A/alpha.wav",
            "Alpha Cut",
        );
        insert_library_asset_in_file(connection, 2, 101, 1, 12, "Singles/beta.wav", "Beta Cut");
        insert_library_asset_in_file(
            connection,
            3,
            200,
            2,
            20,
            "Elsewhere/gamma.wav",
            "Gamma Cut",
        );
        insert_source_fact(connection, 100, "audio");
        insert_source_fact(connection, 101, "audio");
        insert_source_fact(connection, 200, "video");
    }

    fn insert_playlist(connection: &Connection, playlist_id: i64, display_name: &str) {
        connection
            .execute(
                "INSERT INTO Playlists (
                     playlist_id,
                     display_name,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 1, 1)",
                params![playlist_id, display_name],
            )
            .expect("insert playlist");
    }

    fn insert_playlist_entry(
        connection: &Connection,
        playlist_entry_id: i64,
        playlist_id: i64,
        library_asset_id: i64,
        position: i64,
    ) {
        connection
            .execute(
                "INSERT INTO PlaylistEntries (
                     playlist_entry_id,
                     playlist_id,
                     library_asset_id,
                     position,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, 1, 1)",
                params![playlist_entry_id, playlist_id, library_asset_id, position,],
            )
            .expect("insert playlist entry");
    }

    fn insert_prep_policy(connection: &Connection, prep_policy_id: i64, policy_name: &str) {
        connection
            .execute(
                "INSERT INTO PrepPolicies (
                     prep_policy_id,
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 0, 1, 1, 1)",
                params![prep_policy_id, policy_name],
            )
            .expect("insert prep policy");
    }

    fn insert_resolved_prep_target(
        connection: &Connection,
        library_asset_id: i64,
        prep_policy_id: i64,
        capability_kind: &str,
        target_profile_key: &str,
    ) {
        connection
            .execute(
                "INSERT INTO ResolvedLibraryAssetPrepTargets (
                     library_asset_id,
                     capability_kind,
                     target_profile_key,
                     target_quality,
                     target_stability_class,
                     priority_class,
                     resolved_from_policy_id,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, 100, 'stable', 'interactive', ?4, 1)",
                params![
                    library_asset_id,
                    capability_kind,
                    target_profile_key,
                    prep_policy_id,
                ],
            )
            .expect("insert resolved prep target");
    }

    #[test]
    fn browser_scope_uses_accepted_source_locations_for_source_aggregate_scope() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);

        let all_media_window =
            read_window_for_scope(&connection, LibraryBrowseScope::AllMedia, 0, 10)
                .expect("read aggregate all-media scope");
        let source_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::Source(SourceId::new(1).expect("positive id")),
            0,
            10,
        )
        .expect("read source scope");
        let location_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::SourceLocation(SourceLocationId::new(30).expect("positive id")),
            0,
            10,
        )
        .expect("read source location scope");

        assert_eq!(all_media_window.total_rows, 3);
        assert_eq!(
            all_media_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(source_window.total_rows, 1);
        assert_eq!(
            source_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(location_window.total_rows, 1);
        assert_eq!(location_window.rows[0].library_asset_id, 1);
        assert_eq!(location_window.rows[0].scoped_source_file_id, Some(100));
    }

    #[test]
    fn browser_source_scope_ignores_observed_locations_until_acceptance() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        connection
            .execute(
                "UPDATE source_locations
                 SET authority = 'device',
                     location_kind = 'observed_path'
                 WHERE source_location_id = 30",
                [],
            )
            .expect("convert fixture source location to observed path");

        let source_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::Source(SourceId::new(1).expect("positive id")),
            0,
            10,
        )
        .expect("read source scope");
        let location_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::SourceLocation(SourceLocationId::new(30).expect("positive id")),
            0,
            10,
        )
        .expect("read observed source-location scope");

        assert_eq!(source_window.total_rows, 2);
        assert_eq!(
            source_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(location_window.total_rows, 0);
        assert!(location_window.rows.is_empty());
    }

    #[test]
    fn browser_source_scope_does_not_fallback_when_accepted_locations_are_missing() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        connection
            .execute(
                "UPDATE source_locations
                 SET relative_path = 'Missing',
                     display_name = 'Missing'
                 WHERE source_location_id = 30",
                [],
            )
            .expect("move fixture source location to missing path");

        let source_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::Source(SourceId::new(1).expect("positive id")),
            0,
            10,
        )
        .expect("read source scope");

        assert_eq!(source_window.total_rows, 0);
        assert!(source_window.rows.is_empty());
    }

    #[test]
    fn browser_scope_reads_active_view_and_playlist_group_paths() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_playlist(&connection, 50, "Selected");
        insert_playlist_entry(&connection, 500, 50, 3, 0);
        insert_playlist_entry(&connection, 501, 50, 1, 1);

        let audio_window = read_window_for_scope(&connection, LibraryBrowseScope::AllAudio, 0, 10)
            .expect("read all-audio scope");
        let video_window = read_window_for_scope(&connection, LibraryBrowseScope::AllVideos, 0, 10)
            .expect("read all-videos scope");
        let recent_window =
            read_window_for_scope(&connection, LibraryBrowseScope::RecentlyAdded, 0, 10)
                .expect("read recently-added scope");
        let playlist_group_window =
            read_window_for_scope(&connection, LibraryBrowseScope::PlaylistGroup, 0, 10)
                .expect("read playlist-group scope");

        assert_eq!(
            audio_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            video_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            recent_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert_eq!(
            playlist_group_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn needs_preparation_scope_reads_non_ready_required_prep_assets() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        set_prep_readiness_summary(&connection, 1, "preparing");
        set_prep_readiness_summary(&connection, 2, "underprepared");
        set_prep_readiness_summary(&connection, 3, "ready");

        let window =
            read_window_for_scope(&connection, LibraryBrowseScope::NeedsPreparation, 0, 10)
                .expect("read needs-preparation scope");

        assert_eq!(window.total_rows, 2);
        assert_eq!(
            window
                .rows
                .iter()
                .map(|row| (row.library_asset_id, row.prep_readiness_summary.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "preparing"), (2, "underprepared")]
        );
    }

    #[test]
    fn needs_preparation_scope_search_stays_inside_non_ready_required_prep_assets() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        set_prep_readiness_summary(&connection, 1, "blocked");
        set_prep_readiness_summary(&connection, 2, "failed");
        set_prep_readiness_summary(&connection, 3, "ready");

        let excluded_ready = search_window_for_scope(
            &connection,
            LibraryBrowseScope::NeedsPreparation,
            "Gamma",
            0,
            10,
        )
        .expect("search ready asset inside needs-preparation scope");
        let included_failed = search_window_for_scope(
            &connection,
            LibraryBrowseScope::NeedsPreparation,
            "Beta",
            0,
            10,
        )
        .expect("search failed asset inside needs-preparation scope");

        assert_eq!(excluded_ready.total_rows, 0);
        assert!(excluded_ready.rows.is_empty());
        assert_eq!(included_failed.total_rows, 1);
        assert_eq!(included_failed.rows[0].library_asset_id, 2);
    }

    #[test]
    fn browser_scope_search_is_limited_to_selected_source_or_location_scope() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);

        let source_window = search_window_for_scope(
            &connection,
            LibraryBrowseScope::Source(SourceId::new(1).expect("positive id")),
            "Gamma",
            0,
            10,
        )
        .expect("search source scope");
        let location_window = search_window_for_scope(
            &connection,
            LibraryBrowseScope::SourceLocation(SourceLocationId::new(30).expect("positive id")),
            "Alpha",
            0,
            10,
        )
        .expect("search source location scope");

        assert_eq!(source_window.total_rows, 0);
        assert!(source_window.rows.is_empty());
        assert_eq!(location_window.total_rows, 1);
        assert_eq!(location_window.rows[0].library_asset_id, 1);
    }

    #[test]
    fn playlist_scope_reads_are_ordered_by_durable_playlist_position() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_playlist(&connection, 50, "Set");
        insert_playlist_entry(&connection, 500, 50, 3, 0);
        insert_playlist_entry(&connection, 501, 50, 1, 1);

        let playlist_window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::Playlist(PlaylistId::new(50).expect("positive id")),
            0,
            10,
        )
        .expect("read playlist scope");

        assert_eq!(playlist_window.total_rows, 2);
        assert_eq!(
            playlist_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
    }

    #[test]
    fn playlist_scope_search_is_limited_to_playlist_membership() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_playlist(&connection, 50, "Set");
        insert_playlist_entry(&connection, 500, 50, 3, 0);
        insert_playlist_entry(&connection, 501, 50, 1, 1);

        let outside_playlist = search_window_for_scope(
            &connection,
            LibraryBrowseScope::Playlist(PlaylistId::new(50).expect("positive id")),
            "Beta",
            0,
            10,
        )
        .expect("search playlist scope for non-member");
        let inside_playlist = search_window_for_scope(
            &connection,
            LibraryBrowseScope::Playlist(PlaylistId::new(50).expect("positive id")),
            "Gamma",
            0,
            10,
        )
        .expect("search playlist scope for member");

        assert_eq!(outside_playlist.total_rows, 0);
        assert!(outside_playlist.rows.is_empty());
        assert_eq!(inside_playlist.total_rows, 1);
        assert_eq!(inside_playlist.rows[0].library_asset_id, 3);
    }

    #[test]
    fn prep_policy_scope_reads_distinct_assets_from_effective_resolved_targets() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_prep_policy(&connection, 70, "Warm");
        insert_prep_policy(&connection, 71, "Cold");
        insert_resolved_prep_target(&connection, 1, 70, "waveform", "warm-waveform");
        insert_resolved_prep_target(&connection, 1, 70, "stems", "warm-stems");
        insert_resolved_prep_target(&connection, 2, 71, "waveform", "cold-waveform");
        insert_resolved_prep_target(&connection, 3, 70, "waveform", "warm-video");

        let window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::PrepPolicyScope(PrepPolicyId::new(70).expect("positive id")),
            0,
            10,
        )
        .expect("read prep-policy scope");

        assert_eq!(window.total_rows, 2);
        assert_eq!(
            window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn prep_policy_scope_search_is_limited_to_effective_resolved_targets() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_prep_policy(&connection, 70, "Warm");
        insert_prep_policy(&connection, 71, "Cold");
        insert_resolved_prep_target(&connection, 1, 70, "waveform", "warm-waveform");
        insert_resolved_prep_target(&connection, 2, 71, "waveform", "cold-waveform");
        insert_resolved_prep_target(&connection, 3, 70, "waveform", "warm-video");

        let outside_policy = search_window_for_scope(
            &connection,
            LibraryBrowseScope::PrepPolicyScope(PrepPolicyId::new(70).expect("positive id")),
            "Beta",
            0,
            10,
        )
        .expect("search prep-policy scope for asset from another policy");
        let inside_policy = search_window_for_scope(
            &connection,
            LibraryBrowseScope::PrepPolicyScope(PrepPolicyId::new(70).expect("positive id")),
            "Gamma",
            0,
            10,
        )
        .expect("search prep-policy scope for policy asset");

        assert_eq!(outside_policy.total_rows, 0);
        assert!(outside_policy.rows.is_empty());
        assert_eq!(inside_policy.total_rows, 1);
        assert_eq!(inside_policy.rows[0].library_asset_id, 3);
    }

    #[test]
    fn empty_prep_policy_scope_returns_empty_window() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);
        insert_prep_policy(&connection, 72, "Empty");

        let window = read_window_for_scope(
            &connection,
            LibraryBrowseScope::PrepPolicyScope(PrepPolicyId::new(72).expect("positive id")),
            0,
            10,
        )
        .expect("read empty prep-policy scope");

        assert_eq!(window.total_rows, 0);
        assert!(window.rows.is_empty());
    }

    #[test]
    fn empty_scoped_search_falls_back_to_scoped_browse_windows() {
        let connection = open_browser_scope_test_connection();
        populate_scope_fixture(&connection);

        let all_media_window =
            search_window_for_scope(&connection, LibraryBrowseScope::AllMedia, "   ", 0, 10)
                .expect("empty all-media search falls back to aggregate browse");
        let source_window = search_window_for_scope(
            &connection,
            LibraryBrowseScope::Source(SourceId::new(1).expect("positive id")),
            "",
            0,
            10,
        )
        .expect("empty source search falls back to scoped browse");
        let location_window = search_window_for_scope(
            &connection,
            LibraryBrowseScope::SourceLocation(SourceLocationId::new(30).expect("positive id")),
            "",
            0,
            10,
        )
        .expect("empty location search falls back to scoped browse");
        insert_playlist(&connection, 50, "Set");
        insert_playlist_entry(&connection, 500, 50, 3, 0);
        insert_playlist_entry(&connection, 501, 50, 1, 1);
        let playlist_window = search_window_for_scope(
            &connection,
            LibraryBrowseScope::Playlist(PlaylistId::new(50).expect("positive id")),
            "",
            0,
            10,
        )
        .expect("empty playlist search falls back to playlist browse");

        assert_eq!(all_media_window.total_rows, 3);
        assert_eq!(source_window.total_rows, 1);
        assert_eq!(location_window.total_rows, 1);
        assert_eq!(location_window.rows[0].library_asset_id, 1);
        assert_eq!(
            playlist_window
                .rows
                .iter()
                .map(|row| row.library_asset_id)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
    }
}
