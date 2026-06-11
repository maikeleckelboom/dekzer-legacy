use rusqlite::{Connection, params};

use crate::{LibrarySqliteError, LibrarySqliteResult};

use super::baseline::CANONICAL_BASELINE_GENERATION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CapabilitySpecSeed {
    capability_kind: &'static str,
    display_name: &'static str,
    quality_aware: bool,
    default_profile_key: &'static str,
}

const CAPABILITY_SPEC_SEEDS: &[CapabilitySpecSeed] = &[
    CapabilitySpecSeed {
        capability_kind: "beatgrid",
        display_name: "Beatgrid",
        quality_aware: false,
        default_profile_key: "default",
    },
    CapabilitySpecSeed {
        capability_kind: "musical_key",
        display_name: "Musical Key",
        quality_aware: false,
        default_profile_key: "default",
    },
    CapabilitySpecSeed {
        capability_kind: "stems",
        display_name: "Stems",
        quality_aware: true,
        default_profile_key: "default",
    },
    CapabilitySpecSeed {
        capability_kind: "tempo",
        display_name: "Tempo",
        quality_aware: false,
        default_profile_key: "default",
    },
    CapabilitySpecSeed {
        capability_kind: "waveform",
        display_name: "Waveform",
        quality_aware: true,
        default_profile_key: "default",
    },
];

pub(crate) fn install_seed_rows(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    install_library_metadata_row(connection, created_at_ms)?;
    install_search_filter_index_metadata_row(connection, created_at_ms)?;
    install_capability_spec_rows(connection, created_at_ms)?;
    Ok(())
}

pub(crate) fn validate_seed_rows(connection: &Connection) -> LibrarySqliteResult<()> {
    validate_library_metadata_row(connection)?;
    validate_search_filter_index_metadata_row(connection)?;
    validate_capability_spec_rows(connection)?;
    validate_capability_dependency_rows(connection)?;
    Ok(())
}

fn install_library_metadata_row(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO LibraryMetadata (
             library_id,
             schema_generation,
             created_at
         )
         VALUES (1, ?1, ?2)",
        params![CANONICAL_BASELINE_GENERATION, created_at_ms],
    )?;
    Ok(())
}

fn validate_library_metadata_row(connection: &Connection) -> LibrarySqliteResult<()> {
    let found = connection
        .prepare(
            "SELECT library_id, schema_generation, created_at
             FROM LibraryMetadata
             ORDER BY library_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    match found.as_slice() {
        [(1, schema_generation, _created_at)]
            if schema_generation == CANONICAL_BASELINE_GENERATION =>
        {
            Ok(())
        }
        _ => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "LibraryMetadata durable singleton row is malformed: found {found:?}"
        ))),
    }
}

fn install_search_filter_index_metadata_row(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    connection.execute(
        "INSERT INTO search_filter_index_metadata (
             search_filter_index_id,
             indexer_version,
             generation,
             state,
             updated_at
         )
         VALUES (1, 'search_filter_v0', 0, 'ready', ?1)",
        [created_at_ms],
    )?;
    Ok(())
}

fn validate_search_filter_index_metadata_row(connection: &Connection) -> LibrarySqliteResult<()> {
    let found = connection
        .prepare(
            "SELECT search_filter_index_id, indexer_version, generation, state
             FROM search_filter_index_metadata
             ORDER BY search_filter_index_id",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    match found.as_slice() {
        [(1, indexer_version, generation, state)]
            if indexer_version == "search_filter_v0"
                && *generation >= 0
                && matches!(state.as_str(), "ready" | "rebuilding" | "partial") =>
        {
            Ok(())
        }
        _ => Err(LibrarySqliteError::MalformedSchemaState(format!(
            "search_filter_index_metadata durable singleton row is malformed: found {found:?}"
        ))),
    }
}

fn install_capability_spec_rows(
    connection: &Connection,
    created_at_ms: i64,
) -> LibrarySqliteResult<()> {
    for seed in CAPABILITY_SPEC_SEEDS {
        connection.execute(
            "INSERT INTO CapabilitySpecs (
                 capability_kind,
                 display_name,
                 quality_aware,
                 default_profile_key,
                 created_at,
                 updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                seed.capability_kind,
                seed.display_name,
                if seed.quality_aware { 1 } else { 0 },
                seed.default_profile_key,
                created_at_ms,
            ],
        )?;
    }
    Ok(())
}

fn validate_capability_spec_rows(connection: &Connection) -> LibrarySqliteResult<()> {
    let mut stmt = connection.prepare(
        "SELECT capability_kind, display_name, quality_aware, default_profile_key
         FROM CapabilitySpecs
         ORDER BY capability_kind",
    )?;
    let found = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let expected = CAPABILITY_SPEC_SEEDS
        .iter()
        .map(|seed| {
            (
                seed.capability_kind.to_string(),
                seed.display_name.to_string(),
                seed.quality_aware,
                seed.default_profile_key.to_string(),
            )
        })
        .collect::<Vec<_>>();
    if found == expected {
        Ok(())
    } else {
        Err(LibrarySqliteError::MalformedSchemaState(format!(
            "CapabilitySpecs durable seed rows are malformed: found {found:?}"
        )))
    }
}

fn validate_capability_dependency_rows(connection: &Connection) -> LibrarySqliteResult<()> {
    let dependency_row_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM CapabilityDependencies", [], |row| {
            row.get(0)
        })?;
    if dependency_row_count == 0 {
        Ok(())
    } else {
        Err(LibrarySqliteError::MalformedSchemaState(format!(
            "CapabilityDependencies durable seed rows are malformed: expected 0 rows, found {dependency_row_count}"
        )))
    }
}
