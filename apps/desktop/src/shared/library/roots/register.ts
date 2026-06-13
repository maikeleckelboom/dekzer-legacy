export type LocalRootRegistrationState =
  | 'registered'
  | 'proposalRequired'
  | 'rejected'
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

export type LocalRootRegistrationErrorState = Exclude<
  LocalRootRegistrationState,
  'registered' | 'proposalRequired' | 'rejected'
>

export type LocalRootRegistrationRequest = {
  readonly requestedPath: string
}

export type LocalRootRegistrationRoot = {
  readonly rootId: string
  readonly admittedRootPath: string
}

export type SourceRegistrationRootClass =
  | 'normalMusicRoot'
  | 'broadDriveRoot'
  | 'systemVolumeRoot'
  | 'userProfileRoot'
  | 'cloudBackedRoot'
  | 'networkRoot'
  | 'protectedRoot'
  | 'indirectionRoot'
  | 'unknownRoot'

export type LocalRootRegistrationProposal = {
  readonly proposalId: string
  readonly rootClass: SourceRegistrationRootClass
  readonly requestedPath: string
  readonly resolvedPath: string | null
  readonly confirmationRequiredReason: string
  readonly suggestedRootPaths: readonly string[]
}

export type LocalRootRegistrationRejection = {
  readonly rootClass: SourceRegistrationRootClass
  readonly requestedPath: string
  readonly resolvedPath: string | null
  readonly rejectionReason: string
  readonly suggestedRootPaths: readonly string[]
}

export type LocalRootRegistrationResult =
  | {
      readonly state: 'registered'
      readonly root: LocalRootRegistrationRoot
    }
  | {
      readonly state: 'proposalRequired'
      readonly proposal: LocalRootRegistrationProposal
    }
  | {
      readonly state: 'rejected'
      readonly rejection: LocalRootRegistrationRejection
    }
  | {
      readonly state: LocalRootRegistrationErrorState
      readonly error: LocalRootRegistrationError
    }
