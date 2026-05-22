import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type { ReadRequest, ReadResult } from './libraryHierarchy/readChildren'
import type {
  NavigationReadRowsRequest,
  NavigationReadRowsResult
} from './libraryNavigation/readRows'
import type { LocalRootChoiceResult } from './libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from './libraryRoots/readLocalRoots'
import type { LocalRootScanRequest, LocalRootScanResult } from './libraryRoots/runScan'

export type RendererApi = {
  readonly library: LibraryApi
}

export type LibraryApi = {
  readonly host: LibraryHostApi
  readonly navigation: LibraryNavigationApi
  readonly hierarchy: LibraryHierarchyApi
  readonly roots: LibraryRootsApi
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

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
  readLocalRoots(): Promise<ReadLocalRootsOutcome>
}
