use std::collections::BTreeSet;

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::snapshot::{
    ColumnSnapshot, ForeignKeySnapshot, IndexSnapshot, IndexedColumnSnapshot, SchemaSnapshot,
    TableSnapshot, TriggerSnapshot, UniqueConstraintSnapshot,
};

pub(crate) fn compare_snapshots(
    canonical: &SchemaSnapshot,
    live: &SchemaSnapshot,
) -> LibrarySqliteResult<()> {
    compare_object_sets(
        "table",
        canonical.compared_tables.keys().map(String::as_str),
        live.compared_tables.keys().map(String::as_str),
    )?;
    compare_object_sets(
        "index",
        canonical.compared_indexes.keys().map(String::as_str),
        live.compared_indexes.keys().map(String::as_str),
    )?;
    compare_object_sets(
        "trigger",
        canonical.compared_triggers.keys().map(String::as_str),
        live.compared_triggers.keys().map(String::as_str),
    )?;

    for (table_name, canonical_table) in &canonical.compared_tables {
        let live_table = live
            .compared_tables
            .get(table_name)
            .expect("table object-set comparison already proved presence");
        compare_table(table_name, canonical_table, live_table)?;
    }

    for (index_name, canonical_index) in &canonical.compared_indexes {
        let live_index = live
            .compared_indexes
            .get(index_name)
            .expect("index object-set comparison already proved presence");
        compare_index(index_name, canonical_index, live_index)?;
    }

    for (trigger_name, canonical_trigger) in &canonical.compared_triggers {
        let live_trigger = live
            .compared_triggers
            .get(trigger_name)
            .expect("trigger object-set comparison already proved presence");
        compare_trigger(trigger_name, canonical_trigger, live_trigger)?;
    }

    Ok(())
}

fn compare_object_sets<'a>(
    object_kind: &str,
    canonical: impl Iterator<Item = &'a str>,
    live: impl Iterator<Item = &'a str>,
) -> LibrarySqliteResult<()> {
    let canonical = canonical.collect::<BTreeSet<_>>();
    let live = live.collect::<BTreeSet<_>>();
    let missing = canonical.difference(&live).copied().collect::<Vec<_>>();
    let unexpected = live.difference(&canonical).copied().collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        Ok(())
    } else {
        malformed(format!(
            "compared {object_kind} set mismatch: missing={missing:?}, unexpected={unexpected:?}"
        ))
    }
}

fn compare_table(
    table_name: &str,
    canonical: &TableSnapshot,
    live: &TableSnapshot,
) -> LibrarySqliteResult<()> {
    compare_columns(table_name, &canonical.columns, &live.columns)?;
    compare_unique_constraints(
        table_name,
        &canonical.unique_constraints,
        &live.unique_constraints,
    )?;
    compare_foreign_keys(table_name, &canonical.foreign_keys, &live.foreign_keys)?;
    compare_optional_fingerprint(
        &format!("table {table_name} residual SQL fingerprint"),
        canonical.residual_sql_fingerprint.as_deref(),
        live.residual_sql_fingerprint.as_deref(),
    )?;
    Ok(())
}

fn compare_columns(
    table_name: &str,
    canonical: &[ColumnSnapshot],
    live: &[ColumnSnapshot],
) -> LibrarySqliteResult<()> {
    if canonical.len() != live.len() {
        return malformed(format!(
            "table {table_name} column count mismatch: canonical={} live={}",
            canonical.len(),
            live.len()
        ));
    }

    for (index, (canonical_column, live_column)) in canonical.iter().zip(live.iter()).enumerate() {
        let ordinal = index + 1;
        if canonical_column.name != live_column.name {
            return malformed(format!(
                "table {table_name} column[{ordinal}] name mismatch: canonical={:?} live={:?}",
                canonical_column.name, live_column.name
            ));
        }
        if canonical_column.declared_type != live_column.declared_type {
            return malformed(format!(
                "table {table_name} column {} declared type mismatch: canonical={:?} live={:?}",
                canonical_column.name, canonical_column.declared_type, live_column.declared_type
            ));
        }
        if canonical_column.not_null != live_column.not_null {
            return malformed(format!(
                "table {table_name} column {} NOT NULL mismatch: canonical={} live={}",
                canonical_column.name, canonical_column.not_null, live_column.not_null
            ));
        }
        if canonical_column.default_sql != live_column.default_sql {
            return malformed(format!(
                "table {table_name} column {} default SQL mismatch: canonical={:?} live={:?}",
                canonical_column.name, canonical_column.default_sql, live_column.default_sql
            ));
        }
        if canonical_column.pk_ordinal != live_column.pk_ordinal {
            return malformed(format!(
                "table {table_name} column {} primary-key ordinal mismatch: canonical={} live={}",
                canonical_column.name, canonical_column.pk_ordinal, live_column.pk_ordinal
            ));
        }
        if canonical_column.hidden_classification != live_column.hidden_classification {
            return malformed(format!(
                "table {table_name} column {} hidden/generated classification mismatch: canonical={:?} live={:?}",
                canonical_column.name,
                canonical_column.hidden_classification,
                live_column.hidden_classification
            ));
        }
    }

    Ok(())
}

fn compare_unique_constraints(
    table_name: &str,
    canonical: &[UniqueConstraintSnapshot],
    live: &[UniqueConstraintSnapshot],
) -> LibrarySqliteResult<()> {
    if canonical.len() != live.len() {
        return malformed(format!(
            "table {table_name} unique-constraint count mismatch: canonical={} live={}",
            canonical.len(),
            live.len()
        ));
    }

    for (index, (canonical_constraint, live_constraint)) in
        canonical.iter().zip(live.iter()).enumerate()
    {
        if canonical_constraint.columns != live_constraint.columns {
            return malformed(format!(
                "table {table_name} unique constraint[{}] column mismatch: canonical={:?} live={:?}",
                index + 1,
                canonical_constraint.columns,
                live_constraint.columns
            ));
        }
    }

    Ok(())
}

