use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::baseline;
use super::introspection::{
    self, ComparedSchemaObject, ForeignKeyIntrospection, IgnoredSchemaObject,
    IndexColumnIntrospection, IndexIntrospection, IndexOrigin, SchemaObjectKind,
    enumerate_compared_objects, enumerate_ignored_internal_objects, read_index_columns,
    read_partial_index_predicate_fingerprint, read_table_columns, read_table_foreign_keys,
    read_table_indexes, read_trigger_body_fingerprint,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchemaSnapshot {
    pub(crate) compared_tables: BTreeMap<String, TableSnapshot>,
    pub(crate) compared_indexes: BTreeMap<String, IndexSnapshot>,
    pub(crate) compared_triggers: BTreeMap<String, TriggerSnapshot>,
    pub(crate) ignored_objects: Vec<IgnoredSchemaObject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TableSnapshot {
    pub(crate) columns: Vec<ColumnSnapshot>,
    pub(crate) unique_constraints: Vec<UniqueConstraintSnapshot>,
    pub(crate) foreign_keys: Vec<ForeignKeySnapshot>,
    pub(crate) residual_sql_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ColumnSnapshot {
    pub(crate) name: String,
    pub(crate) declared_type: String,
    pub(crate) not_null: bool,
    pub(crate) default_sql: Option<String>,
    pub(crate) pk_ordinal: i64,
    pub(crate) hidden_classification: ColumnVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColumnVisibility {
    Visible,
    Hidden,
    VirtualGenerated,
    StoredGenerated,
    Other(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UniqueConstraintSnapshot {
    pub(crate) columns: Vec<IndexedColumnSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ForeignKeySnapshot {
    pub(crate) parent_table: String,
    pub(crate) on_update: String,
    pub(crate) on_delete: String,
    pub(crate) match_name: String,
    pub(crate) column_mapping: Vec<ForeignKeyColumnSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ForeignKeyColumnSnapshot {
    pub(crate) from: String,
    pub(crate) to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexSnapshot {
    pub(crate) parent_table: String,
    pub(crate) unique: bool,
    pub(crate) partial: bool,
    pub(crate) columns: Vec<IndexedColumnSnapshot>,
    pub(crate) residual_sql_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct IndexedColumnSnapshot {
    pub(crate) name: Option<String>,
    pub(crate) descending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriggerSnapshot {
    pub(crate) parent_table: String,
    pub(crate) residual_sql_fingerprint: Option<String>,
}

pub(crate) fn canonical_snapshot() -> LibrarySqliteResult<SchemaSnapshot> {
    let mut connection = Connection::open_in_memory()?;
    baseline::install_canonical_baseline(&mut connection, 0)?;
    live_snapshot(&connection)
}

pub(crate) fn live_snapshot(connection: &Connection) -> LibrarySqliteResult<SchemaSnapshot> {
    let compared_objects = enumerate_compared_objects(connection)?;
    let ignored_objects = enumerate_ignored_internal_objects(connection)?;

    let compared_table_names = compared_objects
        .iter()
        .filter(|object| object.kind == SchemaObjectKind::Table)
        .map(|object| object.name.clone())
        .collect::<Vec<_>>();

    let mut table_index_cache = BTreeMap::<String, Vec<IndexIntrospection>>::new();
    let mut compared_tables = BTreeMap::new();
    for table_name in &compared_table_names {
        let table_indexes = read_table_indexes(connection, table_name)?;
        let table_snapshot = TableSnapshot {
            columns: read_table_columns(connection, table_name)?
                .into_iter()
                .map(|column| ColumnSnapshot {
                    name: column.name,
                    declared_type: column.declared_type,
                    not_null: column.not_null,
                    default_sql: column.default_sql,
                    pk_ordinal: column.pk_ordinal,
                    hidden_classification: ColumnVisibility::from_hidden_code(column.hidden),
                })
                .collect(),
            unique_constraints: build_unique_constraints(connection, &table_indexes)?,
            foreign_keys: build_foreign_keys(read_table_foreign_keys(connection, table_name)?),
            residual_sql_fingerprint: None,
        };
        table_index_cache.insert(table_name.clone(), table_indexes);
        compared_tables.insert(table_name.clone(), table_snapshot);
    }

    let mut compared_indexes = BTreeMap::new();
    for object in compared_objects
        .iter()
        .filter(|object| object.kind == SchemaObjectKind::Index)
    {
        compared_indexes.insert(
            object.name.clone(),
            build_index_snapshot(connection, object, &table_index_cache)?,
        );
    }

    let mut compared_triggers = BTreeMap::new();
    for object in compared_objects
        .iter()
        .filter(|object| object.kind == SchemaObjectKind::Trigger)
    {
        let residual_sql_fingerprint = read_trigger_body_fingerprint(connection, &object.name)?
            .ok_or_else(|| {
                introspection::missing_schema_object_message(object.kind, &object.name)
            })?;
        compared_triggers.insert(
            object.name.clone(),
            TriggerSnapshot {
                parent_table: object.parent_table.clone(),
                residual_sql_fingerprint: Some(residual_sql_fingerprint),
            },
        );
    }

    Ok(SchemaSnapshot {
        compared_tables,
        compared_indexes,
        compared_triggers,
        ignored_objects,
    })
}

impl ColumnVisibility {
    fn from_hidden_code(hidden: i64) -> Self {
        match hidden {
            0 => Self::Visible,
            1 => Self::Hidden,
            2 => Self::VirtualGenerated,
            3 => Self::StoredGenerated,
            other => Self::Other(other),
        }
    }
}

fn build_unique_constraints(
    connection: &Connection,
    table_indexes: &[IndexIntrospection],
) -> LibrarySqliteResult<Vec<UniqueConstraintSnapshot>> {
    let mut unique_constraints = Vec::new();
    for index in table_indexes {
        if index.origin == IndexOrigin::UniqueConstraint {
            unique_constraints.push(UniqueConstraintSnapshot {
                columns: build_indexed_columns(read_index_columns(connection, &index.name)?),
            });
        }
    }
    unique_constraints.sort();
    Ok(unique_constraints)
}

fn build_foreign_keys(foreign_keys: Vec<ForeignKeyIntrospection>) -> Vec<ForeignKeySnapshot> {
    let mut by_id = BTreeMap::<i64, Vec<ForeignKeyIntrospection>>::new();
    for foreign_key in foreign_keys {
        by_id.entry(foreign_key.id).or_default().push(foreign_key);
    }

    let mut snapshots = by_id
        .into_values()
        .map(|mut grouped_rows| {
            grouped_rows.sort_by_key(|row| row.seq);
            let first = grouped_rows
                .first()
                .expect("foreign_key_list groups always contain at least one row");
            ForeignKeySnapshot {
                parent_table: first.parent_table.clone(),
                on_update: first.on_update.clone(),
                on_delete: first.on_delete.clone(),
                match_name: first.match_name.clone(),
                column_mapping: grouped_rows
                    .into_iter()
                    .map(|row| ForeignKeyColumnSnapshot {
                        from: row.from,
                        to: row.to,
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    snapshots.sort();
    snapshots
}

fn build_index_snapshot(
    connection: &Connection,
    object: &ComparedSchemaObject,
    table_index_cache: &BTreeMap<String, Vec<IndexIntrospection>>,
) -> LibrarySqliteResult<IndexSnapshot> {
    let table_indexes = table_index_cache.get(&object.parent_table).ok_or_else(|| {
        LibrarySqliteError::MalformedSchemaState(format!(
            "missing cached index metadata for compared table {}",
            object.parent_table
        ))
    })?;
    let index = table_indexes
        .iter()
        .find(|index| index.name == object.name)
        .ok_or_else(|| introspection::missing_schema_object_message(object.kind, &object.name))?;

    Ok(IndexSnapshot {
        parent_table: object.parent_table.clone(),
        unique: index.unique,
        partial: index.partial,
        columns: build_indexed_columns(read_index_columns(connection, &object.name)?),
        residual_sql_fingerprint: if index.partial {
            read_partial_index_predicate_fingerprint(connection, &object.name)?
        } else {
            None
        },
    })
}

fn build_indexed_columns(columns: Vec<IndexColumnIntrospection>) -> Vec<IndexedColumnSnapshot> {
    columns
        .into_iter()
        .map(|column| IndexedColumnSnapshot {
            name: column.name,
            descending: column.descending,
        })
        .collect()
}
