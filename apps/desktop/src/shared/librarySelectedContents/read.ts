export const selectedContentsReadChannels = {
  read: 'desktop:library-selected-contents:read'
} as const

export type SelectedContentsReadState =
  | 'ready'
  | 'hostUnavailable'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type SelectedContentsReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type SelectedContentsReadError = {
  readonly code: SelectedContentsReadErrorCode
  readonly message: string
}

export type SelectedContentsReadErrorState = Exclude<SelectedContentsReadState, 'ready'>

export type SelectedContentsScope =
  | {
      readonly kind: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'sourceLocation'
      readonly sourceLocationId: string
    }
  | {
      readonly kind: 'directory'
      readonly sourceId: string
      readonly sourceDirectoryId: string
    }

export type SelectedContentsRequest = {
  readonly scope?: SelectedContentsScope
  readonly limit?: number
  readonly cursor?: string
}

export type SelectedContentsState =
  | 'ready'
  | 'empty'
  | 'partial'
  | 'sourceUnavailable'
  | 'locationMissing'
  | 'blocked'
  | 'failed'

export type SelectedContentsCoverageState =
  | 'complete'
  | 'pending'
  | 'scanning'
  | 'blocked'
  | 'failed'
  | 'sourceUnavailable'
  | 'locationMissing'
  | 'incomplete'

export type SelectedContentsCoverage = {
  readonly state: SelectedContentsCoverageState
  readonly recursiveScopeComplete: boolean
  readonly emptyResultAuthoritative: boolean
  readonly detail?: string
}

export type SelectedContentsRowOrigin = 'libraryAsset' | 'sourceFile'

export type SelectedContentsMediaClass = 'audio' | 'video'
export type SelectedContentsAvailabilityState = 'available' | 'unavailable' | 'degraded'
export type SelectedContentsStemsStateSummary =
  | 'missing'
  | 'queued'
  | 'leased'
  | 'ready'
  | 'stale'
  | 'blocked'
  | 'failed'
export type SelectedContentsPrepReadinessSummary =
  | 'notRequired'
  | 'ready'
  | 'preparing'
  | 'underprepared'
  | 'blocked'
  | 'failed'

export type SelectedContentsRow = {
  readonly stableId: string
  readonly label: string
  readonly origin: SelectedContentsRowOrigin
  readonly libraryAssetId?: string
  readonly rowVersion?: string
  readonly primarySourceFileId?: string
  readonly scopedSourceFileId: string
  readonly sourceId: string
  readonly relativePath: string
  readonly fileName: string
  readonly mediaClass: SelectedContentsMediaClass
  readonly availabilityState: SelectedContentsAvailabilityState
  readonly title?: string
  readonly artist?: string
  readonly album?: string
  readonly durationMs?: number
  readonly musicalKey?: string
  readonly tempoBpm?: number
  readonly waveformQualityCurrent?: number
  readonly waveformQualityTarget?: number
  readonly stemsStateSummary?: SelectedContentsStemsStateSummary
  readonly prepReadinessSummary: SelectedContentsPrepReadinessSummary
  readonly updatedAtMs: number
}

export type SelectedContentsResult = {
  readonly state: SelectedContentsState
  readonly scope: SelectedContentsScope
  readonly rows: readonly SelectedContentsRow[]
  readonly coverage: SelectedContentsCoverage
  readonly nextCursor?: string
  readonly detail?: string
}

export type SelectedContentsReadResult =
  | {
      readonly state: 'ready'
      readonly result: SelectedContentsResult
    }
  | {
      readonly state: SelectedContentsReadErrorState
      readonly error: SelectedContentsReadError
    }
