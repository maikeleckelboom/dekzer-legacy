import type { LocalRootRegistrationError, LocalRootRegistrationRoot } from './registerLocalRoot'

export type LocalRootChoiceState =
  | 'canceled'
  | 'registered'
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
      readonly state: Exclude<LocalRootChoiceState, 'canceled' | 'registered'>
      readonly error: LocalRootChoiceError
    }
