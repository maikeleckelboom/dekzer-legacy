import type {
  LibraryHierarchyReadResult,
  LibraryHierarchyReadWindow
} from '../../shared/libraryHierarchy/read'

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
      readonly window: LibraryHierarchyReadWindow
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type LibraryHierarchyBrowserState = {
  readonly rootReadResult?: LibraryHierarchyReadResult
  readonly directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
}
