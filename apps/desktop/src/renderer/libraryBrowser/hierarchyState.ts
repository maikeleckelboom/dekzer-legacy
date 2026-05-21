import type {
  LibraryHierarchyReadChildrenEntryPoint,
  LibraryHierarchyReadChildrenNode
} from '../../shared/libraryHierarchy/readChildren'
import type {
  LibraryNavigationReadRowsResult,
  LibraryNavigationRow
} from '../../shared/libraryNavigation/readRows'

export type DirectoryReadTarget = {
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label?: string
  readonly sourceDirectoryId: string
}

export type SourceReadTarget = {
  readonly navigationRowId: string
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label: string
}

export type ContinuationReadTarget = {
  readonly ownerNodeId: string
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly parentSourceDirectoryId?: string
  readonly label?: string
  readonly offset: number
  readonly limit: number
}

export type HierarchyContinuationReadState =
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

export type LoadedHierarchyChildrenState = {
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly parentSourceDirectoryId?: string
  readonly label?: string
  readonly rows: readonly LibraryHierarchyReadChildrenNode[]
  readonly totalRows: number
  readonly nextOffset?: number
  readonly limit: number
  readonly continuation?: HierarchyContinuationReadState
}

export type DirectoryReadState =
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
      readonly children: LoadedHierarchyChildrenState
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type SourceReadState =
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
      readonly children: LoadedHierarchyChildrenState
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type HierarchyProjectionRow =
  | {
      readonly kind: 'navigation'
      readonly navigationRow: LibraryNavigationRow
    }
  | {
      readonly kind: 'sourceEntry'
      readonly navigationRow: LibraryNavigationRow
      readonly target: SourceReadTarget
    }
  | {
      readonly kind: 'literalDirectory'
      readonly sourceDirectoryId: string
      readonly parentSourceDirectoryId?: string
      readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
    }
  | {
      readonly kind: 'literalFile'
      readonly sourceFileId: string
      readonly parentSourceDirectoryId?: string
      readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
    }
  | {
      readonly kind: 'readState'
      readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
      readonly ownerId: string
      readonly detail: string
    }
  | {
      readonly kind: 'continuation'
      readonly state: 'available' | 'loading' | 'error'
      readonly ownerId: string
      readonly target: ContinuationReadTarget
      readonly detail: string
    }

export type HierarchyState = {
  readonly navigationReadResult?: LibraryNavigationReadRowsResult
  readonly sourceReadStates: ReadonlyMap<string, SourceReadState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
}
