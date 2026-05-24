use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::authority::write_lane::AdmittedWrite;
use crate::read_models::waveform_profile_selection::RELEVANT_WAVEFORM_TARGET_ORDER_SQL;
use crate::time::unix_time_ms;
use crate::{LibrarySqliteError, LibrarySqliteResult};
use library_domain::{
    NavigationSelector, PlaylistId, PrepPolicyId, ProjectionDomain, SourceId, SourceLocationId,
    encode_selector,
};

const ALL_MEDIA_VIEW_ROW_ID: i64 = -1;
const ALL_AUDIO_VIEW_ROW_ID: i64 = -2;
const ALL_VIDEOS_VIEW_ROW_ID: i64 = -3;
const RECENTLY_ADDED_VIEW_ROW_ID: i64 = -4;
const NEEDS_PREPARATION_VIEW_ROW_ID: i64 = -5;
const PLAYLISTS_FAMILY_ROW_ID: i64 = -7;
const PREP_POLICIES_FAMILY_ROW_ID: i64 = -8;
const PLAYLIST_NAVIGATION_ROW_ID_BASE: i64 = 2_000_000_000_000;
const PREP_POLICY_NAVIGATION_ROW_ID_BASE: i64 = 3_000_000_000_000;
const SOURCE_LOCATION_GROUP_NAVIGATION_ROW_ID_BASE: i64 = 1_500_000_000_000;
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

#[derive(Debug, Clone, PartialEq)]
struct LibraryBrowserProjectionRow {
    library_asset_id: i64,
    primary_source_file_id: Option<i64>,
    // Finite projection vocabulary: available, unavailable, degraded.
    availability_state: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<i64>,
    musical_key: Option<String>,
    tempo_bpm: Option<f64>,
    // Browse-facing summaries derived by projection rebuild logic from
    // capability/artifact state; they are not canonical authority.
    waveform_quality_current: Option<i64>,
    waveform_quality_target: Option<i64>,
    stems_state_summary: Option<String>,
    prep_readiness_summary: String,
    updated_at: i64,
}

// `stems_state_summary` is a compact browse projection over canonical
// LibraryAssetCapabilities state. It deliberately uses only this finite
// vocabulary and never copies adapter/worker payload text into browser rows:
// missing, queued, leased, ready, stale, blocked, failed.
const STEMS_STATE_SUMMARY_MISSING: &str = "missing";
const STEMS_STATE_SUMMARY_QUEUED: &str = "queued";
const STEMS_STATE_SUMMARY_LEASED: &str = "leased";
const STEMS_STATE_SUMMARY_READY: &str = "ready";
const STEMS_STATE_SUMMARY_STALE: &str = "stale";
const STEMS_STATE_SUMMARY_BLOCKED: &str = "blocked";
const STEMS_STATE_SUMMARY_FAILED: &str = "failed";

const PREP_READINESS_NOT_REQUIRED: &str = "not_required";
#[cfg(test)]
const PREP_READINESS_READY: &str = "ready";
#[cfg(test)]
const PREP_READINESS_PREPARING: &str = "preparing";
#[cfg(test)]
const PREP_READINESS_UNDERPREPARED: &str = "underprepared";
#[cfg(test)]
const PREP_READINESS_BLOCKED: &str = "blocked";
#[cfg(test)]
const PREP_READINESS_FAILED: &str = "failed";

pub(crate) fn reseed_current_projection_state(
    write: &AdmittedWrite<'_>,
) -> LibrarySqliteResult<()> {
    reseed_projection_domains(
        write,
        &[
            ProjectionDomain::Navigation,
            ProjectionDomain::LibraryBrowser,
        ],
    )
}

pub(crate) fn reseed_projection_domains(
    write: &AdmittedWrite<'_>,
    domains: &[ProjectionDomain],
) -> LibrarySqliteResult<()> {
    let changed_at = unix_time_ms()?;
    let reseed_navigation = domains.contains(&ProjectionDomain::Navigation);
    let reseed_library_browser = domains.contains(&ProjectionDomain::LibraryBrowser);

    if reseed_navigation {
        rebuild_navigation_projection(write, changed_at)?;
    }
    if reseed_library_browser {
        rebuild_library_browser_projection(write, changed_at)?;
    }
    Ok(())
}

pub(crate) fn invalidate_projection_domain(
    write: &AdmittedWrite<'_>,
    domain: ProjectionDomain,
    reason: &str,
) -> LibrarySqliteResult<()> {
    let changed_at = unix_time_ms()?;
    let row_key = projection_domain_invalidation_row_key(domain, reason);
    let row_version = latest_logged_row_version(write, domain, &row_key)?.unwrap_or(0) + 1;
    let payload_json =
        projection_domain_invalidation_payload_json(write, domain, reason, row_version)?;
    insert_projection_change(
        write,
        domain,
        &row_key,
        "invalidate",
        row_version,
        &payload_json,
        changed_at,
    )
}

