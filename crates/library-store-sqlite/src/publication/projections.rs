use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    NavigationSelector, ProjectionDomain, SourceId, SourceLocationId, encode_selector,
};

const ALL_MEDIA_VIEW_ROW_ID: i64 = -1;
const ALL_AUDIO_VIEW_ROW_ID: i64 = -2;
const ALL_VIDEOS_VIEW_ROW_ID: i64 = -3;
const RECENTLY_ADDED_VIEW_ROW_ID: i64 = -4;
const SOURCE_LOCATION_NAVIGATION_ROW_ID_BASE: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ProjectionRetentionState {
    pub domain: ProjectionDomain,
    pub live_subscriber_count: i64,
    pub live_min_position: Option<i64>,
    pub earliest_retained_change_sequence: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NavigationProjectionRow {
    navigation_row_id: i64,
    stable_key: String,
    parent_navigation_row_id: Option<i64>,
    family: Option<String>,
    row_kind: String,
    display_name: String,
    sibling_position: i64,
    selectable: bool,
    selector_kind: Option<String>,
    selector_payload: Option<String>,
    updated_at: i64,
}

pub(crate) fn reseed_current_projection_state(
    write: &AdmittedWrite<'_>,
) -> LibrarySqliteResult<()> {
    reseed_projection_domains(write, &[ProjectionDomain::Navigation])
}

pub(crate) fn reseed_projection_domains(
    write: &AdmittedWrite<'_>,
    domains: &[ProjectionDomain],
) -> LibrarySqliteResult<()> {
    let changed_at = unix_time_ms()?;
    if domains.contains(&ProjectionDomain::Navigation) {
        rebuild_navigation_projection(write, changed_at)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn sweep_projection_retention(
    connection: &Connection,
    now_ms: i64,
) -> LibrarySqliteResult<Vec<ProjectionRetentionState>> {
    connection.execute(
        "DELETE FROM projection_subscribers
         WHERE expires_at <= ?1",
        [now_ms],
    )?;

    let domain = ProjectionDomain::Navigation;
    let (live_subscriber_count, live_min_position): (i64, Option<i64>) = connection.query_row(
        "SELECT COUNT(*), MIN(position)
         FROM projection_cursors
         WHERE projection_domain = ?1",
        [domain.as_str()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if let Some(live_min_position) = live_min_position {
        connection.execute(
            "DELETE FROM projection_change_log
             WHERE projection_domain = ?1
               AND change_sequence < ?2",
            params![domain.as_str(), live_min_position],
        )?;
    }

    let earliest_retained_change_sequence = connection.query_row(
        "SELECT MIN(change_sequence)
         FROM projection_change_log
         WHERE projection_domain = ?1",
        [domain.as_str()],
        |row| row.get::<_, Option<i64>>(0),
    )?;

    connection.execute(
        "INSERT INTO projection_retention_watermarks (
             projection_domain,
             live_subscriber_count,
             live_min_position,
             earliest_retained_change_sequence,
             updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(projection_domain) DO UPDATE
         SET live_subscriber_count = excluded.live_subscriber_count,
             live_min_position = excluded.live_min_position,
             earliest_retained_change_sequence = excluded.earliest_retained_change_sequence,
             updated_at = excluded.updated_at",
        params![
            domain.as_str(),
            live_subscriber_count,
            live_min_position,
            earliest_retained_change_sequence,
            now_ms,
        ],
    )?;

    let states = vec![ProjectionRetentionState {
        domain,
        live_subscriber_count,
        live_min_position,
        earliest_retained_change_sequence,
    }];

    Ok(states)
}

fn rebuild_navigation_projection(
    write: &AdmittedWrite<'_>,
    changed_at: i64,
) -> LibrarySqliteResult<()> {
    let current_rows = load_current_navigation_rows(write)?;
    let next_rows = load_next_navigation_rows(write)?;

    for (stable_key, current_row) in &current_rows {
        if !next_rows.contains_key(stable_key) {
            let next_row_version = current_row.row_version + 1;
            insert_projection_change(
                write,
                ProjectionDomain::Navigation,
                stable_key,
                "delete",
                next_row_version,
                &deleted_navigation_payload_json(write, current_row, next_row_version)?,
                changed_at,
            )?;
            write.execute(
                "DELETE FROM navigation_rows
                 WHERE navigation_row_id = ?1",
                [current_row.navigation_row_id],
            )?;
        }
    }

    for (stable_key, next_row) in next_rows {
        let current_row = current_rows.get(&stable_key);
        let unchanged = current_row
            .map(|current_row| navigation_rows_match(current_row, &next_row))
            .unwrap_or(false);
        if unchanged {
            continue;
        }

        let next_row_version = match current_row {
            Some(current_row) => current_row.row_version + 1,
            None => {
                latest_logged_row_version(write, ProjectionDomain::Navigation, &stable_key)?
                    .unwrap_or(0)
                    + 1
            }
        };
        upsert_navigation_row(write, &next_row, next_row_version)?;
        insert_projection_change(
            write,
            ProjectionDomain::Navigation,
            &stable_key,
            "upsert",
            next_row_version,
            &navigation_payload_json(write, &next_row, next_row_version)?,
            changed_at,
        )?;
    }

    Ok(())
}

fn load_current_navigation_rows(
    connection: &Connection,
) -> LibrarySqliteResult<BTreeMap<String, crate::read_models::navigation::NavigationRow>> {
    let mut statement = connection.prepare(
        "SELECT navigation_row_id,
                stable_key,
                parent_navigation_row_id,
                family,
                row_kind,
                display_name,
                sibling_position,
                selectable,
                selector_kind,
                selector_payload,
                updated_at,
                row_version
         FROM navigation_rows
         ORDER BY stable_key ASC",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(crate::read_models::navigation::NavigationRow {
                navigation_row_id: row.get(0)?,
                stable_key: row.get(1)?,
                parent_navigation_row_id: row.get(2)?,
                family: row.get(3)?,
                row_kind: row.get(4)?,
                display_name: row.get(5)?,
                sibling_position: row.get(6)?,
                selectable: row.get::<_, i64>(7)? != 0,
                selector_kind: row.get(8)?,
                selector_payload: row.get(9)?,
                updated_at: row.get(10)?,
                row_version: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|row| (row.stable_key.clone(), row))
        .collect())
}

fn load_next_navigation_rows(
    connection: &Connection,
) -> LibrarySqliteResult<BTreeMap<String, NavigationProjectionRow>> {
    let mut rows = BTreeMap::new();
    let structural_updated_at = load_navigation_structural_updated_at(connection)?;
    insert_structural_navigation_rows(&mut rows, structural_updated_at);

    let mut statement = connection.prepare(
        "SELECT s.source_id,
                s.source_class,
                s.display_name,
                s.updated_at,
                ss.updated_at,
                COALESCE(buo.ordinal, 9223372036854775807) AS user_ordinal,
                ROW_NUMBER() OVER (
                    ORDER BY
                        CASE s.source_class
                            WHEN 'internal' THEN 0
                            WHEN 'external_mounted' THEN 1
                            WHEN 'removable_mounted' THEN 2
                            ELSE 3
                        END,
                        COALESCE(buo.ordinal, 9223372036854775807),
                        lower(s.display_name),
                        s.source_id
                ) - 1 AS sibling_position
         FROM sources s
         LEFT JOIN source_state ss
           ON ss.source_id = s.source_id
         LEFT JOIN source_navigation_user_order buo
           ON buo.item_kind = 'source'
          AND buo.item_key = CAST(s.source_id AS TEXT)
          AND buo.parent_source_key IS NULL
         WHERE s.is_user_visible = 1
         ORDER BY sibling_position ASC, s.source_id ASC",
    )?;

    let mut query = statement.query([])?;
    while let Some(row) = query.next()? {
        let source_id: i64 = row.get(0)?;
        let display_name: String = row.get(2)?;
        let source_updated_at: i64 = row.get(3)?;
        let state_updated_at: Option<i64> = row.get(4)?;
        let sibling_position: i64 = row.get(6)?;

        let updated_at = state_updated_at
            .map(|state_updated_at| state_updated_at.max(source_updated_at))
            .unwrap_or(source_updated_at);

        let selector = encode_source_navigation_selector(source_id)?;
        let stable_key = format!("source:{source_id}");
        rows.insert(
            stable_key.clone(),
            NavigationProjectionRow {
                navigation_row_id: source_id,
                stable_key,
                parent_navigation_row_id: None,
                family: Some("sources".to_string()),
                row_kind: "source".to_string(),
                display_name,
                sibling_position,
                selectable: true,
                selector_kind: Some(selector.kind.to_string()),
                selector_payload: Some(selector.payload),
                updated_at,
            },
        );
    }

    insert_source_location_navigation_rows(connection, &mut rows)?;

    Ok(rows)
}

fn load_navigation_structural_updated_at(connection: &Connection) -> LibrarySqliteResult<i64> {
    let updated_at = connection.query_row(
        "SELECT MAX(updated_at)
         FROM (
             SELECT updated_at FROM sources
             UNION ALL
             SELECT updated_at FROM source_state
             UNION ALL
             SELECT updated_at FROM source_locations
          )",
        [],
        |row| row.get::<_, Option<i64>>(0),
    )?;
    Ok(updated_at.unwrap_or(0))
}

fn insert_structural_navigation_rows(
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
    updated_at: i64,
) {
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: ALL_AUDIO_VIEW_ROW_ID,
            stable_key: "view:all_audio",
            parent_navigation_row_id: None,
            family: Some("views"),
            row_kind: "view",
            display_name: "All Audio",
            sibling_position: 1,
            selector: NavigationSelector::AllAudio,
            updated_at,
        },
    );
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: ALL_MEDIA_VIEW_ROW_ID,
            stable_key: "view:all_media",
            parent_navigation_row_id: None,
            family: Some("views"),
            row_kind: "view",
            display_name: "All Media",
            sibling_position: 0,
            selector: NavigationSelector::AllMedia,
            updated_at,
        },
    );
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: ALL_VIDEOS_VIEW_ROW_ID,
            stable_key: "view:all_videos",
            parent_navigation_row_id: None,
            family: Some("views"),
            row_kind: "view",
            display_name: "All Videos",
            sibling_position: 2,
            selector: NavigationSelector::AllVideos,
            updated_at,
        },
    );
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: RECENTLY_ADDED_VIEW_ROW_ID,
            stable_key: "view:recently_added",
            parent_navigation_row_id: None,
            family: Some("views"),
            row_kind: "view",
            display_name: "Recently Added",
            sibling_position: 3,
            selector: NavigationSelector::RecentlyAdded,
            updated_at,
        },
    );
}

