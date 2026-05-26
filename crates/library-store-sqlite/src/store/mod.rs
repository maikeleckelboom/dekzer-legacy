use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use crate::LibrarySqliteResult;
use crate::authority::ingest::DiscoveryTx;
use crate::authority::roots::SourceLifecycleTx;
use crate::authority::write_lane::{AdmittedWrite, admit_write};
use crate::work_control::SourceAdmissionGate;

mod artifacts;
mod bootstrap;
mod contents_reads;
mod context;
mod discovery;
mod library_asset_preparation_detail_reads;
mod library_asset_waveform_reads;
mod library_browser_reads;
mod literal_hierarchy_reads;
mod navigation_reads;
mod playlists;
mod projections;
mod promotion;
mod revisions;
mod sources;
mod work_items;

pub use context::{
    DurableStoreBootstrapStatus, DurableStoreSchemaCompatibility,
    DurableStoreSchemaCompatibilityState, LibraryStoreContext, SqliteDurableStoreAppOwnedState,
    StoreEnvironment, durable_store_path,
};
pub use discovery::RootScanMaterializationResult;
pub use revisions::{MaintainedReadModelRevision, MaintainedReadModelScope};
pub use sources::{
    LocalRoot, LocalRootAvailability, ReadLocalRootsResult, RegisterLocalRootInput,
    UnregisterLocalRootInput, UnregisterLocalRootResult,
};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct SqliteDurableStore {
    path: PathBuf,
    app_owned_state: SqliteDurableStoreAppOwnedState,
    source_admission_gate: Arc<SourceAdmissionGate>,
}

impl SqliteDurableStore {
    pub fn open(path: impl AsRef<Path>) -> LibrarySqliteResult<Self> {
        bootstrap::open(path)
    }

    pub fn open_app_owned_state(
        app_owned_state: SqliteDurableStoreAppOwnedState,
    ) -> LibrarySqliteResult<Self> {
        bootstrap::open_app_owned_state(app_owned_state)
    }

    pub fn bootstrap_or_validate(
        path: impl AsRef<Path>,
    ) -> Result<DurableStoreBootstrapStatus, crate::DurableStoreOpenFailure> {
        bootstrap::bootstrap_or_validate(path)
    }

    pub fn bootstrap_or_validate_app_owned_state(
        app_owned_state: &SqliteDurableStoreAppOwnedState,
    ) -> Result<DurableStoreBootstrapStatus, crate::DurableStoreOpenFailure> {
        bootstrap::bootstrap_or_validate_app_owned_state(app_owned_state)
    }

    pub fn schema_compatibility(path: impl AsRef<Path>) -> DurableStoreSchemaCompatibility {
        bootstrap::schema_compatibility(path)
    }

    fn with_discovery_tx<T, F>(&self, f: F) -> LibrarySqliteResult<T>
    where
        F: for<'write, 'conn> FnOnce(&mut DiscoveryTx<'write, 'conn>) -> LibrarySqliteResult<T>,
    {
        self.with_write(|write| {
            let mut discovery_tx = DiscoveryTx::new(write);
            f(&mut discovery_tx)
        })
    }

    fn with_source_lifecycle_tx<T, F>(&self, f: F) -> LibrarySqliteResult<T>
    where
        F: for<'write, 'conn> FnOnce(
            &mut SourceLifecycleTx<'write, 'conn>,
        ) -> LibrarySqliteResult<T>,
    {
        self.with_write(|write| {
            let mut source_tx = SourceLifecycleTx::new(write);
            f(&mut source_tx)
        })
    }

    fn with_write<T, F>(&self, f: F) -> LibrarySqliteResult<T>
    where
        F: for<'conn> FnOnce(&mut AdmittedWrite<'conn>) -> LibrarySqliteResult<T>,
    {
        let mut connection = bootstrap::open_connection(&self.path)?;
        admit_write(&mut connection, f)
    }

    pub(crate) fn open_read_connection(&self) -> LibrarySqliteResult<Connection> {
        bootstrap::open_connection(&self.path)
    }

    pub fn app_owned_state(&self) -> &SqliteDurableStoreAppOwnedState {
        &self.app_owned_state
    }
}