#[allow(dead_code)]
pub(crate) fn sweep_projection_retention(
    connection: &Connection,
    now_ms: i64,
) -> LibrarySqliteResult<Vec<ProjectionRetentionState>> {
    connection.execute(
        "DELETE FROM ProjectionSubscribers
         WHERE expires_at <= ?1",
        [now_ms],
    )?;

    let mut states = Vec::with_capacity(2);
    for domain in [
        ProjectionDomain::LibraryBrowser,
        ProjectionDomain::Navigation,
    ] {
        let (live_subscriber_count, live_min_position): (i64, Option<i64>) = connection.query_row(
            "SELECT COUNT(*), MIN(position)
                 FROM ProjectionCursors
                 WHERE projection_domain = ?1",
            [domain.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        if let Some(live_min_position) = live_min_position {
            connection.execute(
                "DELETE FROM ProjectionChangeLog
                 WHERE projection_domain = ?1
                   AND change_sequence < ?2",
                params![domain.as_str(), live_min_position],
            )?;
        }

        let earliest_retained_change_sequence = connection.query_row(
            "SELECT MIN(change_sequence)
                 FROM ProjectionChangeLog
                 WHERE projection_domain = ?1",
            [domain.as_str()],
            |row| row.get::<_, Option<i64>>(0),
        )?;

        connection.execute(
            "INSERT INTO ProjectionRetentionWatermarks (
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

        states.push(ProjectionRetentionState {
            domain,
            live_subscriber_count,
            live_min_position,
            earliest_retained_change_sequence,
        });
    }

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

fn rebuild_library_browser_projection(
    write: &AdmittedWrite<'_>,
    changed_at: i64,
) -> LibrarySqliteResult<()> {
    let current_rows = load_current_library_browser_rows(write)?;
    let next_rows = load_next_library_browser_rows(write)?;

    for (library_asset_id, current_row) in &current_rows {
        if !next_rows.contains_key(library_asset_id) {
            let row_key = library_asset_id.to_string();
            let next_row_version = current_row.row_version + 1;
            insert_projection_change(
                write,
                ProjectionDomain::LibraryBrowser,
                &row_key,
                "delete",
                next_row_version,
                &deleted_library_browser_payload_json(write, current_row, next_row_version)?,
                changed_at,
            )?;
            write.execute(
                "DELETE FROM LibraryBrowserRows
                 WHERE library_asset_id = ?1",
                [*library_asset_id],
            )?;
        }
    }

    for (library_asset_id, next_row) in next_rows {
        let row_key = library_asset_id.to_string();
        let current_row = current_rows.get(&library_asset_id);
        let unchanged = current_row
            .map(|current_row| library_browser_rows_match(current_row, &next_row))
            .unwrap_or(false);
        if unchanged {
            continue;
        }

        let next_row_version = match current_row {
            Some(current_row) => current_row.row_version + 1,
            None => {
                latest_logged_row_version(write, ProjectionDomain::LibraryBrowser, &row_key)?
                    .unwrap_or(0)
                    + 1
            }
        };
        upsert_library_browser_row(write, &next_row, next_row_version)?;
        insert_projection_change(
            write,
            ProjectionDomain::LibraryBrowser,
            &row_key,
            "upsert",
            next_row_version,
            &library_browser_payload_json(write, &next_row, next_row_version)?,
            changed_at,
        )?;
    }

    rebuild_library_browser_search_index(write)?;
    Ok(())
}

fn rebuild_library_browser_search_index(write: &AdmittedWrite<'_>) -> LibrarySqliteResult<()> {
    write.execute("DELETE FROM LibraryBrowserRows_fts", [])?;
    write.execute(
        "INSERT INTO LibraryBrowserRows_fts (rowid, title, artist, album)
         SELECT library_asset_id,
                COALESCE(title, ''),
                COALESCE(artist, ''),
                COALESCE(album, '')
         FROM LibraryBrowserRows
         ORDER BY library_asset_id ASC",
        [],
    )?;
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
         LEFT JOIN browser_user_order buo
           ON buo.node_domain = 'source'
          AND buo.node_id = CAST(s.source_id AS TEXT)
          AND buo.parent_scope IS NULL
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
                family: Some("Sources".to_string()),
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

    insert_playlist_navigation_rows(connection, &mut rows)?;
    insert_prep_policy_navigation_rows(connection, &mut rows)?;
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
             UNION ALL
             SELECT updated_at FROM Playlists
             UNION ALL
             SELECT updated_at FROM PrepPolicies
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
            family: Some("Views"),
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
            family: Some("Views"),
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
            family: Some("Views"),
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
            family: Some("Views"),
            row_kind: "view",
            display_name: "Recently Added",
            sibling_position: 3,
            selector: NavigationSelector::RecentlyAdded,
            updated_at,
        },
    );
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: NEEDS_PREPARATION_VIEW_ROW_ID,
            stable_key: "view:needs_preparation",
            parent_navigation_row_id: None,
            family: Some("Views"),
            row_kind: "view",
            display_name: "Needs Preparation",
            sibling_position: 4,
            selector: NavigationSelector::NeedsPreparation,
            updated_at,
        },
    );
    insert_selectable_structural_navigation_row(
        rows,
        SelectableStructuralNavigationRow {
            navigation_row_id: PLAYLISTS_FAMILY_ROW_ID,
            stable_key: "collection-group:playlists",
            parent_navigation_row_id: None,
            family: Some("Collections"),
            row_kind: "collection-group",
            display_name: "Playlists",
            sibling_position: 0,
            selector: NavigationSelector::PlaylistGroup,
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

fn insert_playlist_navigation_rows(
    connection: &Connection,
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
) -> LibrarySqliteResult<()> {
    let mut statement = connection.prepare(
        "WITH ordered_playlists AS (
             SELECT playlist_id,
                    display_name,
                    updated_at,
                    ROW_NUMBER() OVER (
                        ORDER BY lower(display_name), playlist_id ASC
                    ) - 1 AS sibling_position
             FROM Playlists
         )
         SELECT playlist_id,
                display_name,
                updated_at,
                sibling_position
         FROM ordered_playlists
         ORDER BY sibling_position ASC, playlist_id ASC",
    )?;
    let mut query = statement.query([])?;
    while let Some(row) = query.next()? {
        let playlist_id: i64 = row.get(0)?;
        let display_name: String = row.get(1)?;
        let updated_at: i64 = row.get(2)?;
        let sibling_position: i64 = row.get(3)?;
        let selector = encode_playlist_navigation_selector(playlist_id)?;
        let stable_key = format!("playlist:{playlist_id}");

        rows.insert(
            stable_key.clone(),
            NavigationProjectionRow {
                navigation_row_id: playlist_navigation_row_id(playlist_id),
                stable_key,
                parent_navigation_row_id: Some(PLAYLISTS_FAMILY_ROW_ID),
                family: None,
                row_kind: "playlist".to_string(),
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

fn insert_prep_policy_navigation_rows(
    connection: &Connection,
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
) -> LibrarySqliteResult<()> {
    let policy_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM PrepPolicies", [], |row| row.get(0))?;
    if policy_count == 0 {
        return Ok(());
    }

    let group_updated_at = connection.query_row(
        "SELECT COALESCE(MAX(updated_at), 0)
         FROM PrepPolicies",
        [],
        |row| row.get(0),
    )?;
    rows.insert(
        "prep-policy-group:policies".to_string(),
        NavigationProjectionRow {
            navigation_row_id: PREP_POLICIES_FAMILY_ROW_ID,
            stable_key: "prep-policy-group:policies".to_string(),
            parent_navigation_row_id: None,
            family: Some("Preparation".to_string()),
            row_kind: "prep-policy-group".to_string(),
            display_name: "Policies".to_string(),
            sibling_position: 0,
            selectable: false,
            selector_kind: None,
            selector_payload: None,
            updated_at: group_updated_at,
        },
    );

    let mut statement = connection.prepare(
        "WITH ordered_policies AS (
             SELECT prep_policy_id,
                    policy_name,
                    updated_at,
                    ROW_NUMBER() OVER (
                        ORDER BY
                            CASE is_system_policy WHEN 1 THEN 0 ELSE 1 END,
                            lower(policy_name),
                            prep_policy_id
                    ) - 1 AS sibling_position
             FROM PrepPolicies
         )
         SELECT prep_policy_id,
                policy_name,
                updated_at,
                sibling_position
         FROM ordered_policies
         ORDER BY sibling_position ASC, prep_policy_id ASC",
    )?;
    let mut query = statement.query([])?;
    while let Some(row) = query.next()? {
        let prep_policy_id: i64 = row.get(0)?;
        let policy_name: String = row.get(1)?;
        let updated_at: i64 = row.get(2)?;
        let sibling_position: i64 = row.get(3)?;
        let selector = encode_prep_policy_navigation_selector(prep_policy_id)?;
        let stable_key = format!("prep_policy_scope:{prep_policy_id}");

        rows.insert(
            stable_key.clone(),
            NavigationProjectionRow {
                navigation_row_id: prep_policy_navigation_row_id(prep_policy_id),
                stable_key,
                parent_navigation_row_id: Some(PREP_POLICIES_FAMILY_ROW_ID),
                family: None,
                row_kind: "prep-policy-scope".to_string(),
                display_name: policy_name,
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

fn insert_source_location_navigation_rows(
    connection: &Connection,
    rows: &mut BTreeMap<String, NavigationProjectionRow>,
) -> LibrarySqliteResult<()> {
    let mut statement = connection.prepare(
        "WITH location_counts AS (
             SELECT source_id,
                    COUNT(*) AS location_count,
                    MAX(updated_at) AS updated_at
             FROM source_locations
             WHERE authority = 'user'
               AND location_kind = 'registered_subpath'
               AND is_user_visible = 1
             GROUP BY source_id
         ),
         ordered_locations AS (
             SELECT sl.source_location_id,
                    sl.source_id,
                    COALESCE(NULLIF(trim(sl.display_name), ''), sl.relative_path) AS display_name,
                    relative_path,
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
             LEFT JOIN browser_user_order buo
               ON buo.node_domain = 'source_location'
             AND buo.node_id = CAST(sl.source_location_id AS TEXT)
             AND buo.parent_scope = CAST(sl.source_id AS TEXT)
             WHERE sl.authority = 'user'
               AND sl.location_kind = 'registered_subpath'
               AND sl.is_user_visible = 1
         )
         SELECT 'group' AS row_type,
                NULL,
                source_id,
                'Locations',
                '',
                updated_at,
                0
         FROM location_counts
         WHERE location_count > 0
         UNION ALL
         SELECT 'location' AS row_type,
                source_location_id,
                source_id,
                display_name,
                relative_path,
                updated_at,
                sibling_position
         FROM ordered_locations
         ORDER BY source_id ASC, row_type ASC, sibling_position ASC",
    )?;
    let mut query = statement.query([])?;
    while let Some(row) = query.next()? {
        let row_type: String = row.get(0)?;
        let source_location_id: Option<i64> = row.get(1)?;
        let source_id: i64 = row.get(2)?;
        let display_name: String = row.get(3)?;
        let relative_path: String = row.get(4)?;
        let updated_at: i64 = row.get(5)?;
        let sibling_position: i64 = row.get(6)?;
        if row_type == "group" {
            let stable_key = format!("source:{source_id}:locations");
            rows.insert(
                stable_key.clone(),
                NavigationProjectionRow {
                    navigation_row_id: source_location_group_navigation_row_id(source_id),
                    stable_key,
                    parent_navigation_row_id: Some(source_id),
                    family: None,
                    row_kind: "location-group".to_string(),
                    display_name,
                    sibling_position: 0,
                    selectable: false,
                    selector_kind: None,
                    selector_payload: None,
                    updated_at,
                },
            );
        } else if let Some(source_location_id) = source_location_id {
            let selector = encode_source_location_navigation_selector(source_location_id)?;
            let stable_key = format!("source_location:{source_location_id}");
            rows.insert(
                stable_key.clone(),
                NavigationProjectionRow {
                    navigation_row_id: source_location_navigation_row_id(source_location_id),
                    stable_key,
                    parent_navigation_row_id: Some(source_location_group_navigation_row_id(
                        source_id,
                    )),
                    family: None,
                    row_kind: "location".to_string(),
                    display_name: if display_name.trim().is_empty() {
                        relative_path
                    } else {
                        display_name
                    },
                    sibling_position,
                    selectable: true,
                    selector_kind: Some(selector.kind.to_string()),
                    selector_payload: Some(selector.payload),
                    updated_at,
                },
            );
        }
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

fn encode_playlist_navigation_selector(
    playlist_id: i64,
) -> LibrarySqliteResult<library_domain::EncodedNavigationSelector> {
    let playlist_id = PlaylistId::new(playlist_id).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "Playlists.playlist_id must be positive to encode navigation selector, found {playlist_id}"
        ))
    })?;
    Ok(encode_selector(&NavigationSelector::Playlist(playlist_id)))
}

fn encode_prep_policy_navigation_selector(
    prep_policy_id: i64,
) -> LibrarySqliteResult<library_domain::EncodedNavigationSelector> {
    let prep_policy_id = PrepPolicyId::new(prep_policy_id).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "PrepPolicies.prep_policy_id must be positive to encode navigation selector, found {prep_policy_id}"
        ))
    })?;
    Ok(encode_selector(&NavigationSelector::PrepPolicyScope(
        prep_policy_id,
    )))
}

fn playlist_navigation_row_id(playlist_id: i64) -> i64 {
    -(PLAYLIST_NAVIGATION_ROW_ID_BASE + playlist_id)
}

fn prep_policy_navigation_row_id(prep_policy_id: i64) -> i64 {
    -(PREP_POLICY_NAVIGATION_ROW_ID_BASE + prep_policy_id)
}

fn source_location_group_navigation_row_id(source_id: i64) -> i64 {
    -(SOURCE_LOCATION_GROUP_NAVIGATION_ROW_ID_BASE + source_id)
}

fn source_location_navigation_row_id(source_location_id: i64) -> i64 {
    -(SOURCE_LOCATION_NAVIGATION_ROW_ID_BASE + source_location_id)
}

fn load_current_library_browser_rows(
    connection: &Connection,
) -> LibrarySqliteResult<BTreeMap<i64, CurrentLibraryAssetBrowserRow>> {
    let mut statement = connection.prepare(
        "SELECT library_asset_id,
                row_version,
                primary_source_file_id,
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
         FROM LibraryBrowserRows
         ORDER BY library_asset_id ASC",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(CurrentLibraryAssetBrowserRow {
                row: LibraryBrowserProjectionRow {
                    library_asset_id: row.get(0)?,
                    primary_source_file_id: row.get(2)?,
                    availability_state: row.get(3)?,
                    title: row.get(4)?,
                    artist: row.get(5)?,
                    album: row.get(6)?,
                    duration_ms: row.get(7)?,
                    musical_key: row.get(8)?,
                    tempo_bpm: row.get(9)?,
                    waveform_quality_current: row.get(10)?,
                    waveform_quality_target: row.get(11)?,
                    stems_state_summary: row.get(12)?,
                    prep_readiness_summary: row.get(13)?,
                    updated_at: row.get(14)?,
                },
                row_version: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|row| (row.row.library_asset_id, row))
        .collect())
}

fn load_next_library_browser_rows(
    connection: &Connection,
) -> LibrarySqliteResult<BTreeMap<i64, LibraryBrowserProjectionRow>> {
    let query = format!(
        "WITH active_corrections AS (
             SELECT library_asset_id,
                    MAX(CASE WHEN field_name = 'title' AND is_null_correction = 0 THEN value_text END) AS title_text,
                    MAX(CASE WHEN field_name = 'title' AND is_null_correction = 1 THEN 1 ELSE 0 END) AS title_null,
                    MAX(CASE WHEN field_name = 'artist' AND is_null_correction = 0 THEN value_text END) AS artist_text,
                    MAX(CASE WHEN field_name = 'artist' AND is_null_correction = 1 THEN 1 ELSE 0 END) AS artist_null,
                    MAX(CASE WHEN field_name = 'album' AND is_null_correction = 0 THEN value_text END) AS album_text,
                    MAX(CASE WHEN field_name = 'album' AND is_null_correction = 1 THEN 1 ELSE 0 END) AS album_null,
                    MAX(CASE WHEN field_name = 'duration_ms' AND is_null_correction = 0 THEN value_int END) AS duration_ms_int,
                    MAX(CASE WHEN field_name = 'duration_ms' AND is_null_correction = 1 THEN 1 ELSE 0 END) AS duration_null,
                    MAX(applied_at) AS correction_updated_at
             FROM LibraryAssetMetadataCorrections
             WHERE retracted_at IS NULL
             GROUP BY library_asset_id
         ),
         attachment_candidates AS (
             SELECT pia.library_asset_id,
                    pia.updated_at AS attachment_updated_at,
                    ss.ordinal,
                    ss.display_title,
                    ss.display_artist,
                    ss.display_album,
                    CASE
                        WHEN ss.end_offset_ms IS NULL THEN NULL
                        ELSE ss.end_offset_ms - ss.start_offset_ms
                    END AS attachment_duration_ms,
                    sf.source_file_id,
                    sf.source_id,
                    sf.relative_path,
                    sf.name,
                    sf.presence_state,
                    lss.mount_status AS source_mount_status,
                    lss.resolution_status AS source_resolution_status,
                    sfacts.duration_ms AS source_duration_ms,
                    sfacts.updated_at AS source_facts_updated_at,
                    CAST(json_extract(CAST(source_payload.payload AS TEXT), '$.title') AS TEXT) AS source_title,
                    CAST(json_extract(CAST(source_payload.payload AS TEXT), '$.artist') AS TEXT) AS source_artist,
                    CAST(json_extract(CAST(source_payload.payload AS TEXT), '$.album') AS TEXT) AS source_album,
                    ROW_NUMBER() OVER (
                        PARTITION BY pia.library_asset_id
                        ORDER BY pia.accepted_at ASC, ss.ordinal ASC, sf.source_file_id ASC
                    ) AS attachment_rank,
                    CASE
                        WHEN sf.presence_state = 'present'
                             AND COALESCE(lss.mount_status, 'mounted') = 'mounted'
                             AND COALESCE(lss.resolution_status, 'resolved') = 'resolved'
                            THEN 0
                        WHEN sf.presence_state = 'present'
                            THEN 1
                        ELSE 2
                    END AS availability_rank
             FROM LibraryAssetAttachments pia
             JOIN SourceSegments ss
               ON ss.source_segment_id = pia.source_segment_id
             JOIN SourceSegmentSets sss
               ON sss.source_segment_set_id = ss.source_segment_set_id
             JOIN source_files sf
               ON sf.source_file_id = sss.source_file_id
             LEFT JOIN source_state lss
               ON lss.source_id = sf.source_id
             LEFT JOIN SourceFacts sfacts
               ON sfacts.source_file_id = sf.source_file_id
             LEFT JOIN ArtifactInlinePayloads source_payload
               ON source_payload.artifact_id = sfacts.accepted_artifact_id
         ),
         availability AS (
             SELECT library_asset_id,
                    MIN(availability_rank) AS availability_rank
             FROM attachment_candidates
             GROUP BY library_asset_id
         ),
         primary_attachment AS (
             SELECT library_asset_id,
                    source_file_id,
                    display_title,
                    display_artist,
                    display_album,
                    attachment_duration_ms,
                    source_duration_ms,
                    source_title,
                    source_artist,
                    source_album,
                    CASE
                        WHEN source_facts_updated_at IS NOT NULL
                             AND source_facts_updated_at > attachment_updated_at
                            THEN source_facts_updated_at
                        ELSE attachment_updated_at
                    END AS attachment_updated_at
             FROM attachment_candidates
             WHERE attachment_rank = 1
         ),
         metadata_capability_summary AS (
             SELECT pic.library_asset_id,
                    MAX(CASE
                            WHEN pic.capability_kind = 'musical_key' AND pic.state IN ('ready', 'stale')
                                THEN CAST(
                                    COALESCE(
                                        json_extract(CAST(capability_payload.payload AS TEXT), '$.musical_key'),
                                        json_extract(CAST(capability_payload.payload AS TEXT), '$.key')
                                    ) AS TEXT
                                )
                        END) AS musical_key,
                    MAX(CASE
                            WHEN pic.capability_kind = 'tempo' AND pic.state IN ('ready', 'stale')
                                THEN CAST(
                                    COALESCE(
                                        json_extract(CAST(capability_payload.payload AS TEXT), '$.tempo_bpm'),
                                        json_extract(CAST(capability_payload.payload AS TEXT), '$.tempo')
                                    ) AS REAL
                                )
                        END) AS tempo_bpm,
                    MAX(pic.updated_at) AS metadata_capability_updated_at
             FROM LibraryAssetCapabilities pic
             LEFT JOIN ArtifactInlinePayloads capability_payload
               ON capability_payload.artifact_id = pic.selected_artifact_id
             WHERE pic.state IN ('ready', 'stale')
               AND pic.capability_kind IN ('musical_key', 'tempo')
             GROUP BY pic.library_asset_id
         ),
         capability_defaults AS (
             SELECT MAX(CASE WHEN capability_kind = 'waveform' THEN default_profile_key END) AS waveform_default_profile_key,
                    MAX(CASE WHEN capability_kind = 'stems' THEN default_profile_key END) AS stems_default_profile_key
             FROM CapabilitySpecs
         ),
         waveform_target_candidates AS (
             SELECT rpt.library_asset_id,
                    rpt.target_profile_key,
                    rpt.target_quality,
                    rpt.updated_at,
                    ROW_NUMBER() OVER (
                        PARTITION BY rpt.library_asset_id
                        ORDER BY {RELEVANT_WAVEFORM_TARGET_ORDER_SQL}
                    ) AS target_rank
             FROM ResolvedLibraryAssetPrepTargets rpt
             CROSS JOIN capability_defaults
             WHERE rpt.capability_kind = 'waveform'
         ),
         waveform_targets AS (
             SELECT library_asset_id,
                    target_profile_key,
                    target_quality AS waveform_quality_target,
                    updated_at AS target_updated_at
             FROM waveform_target_candidates
             WHERE target_rank = 1
         ),
         waveform_profiles AS (
             SELECT pi.library_asset_id,
                    COALESCE(
                        waveform_targets.target_profile_key,
                        capability_defaults.waveform_default_profile_key
                    ) AS profile_key
             FROM LibraryAssets pi
             CROSS JOIN capability_defaults
             LEFT JOIN waveform_targets
               ON waveform_targets.library_asset_id = pi.library_asset_id
         ),
         waveform_current AS (
             SELECT waveform_profiles.library_asset_id,
                    pic.quality_current AS waveform_quality_current,
                    pic.updated_at AS waveform_current_updated_at
             FROM waveform_profiles
             JOIN LibraryAssetCapabilities pic
               ON pic.library_asset_id = waveform_profiles.library_asset_id
              AND pic.capability_kind = 'waveform'
              AND pic.profile_key = waveform_profiles.profile_key
              AND pic.state IN ('ready', 'stale')
         ),
         stems_target_candidates AS (
             SELECT rpt.library_asset_id,
                    rpt.target_profile_key,
                    rpt.target_quality,
                    ROW_NUMBER() OVER (
                        PARTITION BY rpt.library_asset_id
                        ORDER BY rpt.target_quality DESC,
                                 CASE rpt.target_stability_class WHEN 'stable' THEN 0 ELSE 1 END,
                                 CASE rpt.priority_class
                                     WHEN 'urgent' THEN 0
                                     WHEN 'interactive' THEN 1
                                     ELSE 2
                                 END,
                                 CASE
                                     WHEN rpt.target_profile_key = capability_defaults.stems_default_profile_key THEN 0
                                     ELSE 1
                                 END,
                                 rpt.target_profile_key ASC
                    ) AS target_rank
             FROM ResolvedLibraryAssetPrepTargets rpt
             CROSS JOIN capability_defaults
             WHERE rpt.capability_kind = 'stems'
         ),
         stems_targets AS (
             SELECT library_asset_id,
                    target_profile_key
             FROM stems_target_candidates
             WHERE target_rank = 1
         ),
         stems_profiles AS (
             SELECT pi.library_asset_id,
                    COALESCE(
                        stems_targets.target_profile_key,
                        capability_defaults.stems_default_profile_key
                    ) AS profile_key
             FROM LibraryAssets pi
             CROSS JOIN capability_defaults
             LEFT JOIN stems_targets
               ON stems_targets.library_asset_id = pi.library_asset_id
         ),
         stems_summary AS (
             SELECT stems_profiles.library_asset_id,
                    pic.state AS stems_capability_state,
                    pic.updated_at AS stems_updated_at
             FROM stems_profiles
             LEFT JOIN LibraryAssetCapabilities pic
               ON pic.library_asset_id = stems_profiles.library_asset_id
              AND pic.capability_kind = 'stems'
              AND pic.profile_key = stems_profiles.profile_key
         ),
         prep_target_evaluation AS (
             SELECT rpt.library_asset_id,
                    CASE
                        WHEN pic.library_asset_id IS NOT NULL
                             AND pic.state = 'ready'
                             AND pic.quality_current IS NOT NULL
                             AND pic.quality_current >= rpt.target_quality
                             AND (
                                 (
                                     rpt.target_stability_class = 'provisional'
                                     AND pic.stability_class IN ('provisional', 'stable')
                                 )
                                 OR (
                                     rpt.target_stability_class = 'stable'
                                     AND pic.stability_class = 'stable'
                                 )
                             )
                            THEN 1
                        ELSE 0
                    END AS is_satisfied,
                    CASE WHEN pic.state = 'failed' THEN 1 ELSE 0 END AS is_failed,
                    CASE WHEN pic.state = 'blocked' THEN 1 ELSE 0 END AS is_blocked,
                    CASE WHEN pic.state IN ('queued', 'leased') THEN 1 ELSE 0 END AS is_preparing,
                    rpt.updated_at AS target_updated_at,
                    pic.updated_at AS capability_updated_at
             FROM ResolvedLibraryAssetPrepTargets rpt
             LEFT JOIN LibraryAssetCapabilities pic
               ON pic.library_asset_id = rpt.library_asset_id
              AND pic.capability_kind = rpt.capability_kind
              AND pic.profile_key = rpt.target_profile_key
         ),
         prep_readiness AS (
             SELECT library_asset_id,
                    CASE
                        WHEN MAX(is_failed) = 1 THEN 'failed'
                        WHEN MAX(is_blocked) = 1 THEN 'blocked'
                        WHEN MIN(is_satisfied) = 1 THEN 'ready'
                        WHEN SUM(CASE WHEN is_satisfied = 0 AND is_preparing = 0 THEN 1 ELSE 0 END) = 0
                            THEN 'preparing'
                        ELSE 'underprepared'
                    END AS prep_readiness_summary,
                    CASE
                        WHEN COALESCE(MAX(capability_updated_at), 0) > MAX(target_updated_at)
                            THEN COALESCE(MAX(capability_updated_at), 0)
                        ELSE MAX(target_updated_at)
                    END AS prep_readiness_updated_at
             FROM prep_target_evaluation
             GROUP BY library_asset_id
         )
         SELECT pi.library_asset_id,
                pi.updated_at,
                primary_attachment.source_file_id,
                COALESCE(availability.availability_rank, 2) AS availability_rank,
                active_corrections.title_text,
                COALESCE(active_corrections.title_null, 0) AS title_null,
                primary_attachment.display_title,
                primary_attachment.source_title,
                active_corrections.artist_text,
                COALESCE(active_corrections.artist_null, 0) AS artist_null,
                primary_attachment.display_artist,
                primary_attachment.source_artist,
                active_corrections.album_text,
                COALESCE(active_corrections.album_null, 0) AS album_null,
                primary_attachment.display_album,
                primary_attachment.source_album,
                active_corrections.duration_ms_int,
                COALESCE(active_corrections.duration_null, 0) AS duration_null,
                primary_attachment.attachment_duration_ms,
                primary_attachment.source_duration_ms,
                metadata_capability_summary.musical_key,
                metadata_capability_summary.tempo_bpm,
                waveform_current.waveform_quality_current,
                waveform_targets.waveform_quality_target,
                stems_summary.stems_capability_state,
                prep_readiness.prep_readiness_summary,
                primary_attachment.attachment_updated_at,
                active_corrections.correction_updated_at,
                metadata_capability_summary.metadata_capability_updated_at,
                waveform_current.waveform_current_updated_at,
                waveform_targets.target_updated_at,
                stems_summary.stems_updated_at,
                prep_readiness.prep_readiness_updated_at
         FROM LibraryAssets pi
         LEFT JOIN primary_attachment
           ON primary_attachment.library_asset_id = pi.library_asset_id
         LEFT JOIN availability
           ON availability.library_asset_id = pi.library_asset_id
         LEFT JOIN active_corrections
           ON active_corrections.library_asset_id = pi.library_asset_id
         LEFT JOIN metadata_capability_summary
           ON metadata_capability_summary.library_asset_id = pi.library_asset_id
         LEFT JOIN waveform_current
           ON waveform_current.library_asset_id = pi.library_asset_id
         LEFT JOIN waveform_targets
           ON waveform_targets.library_asset_id = pi.library_asset_id
         LEFT JOIN stems_summary
           ON stems_summary.library_asset_id = pi.library_asset_id
         LEFT JOIN prep_readiness
           ON prep_readiness.library_asset_id = pi.library_asset_id
         ORDER BY pi.library_asset_id ASC",
    );
    let mut statement = connection.prepare(&query)?;

    let rows = statement
        .query_map([], |row| {
            let library_asset_updated_at: i64 = row.get(1)?;
            let attachment_updated_at: Option<i64> = row.get(26)?;
            let correction_updated_at: Option<i64> = row.get(27)?;
            let metadata_capability_updated_at: Option<i64> = row.get(28)?;
            let waveform_current_updated_at: Option<i64> = row.get(29)?;
            let waveform_target_updated_at: Option<i64> = row.get(30)?;
            let stems_updated_at: Option<i64> = row.get(31)?;
            let prep_readiness_updated_at: Option<i64> = row.get(32)?;

            let title = project_text_field(
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)? != 0,
                row.get(6)?,
                row.get(7)?,
            );
            let artist = project_text_field(
                row.get::<_, Option<String>>(8)?,
                row.get::<_, i64>(9)? != 0,
                row.get(10)?,
                row.get(11)?,
            );
            let album = project_text_field(
                row.get::<_, Option<String>>(12)?,
                row.get::<_, i64>(13)? != 0,
                row.get(14)?,
                row.get(15)?,
            );
            let duration_ms = project_int_field(
                row.get::<_, Option<i64>>(16)?,
                row.get::<_, i64>(17)? != 0,
                row.get(18)?,
                row.get(19)?,
            );

            let updated_at = max_timestamp(
                library_asset_updated_at,
                [
                    attachment_updated_at,
                    correction_updated_at,
                    metadata_capability_updated_at,
                    waveform_current_updated_at,
                    waveform_target_updated_at,
                    stems_updated_at,
                    prep_readiness_updated_at,
                ],
            );
            let availability_state = match row.get::<_, i64>(3)? {
                0 => "available",
                1 => "degraded",
                _ => "unavailable",
            }
            .to_string();

            let library_asset_id: i64 = row.get(0)?;
            Ok((
                library_asset_id,
                LibraryBrowserProjectionRow {
                    library_asset_id,
                    primary_source_file_id: row.get(2)?,
                    availability_state,
                    title,
                    artist,
                    album,
                    duration_ms,
                    musical_key: row.get(20)?,
                    tempo_bpm: row.get(21)?,
                    waveform_quality_current: row.get(22)?,
                    waveform_quality_target: row.get(23)?,
                    stems_state_summary: Some(project_stems_state_summary(row.get(24)?)),
                    prep_readiness_summary: row
                        .get::<_, Option<String>>(25)?
                        .unwrap_or_else(|| PREP_READINESS_NOT_REQUIRED.to_string()),
                    updated_at,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows.into_iter().collect())
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

fn library_browser_rows_match(
    current_row: &CurrentLibraryAssetBrowserRow,
    next_row: &LibraryBrowserProjectionRow,
) -> bool {
    current_row.row.library_asset_id == next_row.library_asset_id
        && current_row.row.primary_source_file_id == next_row.primary_source_file_id
        && current_row.row.availability_state == next_row.availability_state
        && current_row.row.title == next_row.title
        && current_row.row.artist == next_row.artist
        && current_row.row.album == next_row.album
        && current_row.row.duration_ms == next_row.duration_ms
        && current_row.row.musical_key == next_row.musical_key
        && option_f64_eq(current_row.row.tempo_bpm, next_row.tempo_bpm)
        && current_row.row.waveform_quality_current == next_row.waveform_quality_current
        && current_row.row.waveform_quality_target == next_row.waveform_quality_target
        && current_row.row.stems_state_summary == next_row.stems_state_summary
        && current_row.row.prep_readiness_summary == next_row.prep_readiness_summary
        && current_row.row.updated_at == next_row.updated_at
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

fn upsert_library_browser_row(
    write: &AdmittedWrite<'_>,
    row: &LibraryBrowserProjectionRow,
    row_version: i64,
) -> LibrarySqliteResult<()> {
    write.execute(
        "INSERT INTO LibraryBrowserRows (
             library_asset_id,
             row_version,
             primary_source_file_id,
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
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT(library_asset_id) DO UPDATE
         SET row_version = excluded.row_version,
             primary_source_file_id = excluded.primary_source_file_id,
             availability_state = excluded.availability_state,
             title = excluded.title,
             artist = excluded.artist,
             album = excluded.album,
             duration_ms = excluded.duration_ms,
             musical_key = excluded.musical_key,
             tempo_bpm = excluded.tempo_bpm,
             waveform_quality_current = excluded.waveform_quality_current,
             waveform_quality_target = excluded.waveform_quality_target,
             stems_state_summary = excluded.stems_state_summary,
             prep_readiness_summary = excluded.prep_readiness_summary,
             updated_at = excluded.updated_at",
        params![
            row.library_asset_id,
            row_version,
            row.primary_source_file_id,
            row.availability_state.as_str(),
            row.title.as_deref(),
            row.artist.as_deref(),
            row.album.as_deref(),
            row.duration_ms,
            row.musical_key.as_deref(),
            row.tempo_bpm,
            row.waveform_quality_current,
            row.waveform_quality_target,
            row.stems_state_summary.as_deref(),
            row.prep_readiness_summary.as_str(),
            row.updated_at,
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
        "INSERT INTO ProjectionChangeLog (
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
             FROM ProjectionChangeLog
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

fn library_browser_payload_json(
    connection: &Connection,
    row: &LibraryBrowserProjectionRow,
    row_version: i64,
) -> LibrarySqliteResult<String> {
    connection
        .query_row(
            "SELECT json_object(
                 'library_asset_id', ?1,
                 'row_version', ?2,
                 'primary_source_file_id', ?3,
                 'availability_state', ?4,
                 'title', ?5,
                 'artist', ?6,
                 'album', ?7,
                 'duration_ms', ?8,
                 'musical_key', ?9,
                 'tempo_bpm', ?10,
                 'waveform_quality_current', ?11,
                 'waveform_quality_target', ?12,
                 'stems_state_summary', ?13,
                 'prep_readiness_summary', ?14,
                 'updated_at', ?15
             )",
            params![
                row.library_asset_id,
                row_version,
                row.primary_source_file_id,
                row.availability_state.as_str(),
                row.title.as_deref(),
                row.artist.as_deref(),
                row.album.as_deref(),
                row.duration_ms,
                row.musical_key.as_deref(),
                row.tempo_bpm,
                row.waveform_quality_current,
                row.waveform_quality_target,
                row.stems_state_summary.as_deref(),
                row.prep_readiness_summary.as_str(),
                row.updated_at,
            ],
            |json_row| json_row.get(0),
        )
        .map_err(Into::into)
}

fn deleted_library_browser_payload_json(
    connection: &Connection,
    row: &CurrentLibraryAssetBrowserRow,
    row_version: i64,
) -> LibrarySqliteResult<String> {
    connection
        .query_row(
            "SELECT json_object(
                 'library_asset_id', ?1,
                 'deleted', 1,
                 'row_version', ?2
             )",
            params![row.row.library_asset_id, row_version],
            |json_row| json_row.get(0),
        )
        .map_err(Into::into)
}

fn projection_domain_invalidation_row_key(domain: ProjectionDomain, reason: &str) -> String {
    format!("__domain_invalidation__:{}:{reason}", domain.as_str())
}

fn projection_domain_invalidation_payload_json(
    connection: &Connection,
    domain: ProjectionDomain,
    reason: &str,
    row_version: i64,
) -> LibrarySqliteResult<String> {
    connection
        .query_row(
            "SELECT json_object(
                 'projection_domain', ?1,
                 'reason', ?2,
                 'invalidated', 1,
                 'row_version', ?3
             )",
            params![domain.as_str(), reason, row_version],
            |json_row| json_row.get(0),
        )
        .map_err(Into::into)
}

fn project_text_field(
    corrected_value: Option<String>,
    corrected_to_null: bool,
    attachment_value: Option<String>,
    fallback_value: Option<String>,
) -> Option<String> {
    if corrected_to_null {
        None
    } else {
        corrected_value
            .filter(|value| !value.trim().is_empty())
            .or_else(|| attachment_value.filter(|value| !value.trim().is_empty()))
            .or_else(|| fallback_value.filter(|value| !value.trim().is_empty()))
    }
}

fn project_int_field(
    corrected_value: Option<i64>,
    corrected_to_null: bool,
    attachment_value: Option<i64>,
    fallback_value: Option<i64>,
) -> Option<i64> {
    if corrected_to_null {
        None
    } else {
        corrected_value.or(attachment_value).or(fallback_value)
    }
}

fn project_stems_state_summary(capability_state: Option<String>) -> String {
    match capability_state.as_deref() {
        Some(STEMS_STATE_SUMMARY_QUEUED) => STEMS_STATE_SUMMARY_QUEUED,
        Some(STEMS_STATE_SUMMARY_LEASED) => STEMS_STATE_SUMMARY_LEASED,
        Some(STEMS_STATE_SUMMARY_READY) => STEMS_STATE_SUMMARY_READY,
        Some(STEMS_STATE_SUMMARY_STALE) => STEMS_STATE_SUMMARY_STALE,
        Some(STEMS_STATE_SUMMARY_BLOCKED) => STEMS_STATE_SUMMARY_BLOCKED,
        Some(STEMS_STATE_SUMMARY_FAILED) => STEMS_STATE_SUMMARY_FAILED,
        Some(STEMS_STATE_SUMMARY_MISSING) | None => STEMS_STATE_SUMMARY_MISSING,
        Some(_) => STEMS_STATE_SUMMARY_MISSING,
    }
    .to_string()
}

fn max_timestamp<const N: usize>(base: i64, candidates: [Option<i64>; N]) -> i64 {
    candidates
        .into_iter()
        .flatten()
        .fold(base, |current, candidate| current.max(candidate))
}

fn option_f64_eq(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CurrentLibraryAssetBrowserRow {
    row: LibraryBrowserProjectionRow,
    row_version: i64,
}

#[cfg(test)]
mod tests {
    use super::{
        PREP_READINESS_BLOCKED, PREP_READINESS_FAILED, PREP_READINESS_NOT_REQUIRED,
        PREP_READINESS_PREPARING, PREP_READINESS_READY, PREP_READINESS_UNDERPREPARED,
        STEMS_STATE_SUMMARY_MISSING, reseed_projection_domains,
    };
    use crate::authority::write_lane::admit_write;
    use crate::schema::install_baseline_schema_for_test;
    use library_domain::ProjectionDomain;
    use rusqlite::{Connection, params};
    use std::collections::BTreeSet;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct BrowserSummary {
        row_version: i64,
        waveform_quality_current: Option<i64>,
        waveform_quality_target: Option<i64>,
        stems_state_summary: Option<String>,
        prep_readiness_summary: String,
        updated_at: i64,
    }

    fn open_projection_test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        install_baseline_schema_for_test(&mut connection).expect("install canonical baseline");
        connection
    }

    fn rebuild_library_browser_projection(connection: &mut Connection) {
        admit_write(connection, |write| {
            reseed_projection_domains(write, &[ProjectionDomain::LibraryBrowser])
        })
        .expect("rebuild library-browser projection");
    }

    fn insert_library_asset(connection: &Connection, library_asset_id: i64, updated_at: i64) {
        connection
            .execute(
                "INSERT INTO LibraryAssets (
                     library_asset_id,
                     equivalence_fingerprint,
                     retention_policy,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 'keep_metadata', ?3, ?3)",
                params![
                    library_asset_id,
                    format!("eq:test:{library_asset_id}"),
                    updated_at,
                ],
            )
            .expect("insert library asset");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_capability(
        connection: &Connection,
        library_asset_id: i64,
        capability_kind: &str,
        profile_key: &str,
        state: &str,
        stability_class: Option<&str>,
        quality_current: Option<i64>,
        updated_at: i64,
    ) {
        let (stability_class, basis_fingerprint) = match state {
            "ready" | "stale" => (
                Some(stability_class.unwrap_or("stable").to_string()),
                Some(format!(
                    "basis:{library_asset_id}:{capability_kind}:{profile_key}:{state}"
                )),
            ),
            _ => (None, None),
        };

        connection
            .execute(
                "INSERT INTO LibraryAssetCapabilities (
                     library_asset_id,
                     capability_kind,
                     profile_key,
                     state,
                     stability_class,
                     quality_current,
                     basis_fingerprint,
                     selected_artifact_id,
                     updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
                params![
                    library_asset_id,
                    capability_kind,
                    profile_key,
                    state,
                    stability_class,
                    quality_current,
                    basis_fingerprint,
                    updated_at,
                ],
            )
            .expect("insert capability");
    }

    fn insert_prep_policy(connection: &Connection, prep_policy_id: i64) {
        connection
            .execute(
                "INSERT OR IGNORE INTO PrepPolicies (
                     prep_policy_id,
                     policy_name,
                     is_system_policy,
                     is_user_editable,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, ?2, 0, 1, 1, 1)",
                params![prep_policy_id, format!("policy:{prep_policy_id}")],
            )
            .expect("insert prep policy");
    }

    fn insert_resolved_target(
        connection: &Connection,
        library_asset_id: i64,
        capability_kind: &str,
        target_profile_key: &str,
        target_quality: i64,
        target_stability_class: &str,
        updated_at: i64,
    ) {
        let prep_policy_id = 1;
        insert_prep_policy(connection, prep_policy_id);
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
                 VALUES (?1, ?2, ?3, ?4, ?5, 'interactive', ?6, ?7)",
                params![
                    library_asset_id,
                    capability_kind,
                    target_profile_key,
                    target_quality,
                    target_stability_class,
                    prep_policy_id,
                    updated_at,
                ],
            )
            .expect("insert resolved target");
    }

    fn load_browser_summary(connection: &Connection, library_asset_id: i64) -> BrowserSummary {
        connection
            .query_row(
                "SELECT row_version,
                        waveform_quality_current,
                        waveform_quality_target,
                        stems_state_summary,
                        prep_readiness_summary,
                        updated_at
                 FROM LibraryBrowserRows
                 WHERE library_asset_id = ?1",
                [library_asset_id],
                |row| {
                    Ok(BrowserSummary {
                        row_version: row.get(0)?,
                        waveform_quality_current: row.get(1)?,
                        waveform_quality_target: row.get(2)?,
                        stems_state_summary: row.get(3)?,
                        prep_readiness_summary: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .expect("load browser summary")
    }

    fn library_browser_change_count(connection: &Connection, library_asset_id: i64) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*)
                 FROM ProjectionChangeLog
                 WHERE projection_domain = 'library_browser'
                   AND row_key = ?1",
                [library_asset_id.to_string()],
                |row| row.get(0),
            )
            .expect("count library-browser changes")
    }

    fn latest_stems_change_payload(
        connection: &Connection,
        library_asset_id: i64,
    ) -> (i64, String) {
        connection
            .query_row(
                "SELECT row_version,
                        CAST(json_extract(payload_json, '$.stems_state_summary') AS TEXT)
                 FROM ProjectionChangeLog
                 WHERE projection_domain = 'library_browser'
                   AND row_key = ?1
                 ORDER BY change_sequence DESC
                 LIMIT 1",
                [library_asset_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load latest stems change payload")
    }

    #[test]
    fn projection_rebuild_derives_waveform_summaries_from_capabilities_and_targets() {
        let mut connection = open_projection_test_connection();

        insert_library_asset(&connection, 1, 10);
        insert_library_asset(&connection, 2, 20);
        insert_library_asset(&connection, 3, 30);
        insert_library_asset(&connection, 4, 40);

        insert_capability(
            &connection,
            1,
            "waveform",
            "default",
            "ready",
            Some("stable"),
            Some(80),
            100,
        );
        insert_capability(
            &connection,
            1,
            "waveform",
            "hires",
            "ready",
            Some("stable"),
            Some(88),
            101,
        );
        insert_resolved_target(&connection, 1, "waveform", "hires", 95, "stable", 110);

        insert_capability(
            &connection,
            2,
            "waveform",
            "default",
            "stale",
            Some("stable"),
            Some(70),
            120,
        );
        insert_resolved_target(&connection, 3, "waveform", "default", 90, "stable", 130);

        rebuild_library_browser_projection(&mut connection);

        assert_eq!(
            load_browser_summary(&connection, 1),
            BrowserSummary {
                row_version: 1,
                waveform_quality_current: Some(88),
                waveform_quality_target: Some(95),
                stems_state_summary: Some(STEMS_STATE_SUMMARY_MISSING.to_string()),
                prep_readiness_summary: PREP_READINESS_UNDERPREPARED.to_string(),
                updated_at: 110,
            }
        );
        assert_eq!(
            load_browser_summary(&connection, 2),
            BrowserSummary {
                row_version: 1,
                waveform_quality_current: Some(70),
                waveform_quality_target: None,
                stems_state_summary: Some(STEMS_STATE_SUMMARY_MISSING.to_string()),
                prep_readiness_summary: PREP_READINESS_NOT_REQUIRED.to_string(),
                updated_at: 120,
            }
        );
        assert_eq!(
            load_browser_summary(&connection, 3),
            BrowserSummary {
                row_version: 1,
                waveform_quality_current: None,
                waveform_quality_target: Some(90),
                stems_state_summary: Some(STEMS_STATE_SUMMARY_MISSING.to_string()),
                prep_readiness_summary: PREP_READINESS_UNDERPREPARED.to_string(),
                updated_at: 130,
            }
        );
        assert_eq!(
            load_browser_summary(&connection, 4),
            BrowserSummary {
                row_version: 1,
                waveform_quality_current: None,
                waveform_quality_target: None,
                stems_state_summary: Some(STEMS_STATE_SUMMARY_MISSING.to_string()),
                prep_readiness_summary: PREP_READINESS_NOT_REQUIRED.to_string(),
                updated_at: 40,
            }
        );
    }

    #[test]
    fn projection_rebuild_derives_stems_summary_from_finite_capability_state_vocabulary() {
        let mut connection = open_projection_test_connection();
        let states = [
            "missing", "queued", "leased", "ready", "stale", "blocked", "failed",
        ];

        for (index, state) in states.iter().enumerate() {
            let library_asset_id = i64::try_from(index).expect("state index fits i64") + 1;
            insert_library_asset(&connection, library_asset_id, 10 + library_asset_id);
            insert_capability(
                &connection,
                library_asset_id,
                "stems",
                "default",
                state,
                Some("stable"),
                Some(4),
                100 + library_asset_id,
            );
        }

        insert_library_asset(&connection, 100, 200);
        insert_library_asset(&connection, 101, 201);
        insert_capability(
            &connection,
            101,
            "stems",
            "default",
            "ready",
            Some("stable"),
            Some(4),
            210,
        );
        insert_capability(
            &connection,
            101,
            "stems",
            "split",
            "blocked",
            None,
            Some(4),
            211,
        );
        insert_resolved_target(&connection, 101, "stems", "split", 4, "stable", 212);

        rebuild_library_browser_projection(&mut connection);

        for (index, state) in states.iter().enumerate() {
            let library_asset_id = i64::try_from(index).expect("state index fits i64") + 1;
            assert_eq!(
                load_browser_summary(&connection, library_asset_id).stems_state_summary,
                Some((*state).to_string())
            );
        }
        assert_eq!(
            load_browser_summary(&connection, 100).stems_state_summary,
            Some(STEMS_STATE_SUMMARY_MISSING.to_string())
        );
        assert_eq!(
            load_browser_summary(&connection, 101).stems_state_summary,
            Some("blocked".to_string())
        );
    }

    #[test]
    fn projection_rebuild_derives_prep_readiness_from_resolved_targets_and_capabilities() {
        let mut connection = open_projection_test_connection();
        for library_asset_id in 1..=13 {
            insert_library_asset(&connection, library_asset_id, 10 + library_asset_id);
        }

        insert_resolved_target(&connection, 2, "waveform", "default", 80, "stable", 120);
        insert_capability(
            &connection,
            2,
            "waveform",
            "default",
            "ready",
            Some("stable"),
            Some(90),
            121,
        );

        insert_resolved_target(&connection, 3, "waveform", "default", 80, "stable", 130);
        insert_capability(
            &connection,
            3,
            "waveform",
            "default",
            "queued",
            None,
            None,
            131,
        );

        insert_resolved_target(&connection, 4, "waveform", "default", 80, "stable", 140);
        insert_resolved_target(&connection, 4, "stems", "split", 4, "stable", 141);
        insert_capability(
            &connection,
            4,
            "waveform",
            "default",
            "queued",
            None,
            None,
            142,
        );
        insert_capability(&connection, 4, "stems", "split", "leased", None, None, 143);

        insert_resolved_target(&connection, 5, "waveform", "missing", 80, "stable", 150);

        insert_resolved_target(&connection, 6, "waveform", "default", 80, "stable", 160);
        insert_capability(
            &connection,
            6,
            "waveform",
            "default",
            "stale",
            Some("stable"),
            Some(100),
            161,
        );

        insert_resolved_target(&connection, 7, "waveform", "default", 100, "stable", 170);
        insert_capability(
            &connection,
            7,
            "waveform",
            "default",
            "ready",
            Some("stable"),
            Some(99),
            171,
        );

        insert_resolved_target(&connection, 8, "waveform", "default", 80, "stable", 180);
        insert_capability(
            &connection,
            8,
            "waveform",
            "default",
            "ready",
            Some("provisional"),
            Some(100),
            181,
        );

        insert_resolved_target(&connection, 9, "waveform", "default", 80, "stable", 190);
        insert_resolved_target(&connection, 9, "stems", "split", 4, "stable", 191);
        insert_capability(
            &connection,
            9,
            "waveform",
            "default",
            "blocked",
            None,
            None,
            192,
        );
        insert_capability(&connection, 9, "stems", "split", "queued", None, None, 193);

        insert_resolved_target(&connection, 10, "waveform", "default", 80, "stable", 200);
        insert_resolved_target(&connection, 10, "stems", "split", 4, "stable", 201);
        insert_capability(
            &connection,
            10,
            "waveform",
            "default",
            "failed",
            None,
            None,
            202,
        );
        insert_capability(
            &connection,
            10,
            "stems",
            "split",
            "blocked",
            None,
            None,
            203,
        );

        insert_resolved_target(
            &connection,
            11,
            "waveform",
            "provisional",
            80,
            "provisional",
            210,
        );
        insert_capability(
            &connection,
            11,
            "waveform",
            "provisional",
            "ready",
            Some("provisional"),
            Some(80),
            211,
        );

        insert_resolved_target(
            &connection,
            12,
            "waveform",
            "stable-for-provisional",
            80,
            "provisional",
            220,
        );
        insert_capability(
            &connection,
            12,
            "waveform",
            "stable-for-provisional",
            "ready",
            Some("stable"),
            Some(80),
            221,
        );

        insert_resolved_target(
            &connection,
            13,
            "waveform",
            "stable-target",
            80,
            "stable",
            230,
        );
        insert_capability(
            &connection,
            13,
            "waveform",
            "stable-target",
            "ready",
            Some("provisional"),
            Some(80),
            231,
        );

        rebuild_library_browser_projection(&mut connection);

        let expected = [
            (1, PREP_READINESS_NOT_REQUIRED),
            (2, PREP_READINESS_READY),
            (3, PREP_READINESS_PREPARING),
            (4, PREP_READINESS_PREPARING),
            (5, PREP_READINESS_UNDERPREPARED),
            (6, PREP_READINESS_UNDERPREPARED),
            (7, PREP_READINESS_UNDERPREPARED),
            (8, PREP_READINESS_UNDERPREPARED),
            (9, PREP_READINESS_BLOCKED),
            (10, PREP_READINESS_FAILED),
            (11, PREP_READINESS_READY),
            (12, PREP_READINESS_READY),
            (13, PREP_READINESS_UNDERPREPARED),
        ];

        for (library_asset_id, expected_summary) in expected {
            assert_eq!(
                load_browser_summary(&connection, library_asset_id).prep_readiness_summary,
                expected_summary
            );
        }

        let vocabulary = (1..=13)
            .map(|library_asset_id| {
                load_browser_summary(&connection, library_asset_id).prep_readiness_summary
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            vocabulary,
            BTreeSet::from([
                PREP_READINESS_NOT_REQUIRED.to_string(),
                PREP_READINESS_READY.to_string(),
                PREP_READINESS_PREPARING.to_string(),
                PREP_READINESS_UNDERPREPARED.to_string(),
                PREP_READINESS_BLOCKED.to_string(),
                PREP_READINESS_FAILED.to_string(),
            ])
        );
    }

    #[test]
    fn projection_summary_changes_advance_row_version_and_change_log_payload() {
        let mut connection = open_projection_test_connection();
        insert_library_asset(&connection, 1, 10);
        insert_capability(
            &connection,
            1,
            "stems",
            "default",
            "queued",
            None,
            Some(4),
            20,
        );

        rebuild_library_browser_projection(&mut connection);

        assert_eq!(
            load_browser_summary(&connection, 1).stems_state_summary,
            Some("queued".to_string())
        );
        assert_eq!(load_browser_summary(&connection, 1).row_version, 1);
        assert_eq!(library_browser_change_count(&connection, 1), 1);
        assert_eq!(
            latest_stems_change_payload(&connection, 1),
            (1, "queued".to_string())
        );

        rebuild_library_browser_projection(&mut connection);
        assert_eq!(library_browser_change_count(&connection, 1), 1);

        connection
            .execute(
                "UPDATE LibraryAssetCapabilities
                 SET state = 'leased',
                     updated_at = 30
                 WHERE library_asset_id = 1
                   AND capability_kind = 'stems'
                   AND profile_key = 'default'",
                [],
            )
            .expect("update stems capability state");

        rebuild_library_browser_projection(&mut connection);

        assert_eq!(
            load_browser_summary(&connection, 1),
            BrowserSummary {
                row_version: 2,
                waveform_quality_current: None,
                waveform_quality_target: None,
                stems_state_summary: Some("leased".to_string()),
                prep_readiness_summary: PREP_READINESS_NOT_REQUIRED.to_string(),
                updated_at: 30,
            }
        );
        assert_eq!(library_browser_change_count(&connection, 1), 2);
        assert_eq!(
            latest_stems_change_payload(&connection, 1),
            (2, "leased".to_string())
        );
    }
}
