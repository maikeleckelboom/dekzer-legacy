export type PersistedLibraryViewState = {
  readonly version: 1
  readonly selectedNodeId?: string
  readonly expandedNodeIds: readonly string[]
  readonly libraryBrowseProfile?: 'audio' | 'playable' | 'allFiles'
  readonly localPreviewMode?: 'musicEvidence' | 'advancedInventory'
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
