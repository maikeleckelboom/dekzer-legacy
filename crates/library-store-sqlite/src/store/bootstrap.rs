use std::path::Path;
use std::sync::Arc;

use rusqlite::{Connection, OpenFlags};

use crate::authority::write_lane::admit_write;
use crate::publication;
use crate::schema::{
    BaselineSchemaBootstrapStatus, check_existing_canonical_baseline_schema, ensure_baseline_schema,
};
use crate::work_control::SourceAdmissionGate;
use crate::{DurableStoreOpenFailure, LibrarySqliteError, LibrarySqliteResult};

use super::{
    DurableStoreBootstrapStatus, DurableStoreSchemaCompatibility, SqliteDurableStore,
    SqliteDurableStoreAppOwnedState,
};

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

pub(super) fn schema_compatibility(path: impl AsRef<Path>) -> DurableStoreSchemaCompatibility {
    let path = path.as_ref();
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return DurableStoreSchemaCompatibility::missing(),
        Err(error) => {
            return DurableStoreSchemaCompatibility::unreadable(format!(
                "failed to inspect durable store path {path:?}: {error}"
            ));
        }
    }

    match validate_existing_store_schema(path) {
        Ok(()) => DurableStoreSchemaCompatibility::compatible(),
        Err(LibrarySqliteError::MalformedSchemaState(detail)) => {
            DurableStoreSchemaCompatibility::incompatible(format!(
                "database schema does not match the canonical substrate baseline: {detail}"
            ))
        }
        Err(error) => DurableStoreSchemaCompatibility::unreadable(error.to_string()),
    }
}

fn validate_existing_store_schema(path: &Path) -> LibrarySqliteResult<()> {
    let connection = open_existing_schema_status_connection(path)?;
    check_existing_canonical_baseline_schema(&connection)
}

fn open_existing_schema_status_connection(path: &Path) -> LibrarySqliteResult<Connection> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(connection)
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

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    use super::open_connection;

    #[test]
    fn configured_connections_wait_for_transient_write_locks() {
        let tempdir = TempDir::new().expect("create tempdir");
        let database_path = tempdir.path().join("library.sqlite3");
        let mut holder = open_connection(&database_path).expect("open holder connection");
        holder
            .execute(
                "CREATE TABLE busy_timeout_probe (id INTEGER PRIMARY KEY)",
                [],
            )
            .expect("create probe table");
        let tx = holder.transaction().expect("start holder transaction");
        tx.execute("INSERT INTO busy_timeout_probe (id) VALUES (1)", [])
            .expect("hold write lock");

        let database_path_for_writer = database_path.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let writer = open_connection(&database_path_for_writer).expect("open writer");
            started_tx.send(()).expect("signal writer ready");
            writer.execute("INSERT INTO busy_timeout_probe (id) VALUES (2)", [])
        });

        started_rx.recv().expect("writer started");
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            !writer.is_finished(),
            "configured busy_timeout should wait instead of failing immediately"
        );

        let released_at = Instant::now();
        tx.commit().expect("release write lock");
        let writer_result = writer.join().expect("writer thread");
        assert_eq!(writer_result.expect("writer insert waits then succeeds"), 1);
        assert!(
            released_at.elapsed() < Duration::from_secs(5),
            "writer should finish after lock release without exhausting busy_timeout"
        );
    }
}
