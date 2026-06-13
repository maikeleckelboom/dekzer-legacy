use rusqlite::{Connection, OptionalExtension, params};

use crate::LibrarySqliteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRow {
    pub navigation_row_id: i64,
    pub stable_key: String,
    pub parent_navigation_row_id: Option<i64>,
    pub family: Option<String>,
    pub row_kind: String,
    pub display_name: String,
    pub sibling_position: i64,
    pub selectable: bool,
    pub selector_kind: Option<String>,
    pub selector_payload: Option<String>,
    pub updated_at: i64,
    pub row_version: i64,
}

pub fn read_rows(
    connection: &Connection,
    parent_navigation_row_id: Option<i64>,
) -> LibrarySqliteResult<Vec<NavigationRow>> {
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
         WHERE (parent_navigation_row_id IS NULL AND ?1 IS NULL)
            OR parent_navigation_row_id = ?1
         ORDER BY CASE
                      WHEN parent_navigation_row_id IS NOT NULL THEN 0
                      WHEN family = 'Views' THEN 0
                      WHEN family = 'Sources' THEN 1
                      ELSE 2
                  END ASC,
                  sibling_position ASC,
                  navigation_row_id ASC",
    )?;
    let rows = statement
        .query_map(params![parent_navigation_row_id], navigation_row_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn load_row(
    connection: &Connection,
    navigation_row_id: i64,
) -> LibrarySqliteResult<Option<NavigationRow>> {
    connection
        .query_row(
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
             WHERE navigation_row_id = ?1",
            [navigation_row_id],
            navigation_row_from_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn load_row_by_stable_key(
    connection: &Connection,
    stable_key: &str,
) -> LibrarySqliteResult<Option<NavigationRow>> {
    connection
        .query_row(
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
             WHERE stable_key = ?1",
            [stable_key],
            navigation_row_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn navigation_row_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NavigationRow> {
    Ok(NavigationRow {
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
}