struct SelectableStructuralNavigationRow<'a> {
    navigation_row_id: i64,
    stable_key: &'a str,
    parent_navigation_row_id: Option<i64>,
    family: Option<&'a str>,
    row_kind: &'a str,
    display_name: &'a str,
    sibling_position: i64,
    selector: NavigationSelector,
    updated_at: i64,
}

fn insert_selectable_structural_navigation_row(
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
    input: SelectableStructuralNavigationRow<'_>,
) {
    let selector = encode_selector(&input.selector);
    rows.insert(
        input.stable_key.to_string(),
        NavigationProjectionRow {
            navigation_row_id: input.navigation_row_id,
            stable_key: input.stable_key.to_string(),
            parent_navigation_row_id: input.parent_navigation_row_id,
            family: input.family.map(str::to_string),
            row_kind: input.row_kind.to_string(),
            display_name: input.display_name.to_string(),
            sibling_position: input.sibling_position,
            selectable: true,
            selector_kind: Some(selector.kind.to_string()),
            selector_payload: Some(selector.payload),
            updated_at: input.updated_at,
        },
    );
}

fn insert_source_location_navigation_rows(
    connection: &Connection,
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
) -> LibrarySqliteResult<()> {
    let mut statement = connection.prepare(
        "SELECT sl.source_location_id,
                sl.source_id,
                COALESCE(NULLIF(trim(sl.display_name), ''), sl.relative_path) AS display_name,
                sl.updated_at,
                ROW_NUMBER() OVER (
                    PARTITION BY sl.source_id
                    ORDER BY
                        COALESCE(buo.ordinal, 9223372036854775807),
                        lower(COALESCE(NULLIF(trim(sl.display_name), ''), sl.relative_path)),
                        lower(sl.relative_path),
                        sl.source_location_id
                ) - 1 AS sibling_position
         FROM source_locations sl
         LEFT JOIN source_navigation_user_order buo
           ON buo.item_kind = 'source_location'
          AND buo.item_key = CAST(sl.source_location_id AS TEXT)
          AND buo.parent_source_key = CAST(sl.source_id AS TEXT)
         WHERE sl.authority = 'user'
           AND sl.location_kind = 'registered_subpath'
           AND sl.is_user_visible = 1
         ORDER BY sl.source_id ASC, sibling_position ASC",
    )?;
    let mut query = statement.query([])?;
    while let Some(row) = query.next()? {
        let source_location_id: i64 = row.get(0)?;
        let source_id: i64 = row.get(1)?;
        let display_name: String = row.get(2)?;
        let updated_at: i64 = row.get(3)?;
        let sibling_position: i64 = row.get(4)?;
        let selector = encode_source_location_navigation_selector(source_location_id)?;
        let stable_key = format!("source_location:{source_location_id}");
        rows.insert(
            stable_key.clone(),
            NavigationProjectionRow {
                navigation_row_id: source_location_navigation_row_id(source_location_id),
                stable_key,
                parent_navigation_row_id: Some(source_id),
                family: None,
                row_kind: "location".to_string(),
                display_name,
                sibling_position,
                selectable: true,
                selector_kind: Some(selector.kind.to_string()),
                selector_payload: Some(selector.payload),
                updated_at,
            },
        );
    }
    Ok(())
}

