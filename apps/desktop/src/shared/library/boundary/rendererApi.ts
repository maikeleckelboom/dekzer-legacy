import type { LibraryBoundaryHostStatus, LibraryBoundaryHostStatusChangedCallback } from './status'
import type { BoundaryEventDeliveryPayload } from './events'
import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from '../viewState/persistence'
import type { ReadRequest, ReadResult } from '../hierarchy/read'
import type { ReadLocalBrowseItemsOutcome, ReadLocalBrowseItemsRequest } from '../localBrowse/items'
import type { ReadLocalBrowseEntryPointsOutcome } from '../localBrowse/entryPoints'
import type { NavigationReadRowsRequest, NavigationReadRowsResult } from '../navigation/read'
import type { ContentsReadRequest, ContentsReadResult } from '../contents/read'
import type { SearchFilterReadRequest, SearchFilterReadResult } from '../searchFilter/read'
import type { ReadSourceActivityRequest, SourceActivityReadResult } from '../source/activity'
import type { ReadSourceLifecycleRequest, ReadSourceLifecycleResult } from '../source/lifecycle'
import type { ReadSourceIntegrityRequest, SourceIntegrityReadResult } from '../source/integrity'
import type {
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from '../attachmentIdentity/read'
import type {
  HashSourceFilesBlake3Request,
  HashSourceFilesBlake3Result
} from '../source/fileHashing'
import type {
  ReadSourceMaintenanceRequest,
  ReadSourceMaintenanceResult,
  RunSourceMaintenanceRequest,
  RunSourceMaintenanceResult
} from '../source/maintenance'
import type {
  TrackIdentityDecisionCommandResult,
  TrackIdentityDecisionRequest
} from '../trackIdentity/decisions'
import type { ReadCandidatesRequest, ReadCandidatesResult } from '../trackIdentity/candidates'
import type { MusicalAnalysisRequest, MusicalAnalysisResult } from '../musicalAnalysis/analyze'
import type { LocalRootChoiceResult } from '../roots/chooseLocal'
import type { ReadLocalRootsOutcome } from '../roots/read'
import type { LocalRootRegistrationRequest, LocalRootRegistrationResult } from '../roots/register'
import type { LocalRootScanRequest, LocalRootScanResult } from '../roots/scan'
import type { CancelRootScanRequest, CancelRootScanResult } from '../roots/cancel'
import type { UnregisterLocalRootRequest, UnregisterLocalRootResult } from '../roots/unregister'

export type LibraryApi = {
  readonly host: LibraryHostApi
  readonly localBrowse: LibraryLocalBrowseApi
  readonly navigation: LibraryNavigationApi
  readonly hierarchy: LibraryHierarchyApi
  readonly sourceLifecycle: LibrarySourceLifecycleApi
  readonly sourceIntegrity: LibrarySourceIntegrityApi
  readonly sourceActivity: LibrarySourceActivityApi
  readonly attachmentIdentity: LibraryAttachmentIdentityApi
  readonly hashing: LibraryHashingApi
  readonly sourceMaintenance: LibrarySourceMaintenanceApi
  readonly musicalAnalysis: LibraryMusicalAnalysisApi
  readonly trackIdentityDecisions: LibraryTrackIdentityDecisionApi
  readonly trackIdentityReview: TrackIdentityReviewApi
  readonly contents: LibraryContentsApi
  readonly searchFilter: LibrarySearchFilterApi
  readonly roots: LibraryRootsApi
  readonly viewState: LibraryViewStateApi
  readonly events: LibraryBoundaryEventApi
}

export type LibraryBoundaryEventApi = {
  subscribe(callback: BoundaryEventDeliveryCallback): () => void
}

export type BoundaryEventDeliveryCallback = (payload: BoundaryEventDeliveryPayload) => void

export type LibraryHostApi = {
  getStatus(): Promise<LibraryBoundaryHostStatus>
  onStatusChanged(callback: LibraryBoundaryHostStatusChangedCallback): () => void
}

export type LibraryLocalBrowseApi = {
  readEntryPoints(): Promise<ReadLocalBrowseEntryPointsOutcome>
  readItems(request: ReadLocalBrowseItemsRequest): Promise<ReadLocalBrowseItemsOutcome>
}

export type LibraryHierarchyApi = {
  readChildren(request: ReadRequest): Promise<ReadResult>
}

export type LibraryNavigationApi = {
  readRows(request: NavigationReadRowsRequest): Promise<NavigationReadRowsResult>
}

export type LibraryContentsApi = {
  read(request: ContentsReadRequest): Promise<ContentsReadResult>
}

export type LibrarySearchFilterApi = {
  read(request: SearchFilterReadRequest): Promise<SearchFilterReadResult>
}

export type LibrarySourceLifecycleApi = {
  readSourceLifecycle(request: ReadSourceLifecycleRequest): Promise<ReadSourceLifecycleResult>
}

export type LibrarySourceIntegrityApi = {
  readSourceIntegrity(request: ReadSourceIntegrityRequest): Promise<SourceIntegrityReadResult>
}

export type LibrarySourceActivityApi = {
  readSourceActivity(request: ReadSourceActivityRequest): Promise<SourceActivityReadResult>
}

export type LibraryAttachmentIdentityApi = {
  readSourceFileAttachment(
    request: ReadSourceFileAttachmentRequest
  ): Promise<ReadSourceFileAttachmentResult>
  readAttachmentSourceFiles(
    request: ReadAttachmentSourceFilesRequest
  ): Promise<ReadAttachmentSourceFilesResult>
  readSourceAttachmentSummary(
    request: ReadSourceAttachmentSummaryRequest
  ): Promise<ReadSourceAttachmentSummaryResult>
}

export type LibraryHashingApi = {
  hashSourceFilesBlake3(request: HashSourceFilesBlake3Request): Promise<HashSourceFilesBlake3Result>
}

export type LibrarySourceMaintenanceApi = {
  runSourceMaintenance(request: RunSourceMaintenanceRequest): Promise<RunSourceMaintenanceResult>
  readSourceMaintenance(request: ReadSourceMaintenanceRequest): Promise<ReadSourceMaintenanceResult>
}

export type LibraryTrackIdentityDecisionApi = {
  acceptTrackIdentityCandidate(
    request: TrackIdentityDecisionRequest
  ): Promise<TrackIdentityDecisionCommandResult>
  rejectTrackIdentityCandidate(
    request: TrackIdentityDecisionRequest
  ): Promise<TrackIdentityDecisionCommandResult>
  deferTrackIdentityCandidate(
    request: TrackIdentityDecisionRequest
  ): Promise<TrackIdentityDecisionCommandResult>
}

export type LibraryMusicalAnalysisApi = {
  analyzePlayableMedia(request: MusicalAnalysisRequest): Promise<MusicalAnalysisResult>
}

export type TrackIdentityReviewApi = {
  readTrackIdentityReviewCandidates(request: ReadCandidatesRequest): Promise<ReadCandidatesResult>
}

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  registerLocalPath(request: LocalRootRegistrationRequest): Promise<LocalRootRegistrationResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
  cancelScan(request: CancelRootScanRequest): Promise<CancelRootScanResult>
  readLocalRoots(): Promise<ReadLocalRootsOutcome>
  unregisterLocalRoot(request: UnregisterLocalRootRequest): Promise<UnregisterLocalRootResult>
}

export type LibraryViewStateApi = {
  readViewState(): Promise<LibraryViewStateReadResult>
  writeViewState(viewState: PersistedLibraryViewState): Promise<LibraryViewStateWriteResult>
}
