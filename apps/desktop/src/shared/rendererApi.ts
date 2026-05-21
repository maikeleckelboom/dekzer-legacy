import type {
  LibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusChangedCallback
} from './libraryBoundary/status'
import type {
  LibraryHierarchyReadChildrenRequest,
  LibraryHierarchyReadChildrenResult
} from './libraryHierarchy/readChildren'
import type {
  LocalRootRegistrationRequest,
  LocalRootRegistrationResult
} from './libraryRoots/registerLocalRoot'

export type RendererApi = {
  readonly library: LibraryApi
}

export type LibraryApi = {
  readonly host: LibraryHostApi
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

export type LibraryRootsApi = {
  registerLocal(request: LocalRootRegistrationRequest): Promise<LocalRootRegistrationResult>
}
