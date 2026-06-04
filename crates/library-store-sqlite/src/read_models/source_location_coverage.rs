use rusqlite::{Connection, OptionalExtension, params};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceLocationCoverage {
    Present,
    Missing,
    Unknown,
    Blocked,
    Failed,
    Pending,
    Scanning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcceptedSourceLocationCoverage {
    AllPresent,
    AllMissing,
    MixedMissing,
    Blocked,
    Failed,
    Scanning,
    Pending,
}

pub(crate) fn classify_source_location_coverage(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
    scan_phase: Option<&str>,
) -> LibrarySqliteResult<SourceLocationCoverage> {
    let direct = load_directory_presence_and_scan_state(connection, source_id, relative_path)?;

    if let Some((presence_state, dir_scan_state)) = direct {
        return Ok(match presence_state.as_str() {
            "present" => dir_scan_state_to_coverage(&dir_scan_state),
            "missing" => SourceLocationCoverage::Missing,
            "removed" => SourceLocationCoverage::Missing,
            _ => SourceLocationCoverage::Unknown,
        });
    }

    let segments: Vec<&str> = relative_path.split('/').collect();
    for i in (1..segments.len()).rev() {
        let prefix = segments[..i].join("/");
        let ancestor = load_directory_presence_and_scan_state(connection, source_id, &prefix)?;

        if let Some((presence_state, dir_scan_state)) = ancestor {
            return Ok(match presence_state.as_str() {
                "missing" | "removed" => SourceLocationCoverage::Missing,
                "present" => match dir_scan_state.as_deref() {
                    Some("blocked") => SourceLocationCoverage::Blocked,
                    Some("failed") => SourceLocationCoverage::Failed,
                    Some("scanning") => SourceLocationCoverage::Scanning,
                    Some("pending") => SourceLocationCoverage::Pending,
                    Some("complete") => SourceLocationCoverage::Missing,
                    _ => SourceLocationCoverage::Unknown,
                },
                _ => SourceLocationCoverage::Unknown,
            });
        }
    }

    Ok(match scan_phase {
        Some("complete") => SourceLocationCoverage::Missing,
        Some("partial") => SourceLocationCoverage::Unknown,
        Some("scanning") => SourceLocationCoverage::Scanning,
        Some("idle") => SourceLocationCoverage::Pending,
        Some("blocked") => SourceLocationCoverage::Blocked,
        Some("failed") => SourceLocationCoverage::Failed,
        _ => SourceLocationCoverage::Pending,
    })
}

pub(crate) fn aggregate_source_location_coverages(
    coverages: &[SourceLocationCoverage],
) -> AcceptedSourceLocationCoverage {
    let mut has_present = false;
    let mut has_missing = false;
    let mut has_blocked = false;
    let mut has_failed = false;
    let mut has_scanning = false;
    let mut has_pending = false;

    for coverage in coverages {
        match coverage {
            SourceLocationCoverage::Present => has_present = true,
            SourceLocationCoverage::Missing => has_missing = true,
            SourceLocationCoverage::Blocked => has_blocked = true,
            SourceLocationCoverage::Failed => has_failed = true,
            SourceLocationCoverage::Scanning => has_scanning = true,
            SourceLocationCoverage::Pending => has_pending = true,
            SourceLocationCoverage::Unknown => has_pending = true,
        }
    }

    if has_blocked {
        return AcceptedSourceLocationCoverage::Blocked;
    }
    if has_failed {
        return AcceptedSourceLocationCoverage::Failed;
    }
    if has_missing && has_present {
        return AcceptedSourceLocationCoverage::MixedMissing;
    }
    if has_missing && (has_scanning || has_pending) {
        return AcceptedSourceLocationCoverage::MixedMissing;
    }
    if has_missing {
        return AcceptedSourceLocationCoverage::AllMissing;
    }
    if has_scanning {
        return AcceptedSourceLocationCoverage::Scanning;
    }
    if has_pending {
        return AcceptedSourceLocationCoverage::Pending;
    }
    AcceptedSourceLocationCoverage::AllPresent
}

fn load_directory_presence_and_scan_state(
    connection: &Connection,
    source_id: i64,
    relative_path: &str,
) -> LibrarySqliteResult<Option<(String, Option<String>)>> {
    connection
        .query_row(
            "SELECT presence_state, dir_scan_state
             FROM source_directories
             WHERE source_id = ?1
               AND relative_path COLLATE BINARY = ?2 COLLATE BINARY",
            params![source_id, relative_path],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(Into::into)
}

fn dir_scan_state_to_coverage(dir_scan_state: &Option<String>) -> SourceLocationCoverage {
    match dir_scan_state.as_deref() {
        Some("blocked") => SourceLocationCoverage::Blocked,
        Some("failed") => SourceLocationCoverage::Failed,
        Some("scanning") => SourceLocationCoverage::Scanning,
        Some("pending") => SourceLocationCoverage::Pending,
        _ => SourceLocationCoverage::Present,
    }
}

#[cfg(test)]
mod tests {
    use super::SourceLocationCoverage;
    use crate::schema::install_baseline_schema_for_test;
    use rusqlite::Connection;

    fn setup_connection() -> Connection {
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
                rusqlite::params![
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
                     access_state,
                     access_checked_at,
                     mount_root,
                     effective_path,
                     updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'accessible', 1, 'root', 'root', 1)",
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

    fn set_source_scan_phase(connection: &Connection, source_id: i64, scan_phase: &str) {
        connection
            .execute(
                "UPDATE source_scan_state SET scan_phase = ?2, scan_issue_kind = CASE WHEN ?2 IN ('blocked', 'partial') THEN 'permission_denied' ELSE NULL END WHERE source_id = ?1",
                rusqlite::params![source_id, scan_phase],
            )
            .expect("set source scan phase");
    }

    struct DirectoryFixture {
        source_directory_id: i64,
        source_id: i64,
        parent_source_directory_id: Option<i64>,
        relative_path: String,
        presence_state: String,
        dir_scan_state: String,
        dir_scan_issue_kind: Option<String>,
    }

    fn insert_directory(connection: &Connection, fix: &DirectoryFixture) {
        let name = fix
            .relative_path
            .rsplit('/')
            .next()
            .unwrap_or(&fix.relative_path);
        let name_browse_sort_key = crate::browse_sort_key::compute_name_browse_sort_key(name);
        connection
            .execute(
                "INSERT INTO source_directories (
                     source_directory_id,
                     source_id,
                     parent_source_directory_id,
                     name,
                     name_browse_sort_key,
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, 0, ?8, ?9, 1, 1, 1)",
                rusqlite::params![
                    fix.source_directory_id,
                    fix.source_id,
                    fix.parent_source_directory_id,
                    name,
                    name_browse_sort_key,
                    fix.relative_path,
                    fix.presence_state,
                    fix.dir_scan_state,
                    fix.dir_scan_issue_kind,
                ],
            )
            .expect("insert source directory");
    }

    #[test]
    fn present_location_is_present() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage =
            super::classify_source_location_coverage(&connection, 1, "Music", Some("complete"))
                .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Present);
    }

    #[test]
    fn explicitly_missing_location_is_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "missing".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage =
            super::classify_source_location_coverage(&connection, 1, "Music", Some("complete"))
                .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Missing);
    }

    #[test]
    fn removed_location_is_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "removed".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage =
            super::classify_source_location_coverage(&connection, 1, "Music", Some("complete"))
                .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Missing);
    }

    #[test]
    fn absent_location_with_complete_parent_is_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/DeletedFolder",
            Some("complete"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Missing);
    }

    #[test]
    fn absent_location_under_blocked_parent_is_not_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        set_source_scan_phase(&connection, 1, "partial");
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Locked".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "blocked".to_string(),
                dir_scan_issue_kind: Some("permission_denied".to_string()),
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/Locked/DeletedFolder",
            Some("partial"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Blocked);
    }

    #[test]
    fn absent_location_under_failed_parent_is_failed() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Crashed".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "failed".to_string(),
                dir_scan_issue_kind: Some("unknown_io".to_string()),
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/Crashed/DeletedFolder",
            Some("partial"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Failed);
    }

    #[test]
    fn absent_location_under_pending_parent_is_pending() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Pending".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "pending".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/Pending/SubFolder",
            Some("scanning"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Pending);
    }

    #[test]
    fn absent_location_under_scanning_parent_is_scanning() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Scanning".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "scanning".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/Scanning/SubFolder",
            Some("scanning"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Scanning);
    }

    #[test]
    fn absent_location_under_missing_parent_is_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Gone".to_string(),
                presence_state: "missing".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/Gone/SubFolder",
            Some("complete"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Missing);
    }

    #[test]
    fn absent_location_with_no_directory_rows_under_partial_scan_is_unknown() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        set_source_scan_phase(&connection, 1, "partial");

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "UnknownPath",
            Some("partial"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Unknown);
    }

    #[test]
    fn absent_location_with_no_directory_rows_under_complete_scan_is_missing() {
        let connection = setup_connection();
        insert_source(&connection, 1);

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "UnknownPath",
            Some("complete"),
        )
        .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Missing);
    }

    #[test]
    fn blocked_location_is_blocked() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "blocked".to_string(),
                dir_scan_issue_kind: Some("permission_denied".to_string()),
            },
        );

        let coverage =
            super::classify_source_location_coverage(&connection, 1, "Music", Some("partial"))
                .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Blocked);
    }

    #[test]
    fn pending_location_is_pending() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "pending".to_string(),
                dir_scan_issue_kind: None,
            },
        );

        let coverage =
            super::classify_source_location_coverage(&connection, 1, "Music", Some("scanning"))
                .expect("classify");

        assert_eq!(coverage, SourceLocationCoverage::Pending);
    }

    #[test]
    fn complete_parent_proves_absence_under_partial_scan() {
        let connection = setup_connection();
        insert_source(&connection, 1);
        set_source_scan_phase(&connection, 1, "partial");
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 10,
                source_id: 1,
                parent_source_directory_id: None,
                relative_path: "Music".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "complete".to_string(),
                dir_scan_issue_kind: None,
            },
        );
        insert_directory(
            &connection,
            &DirectoryFixture {
                source_directory_id: 11,
                source_id: 1,
                parent_source_directory_id: Some(10),
                relative_path: "Music/Locked".to_string(),
                presence_state: "present".to_string(),
                dir_scan_state: "blocked".to_string(),
                dir_scan_issue_kind: Some("permission_denied".to_string()),
            },
        );

        let coverage = super::classify_source_location_coverage(
            &connection,
            1,
            "Music/DeletedFolder",
            Some("partial"),
        )
        .expect("classify");

        assert_eq!(
            coverage,
            SourceLocationCoverage::Missing,
            "Music is complete so DeletedFolder is missing even under partial scan"
        );
    }

    #[test]
    fn aggregate_all_present() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Present,
                SourceLocationCoverage::Present,
            ]),
            AcceptedSourceLocationCoverage::AllPresent
        );
    }

    #[test]
    fn aggregate_all_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Missing,
            ]),
            AcceptedSourceLocationCoverage::AllMissing
        );
    }

    #[test]
    fn aggregate_present_plus_missing_is_mixed_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Present,
                SourceLocationCoverage::Missing,
            ]),
            AcceptedSourceLocationCoverage::MixedMissing
        );
    }

    #[test]
    fn aggregate_missing_plus_blocked_is_blocked_not_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Blocked,
            ]),
            AcceptedSourceLocationCoverage::Blocked
        );
    }

    #[test]
    fn aggregate_missing_plus_failed_is_failed_not_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Failed,
            ]),
            AcceptedSourceLocationCoverage::Failed
        );
    }

    #[test]
    fn aggregate_present_plus_blocked_is_blocked() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Present,
                SourceLocationCoverage::Blocked,
            ]),
            AcceptedSourceLocationCoverage::Blocked
        );
    }

    #[test]
    fn aggregate_blocked_plus_scanning_is_blocked() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Blocked,
                SourceLocationCoverage::Scanning,
            ]),
            AcceptedSourceLocationCoverage::Blocked
        );
    }

    #[test]
    fn aggregate_pending_plus_present_is_pending() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Pending,
                SourceLocationCoverage::Present,
            ]),
            AcceptedSourceLocationCoverage::Pending
        );
    }

    #[test]
    fn aggregate_missing_plus_scanning_is_mixed_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Scanning,
            ]),
            AcceptedSourceLocationCoverage::MixedMissing
        );
    }

    #[test]
    fn aggregate_missing_plus_pending_is_mixed_missing() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Pending,
            ]),
            AcceptedSourceLocationCoverage::MixedMissing
        );
    }

    #[test]
    fn aggregate_present_plus_missing_plus_blocked_is_blocked() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Present,
                SourceLocationCoverage::Missing,
                SourceLocationCoverage::Blocked,
            ]),
            AcceptedSourceLocationCoverage::Blocked
        );
    }

    #[test]
    fn aggregate_scanning_is_scanning() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[
                SourceLocationCoverage::Scanning,
                SourceLocationCoverage::Scanning,
            ]),
            AcceptedSourceLocationCoverage::Scanning
        );
    }

    #[test]
    fn aggregate_unknown_is_pending() {
        use super::AcceptedSourceLocationCoverage;
        use super::aggregate_source_location_coverages;

        assert_eq!(
            aggregate_source_location_coverages(&[SourceLocationCoverage::Unknown]),
            AcceptedSourceLocationCoverage::Pending
        );
    }
}
