import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type {
  LibraryBrowserViewStateReadResult,
  LibraryBrowserViewStateWriteResult,
  PersistedLibraryBrowserViewState
} from './libraryBrowser/viewState'
import type { ReadRequest, ReadResult } from './libraryHierarchy/readChildren'
import type {
  NavigationReadRowsRequest,
  NavigationReadRowsResult
} from './libraryNavigation/readRows'
import type {
  SelectedContentsReadResult,
  SelectedContentsRequest
} from './librarySelectedContents/read'
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
  readonly selectedContents: LibrarySelectedContentsApi
  readonly roots: LibraryRootsApi
  readonly browser: LibraryBrowserApi
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

export type LibrarySelectedContentsApi = {
  read(request: SelectedContentsRequest): Promise<SelectedContentsReadResult>
}

export type LibraryRootsApi = {
  chooseAndRegisterLocal(): Promise<LocalRootChoiceResult>
  runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult>
  readLocalRoots(): Promise<ReadLocalRootsOutcome>
  unregisterLocalRoot(request: UnregisterLocalRootRequest): Promise<UnregisterLocalRootResult>
}

export type LibraryBrowserApi = {
  readonly viewState: {
    readViewState(): Promise<LibraryBrowserViewStateReadResult>
    writeViewState(
      viewState: PersistedLibraryBrowserViewState
    ): Promise<LibraryBrowserViewStateWriteResult>
  }
}
