import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type {
  BoundaryEventReadAfterRequest,
  BoundaryEventReadAfterReply
} from './libraryBoundary/events'
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
import type { ContentsReadResult, ContentsReadRequest } from './libraryContents/read'
import type { LocalRootChoiceResult } from './libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from './libraryRoots/readLocalRoots'
import type { LocalRootScanRequest, LocalRootScanResult } from './libraryRoots/runScan'
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
  readonly contents: LibraryContentsApi
  readonly roots: LibraryRootsApi
  readonly viewState: LibraryViewStateApi
  readonly events: LibraryBoundaryEventApi
}

export type LibraryBoundaryEventApi = {
  readAfter(request: BoundaryEventReadAfterRequest): Promise<BoundaryEventReadAfterReply>
}

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

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
  readLocalRoots(): Promise<ReadLocalRootsOutcome>
  unregisterLocalRoot(request: UnregisterLocalRootRequest): Promise<UnregisterLocalRootResult>
}

export type LibraryViewStateApi = {
  readViewState(): Promise<LibraryViewStateReadResult>
  writeViewState(viewState: PersistedLibraryViewState): Promise<LibraryViewStateWriteResult>
}
