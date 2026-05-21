import type {
  LibraryHierarchyReadChildrenEntryPoint,
  LibraryHierarchyReadChildrenWindow
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
      readonly window: LibraryHierarchyReadChildrenWindow
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
      readonly window: LibraryHierarchyReadChildrenWindow
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

export type HierarchyState = {
  readonly navigationReadResult?: LibraryNavigationReadRowsResult
  readonly sourceReadStates: ReadonlyMap<string, SourceReadState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
}
