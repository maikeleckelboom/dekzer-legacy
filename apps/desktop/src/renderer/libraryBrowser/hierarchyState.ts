import type {
  LibraryHierarchyReadChildrenResult,
  LibraryHierarchyReadChildrenWindow
} from '../../shared/libraryHierarchy/readChildren'

export type DirectoryReadTarget = {
  readonly sourceDirectoryId: string
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

export type HierarchyState = {
  readonly rootReadResult?: LibraryHierarchyReadChildrenResult
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
}
