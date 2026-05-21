export const libraryRootsIpcChannels = {
  registerLocal: 'desktop:library-roots:register-local'
} as const

export type LocalRootRegistrationState =
  | 'registered'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'registrationFailed'

export type LocalRootRegistrationErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'registrationFailed'

export type LocalRootRegistrationError = {
  readonly code: LocalRootRegistrationErrorCode
  readonly message: string
}

export type LocalRootRegistrationErrorState = Exclude<LocalRootRegistrationState, 'registered'>

export type LocalRootRegistrationRequest = {
  readonly absolutePath: string
}

export type LocalRootRegistrationRoot = {
  readonly rootId: string
  readonly canonicalPath: string
}

export type LocalRootRegistrationResult =
  | {
      readonly state: 'registered'
      readonly root: LocalRootRegistrationRoot
    }
  | {
      readonly state: LocalRootRegistrationErrorState
      readonly error: LocalRootRegistrationError
    }