fn encode_source_navigation_selector(
    source_id: i64,
) -> LibrarySqliteResult<library_domain::EncodedNavigationSelector> {
    let source_id = SourceId::new(source_id).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "sources.source_id must be positive to encode navigation selector, found {source_id}"
        ))
    })?;
    Ok(encode_selector(&NavigationSelector::Source(source_id)))
}

fn encode_source_location_navigation_selector(
    source_location_id: i64,
) -> LibrarySqliteResult<library_domain::EncodedNavigationSelector> {
    let source_location_id = SourceLocationId::new(source_location_id).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "source_locations.source_location_id must be positive to encode navigation selector, found {source_location_id}"
        ))
    })?;
    Ok(encode_selector(&NavigationSelector::SourceLocation(
        source_location_id,
    )))
}

fn source_location_navigation_row_id(source_location_id: i64) -> i64 {
    -(SOURCE_LOCATION_NAVIGATION_ROW_ID_BASE + source_location_id)
}

fn navigation_rows_match(
    current_row: &crate::read_models::navigation::NavigationRow,
    next_row: &NavigationProjectionRow,
) -> bool {
    current_row.navigation_row_id == next_row.navigation_row_id
        && current_row.parent_navigation_row_id == next_row.parent_navigation_row_id
        && current_row.family == next_row.family
        && current_row.row_kind == next_row.row_kind
        && current_row.display_name == next_row.display_name
        && current_row.sibling_position == next_row.sibling_position
        && current_row.selectable == next_row.selectable
        && current_row.selector_kind == next_row.selector_kind
        && current_row.selector_payload == next_row.selector_payload
        && current_row.updated_at == next_row.updated_at
}

