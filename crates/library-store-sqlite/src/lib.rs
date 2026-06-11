#![deny(unsafe_code)]

mod authority;
mod browse_media;
mod browse_sort_key;
mod error;
mod publication;
mod read_models;
mod schema;
mod source_media;
mod store;
mod time;
mod track_identity_evidence_predicates;
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
    CommitAcceptedSourceFactsInput, CommitAcceptedSourceFactsMergePolicy, ContentHashEvidence,
    DeleteSourceLocationInput, RecordSourceFileObservationInput, SourceLocatorInput,
    UpsertSourceDirectoryInput, UpsertSourceInput, UpsertSourceLocationInput,
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
pub use browse_media::SourceFileClassFilter;
pub use error::{
    CanonicalError, CanonicalErrorCode, CanonicalErrorCodeParseError, DurableStoreOpenFailure,
    DurableStoreOpenFailureKind, LibrarySqliteError, LibrarySqliteResult,
};
pub use read_models::attachment_identity::{
    StoreAttachmentIdentity, StoreAttachmentSourceFiles, StoreSourceAttachmentSummary,
    StoreSourceFileAttachmentLink, StoreSourceFileAttachmentLinkStatus,
    get_attachment_for_source_file, get_attachment_identity, get_source_attachment_summary,
    get_source_files_for_attachment, get_source_files_for_attachment_limited,
};
pub use read_models::contents::{
    StoreContentsFileClass, StoreContentsFileRow, StoreContentsReadPolicy, StoreContentsResult,
    StoreContentsRowOrigin, StoreContentsScope, StoreContentsScopeCoverage,
    StoreContentsScopeCoverageState, StoreContentsScopeDepth, StoreContentsState,
    StorePrimaryMediaKind, StorePrimaryMediaSummary,
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
    StoreLiteralHierarchyCoverage, StoreLiteralHierarchyCoverageState,
    StoreLiteralHierarchyEntryPoint, StoreLiteralHierarchyNode, StoreLiteralHierarchyWindow,
};
pub use read_models::navigation::NavigationRow;
pub use read_models::observed_file_facts::{
    StoreContentHashEvidence, StoreObservedFileFactStatus, StoreObservedFileFacts,
    read_observed_file_facts_for_source_file,
};
pub use read_models::search_filter::{
    RebuildSearchFilterIndexForSourceResult, StoreSearchAccessState,
    StoreSearchAttachmentLinkState, StoreSearchAuthorityLayer, StoreSearchEvidenceAvailability,
    StoreSearchEvidenceCoverageState, StoreSearchFileClass, StoreSearchFileKind,
    StoreSearchFilters, StoreSearchIndexState, StoreSearchMatchReason, StoreSearchMediaRelevance,
    StoreSearchPresenceState, StoreSearchQueryIdentity, StoreSearchRecursion, StoreSearchRequest,
    StoreSearchResult, StoreSearchResultKind, StoreSearchResultRow, StoreSearchScope,
    StoreSearchSort, StoreSearchState,
};
pub use read_models::source_lifecycle::StoreSourceLifecycle;
pub use read_models::track_identity_candidates::{
    StoreTrackIdentityCandidate, StoreTrackIdentityCandidateEvidence,
    StoreTrackIdentityCandidateEvidenceStatus, StoreTrackIdentityCandidateMember,
    StoreTrackIdentityCandidateStatus, read_track_identity_candidates_for_source,
};
pub use read_models::track_identity_decisions::{
    StoreTrackIdentityDecision, StoreTrackIdentityDecisionCurrentStatus,
    StoreTrackIdentityDecisionEvidence, StoreTrackIdentityDecisionState,
    StoreTrackIdentityEffectiveDecisionCurrentStatus,
    StoreTrackIdentityEffectiveDecisionPrecedence, StoreTrackIdentityEffectiveDecisionSummary,
    StoreTrackIdentityUserBlockingDecisionState,
    read_effective_track_identity_decision_for_candidate,
    read_track_identity_decisions_for_candidate, read_track_identity_decisions_for_source,
};
pub use read_models::track_identity_review::{
    ReviewCandidate as StoreTrackIdentityReviewCandidate,
    ReviewDecision as StoreTrackIdentityReviewDecision,
    ReviewEvidenceSummary as StoreTrackIdentityReviewEvidenceSummary,
    ReviewSourceSample as StoreTrackIdentityReviewSourceSample,
    ReviewSourceSummary as StoreTrackIdentityReviewSourceSummary,
    ReviewState as StoreTrackIdentityReviewState, read_track_identity_review_candidates,
};
pub use store::{
    DurableStoreBootstrapStatus, DurableStoreSchemaCompatibility,
    DurableStoreSchemaCompatibilityState, HashSourceFileBlake3BatchInput,
    HashSourceFileBlake3BatchOutcome, HashSourceFileBlake3BatchOutcomeStatus,
    HashSourceFileBlake3BatchResult, LibraryStoreContext, LocalRoot, LocalRootAvailability,
    MaintainedReadModelRevision, MaintainedReadModelScope, MaterializeAttachmentsForSourceResult,
    ProbeSourceFileMediaBatchInput, ProbeSourceFileMediaBatchOutcome,
    ProbeSourceFileMediaBatchOutcomeStatus, ProbeSourceFileMediaBatchResult,
    ProduceTrackIdentityCandidatesForSourceResult, ProduceTrackIdentityDecisionsForSourceResult,
    PromotePrimaryMediaForSourceResult, ReadLocalRootsResult,
    ReadSourceFileBlake3HashCandidatesInput, ReadSourceFileMediaProbeCandidatesInput,
    RegisterLocalRootInput, RootNavigationWindowEstablishment,
    RootNavigationWindowEstablishmentState, RootScanHierarchyObservationReason,
    RootScanMaterializationResult, RootScanObservation, SOURCE_FILE_BLAKE3_ALGORITHM,
    SourceFileBlake3HashAdmissionScope, SourceFileBlake3HashCandidate,
    SourceFileBlake3HashCandidateReason, SourceFileBlake3HashFailure,
    SourceFileBlake3HashSkipReason, SourceFileMediaProbeAdmissionScope,
    SourceFileMediaProbeCandidate, SourceFileMediaProbeCandidateReason, SourceFileMediaProbeFacts,
    SourceFileMediaProbeFailure, SourceFileMediaProbeSkipReason, SqliteDurableStore,
    SqliteDurableStoreAppOwnedState, StoreEnvironment,
    TRACK_IDENTITY_DECISION_SOURCE_SYSTEM_EXACT_CONTENT_V0,
    TRACK_IDENTITY_DECISION_SOURCE_USER_LOCAL_V0, TrackIdentityDecisionChangeFailure,
    TrackIdentityDecisionChangeResult, TrackIdentityDecisionChangeSuccess,
    UnregisterLocalRootInput, UnregisterLocalRootResult, durable_store_path,
    effective_hash_batch_limit, effective_media_probe_batch_limit,
    effective_primary_media_promotion_limit, effective_track_identity_candidate_limit,
    effective_track_identity_decision_limit,
};
