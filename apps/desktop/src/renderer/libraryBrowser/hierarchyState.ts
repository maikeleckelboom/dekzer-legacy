import type {
  LibraryHierarchyReadChildrenResult,
  LibraryHierarchyReadChildrenWindow
} from '../../shared/libraryHierarchy/readChildren'

export type LibraryHierarchyDirectoryReadTarget = {
  readonly sourceDirectoryId: string
}

export type LibraryHierarchyDirectoryReadState =
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

export type LibraryHierarchyBrowserState = {
  readonly rootReadResult?: LibraryHierarchyReadChildrenResult
  readonly directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
}