fn upsert_navigation_row(
    write: &AdmittedWrite<'_>,
    row: &NavigationProjectionRow,
    row_version: i64,
) -> LibrarySqliteResult<()> {
    write.execute(
        "INSERT INTO navigation_rows (
             navigation_row_id,
             stable_key,
             parent_navigation_row_id,
             family,
             row_kind,
             display_name,
             sibling_position,
             selectable,
             selector_kind,
             selector_payload,
             updated_at,
             row_version
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(navigation_row_id) DO UPDATE
         SET stable_key = excluded.stable_key,
             parent_navigation_row_id = excluded.parent_navigation_row_id,
             family = excluded.family,
             row_kind = excluded.row_kind,
             display_name = excluded.display_name,
             sibling_position = excluded.sibling_position,
             selectable = excluded.selectable,
             selector_kind = excluded.selector_kind,
             selector_payload = excluded.selector_payload,
             updated_at = excluded.updated_at,
             row_version = excluded.row_version",
        params![
            row.navigation_row_id,
            row.stable_key.as_str(),
            row.parent_navigation_row_id,
            row.family.as_deref(),
            row.row_kind.as_str(),
            row.display_name.as_str(),
            row.sibling_position,
            if row.selectable { 1 } else { 0 },
            row.selector_kind.as_deref(),
            row.selector_payload.as_deref(),
            row.updated_at,
            row_version,
        ],
    )?;
    Ok(())
}

