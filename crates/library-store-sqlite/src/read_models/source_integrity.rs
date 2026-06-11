use rusqlite::{Connection, Row};

use crate::LibrarySqliteResult;
use crate::read_models::source_lifecycle::{StoreSourceLifecycle, read_source_lifecycle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceIntegrity {
    pub source_id: i64,
    pub lifecycle: Option<StoreSourceLifecycle>,
    pub coverage: StoreSourceIntegrityCoverage,
    pub inventory: Option<StoreSourceIntegrityInventory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSourceIntegrityCoverageState {
    Complete,
    Pending,
    Scanning,
    Blocked,
    Failed,
    SourceUnavailable,
    LocationMissing,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceIntegrityCoverage {
    pub state: StoreSourceIntegrityCoverageState,
    pub subtree_coverage_complete: bool,
    pub empty_result_authoritative: bool,
    pub total_directories_count: usize,
    pub missing_directories_count: usize,
    pub pending_directories_count: usize,
    pub scanning_directories_count: usize,
    pub blocked_directories_count: usize,
    pub failed_directories_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceIntegrityInventory {
    pub counts_by_presence_state: Vec<StoreSourceIntegrityCount>,
    pub counts_by_file_class: Vec<StoreSourceIntegrityCount>,
    pub counts_by_file_kind: Vec<StoreSourceIntegrityCount>,
    pub media_relevant_files_count: usize,
    pub present_media_relevant_files_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceIntegrityCount {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirectoryCoverageCounts {
    total: usize,
    missing: usize,
    pending: usize,
    scanning: usize,
    blocked: usize,
    failed: usize,
}

pub fn read_source_integrity(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<StoreSourceIntegrity> {
    let lifecycle = read_source_lifecycle(connection, source_id)?;
    let coverage = read_source_coverage(connection, source_id, lifecycle.as_ref())?;
    let inventory = if lifecycle.is_some() {
        Some(read_inventory(connection, source_id)?)
    } else {
        None
    };

    Ok(StoreSourceIntegrity {
        source_id,
        lifecycle,
        coverage,
        inventory,
    })
}

fn read_source_coverage(
    connection: &Connection,
    source_id: i64,
    lifecycle: Option<&StoreSourceLifecycle>,
) -> LibrarySqliteResult<StoreSourceIntegrityCoverage> {
    let Some(lifecycle) = lifecycle else {
        return Ok(source_coverage(
            StoreSourceIntegrityCoverageState::SourceUnavailable,
            false,
            DirectoryCoverageCounts {
                total: 0,
                missing: 0,
                pending: 0,
                scanning: 0,
                blocked: 0,
                failed: 0,
            },
        ));
    };

    let counts = read_directory_coverage_counts(connection, source_id)?;
    if let Some(state) = lifecycle_failure_coverage_state(lifecycle) {
        return Ok(source_coverage(state, false, counts));
    }

    Ok(source_coverage_from_counts(lifecycle, counts))
}

fn lifecycle_failure_coverage_state(
    lifecycle: &StoreSourceLifecycle,
) -> Option<StoreSourceIntegrityCoverageState> {
    if lifecycle.source_class != "internal" && lifecycle.mount_status != "mounted" {
        return Some(StoreSourceIntegrityCoverageState::SourceUnavailable);
    }

    match lifecycle.access_state.as_str() {
        "missing" => return Some(StoreSourceIntegrityCoverageState::LocationMissing),
        "blocked" => {
            return Some(
                if lifecycle.access_issue_kind.as_deref() == Some("unavailable_mount") {
                    StoreSourceIntegrityCoverageState::SourceUnavailable
                } else {
                    StoreSourceIntegrityCoverageState::Blocked
                },
            );
        }
        _ => {}
    }

    match lifecycle.scan_phase.as_str() {
        "blocked" => Some(
            if lifecycle.scan_issue_kind.as_deref() == Some("unavailable_mount") {
                StoreSourceIntegrityCoverageState::SourceUnavailable
            } else {
                StoreSourceIntegrityCoverageState::Blocked
            },
        ),
        "failed" => Some(StoreSourceIntegrityCoverageState::Failed),
        _ => None,
    }
}

fn read_directory_coverage_counts(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<DirectoryCoverageCounts> {
    connection
        .query_row(
            "SELECT SUM(CASE WHEN presence_state IN ('present', 'missing') THEN 1 ELSE 0 END),
                    SUM(CASE WHEN presence_state = 'missing' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN presence_state = 'present' AND dir_scan_state = 'pending' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN presence_state = 'present' AND dir_scan_state = 'scanning' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN presence_state = 'present' AND dir_scan_state = 'blocked' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN presence_state = 'present' AND dir_scan_state = 'failed' THEN 1 ELSE 0 END)
             FROM source_directories
             WHERE source_id = ?1",
            [source_id],
            |row| {
                Ok(DirectoryCoverageCounts {
                    total: read_optional_count(row, 0)?,
                    missing: read_optional_count(row, 1)?,
                    pending: read_optional_count(row, 2)?,
                    scanning: read_optional_count(row, 3)?,
                    blocked: read_optional_count(row, 4)?,
                    failed: read_optional_count(row, 5)?,
                })
            },
        )
        .map_err(Into::into)
}

fn source_coverage_from_counts(
    lifecycle: &StoreSourceLifecycle,
    counts: DirectoryCoverageCounts,
) -> StoreSourceIntegrityCoverage {
    if counts.total == 0 {
        if lifecycle.scan_phase == "complete" {
            return source_coverage(StoreSourceIntegrityCoverageState::Complete, true, counts);
        }
        return source_coverage(pending_or_scanning_coverage_state(lifecycle), false, counts);
    }

    if counts.blocked > 0 {
        return source_coverage(StoreSourceIntegrityCoverageState::Blocked, false, counts);
    }
    if counts.failed > 0 {
        return source_coverage(StoreSourceIntegrityCoverageState::Failed, false, counts);
    }
    if counts.scanning > 0 {
        return source_coverage(StoreSourceIntegrityCoverageState::Scanning, false, counts);
    }
    if counts.pending > 0 {
        return source_coverage(StoreSourceIntegrityCoverageState::Pending, false, counts);
    }
    if counts.missing > 0 {
        return source_coverage(StoreSourceIntegrityCoverageState::Incomplete, false, counts);
    }

    match lifecycle.scan_phase.as_str() {
        "complete" => source_coverage(StoreSourceIntegrityCoverageState::Complete, true, counts),
        "partial" => source_coverage(StoreSourceIntegrityCoverageState::Incomplete, false, counts),
        _ => source_coverage(pending_or_scanning_coverage_state(lifecycle), false, counts),
    }
}

fn pending_or_scanning_coverage_state(
    lifecycle: &StoreSourceLifecycle,
) -> StoreSourceIntegrityCoverageState {
    if lifecycle.scan_phase == "scanning" {
        StoreSourceIntegrityCoverageState::Scanning
    } else {
        StoreSourceIntegrityCoverageState::Pending
    }
}

fn source_coverage(
    state: StoreSourceIntegrityCoverageState,
    subtree_coverage_complete: bool,
    counts: DirectoryCoverageCounts,
) -> StoreSourceIntegrityCoverage {
    StoreSourceIntegrityCoverage {
        state,
        subtree_coverage_complete,
        empty_result_authoritative: state == StoreSourceIntegrityCoverageState::Complete
            && subtree_coverage_complete,
        total_directories_count: counts.total,
        missing_directories_count: counts.missing,
        pending_directories_count: counts.pending,
        scanning_directories_count: counts.scanning,
        blocked_directories_count: counts.blocked,
        failed_directories_count: counts.failed,
    }
}

fn read_inventory(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<StoreSourceIntegrityInventory> {
    Ok(StoreSourceIntegrityInventory {
        counts_by_presence_state: read_group_counts(
            connection,
            source_id,
            "presence_state",
            &["present", "missing", "removed"],
        )?,
        counts_by_file_class: read_group_counts(
            connection,
            source_id,
            "file_class",
            &["audio", "video", "image", "unsupported", "none"],
        )?,
        counts_by_file_kind: read_group_counts(
            connection,
            source_id,
            "file_kind",
            &[
                "audio",
                "video",
                "image",
                "cue_sheet",
                "log_doc",
                "text_doc",
                "archive",
                "other",
                "unknown",
            ],
        )?,
        media_relevant_files_count: read_media_relevant_count(connection, source_id, false)?,
        present_media_relevant_files_count: read_media_relevant_count(connection, source_id, true)?,
    })
}

fn read_group_counts(
    connection: &Connection,
    source_id: i64,
    column_name: &str,
    order: &[&str],
) -> LibrarySqliteResult<Vec<StoreSourceIntegrityCount>> {
    let order_sql = order
        .iter()
        .enumerate()
        .map(|(index, value)| format!("WHEN '{value}' THEN {index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let sql = format!(
        "SELECT {column_name}, COUNT(*)
         FROM source_files
         WHERE source_id = ?1
         GROUP BY {column_name}
         ORDER BY CASE {column_name} {order_sql} ELSE {} END ASC,
                  {column_name} ASC",
        order.len()
    );
    let mut stmt = connection.prepare(&sql)?;
    stmt.query_map([source_id], |row| {
        Ok(StoreSourceIntegrityCount {
            value: row.get(0)?,
            count: read_count(row, 1)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}

fn read_media_relevant_count(
    connection: &Connection,
    source_id: i64,
    present_only: bool,
) -> LibrarySqliteResult<usize> {
    let presence_predicate = if present_only {
        "AND presence_state = 'present'"
    } else {
        ""
    };
    let sql = format!(
        "SELECT COUNT(*)
         FROM source_files
         WHERE source_id = ?1
           {presence_predicate}
           AND (
               file_class IN ('audio', 'video', 'image')
               OR (file_class = 'unsupported' AND file_kind = 'cue_sheet')
           )"
    );
    connection
        .query_row(&sql, [source_id], |row| read_count(row, 0))
        .map_err(Into::into)
}

fn read_count(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, i64>(index)?;
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

fn read_optional_count(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let count = row.get::<_, Option<i64>>(index)?.unwrap_or(0);
    usize::try_from(count).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, count))
}

#[cfg(test)]
mod tests {
    use super::{
        StoreSourceIntegrityCoverageState, read_media_relevant_count, read_source_integrity,
    };
    use crate::schema::install_baseline_schema_for_test;
    use rusqlite::{Connection, params};

    fn test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        connection
    }

    fn insert_source(connection: &Connection, source_id: i64, scan_phase: &str) {
        connection
            .execute(
                "INSERT INTO sources (
                     source_id, source_class, authority, identity_key, display_name,
                     is_user_visible, created_at, updated_at
                 )
                 VALUES (?1, 'external_mounted', 'device', ?2, 'Fixture', 1, 1, 1)",
                params![source_id, format!("source:{source_id}")],
            )
            .expect("insert source");
        connection
            .execute(
                "INSERT INTO source_state (
                     source_id, mount_status, mount_epoch, access_state, access_issue_kind,
                     access_checked_at, effective_path, updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'accessible', NULL, 1, 'root', 1)",
                [source_id],
            )
            .expect("insert source state");
        connection
            .execute(
                "INSERT INTO source_scan_state (
                     source_id, scan_phase, last_scan_started_at, last_scan_finished_at,
                     last_successful_scan_at, scan_issue_kind, updated_at
                 )
                 VALUES (?1, ?2, 1, 2, 2, NULL, 2)",
                params![source_id, scan_phase],
            )
            .expect("insert scan state");
    }

    fn insert_directory(
        connection: &Connection,
        directory_id: i64,
        source_id: i64,
        relative_path: &str,
        scan_state: &str,
    ) {
        insert_directory_with_presence(
            connection,
            directory_id,
            source_id,
            relative_path,
            "present",
            scan_state,
        );
    }

    fn insert_directory_with_presence(
        connection: &Connection,
        directory_id: i64,
        source_id: i64,
        relative_path: &str,
        presence_state: &str,
        scan_state: &str,
    ) {
        let issue_kind = match scan_state {
            "blocked" => Some("permission_denied"),
            "failed" => Some("unknown_io"),
            _ => None,
        };
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id, source_id, parent_source_directory_id, name,
                     name_browse_sort_key, relative_path, presence_state, dir_scan_state,
                     dir_scan_issue_kind, dir_scan_updated_at, created_at, updated_at
                 )
                 VALUES (?1, ?2, NULL, ?3, ?3, ?3, ?4, ?5, ?6, 1, 1, 1)",
                params![
                    directory_id,
                    source_id,
                    relative_path,
                    presence_state,
                    scan_state,
                    issue_kind
                ],
            )
            .expect("insert directory");
    }

    fn insert_file(
        connection: &Connection,
        file_id: i64,
        source_id: i64,
        relative_path: &str,
        file_kind: &str,
        file_class: &str,
        presence_state: &str,
    ) {
        connection
            .execute(
                "INSERT INTO source_files (
                     source_file_id, source_id, parent_source_directory_id, name,
                     name_browse_sort_key, relative_path_browse_sort_key, relative_path,
                     size_bytes, mtime_ns, file_kind, file_class, presence_state,
                     first_discovered_at, last_observed_at, last_presence_change_at,
                     created_at, updated_at
                 )
                 VALUES (?1, ?2, NULL, ?3, ?3, ?3, ?3, 10, 20, ?4, ?5, ?6, 1, 1, 1, 1, 1)",
                params![
                    file_id,
                    source_id,
                    relative_path,
                    file_kind,
                    file_class,
                    presence_state
                ],
            )
            .expect("insert file");
    }

    #[test]
    fn source_integrity_reports_inventory_counts_for_known_complete_source() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_directory(&connection, 70, 7, "Music", "complete");
        insert_file(
            &connection,
            1,
            7,
            "Music/a.flac",
            "audio",
            "audio",
            "present",
        );
        insert_file(
            &connection,
            2,
            7,
            "Music/v.mp4",
            "video",
            "video",
            "missing",
        );
        insert_file(
            &connection,
            3,
            7,
            "Music/album.cue",
            "cue_sheet",
            "unsupported",
            "present",
        );
        insert_file(
            &connection,
            4,
            7,
            "Music/readme.txt",
            "text_doc",
            "unsupported",
            "present",
        );

        let read = read_source_integrity(&connection, 7).expect("read source integrity");
        assert!(read.lifecycle.is_some());
        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::Complete
        );
        assert!(read.coverage.empty_result_authoritative);

        let inventory = read.inventory.expect("known source inventory");
        assert_eq!(inventory.media_relevant_files_count, 3);
        assert_eq!(inventory.present_media_relevant_files_count, 2);
        assert!(
            inventory
                .counts_by_presence_state
                .iter()
                .any(|count| count.value == "missing" && count.count == 1)
        );
    }

    #[test]
    fn missing_directory_only_makes_complete_source_non_authoritative() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_directory_with_presence(&connection, 70, 7, "Missing", "missing", "complete");

        let read = read_source_integrity(&connection, 7).expect("read source integrity");

        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::Incomplete
        );
        assert!(!read.coverage.subtree_coverage_complete);
        assert!(!read.coverage.empty_result_authoritative);
        assert_eq!(read.coverage.total_directories_count, 1);
        assert_eq!(read.coverage.missing_directories_count, 1);
    }

    #[test]
    fn missing_directory_with_complete_present_directories_keeps_coverage_incomplete() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_directory(&connection, 70, 7, "Present", "complete");
        insert_directory_with_presence(&connection, 71, 7, "Missing", "missing", "complete");

        let read = read_source_integrity(&connection, 7).expect("read source integrity");

        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::Incomplete
        );
        assert!(!read.coverage.empty_result_authoritative);
        assert_eq!(read.coverage.total_directories_count, 2);
        assert_eq!(read.coverage.missing_directories_count, 1);
    }

    #[test]
    fn source_level_missing_state_wins_over_descendant_missing_directory() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_directory_with_presence(&connection, 70, 7, "Missing", "missing", "complete");
        connection
            .execute(
                "UPDATE source_state
                 SET access_state = 'missing',
                     access_issue_kind = 'missing'
                 WHERE source_id = 7",
                [],
            )
            .expect("mark source root missing");

        let read = read_source_integrity(&connection, 7).expect("read source integrity");

        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::LocationMissing
        );
        assert!(!read.coverage.empty_result_authoritative);
        assert_eq!(read.coverage.missing_directories_count, 1);
    }

    #[test]
    fn unavailable_mount_wins_over_descendant_missing_directory() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_directory_with_presence(&connection, 70, 7, "Missing", "missing", "complete");
        connection
            .execute(
                "UPDATE source_state
                 SET mount_status = 'unmounted'
                 WHERE source_id = 7",
                [],
            )
            .expect("mark source unavailable");

        let read = read_source_integrity(&connection, 7).expect("read source integrity");

        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::SourceUnavailable
        );
        assert!(!read.coverage.empty_result_authoritative);
        assert_eq!(read.coverage.missing_directories_count, 1);
    }

    #[test]
    fn blocked_and_failed_directory_precedence_wins_over_missing_directory() {
        for (scan_state, expected) in [
            ("blocked", StoreSourceIntegrityCoverageState::Blocked),
            ("failed", StoreSourceIntegrityCoverageState::Failed),
        ] {
            let connection = test_connection();
            insert_source(&connection, 7, "complete");
            insert_directory(&connection, 70, 7, "Trouble", scan_state);
            insert_directory_with_presence(&connection, 71, 7, "Missing", "missing", "complete");

            let read = read_source_integrity(&connection, 7).expect("read source integrity");

            assert_eq!(read.coverage.state, expected, "{scan_state}");
            assert!(!read.coverage.empty_result_authoritative);
            assert_eq!(read.coverage.missing_directories_count, 1);
        }
    }

    #[test]
    fn blocked_source_does_not_report_authoritative_empty_coverage() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        connection
            .execute(
                "UPDATE source_state
                 SET access_state = 'blocked',
                     access_issue_kind = 'permission_denied'
                 WHERE source_id = 7",
                [],
            )
            .expect("block source");

        let read = read_source_integrity(&connection, 7).expect("read source integrity");
        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::Blocked
        );
        assert!(!read.coverage.empty_result_authoritative);
    }

    #[test]
    fn missing_source_is_distinct_from_complete_empty_source() {
        let connection = test_connection();

        let read = read_source_integrity(&connection, 999).expect("read source integrity");
        assert!(read.lifecycle.is_none());
        assert_eq!(
            read.coverage.state,
            StoreSourceIntegrityCoverageState::SourceUnavailable
        );
        assert!(!read.coverage.empty_result_authoritative);
        assert!(read.inventory.is_none());
    }

    #[test]
    fn media_relevant_count_excludes_broad_unsupported_rows() {
        let connection = test_connection();
        insert_source(&connection, 7, "complete");
        insert_file(
            &connection,
            1,
            7,
            "a.cue",
            "cue_sheet",
            "unsupported",
            "present",
        );
        insert_file(
            &connection,
            2,
            7,
            "notes.txt",
            "text_doc",
            "unsupported",
            "present",
        );
        assert_eq!(
            read_media_relevant_count(&connection, 7, false).expect("count media relevant"),
            1
        );
    }
}
