import type { LibraryBoundaryHostStatus } from '../../shared/library/boundary/status'
import type { EntryPoint, ChildRow, HierarchyCoverage } from '../../shared/library/hierarchy/read'
import type { NavigationReadRowsResult, NavigationRow } from '../../shared/library/navigation/read'
import type { LocalBrowseEntryPoint } from '../../shared/library/localBrowse/entryPoints'
import type { LocalBrowseItem } from '../../shared/library/localBrowse/items'
import type {
  LocalBrowseDirectoryTarget,
  LocalBrowseEntryPointTarget,
  LocalBrowseEntryPointsState,
  LocalBrowseItemState,
  LocalBrowseMoreTarget
} from './localBrowse/types'
import type { SourceReadiness } from './runtime/sourceReadiness'
import type { LibraryBrowseProfile } from './libraryBrowseProfile/types'
import type { AddSourceView } from './addSource/view'

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
  readonly parentNodeId: string
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
  readonly coverage: HierarchyCoverage
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
      readonly kind: 'refreshing'
      readonly children: LoadedChildren
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
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
      readonly kind: 'refreshing'
      readonly children: LoadedChildren
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
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
      readonly parentNodeId: string
      readonly detail: string
    }
  | {
      readonly kind: 'more'
      readonly state: 'available' | 'loading' | 'error'
      readonly parentNodeId: string
      readonly target: MoreTarget
      readonly detail: string
    }
  | {
      readonly kind: 'addSourceSection'
    }
  | {
      readonly kind: 'localBrowseEntryPoint'
      readonly entry: LocalBrowseEntryPoint
      readonly target: LocalBrowseEntryPointTarget
    }
  | {
      readonly kind: 'localBrowseItem'
      readonly item: LocalBrowseItem
      readonly target?: LocalBrowseDirectoryTarget
    }
  | {
      readonly kind: 'localBrowseMore'
      readonly state: 'available' | 'loading' | 'error'
      readonly parentNodeId: string
      readonly target: LocalBrowseMoreTarget
      readonly detail: string
    }

export type BrowserState = {
  readonly libraryBrowseProfile?: LibraryBrowseProfile
  readonly addSourceView?: AddSourceView
  readonly hostStatus?: LibraryBoundaryHostStatus
  readonly navigationReadResult?: NavigationReadRowsResult
  readonly sourceReadinessByNodeId?: ReadonlyMap<string, SourceReadiness>
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly localBrowseEntryPointsState?: LocalBrowseEntryPointsState
  readonly localBrowseItemStates?: ReadonlyMap<string, LocalBrowseItemState>
}
