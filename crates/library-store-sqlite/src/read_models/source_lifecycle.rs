use rusqlite::{Connection, OptionalExtension};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSourceLifecycle {
    pub source_id: i64,
    pub source_class: String,
    pub is_user_visible: bool,
    pub mount_status: String,
    pub access_state: String,
    pub access_issue_kind: Option<String>,
    pub scan_phase: String,
    pub scan_issue_kind: Option<String>,
    pub last_scan_started_at: Option<i64>,
    pub last_scan_finished_at: Option<i64>,
    pub last_successful_scan_at: Option<i64>,
    pub last_seen_at: Option<i64>,
    pub updated_at: i64,
}

pub fn read_source_lifecycle(
    connection: &Connection,
    source_id: i64,
) -> LibrarySqliteResult<Option<StoreSourceLifecycle>> {
    connection
        .query_row(
            "SELECT s.source_id,
                    s.source_class,
                    s.is_user_visible,
                    COALESCE(ss.mount_status, 'unknown'),
                    COALESCE(ss.access_state, 'unknown'),
                    ss.access_issue_kind,
                    COALESCE(sss.scan_phase, 'idle'),
                    sss.scan_issue_kind,
                    sss.last_scan_started_at,
                    sss.last_scan_finished_at,
                    sss.last_successful_scan_at,
                    ss.last_seen_at,
                    MAX(
                        s.updated_at,
                        COALESCE(ss.updated_at, s.updated_at),
                        COALESCE(sss.updated_at, s.updated_at)
                    )
             FROM sources s
             LEFT JOIN source_state ss
               ON ss.source_id = s.source_id
             LEFT JOIN source_scan_state sss
               ON sss.source_id = s.source_id
             WHERE s.source_id = ?1",
            [source_id],
            |row| {
                Ok(StoreSourceLifecycle {
                    source_id: row.get(0)?,
                    source_class: row.get(1)?,
                    is_user_visible: row.get::<_, i64>(2)? != 0,
                    mount_status: row.get(3)?,
                    access_state: row.get(4)?,
                    access_issue_kind: row.get(5)?,
                    scan_phase: row.get(6)?,
                    scan_issue_kind: row.get(7)?,
                    last_scan_started_at: row.get(8)?,
                    last_scan_finished_at: row.get(9)?,
                    last_successful_scan_at: row.get(10)?,
                    last_seen_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::read_source_lifecycle;
    use crate::schema::install_baseline_schema_for_test;
    use rusqlite::{Connection, params};

    fn test_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        install_baseline_schema_for_test(&mut connection).expect("install baseline");
        connection
    }

    fn delete_source_state(connection: &Connection, source_id: i64) {
        connection
            .execute("DELETE FROM source_state WHERE source_id = ?1", [source_id])
            .expect("delete source state");
    }

    fn delete_source_scan_state(connection: &Connection, source_id: i64) {
        connection
            .execute(
                "DELETE FROM source_scan_state WHERE source_id = ?1",
                [source_id],
            )
            .expect("delete source scan state");
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
                     is_user_visible,
                     created_at,
                     updated_at
                 )
                 VALUES (?1, 'external_mounted', 'device', ?2, 'Fixture', 1, 1, 10)",
                params![source_id, format!("source:{source_id}")],
            )
            .expect("insert source");
        connection
            .execute(
                "INSERT INTO source_state (
                     source_id,
                     mount_status,
                     mount_epoch,
                     access_state,
                     access_issue_kind,
                     access_checked_at,
                     effective_path,
                     last_seen_at,
                     updated_at
                 )
                 VALUES (?1, 'mounted', 1, 'accessible', NULL, 2, 'root', 7, 11)",
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
                     scan_issue_kind,
                     updated_at
                 )
                 VALUES (?1, 'complete', 3, 5, 5, NULL, 12)",
                [source_id],
            )
            .expect("insert source scan state");
    }

    #[test]
    fn read_source_lifecycle_returns_known_user_visible_source_when_unavailable() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_state
                 SET mount_status = 'unmounted',
                     access_state = 'blocked',
                     access_issue_kind = 'unavailable_mount',
                     updated_at = 20
                 WHERE source_id = 7",
                [],
            )
            .expect("mark source unavailable");

        let lifecycle = read_source_lifecycle(&connection, 7)
            .expect("read source lifecycle")
            .expect("source lifecycle exists");

        assert_eq!(lifecycle.source_id, 7);
        assert!(lifecycle.is_user_visible);
        assert_eq!(lifecycle.mount_status, "unmounted");
        assert_eq!(lifecycle.access_state, "blocked");
        assert_eq!(
            lifecycle.access_issue_kind.as_deref(),
            Some("unavailable_mount")
        );
        assert_eq!(lifecycle.updated_at, 20);
    }

    #[test]
    fn read_source_lifecycle_not_found_is_distinct_from_unavailable() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_state
                 SET mount_status = 'unmounted',
                     access_state = 'unknown',
                     access_issue_kind = NULL
                 WHERE source_id = 7",
                [],
            )
            .expect("mark unavailable");

        assert!(
            read_source_lifecycle(&connection, 999)
                .expect("read missing source")
                .is_none()
        );
        assert!(
            read_source_lifecycle(&connection, 7)
                .expect("read unavailable source")
                .is_some()
        );
    }

    #[test]
    fn read_source_lifecycle_defaults_missing_side_rows_without_not_found() {
        let connection = test_connection();
        insert_source(&connection, 7);
        delete_source_state(&connection, 7);
        delete_source_scan_state(&connection, 7);

        let lifecycle = read_source_lifecycle(&connection, 7)
            .expect("read source lifecycle")
            .expect("source row remains known");

        assert_eq!(lifecycle.source_id, 7);
        assert_eq!(lifecycle.mount_status, "unknown");
        assert_eq!(lifecycle.access_state, "unknown");
        assert_eq!(lifecycle.access_issue_kind, None);
        assert_eq!(lifecycle.scan_phase, "idle");
        assert_eq!(lifecycle.scan_issue_kind, None);
        assert_eq!(lifecycle.last_scan_started_at, None);
        assert_eq!(lifecycle.last_scan_finished_at, None);
        assert_eq!(lifecycle.last_successful_scan_at, None);
        assert_eq!(lifecycle.last_seen_at, None);
        assert_eq!(lifecycle.updated_at, 10);
    }

    #[test]
    fn read_source_lifecycle_defaults_only_missing_scan_side_row() {
        let connection = test_connection();
        insert_source(&connection, 7);
        delete_source_scan_state(&connection, 7);

        let lifecycle = read_source_lifecycle(&connection, 7)
            .expect("read source lifecycle")
            .expect("source row remains known");

        assert_eq!(lifecycle.mount_status, "mounted");
        assert_eq!(lifecycle.access_state, "accessible");
        assert_eq!(lifecycle.scan_phase, "idle");
        assert_eq!(lifecycle.updated_at, 11);
    }

    #[test]
    fn read_source_lifecycle_returns_only_substrate_scan_facts_without_children() {
        let connection = test_connection();
        insert_source(&connection, 7);
        connection
            .execute(
                "UPDATE source_scan_state
                 SET scan_phase = 'blocked',
                     scan_issue_kind = 'permission_denied',
                     last_scan_started_at = 30,
                     last_scan_finished_at = 40,
                     last_successful_scan_at = NULL,
                     updated_at = 41
                 WHERE source_id = 7",
                [],
            )
            .expect("set blocked scan state");
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
                      dir_scan_state,
                      dir_scan_updated_at,
                      created_at,
                      updated_at
                  )
                  VALUES (21, 7, NULL, 'Music', 'v1|tmtutstitc', 'Music', 'present', 0, 0, 'complete', 1, 1, 1)",
                [],
            )
            .expect("insert directory that lifecycle must not synthesize");

        let lifecycle = read_source_lifecycle(&connection, 7)
            .expect("read source lifecycle")
            .expect("source lifecycle exists");

        assert_eq!(lifecycle.scan_phase, "blocked");
        assert_eq!(
            lifecycle.scan_issue_kind.as_deref(),
            Some("permission_denied")
        );
        assert_eq!(lifecycle.last_scan_started_at, Some(30));
        assert_eq!(lifecycle.last_scan_finished_at, Some(40));
        assert_eq!(lifecycle.last_successful_scan_at, None);
    }
}
