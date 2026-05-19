#![deny(unsafe_code)]

mod service;
mod session_events;
mod snapshot_read_protocol;

pub use library_boundary_protocol::{ProtocolError, ProtocolResult};
pub use library_store_sqlite::{LibraryStoreContext, StoreEnvironment};
pub use service::LibraryBoundaryService;

pub type LibraryBoundaryServiceResult<T> = ProtocolResult<T>;
