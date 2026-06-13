export type LocalBrowseEntryPointKind =
  | 'systemDriveRoot'
  | 'localDataVolumeRoot'
  | 'removableVolumeRoot'
  | 'userHome'
  | 'desktop'
  | 'downloads'
  | 'music'

export type LocalBrowseEntryPointStatus =
  | 'resolving'
  | 'available'
  | 'unavailable'
  | 'permissionBlocked'
  | 'missing'
  | 'unsupportedPlatform'
  | 'duplicateOfAdmittedSource'

export type LocalBrowsePlatform = 'windows' | 'macos' | 'linux' | 'unsupported'

export type LocalBrowseAdmissionAction =
  | 'requestAdmission'
  | 'requestDefaultMusicFolderAdmission'
  | 'requestParentAdmission'

export type LocalBrowseEntryPointFailureCode =
  | 'unsupportedPlatform'
  | 'knownFolderUnavailable'
  | 'systemDriveUnavailable'
  | 'volumeEnumerationUnavailable'
  | 'metadataUnavailable'

export type LocalBrowseEntryPointsReadStatus =
  | 'complete'
  | 'partialFailure'
  | 'failed'
  | 'unsupportedPlatform'

export type LocalBrowseEntryPointIdentity = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly canonicalPath: string | null
}

export type LocalBrowseAvailableActions = {
  readonly canBrowse: boolean
  readonly canRequestAdmission: boolean
  readonly canChooseDescendant: boolean
  readonly canRequestParentAdmission: boolean
}

export type LocalBrowseEntryPointFailure = {
  readonly code: LocalBrowseEntryPointFailureCode
  readonly detail: string
}

export type LocalBrowseEntryPoint = {
  readonly identity: LocalBrowseEntryPointIdentity
  readonly displayName: string
  readonly status: LocalBrowseEntryPointStatus
  readonly platform: LocalBrowsePlatform
  readonly admissionAction: LocalBrowseAdmissionAction | null
  readonly availableActions: LocalBrowseAvailableActions
  readonly failure: LocalBrowseEntryPointFailure | null
}

export type ReadLocalBrowseEntryPointsResult = {
  readonly state: 'read'
  readonly status: LocalBrowseEntryPointsReadStatus
  readonly entries: readonly LocalBrowseEntryPoint[]
  readonly failure: LocalBrowseEntryPointFailure | null
}

export type ReadLocalBrowseEntryPointsErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadLocalBrowseEntryPointsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadLocalBrowseEntryPointsError = {
  readonly code: ReadLocalBrowseEntryPointsErrorCode
  readonly message: string
}

export type ReadLocalBrowseEntryPointsOutcome =
  | ReadLocalBrowseEntryPointsResult
  | {
      readonly state: ReadLocalBrowseEntryPointsErrorState
      readonly error: ReadLocalBrowseEntryPointsError
    }
