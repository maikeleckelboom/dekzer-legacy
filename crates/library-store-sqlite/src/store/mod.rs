use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use crate::LibrarySqliteResult;
use crate::authority::ingest::DiscoveryTx;
use crate::authority::roots::SourceLifecycleTx;
use crate::authority::write_lane::{AdmittedWrite, admit_write};
use crate::work_control::SourceAdmissionGate;

mod artifacts;
mod attachment_identity;
mod attachment_identity_reads;
mod bootstrap;
mod contents_reads;
mod context;
mod discovery;
mod library_asset_preparation_detail_reads;
mod library_asset_waveform_reads;
mod library_browser_reads;
mod literal_hierarchy_reads;
mod navigation_reads;
mod observed_file_facts_reads;
mod playlists;
mod primary_media_promotion;
mod projections;
mod promotion;
mod revisions;
mod source_file_hash;
mod source_file_media_probe;
mod source_lifecycle_reads;
mod sources;
mod work_items;

pub use attachment_identity::MaterializeAttachmentsForSourceResult;
pub use context::{
    DurableStoreBootstrapStatus, DurableStoreSchemaCompatibility,
    DurableStoreSchemaCompatibilityState, LibraryStoreContext, SqliteDurableStoreAppOwnedState,
    StoreEnvironment, durable_store_path,
};
pub use discovery::{
    RootScanHierarchyObservationReason, RootScanMaterializationResult, RootScanObservation,
};
pub use primary_media_promotion::{
    PromotePrimaryMediaForSourceResult, effective_primary_media_promotion_limit,
};
pub use revisions::{MaintainedReadModelRevision, MaintainedReadModelScope};
pub use source_file_hash::{
    HashSourceFileBlake3BatchInput, HashSourceFileBlake3BatchOutcome,
    HashSourceFileBlake3BatchOutcomeStatus, HashSourceFileBlake3BatchResult,
    ReadSourceFileBlake3HashCandidatesInput, SOURCE_FILE_BLAKE3_ALGORITHM,
    SourceFileBlake3HashAdmissionScope, SourceFileBlake3HashCandidate,
    SourceFileBlake3HashCandidateReason, SourceFileBlake3HashFailure,
    SourceFileBlake3HashSkipReason, effective_hash_batch_limit,
};
pub use source_file_media_probe::{
    ProbeSourceFileMediaBatchInput, ProbeSourceFileMediaBatchOutcome,
    ProbeSourceFileMediaBatchOutcomeStatus, ProbeSourceFileMediaBatchResult,
    ReadSourceFileMediaProbeCandidatesInput, SourceFileMediaProbeAdmissionScope,
    SourceFileMediaProbeCandidate, SourceFileMediaProbeCandidateReason, SourceFileMediaProbeFacts,
    SourceFileMediaProbeFailure, SourceFileMediaProbeSkipReason, effective_media_probe_batch_limit,
};
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
