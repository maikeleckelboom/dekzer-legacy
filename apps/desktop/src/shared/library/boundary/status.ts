export type LibraryBoundaryHostStatusState =
  | 'idle'
  | 'starting'
  | 'started'
  | 'stopping'
  | 'stopped'
  | 'failed'

export type LibraryBoundaryHostStatusEnvironment = 'development' | 'production'

export type LibraryBoundaryHostStatusBinaryPolicy =
  | {
      readonly kind: 'developmentBinary'
      readonly source: 'environmentOverride' | 'repoDebugTarget'
    }
  | {
      readonly kind: 'packagedBinaryUnavailable'
      readonly executableName: string
    }

export type LibraryBoundaryHostStatusError = {
  readonly code:
    | 'invalidUserDataPath'
    | 'missingDevelopmentBinary'
    | 'packagedBinaryUnavailable'
    | 'stdioTransportStartupFailure'
    | 'alreadyStarted'
    | 'notStarted'
    | 'stopping'
    | 'stopped'
    | 'unknown'
  readonly message: string
  readonly detail?: string
}

export type LibraryBoundaryHostStatus = {
  readonly state: LibraryBoundaryHostStatusState
  readonly environment: LibraryBoundaryHostStatusEnvironment
  readonly binaryPolicy: LibraryBoundaryHostStatusBinaryPolicy
  readonly lastError: LibraryBoundaryHostStatusError | null
}

export type LibraryBoundaryHostStatusChangedCallback = (status: LibraryBoundaryHostStatus) => void
