import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundary/status'
import type { EntryPoint, ChildRow } from '../../shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../shared/libraryNavigation/readRows'

export type DirectoryTarget = {
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly directoryId: string
}

export type SourceTarget = {
  readonly navigationRowId: string
  readonly entryPoint: EntryPoint
  readonly label: string
}

export type MoreTarget = {
  readonly ownerNodeId: string
  readonly entryPoint: EntryPoint
  readonly parentDirectoryId?: string
  readonly label?: string
  readonly offset: number
  readonly limit: number
}

export type MoreState =
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type LoadedChildren = {
  readonly entryPoint: EntryPoint
  readonly parentDirectoryId?: string
  readonly label?: string
  readonly rows: readonly ChildRow[]
  readonly totalRows: number
  readonly nextOffset?: number
  readonly limit: number
  readonly more?: MoreState
}

export type DirectoryState =
  | {
      readonly kind: 'unloaded'
      readonly detail?: string
    }
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'loaded'
      readonly children: LoadedChildren
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type SourceState =
  | {
      readonly kind: 'unloaded'
      readonly detail?: string
    }
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'loaded'
      readonly children: LoadedChildren
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type RowBinding =
  | {
      readonly kind: 'navigation'
      readonly navigationRow: NavigationRow
    }
  | {
      readonly kind: 'source'
      readonly navigationRow: NavigationRow
      readonly target: SourceTarget
    }
  | {
      readonly kind: 'directory'
      readonly sourceId: string
      readonly directoryId: string
      readonly parentDirectoryId?: string
      readonly entryPoint: EntryPoint
      readonly label?: string
    }
  | {
      readonly kind: 'file'
      readonly sourceId: string
      readonly fileId: string
      readonly parentDirectoryId?: string
      readonly entryPoint: EntryPoint
    }
  | {
      readonly kind: 'readState'
      readonly state: 'notLoaded' | 'loading' | 'empty' | 'unavailable' | 'error'
      readonly ownerId: string
      readonly detail: string
    }
  | {
      readonly kind: 'more'
      readonly state: 'available' | 'loading' | 'error'
      readonly ownerId: string
      readonly target: MoreTarget
      readonly detail: string
    }

export type BrowserState = {
  readonly hostStatus?: LibraryBoundaryHostStatus
  readonly navigationReadResult?: NavigationReadRowsResult
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
}
