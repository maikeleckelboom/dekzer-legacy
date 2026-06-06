export const contentsReadChannels = {
  read: 'desktop:library-contents:read'
} as const

export type ContentsReadState =
  | 'ready'
  | 'hostUnavailable'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'policyConflict'
  | 'cursorInvalid'
  | 'readFailed'

export type ContentsReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'policyConflict'
  | 'cursorInvalid'
  | 'readFailed'

export type ContentsReadError = {
  readonly code: ContentsReadErrorCode
  readonly message: string
}

export type ContentsReadErrorState = Exclude<ContentsReadState, 'ready'>

export type ContentsScope =
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

export type ContentsScopeDepth = 'immediate' | 'recursive'

export type ContentsFileClass = 'audio' | 'video' | 'image' | 'unsupported'
export type PrimaryMediaKind = 'audio' | 'video'
export type ContentsFileKind =
  | 'audio'
  | 'video'
  | 'image'
  | 'cueSheet'
  | 'logDoc'
  | 'textDoc'
  | 'archive'
  | 'other'
  | 'unknown'

export type ContentsReadPolicy =
  | {
      readonly kind: 'playableMediaBrowse'
    }
  | {
      readonly kind: 'audioBrowse'
    }
  | {
      readonly kind: 'sourceFileInventory'
      readonly fileClasses: readonly ContentsFileClass[]
    }
  | {
      readonly kind: 'primaryMedia'
      readonly mediaKinds: readonly PrimaryMediaKind[]
    }

export type ContentsReadRequest = {
  readonly scope: ContentsScope
  readonly policy: ContentsReadPolicy
  readonly scopeDepth: ContentsScopeDepth
  readonly limit?: number
  readonly cursor?: string
}

export type ContentsState =
  | 'ready'
  | 'empty'
  | 'partial'
  | 'sourceUnavailable'
  | 'locationMissing'
  | 'blocked'
  | 'failed'

export type ContentsScopeCoverageState =
  | 'complete'
  | 'pending'
  | 'scanning'
  | 'blocked'
  | 'failed'
  | 'sourceUnavailable'
  | 'locationMissing'
  | 'incomplete'

export type ContentsScopeCoverage = {
  readonly state: ContentsScopeCoverageState
  readonly recursiveScopeComplete: boolean
  readonly emptyResultAuthoritative: boolean
  readonly detail?: string
}

export type ContentsRowOrigin = 'libraryAsset' | 'sourceFile' | 'primaryMediaCandidate'
export type ContentsPresence = 'present' | 'missing' | 'removed'
export type ContentsAvailabilityState = 'available' | 'unavailable' | 'degraded'
export type ContentsStemsStateSummary =
  | 'missing'
  | 'queued'
  | 'leased'
  | 'ready'
  | 'stale'
  | 'blocked'
  | 'failed'
export type ContentsPrepReadinessSummary =
  | 'notRequired'
  | 'ready'
  | 'preparing'
  | 'underprepared'
  | 'blocked'
  | 'failed'

export type PrimaryMediaSummary = {
  readonly origin: ContentsRowOrigin
  readonly primaryMediaCandidateId?: string
  readonly attachmentId?: string
  readonly contentHashAlgorithm?: string
  readonly contentHashValue?: string
  readonly evidenceSourceFileId?: string
  readonly mediaKind?: string
  readonly mimeType?: string
  readonly libraryAssetId?: string
  readonly rowVersion?: string
  readonly primarySourceFileId?: string
  readonly title?: string
  readonly artist?: string
  readonly album?: string
  readonly durationMs?: number
  readonly sampleRateHz?: number
  readonly channels?: number
  readonly bitDepth?: number
  readonly codec?: string
  readonly musicalKey?: string
  readonly tempoBpm?: number
  readonly waveformQualityCurrent?: number
  readonly waveformQualityTarget?: number
  readonly stemsStateSummary?: ContentsStemsStateSummary
  readonly prepReadinessSummary?: ContentsPrepReadinessSummary
}

export type ContentsFileRow = {
  readonly id: string
  readonly sourceId: string
  readonly sourceFileId: string
  readonly parentDirectoryId?: string
  readonly label: string
  readonly relativePath?: string
  readonly fileName: string
  readonly fileClass: ContentsFileClass
  readonly fileKind: ContentsFileKind
  readonly presence: ContentsPresence
  readonly availabilityState?: ContentsAvailabilityState
  readonly primaryMedia?: PrimaryMediaSummary
  readonly updatedAtMs?: number
}

export type ContentsResult = {
  readonly state: ContentsState
  readonly scope: ContentsScope
  readonly policy: ContentsReadPolicy
  readonly scopeDepth: ContentsScopeDepth
  readonly rows: readonly ContentsFileRow[]
  readonly scopeCoverage: ContentsScopeCoverage
  readonly hasPolicyOmittedRows: boolean
  readonly nextCursor?: string
  readonly detail?: string
}

export type ContentsReadResult =
  | {
      readonly state: 'ready'
      readonly result: ContentsResult
    }
  | {
      readonly state: ContentsReadErrorState
      readonly error: ContentsReadError
    }
