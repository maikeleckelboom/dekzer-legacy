import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type {
  LibraryHierarchyReadChildrenRequest,
  LibraryHierarchyReadChildrenResult
} from './libraryHierarchy/readChildren'
import type {
  LibraryNavigationReadRowsRequest,
  LibraryNavigationReadRowsResult
} from './libraryNavigation/readRows'
import type { LocalRootChoiceResult } from './libraryRoots/chooseAndRegisterLocal'
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
  readChildren(
    request: LibraryHierarchyReadChildrenRequest
  ): Promise<LibraryHierarchyReadChildrenResult>
}

export type LibraryNavigationApi = {
  readRows(request: LibraryNavigationReadRowsRequest): Promise<LibraryNavigationReadRowsResult>
}

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
}
