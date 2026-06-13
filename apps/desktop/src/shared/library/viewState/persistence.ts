export type LibraryPanelSurface = 'libraryBrowse' | 'addSource'

export type PersistedLibraryViewState = {
  readonly version: 3
  readonly activeSurface: LibraryPanelSurface
  readonly selectedLibraryNodeId?: string
  readonly selectedAddSourceNodeId?: string
  readonly expandedLibraryNodeIds: readonly string[]
  readonly expandedAddSourceNodeIds: readonly string[]
  readonly libraryBrowseProfile?: 'audio' | 'playable' | 'allFiles'
  readonly addSourceView?: 'preview' | 'inventory'
}

export type LibraryViewStateReadResult =
  | {
      readonly state: 'ready'
      readonly viewState: PersistedLibraryViewState
    }
  | {
      readonly state: 'empty'
    }

export type LibraryViewStateWriteResult =
  | {
      readonly state: 'written'
    }
  | {
      readonly state: 'failed'
      readonly detail: string
    }
