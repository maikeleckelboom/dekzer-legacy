use std::ops::Deref;

use rusqlite::{Connection, Savepoint, Transaction};

use crate::LibrarySqliteResult;

type WriteSideEffect = Box<dyn FnOnce() + 'static>;

#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct SavepointWriteSideEffects {
    rollback_cleanup: Vec<WriteSideEffect>,
}

#[allow(dead_code)]
impl SavepointWriteSideEffects {
    pub(crate) fn register_rollback_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + 'static,
    {
        self.rollback_cleanup.push(Box::new(cleanup));
    }

    fn drain_rollback_cleanup(self) -> Vec<WriteSideEffect> {
        self.rollback_cleanup
    }

    fn run_rollback_cleanup(&mut self) {
        for cleanup in self.rollback_cleanup.drain(..).rev() {
            cleanup();
        }
    }
}

pub(crate) struct AdmittedWrite<'conn> {
    tx: Option<Transaction<'conn>>,
    rollback_cleanup: Vec<WriteSideEffect>,
    post_commit: Vec<WriteSideEffect>,
}

impl<'conn> AdmittedWrite<'conn> {
    fn new(tx: Transaction<'conn>) -> Self {
        Self {
            tx: Some(tx),
            rollback_cleanup: Vec::new(),
            post_commit: Vec::new(),
        }
    }

    fn connection(&self) -> &Connection {
        self.tx
            .as_ref()
            .expect("admitted write transaction must remain active")
    }

    #[allow(dead_code)]
    pub(crate) fn with_savepoint<T>(
        &mut self,
        body: impl FnOnce(&Savepoint<'_>, &mut SavepointWriteSideEffects) -> LibrarySqliteResult<T>,
    ) -> LibrarySqliteResult<T> {
        let savepoint = self
            .tx
            .as_mut()
            .expect("admitted write transaction must remain active")
            .savepoint()?;
        let mut savepoint_side_effects = SavepointWriteSideEffects::default();
        let result = match body(&savepoint, &mut savepoint_side_effects) {
            Ok(result) => result,
            Err(error) => {
                drop(savepoint);
                savepoint_side_effects.run_rollback_cleanup();
                return Err(error);
            }
        };
        match savepoint.commit() {
            Ok(()) => {
                self.rollback_cleanup
                    .extend(savepoint_side_effects.drain_rollback_cleanup());
                Ok(result)
            }
            Err(error) => {
                savepoint_side_effects.run_rollback_cleanup();
                Err(error.into())
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn register_rollback_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + 'static,
    {
        self.rollback_cleanup.push(Box::new(cleanup));
    }

    pub(crate) fn register_post_commit<F>(&mut self, finalize: F)
    where
        F: FnOnce() + 'static,
    {
        self.post_commit.push(Box::new(finalize));
    }

    fn commit(mut self) -> LibrarySqliteResult<()> {
        if let Some(tx) = self.tx.take() {
            match tx.commit() {
                Ok(()) => self.run_post_commit(),
                Err(error) => {
                    self.run_rollback_cleanup();
                    return Err(error.into());
                }
            }
        }

        Ok(())
    }

    fn run_rollback_cleanup(&mut self) {
        self.post_commit.clear();

        for cleanup in self.rollback_cleanup.drain(..).rev() {
            cleanup();
        }
    }

    fn run_post_commit(&mut self) {
        self.rollback_cleanup.clear();

        for finalize in self.post_commit.drain(..) {
            finalize();
        }
    }
}

impl Deref for AdmittedWrite<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.connection()
    }
}

impl Drop for AdmittedWrite<'_> {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            drop(tx);
        }

        self.run_rollback_cleanup();
    }
}

