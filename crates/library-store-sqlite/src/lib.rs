#![deny(unsafe_code)]

mod authority;
mod browse_media;
mod error;
mod publication;
mod read_models;
mod schema;
mod source_media;
mod store;
mod time;
mod work_control;

pub fn canonical_baseline_generation() -> &'static str {
    schema::canonical_baseline_generation()
}

pub use authority::library_asset::{
    AcceptedSourceSegmentInput, ApplyLibraryAssetMetadataCorrectionInput,
    MintOrReuseLibraryAssetResult, ReplaceAcceptedSourceSegmentSetInput,
    ReplaceLibraryAssetCapabilityInput, RetractLibraryAssetMetadataCorrectionInput,
};
pub use authority::playlists::{
    AppendLibraryAssetToPlaylistInput, CreatePlaylistInput, DeletePlaylistInput,
    MovePlaylistEntryInput, PlaylistsAuthorityTx, RemoveLibraryAssetFromPlaylistInput,
    RenamePlaylistInput,
};
pub use authority::promotion::{
    AcceptSegmentationPromotionInput, AcceptSegmentationPromotionResult,
    ComputeCapabilityPromotionInput, ComputeCapabilityPromotionResult, InspectSourcePromotionInput,
    InspectSourcePromotionResult, RebindSourcePromotionInput, RebindSourcePromotionResult,
    RebuildProjectionPromotionInput, RebuildProjectionPromotionResult,
    ResolveLibraryAssetPromotionInput, ResolveLibraryAssetPromotionResult,
};
pub use authority::sources::{
    CommitAcceptedSourceFactsInput, DeleteSourceLocationInput, RecordSourceFileObservationInput,
    SourceLocatorInput, UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocationInput,
    UpsertSourceLocatorInput, UpsertSourceScanStateInput, UpsertSourceStateInput,
    canonicalize_source_location_relative_path,
};
pub use authority::work::{
    BlockMachineWorkInput, ClaimMachineWorkBatchInput, ClaimedMachineWorkItem,
    CompleteMachineWorkInput, FailMachineWorkInput, FinishWorkRunInput, MachineWorkKey,
    MarkCapabilitiesStaleFromBasisInput, MarkCapabilitiesStaleResult,
    MarkCapabilityStaleFromDependencyInput, PrepAssignmentInput, PrepPolicyTargetInput,
    QueueAcceptSegmentationWorkInput, QueueComputeCapabilityWorkInput, QueueInspectSourceWorkInput,
    QueueMachineWorkInput, QueueMachineWorkResult, QueueRebindSourceWorkInput,
    QueueRebuildProjectionWorkInput, RecordArtifactInput, RecordFileStoreArtifactInput,
    RecordInlineArtifactInput, RecordedArtifact, ReplacePrepAssignmentsInput,
    ReplaceResolvedLibraryAssetPrepTargetsInput, ResolvedLibraryAssetPrepTargetInput,
    StaleCapabilityChange, StartWorkRunInput, StartedWorkRun, UpsertPrepPolicyInput,
};
pub use error::{
    CanonicalError, CanonicalErrorCode, CanonicalErrorCodeParseError, DurableStoreOpenFailure,
    DurableStoreOpenFailureKind, LibrarySqliteError, LibrarySqliteResult,
};
pub use read_models::library_asset_preparation_detail::{
    StoreLibraryAssetPreparationDetail, StoreLibraryAssetPreparationDetailGroup,
    StoreLibraryAssetPreparationDetailRow, StoreLibraryAssetPreparationProgress,
};
pub use read_models::library_asset_waveform_overview::{
    StoreLibraryAssetWaveformOverview, StoreLibraryAssetWaveformOverviewAmplitudeScale,
    StoreLibraryAssetWaveformOverviewBucket, StoreLibraryAssetWaveformOverviewCapabilityState,
};
pub use read_models::library_browser::{ScopedLibraryAssetBrowserRow, StoreLibraryBrowserWindow};
pub use read_models::literal_hierarchy::{
    StoreLiteralHierarchyEntryPoint, StoreLiteralHierarchyNode, StoreLiteralHierarchyWindow,
};
pub use read_models::navigation::NavigationRow;
pub use store::{
    DurableStoreBootstrapStatus, LibraryStoreContext, MaintainedReadModelRevision,
    MaintainedReadModelScope, ReadRegisteredLocalRootsResult, RegisterLocalRootInput,
    RegisteredLocalRoot, RootScanMaterializationResult, SqliteDurableStore,
    SqliteDurableStoreAppOwnedState, StoreEnvironment, durable_store_path,
};
