import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type { BoundaryEventDeliveryPayload } from './libraryBoundary/events'
import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from './libraryViewState/viewState'
import type { ReadRequest, ReadResult } from './libraryHierarchy/readChildren'
import type {
  NavigationReadRowsRequest,
  NavigationReadRowsResult
} from './libraryNavigation/readRows'
import type { ContentsReadRequest, ContentsReadResult } from './libraryContents/read'
import type {
  ReadSourceLifecycleRequest,
  ReadSourceLifecycleResult
} from './librarySourceLifecycle/readSourceLifecycle'
import type {
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from './libraryAttachmentIdentity/read'
import type {
  HashSourceFilesBlake3Request,
  HashSourceFilesBlake3Result
} from './librarySourceFileHashing/hashSourceFilesBlake3'
import type {
  ReadSourceMaintenanceRequest,
  ReadSourceMaintenanceResult,
  RunSourceMaintenanceRequest,
  RunSourceMaintenanceResult
} from './librarySourceMaintenance/sourceMaintenance'
import type {
  TrackIdentityDecisionRequest,
  TrackIdentityDecisionCommandResult
} from './libraryTrackIdentityDecisions/decisionCommands'
import type {
  ReadTrackIdentityReviewCandidatesRequest,
  ReadTrackIdentityReviewCandidatesResult
} from './libraryTrackIdentityReview/reviewCandidates'
import type { LocalRootChoiceResult } from './libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from './libraryRoots/readLocalRoots'
import type { LocalRootScanRequest, LocalRootScanResult } from './libraryRoots/runScan'
import type { CancelRootScanRequest, CancelRootScanResult } from './libraryRoots/cancelScan'
import type {
  UnregisterLocalRootRequest,
  UnregisterLocalRootResult
} from './libraryRoots/unregisterLocalRoot'

export type RendererApi = {
  readonly library: LibraryApi
}

export type LibraryApi = {
  readonly host: LibraryHostApi
  readonly navigation: LibraryNavigationApi
  readonly hierarchy: LibraryHierarchyApi
  readonly sourceLifecycle: LibrarySourceLifecycleApi
  readonly attachmentIdentity: LibraryAttachmentIdentityApi
  readonly hashing: LibraryHashingApi
  readonly sourceMaintenance: LibrarySourceMaintenanceApi
  readonly trackIdentityDecisions: LibraryTrackIdentityDecisionApi
  readonly trackIdentityReview: TrackIdentityReviewApi
  readonly contents: LibraryContentsApi
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

export type LibraryHierarchyApi = {
  readChildren(request: ReadRequest): Promise<ReadResult>
}

export type LibraryNavigationApi = {
  readRows(request: NavigationReadRowsRequest): Promise<NavigationReadRowsResult>
}

export type LibraryContentsApi = {
  read(request: ContentsReadRequest): Promise<ContentsReadResult>
}

export type LibrarySourceLifecycleApi = {
  readSourceLifecycle(request: ReadSourceLifecycleRequest): Promise<ReadSourceLifecycleResult>
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

export type TrackIdentityReviewApi = {
  readTrackIdentityReviewCandidates(
    request: ReadTrackIdentityReviewCandidatesRequest
  ): Promise<ReadTrackIdentityReviewCandidatesResult>
}

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
  cancelScan(request: CancelRootScanRequest): Promise<CancelRootScanResult>
  readLocalRoots(): Promise<ReadLocalRootsOutcome>
  unregisterLocalRoot(request: UnregisterLocalRootRequest): Promise<UnregisterLocalRootResult>
}

export type LibraryViewStateApi = {
  readViewState(): Promise<LibraryViewStateReadResult>
  writeViewState(viewState: PersistedLibraryViewState): Promise<LibraryViewStateWriteResult>
}
