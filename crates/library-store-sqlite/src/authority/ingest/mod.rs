// Root discovery/scan ingest remains lower-layer substrate. The boundary service
// mounts it only through the narrow root-scan command path.
#![allow(dead_code)]

mod discovery;
mod filesystem_walk;
pub(crate) use discovery::{
    DirectoryEnumerationOutcome, DirectoryEnumerationOutcomeKind, DiscoveredFileCommitResult,
    DiscoveredFileInput, DiscoveredLocationInput, DiscoveredLocationKind, DiscoveryBatch,
    DiscoveryCommitResult,
};
pub(crate) use discovery::{
    DiscoveryChunkCommitResult, DiscoveryFinalizeResult, DiscoveryTx,
    build_discovered_locations_from_files,
};
pub(crate) use filesystem_walk::FilesystemWalkEntries;