pub(crate) fn admit_write<T>(
    connection: &mut Connection,
    body: impl for<'conn> FnOnce(&mut AdmittedWrite<'conn>) -> LibrarySqliteResult<T>,
) -> LibrarySqliteResult<T> {
    let tx = connection.transaction()?;
    let mut admitted = AdmittedWrite::new(tx);

    let result = body(&mut admitted);
    match result {
        Ok(result) => {
            admitted.commit()?;
            Ok(result)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::admit_write;
    use crate::{LibrarySqliteError, LibrarySqliteResult};
    use rusqlite::Connection;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn rollback_cleanup_runs_when_body_returns_error() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        let result: LibrarySqliteResult<()> = admit_write(&mut connection, |write| {
            write.register_rollback_cleanup(move || {
                cleanup_called_clone.store(true, Ordering::SeqCst);
            });
            Err(LibrarySqliteError::MalformedSchemaState(
                "force rollback".to_string(),
            ))
        });

        assert!(result.is_err());
        assert!(cleanup_called.load(Ordering::SeqCst));
    }

    #[test]
    fn commit_failure_runs_rollback_cleanup() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", [])
            .expect("create items table");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        let commit = admit_write(&mut connection, |write| {
            write
                .execute("INSERT INTO items (id) VALUES (1)", [])
                .expect("insert row inside admitted write");
            write.register_rollback_cleanup(move || {
                cleanup_called_clone.store(true, Ordering::SeqCst);
            });
            write
                .commit_hook(Some(|| true))
                .expect("install commit hook");
            Ok(())
        });
        connection
            .commit_hook(None::<fn() -> bool>)
            .expect("clear commit hook");

        assert!(commit.is_err());
        assert!(cleanup_called.load(Ordering::SeqCst));
    }

    #[test]
    fn successful_admitted_write_commits_and_runs_post_commit() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", [])
            .expect("create items table");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);
        let post_commit_called = Arc::new(AtomicBool::new(false));
        let post_commit_called_clone = Arc::clone(&post_commit_called);

        admit_write(&mut connection, |write| {
            write
                .execute("INSERT INTO items (id) VALUES (1)", [])
                .expect("insert row inside admitted write");
            write.register_rollback_cleanup(move || {
                cleanup_called_clone.store(true, Ordering::SeqCst);
            });
            write.register_post_commit(move || {
                post_commit_called_clone.store(true, Ordering::SeqCst);
            });
            Ok(())
        })
        .expect("commit admitted write");

        let row_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .expect("count committed rows");
        assert_eq!(row_count, 1);
        assert!(!cleanup_called.load(Ordering::SeqCst));
        assert!(post_commit_called.load(Ordering::SeqCst));
    }

    #[test]
    fn savepoint_rollback_cleanup_runs_when_body_returns_error() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", [])
            .expect("create items table");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        let result: LibrarySqliteResult<()> = admit_write(&mut connection, |write| {
            let _: () = write.with_savepoint(|savepoint, side_effects| {
                savepoint
                    .execute("INSERT INTO items (id) VALUES (1)", [])
                    .expect("insert item inside savepoint");
                side_effects.register_rollback_cleanup(move || {
                    cleanup_called_clone.store(true, Ordering::SeqCst);
                });
                Err(LibrarySqliteError::MalformedSchemaState(
                    "force savepoint rollback".to_string(),
                ))
            })?;
            Ok(())
        });

        assert!(result.is_err());
        assert!(cleanup_called.load(Ordering::SeqCst));
        let row_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .expect("count rolled back rows");
        assert_eq!(row_count, 0);
    }

    #[test]
    fn committed_savepoint_cleanup_is_promoted_to_outer_rollback_cleanup() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", [])
            .expect("create items table");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        let commit = admit_write(&mut connection, |write| {
            let _: () = write.with_savepoint(|savepoint, side_effects| {
                savepoint
                    .execute("INSERT INTO items (id) VALUES (1)", [])
                    .expect("insert item inside savepoint");
                side_effects.register_rollback_cleanup(move || {
                    cleanup_called_clone.store(true, Ordering::SeqCst);
                });
                Ok(())
            })?;
            write
                .commit_hook(Some(|| true))
                .expect("install commit failure hook");
            Ok(())
        });
        connection
            .commit_hook(None::<fn() -> bool>)
            .expect("clear commit hook");

        assert!(commit.is_err());
        assert!(cleanup_called.load(Ordering::SeqCst));
    }

    #[test]
    fn committed_savepoint_cleanup_does_not_run_after_successful_outer_commit() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute("CREATE TABLE items (id INTEGER PRIMARY KEY)", [])
            .expect("create items table");
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = Arc::clone(&cleanup_called);

        admit_write(&mut connection, |write| {
            let _: () = write.with_savepoint(|savepoint, side_effects| {
                savepoint
                    .execute("INSERT INTO items (id) VALUES (1)", [])
                    .expect("insert item inside savepoint");
                side_effects.register_rollback_cleanup(move || {
                    cleanup_called_clone.store(true, Ordering::SeqCst);
                });
                Ok(())
            })?;
            Ok(())
        })
        .expect("commit outer transaction");

        assert!(!cleanup_called.load(Ordering::SeqCst));
        let row_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .expect("count committed rows");
        assert_eq!(row_count, 1);
    }
}