fn compare_foreign_keys(
    table_name: &str,
    canonical: &[ForeignKeySnapshot],
    live: &[ForeignKeySnapshot],
) -> LibrarySqliteResult<()> {
    if canonical.len() != live.len() {
        return malformed(format!(
            "table {table_name} foreign-key count mismatch: canonical={} live={}",
            canonical.len(),
            live.len()
        ));
    }

    for (index, (canonical_foreign_key, live_foreign_key)) in
        canonical.iter().zip(live.iter()).enumerate()
    {
        let ordinal = index + 1;
        if canonical_foreign_key.parent_table != live_foreign_key.parent_table {
            return malformed(format!(
                "table {table_name} foreign key[{ordinal}] parent table mismatch: canonical={:?} live={:?}",
                canonical_foreign_key.parent_table, live_foreign_key.parent_table
            ));
        }
        if canonical_foreign_key.on_update != live_foreign_key.on_update {
            return malformed(format!(
                "table {table_name} foreign key[{ordinal}] ON UPDATE mismatch: canonical={:?} live={:?}",
                canonical_foreign_key.on_update, live_foreign_key.on_update
            ));
        }
        if canonical_foreign_key.on_delete != live_foreign_key.on_delete {
            return malformed(format!(
                "table {table_name} foreign key[{ordinal}] ON DELETE mismatch: canonical={:?} live={:?}",
                canonical_foreign_key.on_delete, live_foreign_key.on_delete
            ));
        }
        if canonical_foreign_key.match_name != live_foreign_key.match_name {
            return malformed(format!(
                "table {table_name} foreign key[{ordinal}] MATCH mismatch: canonical={:?} live={:?}",
                canonical_foreign_key.match_name, live_foreign_key.match_name
            ));
        }
        if canonical_foreign_key.column_mapping != live_foreign_key.column_mapping {
            return malformed(format!(
                "table {table_name} foreign key[{ordinal}] column mapping mismatch: canonical={:?} live={:?}",
                canonical_foreign_key.column_mapping, live_foreign_key.column_mapping
            ));
        }
    }

    Ok(())
}

fn compare_index(
    index_name: &str,
    canonical: &IndexSnapshot,
    live: &IndexSnapshot,
) -> LibrarySqliteResult<()> {
    if canonical.parent_table != live.parent_table {
        return malformed(format!(
            "index {index_name} parent table mismatch: canonical={:?} live={:?}",
            canonical.parent_table, live.parent_table
        ));
    }
    if canonical.unique != live.unique {
        return malformed(format!(
            "index {index_name} uniqueness mismatch: canonical={} live={}",
            canonical.unique, live.unique
        ));
    }
    if canonical.partial != live.partial {
        return malformed(format!(
            "index {index_name} partial-flag mismatch: canonical={} live={}",
            canonical.partial, live.partial
        ));
    }
    compare_index_columns(index_name, &canonical.columns, &live.columns)?;
    compare_optional_fingerprint(
        &format!("index {index_name} partial predicate fingerprint"),
        canonical.residual_sql_fingerprint.as_deref(),
        live.residual_sql_fingerprint.as_deref(),
    )
}

fn compare_index_columns(
    index_name: &str,
    canonical: &[IndexedColumnSnapshot],
    live: &[IndexedColumnSnapshot],
) -> LibrarySqliteResult<()> {
    if canonical.len() != live.len() {
        return malformed(format!(
            "index {index_name} column count mismatch: canonical={} live={}",
            canonical.len(),
            live.len()
        ));
    }

    for (index, (canonical_column, live_column)) in canonical.iter().zip(live.iter()).enumerate() {
        let ordinal = index + 1;
        if canonical_column.name != live_column.name {
            return malformed(format!(
                "index {index_name} column[{ordinal}] name mismatch: canonical={:?} live={:?}",
                canonical_column.name, live_column.name
            ));
        }
        if canonical_column.descending != live_column.descending {
            return malformed(format!(
                "index {index_name} column {:?} descending-order mismatch: canonical={} live={}",
                canonical_column.name, canonical_column.descending, live_column.descending
            ));
        }
    }

    Ok(())
}

fn compare_trigger(
    trigger_name: &str,
    canonical: &TriggerSnapshot,
    live: &TriggerSnapshot,
) -> LibrarySqliteResult<()> {
    if canonical.parent_table != live.parent_table {
        return malformed(format!(
            "trigger {trigger_name} parent table mismatch: canonical={:?} live={:?}",
            canonical.parent_table, live.parent_table
        ));
    }
    compare_optional_fingerprint(
        &format!("trigger {trigger_name} body fingerprint"),
        canonical.residual_sql_fingerprint.as_deref(),
        live.residual_sql_fingerprint.as_deref(),
    )
}

fn compare_optional_fingerprint(
    subject: &str,
    canonical: Option<&str>,
    live: Option<&str>,
) -> LibrarySqliteResult<()> {
    if canonical == live {
        Ok(())
    } else {
        malformed(format!(
            "{subject} mismatch: canonical={canonical:?} live={live:?}"
        ))
    }
}

fn malformed<T>(detail: String) -> LibrarySqliteResult<T> {
    Err(LibrarySqliteError::MalformedSchemaState(detail))
}
