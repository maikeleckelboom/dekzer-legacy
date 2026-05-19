use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;

use crate::authority::write_lane::admit_write;
use crate::publication;
use crate::schema::{BaselineSchemaBootstrapStatus, ensure_baseline_schema};
use crate::work_control::SourceAdmissionGate;
use crate::{DurableStoreOpenFailure, LibrarySqliteError, LibrarySqliteResult};

use super::{DurableStoreBootstrapStatus, SqliteDurableStore, SqliteDurableStoreAppOwnedState};

pub(super) fn open(path: impl AsRef<Path>) -> LibrarySqliteResult<SqliteDurableStore> {
    open_app_owned_state(SqliteDurableStoreAppOwnedState::from_durable_store_path(
        path,
    ))
}

pub(super) fn open_app_owned_state(
    app_owned_state: SqliteDurableStoreAppOwnedState,
) -> LibrarySqliteResult<SqliteDurableStore> {
    let mut connection = open_connection(app_owned_state.durable_store_path())?;
    ensure_baseline_schema(&mut connection)?;
    admit_write(&mut connection, |write| {
        publication::reseed_current_projection_state(write)
    })?;

    Ok(SqliteDurableStore {
        path: app_owned_state.durable_store_path().to_path_buf(),
        app_owned_state,
        source_admission_gate: Arc::new(SourceAdmissionGate::default()),
    })
}

pub(super) fn bootstrap_or_validate(
    path: impl AsRef<Path>,
) -> Result<DurableStoreBootstrapStatus, DurableStoreOpenFailure> {
    bootstrap_or_validate_app_owned_state(
        &SqliteDurableStoreAppOwnedState::from_durable_store_path(path),
    )
}

pub(super) fn bootstrap_or_validate_app_owned_state(
    app_owned_state: &SqliteDurableStoreAppOwnedState,
) -> Result<DurableStoreBootstrapStatus, DurableStoreOpenFailure> {
    let mut connection = open_connection(app_owned_state.durable_store_path())
        .map_err(DurableStoreOpenFailure::from)?;
    let status = ensure_baseline_schema(&mut connection).map_err(DurableStoreOpenFailure::from)?;
    admit_write(&mut connection, |write| {
        publication::reseed_current_projection_state(write)
    })
    .map_err(DurableStoreOpenFailure::from)?;

    Ok(match status {
        BaselineSchemaBootstrapStatus::InstalledCanonicalBaseline => {
            DurableStoreBootstrapStatus::InstalledCanonicalBaseline
        }
        BaselineSchemaBootstrapStatus::ExistingCanonicalBaseline => {
            DurableStoreBootstrapStatus::OpenedCanonicalStore
        }
    })
}

pub(super) fn open_connection(path: &Path) -> LibrarySqliteResult<Connection> {
    let connection = Connection::open(path)?;
    configure_connection(&connection)?;
    Ok(connection)
}

fn configure_connection(connection: &Connection) -> LibrarySqliteResult<()> {
    ensure_wal_mode(connection)?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

fn ensure_wal_mode(connection: &Connection) -> LibrarySqliteResult<()> {
    let journal_mode: String =
        connection.query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;

    if journal_mode.eq_ignore_ascii_case("wal") {
        Ok(())
    } else {
        Err(LibrarySqliteError::JournalMode(journal_mode))
    }
}
