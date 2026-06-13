export type LocalBrowserEntryPointKind =
  | 'systemDriveRoot'
  | 'localDataVolumeRoot'
  | 'removableVolumeRoot'
  | 'userHome'
  | 'desktop'
  | 'downloads'
  | 'music'

export type LocalBrowserEntryPointStatus =
  | 'resolving'
  | 'available'
  | 'unavailable'
  | 'permissionBlocked'
  | 'missing'
  | 'unsupportedPlatform'
  | 'duplicateOfAdmittedSource'

export type LocalBrowserEntryPointPlatform = 'windows' | 'macos' | 'linux' | 'unsupported'

export type LocalBrowserEntryPointAdmissionHint =
  | 'notDirectlyAdmissible'
  | 'requiresConfirmation'
  | 'defaultMusicFolder'
  | 'duplicateOfAdmittedSource'
  | 'unavailable'
  | 'unsupportedPlatform'

export type LocalBrowserEntryPointFailureCode =
  | 'unsupportedPlatform'
  | 'knownFolderUnavailable'
  | 'systemDriveUnavailable'
  | 'volumeEnumerationUnavailable'
  | 'metadataUnavailable'

export type LocalBrowserEntryPointsReadStatus =
  | 'complete'
  | 'partialFailure'
  | 'failed'
  | 'unsupportedPlatform'

export type LocalBrowserEntryPointIdentity = {
  readonly entryPointKind: LocalBrowserEntryPointKind
  readonly canonicalPath: string | null
}

export type LocalBrowserEntryPointAffordances = {
  readonly canBrowse: boolean
  readonly canRequestAdmission: boolean
  readonly canChooseDescendant: boolean
  readonly requiresConfirmation: boolean
}

export type LocalBrowserEntryPointFailure = {
  readonly code: LocalBrowserEntryPointFailureCode
  readonly detail: string
}

export type LocalBrowserEntryPoint = {
  readonly identity: LocalBrowserEntryPointIdentity
  readonly displayName: string
  readonly status: LocalBrowserEntryPointStatus
  readonly platform: LocalBrowserEntryPointPlatform
  readonly admissionHint: LocalBrowserEntryPointAdmissionHint
  readonly affordances: LocalBrowserEntryPointAffordances
  readonly failure: LocalBrowserEntryPointFailure | null
}

export type ReadLocalBrowserEntryPointsResult = {
  readonly state: 'read'
  readonly status: LocalBrowserEntryPointsReadStatus
  readonly entries: readonly LocalBrowserEntryPoint[]
  readonly failure: LocalBrowserEntryPointFailure | null
}

export type ReadLocalBrowserEntryPointsErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadLocalBrowserEntryPointsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadLocalBrowserEntryPointsError = {
  readonly code: ReadLocalBrowserEntryPointsErrorCode
  readonly message: string
}

export type ReadLocalBrowserEntryPointsOutcome =
  | ReadLocalBrowserEntryPointsResult
  | {
      readonly state: ReadLocalBrowserEntryPointsErrorState
      readonly error: ReadLocalBrowserEntryPointsError
    }
