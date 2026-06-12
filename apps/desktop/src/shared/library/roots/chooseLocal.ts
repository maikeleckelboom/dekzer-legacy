import type {
  LocalRootRegistrationError,
  LocalRootRegistrationProposal,
  LocalRootRegistrationRejection,
  LocalRootRegistrationRoot
} from './register'

export type LocalRootChoiceState =
  | 'canceled'
  | 'registered'
  | 'proposalRequired'
  | 'rejected'
  | 'hostUnavailable'
  | 'registrationFailed'
  | 'dialogFailed'

export type LocalRootChoiceErrorCode = LocalRootRegistrationError['code'] | 'dialogFailed'

export type LocalRootChoiceError = {
  readonly code: LocalRootChoiceErrorCode
  readonly message: string
}

export type LocalRootChoiceResult =
  | {
      readonly state: 'canceled'
    }
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
      readonly state: Exclude<
        LocalRootChoiceState,
        'canceled' | 'registered' | 'proposalRequired' | 'rejected'
      >
      readonly error: LocalRootChoiceError
    }
