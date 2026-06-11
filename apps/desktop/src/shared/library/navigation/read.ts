export type NavigationReadRowsState = 'ready' | 'hostUnavailable' | 'invalidRequest' | 'readFailed'

export type NavigationReadRowsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type NavigationReadRowsError = {
  readonly code: NavigationReadRowsErrorCode
  readonly message: string
}

export type NavigationReadRowsErrorState = Exclude<NavigationReadRowsState, 'ready'>

export type NavigationRowFamily = 'views' | 'collections' | 'preparation' | 'sources'

export type NavigationRowKind =
  | 'view'
  | 'collectionGroup'
  | 'playlist'
  | 'prepPolicyGroup'
  | 'prepPolicyScope'
  | 'source'
  | 'locationGroup'
  | 'location'

export type NavigationRowSelectorKind =
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

export type NavigationRow = {
  readonly navigationRowId: string
  readonly stableKey: string
  readonly parentNavigationRowId: string | null
  readonly family: NavigationRowFamily | null
  readonly rowKind: NavigationRowKind
  readonly displayName: string
  readonly siblingPosition: number
  readonly selectable: boolean
  readonly selectorKind: NavigationRowSelectorKind | null
  readonly selectorPayload: string | null
  readonly updatedAtMs: number
  readonly rowVersion: string
}

export type NavigationReadRowsRequest = {
  readonly parentNavigationRowId?: string | null
}

export type NavigationReadRowsResult =
  | {
      readonly state: 'ready'
      readonly rows: readonly NavigationRow[]
    }
  | {
      readonly state: NavigationReadRowsErrorState
      readonly error: NavigationReadRowsError
    }
