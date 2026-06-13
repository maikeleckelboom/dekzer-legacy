pub(crate) mod source_access;
pub(crate) mod source_directories;
pub(crate) mod source_file_observations;
pub(crate) mod source_files;
pub(crate) mod source_locations;
pub(crate) mod source_locators;
pub(crate) mod source_records;
pub(crate) mod source_root_navigation_state;
pub(crate) mod source_state;

pub(crate) use source_access::{
    SourceAccessProbeResult, probe_source_access, source_access_issue_kind_from_io_error,
};
pub use source_directories::{
    EstablishRootChildDirectoryInput, SourceDirectoriesAuthorityTx, UpsertSourceDirectoryInput,
};
pub(crate) use source_file_observations::SourceFileObservationAuthorityTx;
pub use source_file_observations::{
    CommitAcceptedSourceFileObservationInput, CommitAcceptedSourceFileObservationMergePolicy,
    ContentHashEvidence,
};
pub use source_files::{RecordSourceFileObservationInput, SourceFilesAuthorityTx};
pub use source_locations::{
    DeleteSourceLocationInput, SourceLocationsAuthorityTx, UpsertSourceLocationInput,
    canonicalize_source_location_relative_path,
};
pub use source_locators::{
    SourceLocatorInput, SourceLocatorsAuthorityTx, UpsertSourceLocatorInput,
};
pub use source_records::{SourcesAuthorityTx, UpsertSourceInput, format_source_identity_key};
pub(crate) use source_root_navigation_state::{
    SourceRootNavigationStateAuthorityTx, SourceRootNavigationStateRecord,
    SourceRootNavigationWindowState, UpsertSourceRootNavigationStateInput,
    read_source_root_navigation_state,
};
pub use source_state::{
    SourceStateAuthorityTx, UpsertSourceScanStateInput, UpsertSourceStateInput,
};
