export const libraryViewStateChannels = {
  readViewState: 'desktop:library:view-state:read',
  writeViewState: 'desktop:library:view-state:write'
} as const

export type PersistedLibraryViewState = {
  readonly version: 1
  readonly selectedNodeId?: string
  readonly expandedNodeIds: readonly string[]
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