fn insert_projection_change(
    write: &AdmittedWrite<'_>,
    domain: ProjectionDomain,
    row_key: &str,
    change_kind: &str,
    row_version: i64,
    payload_json: &str,
    changed_at: i64,
) -> LibrarySqliteResult<()> {
    write.execute(
        "INSERT INTO projection_change_log (
             projection_domain,
             row_key,
             change_kind,
             row_version,
             payload_json,
             created_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            domain.as_str(),
            row_key,
            change_kind,
            row_version,
            payload_json,
            changed_at,
        ],
    )?;
    Ok(())
}

fn latest_logged_row_version(
    connection: &Connection,
    domain: ProjectionDomain,
    row_key: &str,
) -> LibrarySqliteResult<Option<i64>> {
    connection
        .query_row(
            "SELECT row_version
             FROM projection_change_log
             WHERE projection_domain = ?1
               AND row_key = ?2
             ORDER BY change_sequence DESC
             LIMIT 1",
            params![domain.as_str(), row_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn navigation_payload_json(
    connection: &Connection,
    row: &NavigationProjectionRow,
    row_version: i64,
) -> LibrarySqliteResult<String> {
    connection
        .query_row(
            "SELECT json_object(
                 'navigation_row_id', ?1,
                 'stable_key', ?2,
                 'parent_navigation_row_id', ?3,
                 'family', ?4,
                 'row_kind', ?5,
                 'display_name', ?6,
                 'sibling_position', ?7,
                 'selectable', ?8,
                 'selector_kind', ?9,
                 'selector_payload', ?10,
                 'updated_at', ?11,
                 'row_version', ?12
             )",
            params![
                row.navigation_row_id,
                row.stable_key.as_str(),
                row.parent_navigation_row_id,
                row.family.as_deref(),
                row.row_kind.as_str(),
                row.display_name.as_str(),
                row.sibling_position,
                if row.selectable { 1 } else { 0 },
                row.selector_kind.as_deref(),
                row.selector_payload.as_deref(),
                row.updated_at,
                row_version,
            ],
            |json_row| json_row.get(0),
        )
        .map_err(Into::into)
}

fn deleted_navigation_payload_json(
    connection: &Connection,
    row: &crate::read_models::navigation::NavigationRow,
    row_version: i64,
) -> LibrarySqliteResult<String> {
    connection
        .query_row(
            "SELECT json_object(
                 'navigation_row_id', ?1,
                 'stable_key', ?2,
                 'deleted', 1,
                 'row_version', ?3
             )",
            params![row.navigation_row_id, row.stable_key, row_version],
            |json_row| json_row.get(0),
        )
        .map_err(Into::into)
}
