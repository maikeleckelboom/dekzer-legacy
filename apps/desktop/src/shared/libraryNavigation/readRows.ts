export const navigationReadChannels = {
  readRows: 'desktop:library-navigation:read-rows'
} as const

export type LibraryNavigationReadRowsState =
  | 'ready'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryNavigationReadRowsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryNavigationReadRowsError = {
  readonly code: LibraryNavigationReadRowsErrorCode
  readonly message: string
}

export type LibraryNavigationReadRowsErrorState = Exclude<LibraryNavigationReadRowsState, 'ready'>

export type LibraryNavigationRowFamily = 'views' | 'collections' | 'preparation' | 'sources'

export type LibraryNavigationRowKind =
  | 'view'
  | 'collectionGroup'
  | 'playlist'
  | 'prepPolicyGroup'
  | 'prepPolicyScope'
  | 'source'
  | 'locationGroup'
  | 'location'

export type LibraryNavigationRowSelectorKind =
  | 'allMedia'
  | 'allAudio'
  | 'allVideos'
  | 'recentlyAdded'
  | 'needsPreparation'
  | 'playlistGroup'
  | 'source'
  | 'sourceLocation'
  | 'playlist'
  | 'prepPolicyScope'

export type LibraryNavigationRow = {
  readonly navigationRowId: string
  readonly stableKey: string
  readonly parentNavigationRowId: string | null
  readonly family: LibraryNavigationRowFamily | null
  readonly rowKind: LibraryNavigationRowKind
  readonly displayName: string
  readonly siblingPosition: number
  readonly selectable: boolean
  readonly selectorKind: LibraryNavigationRowSelectorKind | null
  readonly selectorPayload: string | null
  readonly updatedAtMs: number
  readonly rowVersion: string
}

export type LibraryNavigationReadRowsRequest = {
  readonly parentNavigationRowId?: string | null
}

export type LibraryNavigationReadRowsResult =
  | {
      readonly state: 'ready'
      readonly rows: readonly LibraryNavigationRow[]
    }
  | {
      readonly state: LibraryNavigationReadRowsErrorState
      readonly error: LibraryNavigationReadRowsError
    }
