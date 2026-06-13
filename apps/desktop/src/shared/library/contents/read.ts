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
export type PlayableMediaKind = 'audio' | 'video'
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
      readonly kind: 'playableMedia'
      readonly mediaKinds: readonly PlayableMediaKind[]
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
  readonly subtreeCoverageComplete: boolean
  readonly emptyResultAuthoritative: boolean
  readonly detail?: string
}

export type ContentsPresence = 'present' | 'missing' | 'removed'

export type PlayableMedia = {
  readonly playableMediaId: string
  readonly attachmentId: string
  readonly contentHashAlgorithm: string
  readonly contentHashValue: string
  readonly evidenceSourceFileId: string
  readonly mediaKind: string
  readonly mimeType?: string
  readonly durationMs?: number
  readonly sampleRateHz?: number
  readonly channels?: number
  readonly bitDepth?: number
  readonly codec?: string
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
  readonly playableMedia?: PlayableMedia
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
