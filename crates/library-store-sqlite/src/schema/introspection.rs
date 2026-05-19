use rusqlite::{Connection, OptionalExtension, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SchemaObjectKind {
    Table,
    Index,
    Trigger,
}

impl SchemaObjectKind {
    pub(crate) fn sql_type(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Index => "index",
            Self::Trigger => "trigger",
        }
    }

    fn from_sql_type(sql_type: &str) -> Option<Self> {
        match sql_type {
            "table" => Some(Self::Table),
            "index" => Some(Self::Index),
            "trigger" => Some(Self::Trigger),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum IgnoredObjectReason {
    SqliteInternalPrefix,
    LibraryBrowserFtsShadowObject,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ComparedSchemaObject {
    pub(crate) kind: SchemaObjectKind,
    pub(crate) name: String,
    pub(crate) parent_table: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IgnoredSchemaObject {
    pub(crate) kind: SchemaObjectKind,
    pub(crate) name: String,
    pub(crate) reason: IgnoredObjectReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableColumnIntrospection {
    pub(crate) name: String,
    pub(crate) declared_type: String,
    pub(crate) not_null: bool,
    pub(crate) default_sql: Option<String>,
    pub(crate) pk_ordinal: i64,
    pub(crate) hidden: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKeyIntrospection {
    pub(crate) id: i64,
    pub(crate) seq: i64,
    pub(crate) parent_table: String,
    pub(crate) from: String,
    pub(crate) to: Option<String>,
    pub(crate) on_update: String,
    pub(crate) on_delete: String,
    pub(crate) match_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexOrigin {
    CreateStatement,
    UniqueConstraint,
    PrimaryKey,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexIntrospection {
    pub(crate) name: String,
    pub(crate) unique: bool,
    pub(crate) origin: IndexOrigin,
    pub(crate) partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexColumnIntrospection {
    pub(crate) name: Option<String>,
    pub(crate) descending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForeignKeyViolation {
    pub(crate) table_name: String,
    pub(crate) row_id: i64,
    pub(crate) parent_table: String,
    pub(crate) foreign_key_index: i64,
}

#[derive(Debug, Clone)]
struct RawSchemaObject {
    kind: SchemaObjectKind,
    name: String,
    parent_table: String,
}

enum ObjectDisposition {
    Compared,
    Ignored(IgnoredObjectReason),
}

pub(crate) fn enumerate_compared_objects(
    connection: &Connection,
) -> LibrarySqliteResult<Vec<ComparedSchemaObject>> {
    let mut compared = Vec::new();
    for object in raw_schema_objects(connection)? {
        if matches!(schema_object_policy(&object), ObjectDisposition::Compared) {
            compared.push(ComparedSchemaObject {
                kind: object.kind,
                name: object.name,
                parent_table: object.parent_table,
            });
        }
    }
    compared.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.name.cmp(&right.name)));
    Ok(compared)
}

pub(crate) fn enumerate_ignored_internal_objects(
    connection: &Connection,
) -> LibrarySqliteResult<Vec<IgnoredSchemaObject>> {
    let mut ignored = Vec::new();
    for object in raw_schema_objects(connection)? {
        if let ObjectDisposition::Ignored(reason) = schema_object_policy(&object) {
            ignored.push(IgnoredSchemaObject {
                kind: object.kind,
                name: object.name,
                reason,
            });
        }
    }
    ignored.sort();
    Ok(ignored)
}

pub(crate) fn read_table_columns(
    connection: &Connection,
    table_name: &str,
) -> LibrarySqliteResult<Vec<TableColumnIntrospection>> {
    let mut stmt = connection.prepare(
        "SELECT name, \"type\", \"notnull\", dflt_value, pk, hidden
         FROM pragma_table_xinfo(?1)
         ORDER BY cid",
    )?;
    let rows = stmt.query_map(params![table_name], |row| {
        Ok(TableColumnIntrospection {
            name: row.get(0)?,
            declared_type: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            not_null: row.get::<_, i64>(2)? != 0,
            default_sql: row.get(3)?,
            pk_ordinal: row.get(4)?,
            hidden: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub(crate) fn read_table_foreign_keys(
    connection: &Connection,
    table_name: &str,
) -> LibrarySqliteResult<Vec<ForeignKeyIntrospection>> {
    let mut stmt = connection.prepare(
        "SELECT id, seq, \"table\", \"from\", \"to\", on_update, on_delete, \"match\"
         FROM pragma_foreign_key_list(?1)
         ORDER BY id, seq",
    )?;
    let rows = stmt.query_map(params![table_name], |row| {
        Ok(ForeignKeyIntrospection {
            id: row.get(0)?,
            seq: row.get(1)?,
            parent_table: row.get(2)?,
            from: row.get(3)?,
            to: row.get(4)?,
            on_update: row.get(5)?,
            on_delete: row.get(6)?,
            match_name: row.get(7)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub(crate) fn read_table_indexes(
    connection: &Connection,
    table_name: &str,
) -> LibrarySqliteResult<Vec<IndexIntrospection>> {
    let mut stmt = connection.prepare(
        "SELECT name, \"unique\", origin, partial
         FROM pragma_index_list(?1)
         ORDER BY seq",
    )?;
    let rows = stmt.query_map(params![table_name], |row| {
        let origin = match row.get::<_, String>(2)?.as_str() {
            "c" => IndexOrigin::CreateStatement,
            "u" => IndexOrigin::UniqueConstraint,
            "pk" => IndexOrigin::PrimaryKey,
            _ => IndexOrigin::Other,
        };
        Ok(IndexIntrospection {
            name: row.get(0)?,
            unique: row.get::<_, i64>(1)? != 0,
            origin,
            partial: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub(crate) fn read_index_columns(
    connection: &Connection,
    index_name: &str,
) -> LibrarySqliteResult<Vec<IndexColumnIntrospection>> {
    let mut stmt = connection.prepare(
        "SELECT name, \"desc\"
         FROM pragma_index_xinfo(?1)
         WHERE \"key\" = 1
         ORDER BY seqno",
    )?;
    let rows = stmt.query_map(params![index_name], |row| {
        Ok(IndexColumnIntrospection {
            name: row.get(0)?,
            descending: row.get::<_, i64>(1)? != 0,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub(crate) fn read_schema_sql(
    connection: &Connection,
    object_kind: SchemaObjectKind,
    object_name: &str,
) -> LibrarySqliteResult<Option<String>> {
    connection
        .query_row(
            "SELECT sql
             FROM sqlite_master
             WHERE type = ?1 AND name = ?2",
            params![object_kind.sql_type(), object_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn read_normalized_schema_sql(
    connection: &Connection,
    object_kind: SchemaObjectKind,
    object_name: &str,
) -> LibrarySqliteResult<Option<String>> {
    read_schema_sql(connection, object_kind, object_name)
        .map(|sql| sql.map(|value| normalize_sql(&value)))
}

pub(crate) fn read_partial_index_predicate_fingerprint(
    connection: &Connection,
    index_name: &str,
) -> LibrarySqliteResult<Option<String>> {
    read_schema_sql(connection, SchemaObjectKind::Index, index_name)
        .map(|sql| sql.map(|value| fingerprint_from_marker(&value, " WHERE ")))
}

pub(crate) fn read_trigger_body_fingerprint(
    connection: &Connection,
    trigger_name: &str,
) -> LibrarySqliteResult<Option<String>> {
    read_schema_sql(connection, SchemaObjectKind::Trigger, trigger_name)
        .map(|sql| sql.map(|value| fingerprint_from_marker(&value, "BEGIN ")))
}

pub(crate) fn read_auto_vacuum_mode(connection: &Connection) -> LibrarySqliteResult<i64> {
    connection
        .query_row("PRAGMA auto_vacuum;", [], |row| row.get(0))
        .map_err(Into::into)
}

pub(crate) fn run_foreign_key_check(
    connection: &Connection,
) -> LibrarySqliteResult<Option<ForeignKeyViolation>> {
    let mut stmt = connection.prepare("PRAGMA foreign_key_check")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(ForeignKeyViolation {
            table_name: row.get(0)?,
            row_id: row.get(1)?,
            parent_table: row.get(2)?,
            foreign_key_index: row.get(3)?,
        }));
    }
    Ok(None)
}

pub(crate) fn missing_schema_object_message(
    object_kind: SchemaObjectKind,
    object_name: &str,
) -> LibrarySqliteError {
    LibrarySqliteError::MalformedSchemaState(format!(
        "baseline schema is missing compared {} {}",
        object_kind.sql_type(),
        object_name
    ))
}

fn raw_schema_objects(connection: &Connection) -> LibrarySqliteResult<Vec<RawSchemaObject>> {
    let mut stmt = connection.prepare(
        "SELECT type, name, tbl_name
         FROM sqlite_master
         WHERE type IN ('table', 'index', 'trigger')
           AND name IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        let sql_type: String = row.get(0)?;
        let kind = SchemaObjectKind::from_sql_type(&sql_type).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unsupported sqlite_master object type {sql_type:?}"),
                )),
            )
        })?;
        Ok(RawSchemaObject {
            kind,
            name: row.get(1)?,
            parent_table: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn schema_object_policy(object: &RawSchemaObject) -> ObjectDisposition {
    if object.name.starts_with("sqlite_") {
        return ObjectDisposition::Ignored(IgnoredObjectReason::SqliteInternalPrefix);
    }
    if object.name.starts_with("LibraryBrowserRows_fts_") {
        return ObjectDisposition::Ignored(IgnoredObjectReason::LibraryBrowserFtsShadowObject);
    }
    ObjectDisposition::Compared
}

fn fingerprint_from_marker(sql: &str, marker: &str) -> String {
    let normalized = normalize_sql(sql);
    let uppercase = normalized.to_uppercase();
    let marker_uppercase = marker.to_uppercase();
    if let Some(index) = uppercase.find(&marker_uppercase) {
        normalized[index..].to_string()
    } else {
        normalized
    }
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}
