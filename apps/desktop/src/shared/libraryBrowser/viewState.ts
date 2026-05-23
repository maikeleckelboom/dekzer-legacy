export const libraryBrowserChannels = {
  readViewState: 'desktop:library-browser:read-view-state',
  writeViewState: 'desktop:library-browser:write-view-state'
} as const

export type PersistedLibraryBrowserViewState = {
  readonly version: 1
  readonly selectedNodeId?: string
  readonly expandedNodeIds: readonly string[]
}

export type LibraryBrowserViewStateReadResult =
  | {
      readonly state: 'ready'
      readonly viewState: PersistedLibraryBrowserViewState
    }
  | {
      readonly state: 'empty'
    }

export type LibraryBrowserViewStateWriteResult =
  | {
      readonly state: 'written'
    }
  | {
      readonly state: 'failed'
      readonly detail: string
    }
