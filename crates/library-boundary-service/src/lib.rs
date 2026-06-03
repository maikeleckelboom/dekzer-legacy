#![deny(unsafe_code)]

mod service;
mod session_events;
mod snapshot_read_protocol;
mod source_file_hash_protocol;
mod source_maintenance;
mod storage_environment;
mod track_identity_decisions;

pub use library_boundary_protocol::{ProtocolError, ProtocolResult};
pub use library_store_sqlite::{LibraryStoreContext, StoreEnvironment};
pub use service::LibraryBoundaryService;
pub use storage_environment::{
    LibraryStorageEnvironment, LibraryStorageEnvironmentError, LibraryStorageResetReport,
    reset_development_library_storage, resolve_library_storage_environment,
};

pub type LibraryBoundaryServiceResult<T> = ProtocolResult<T>;
