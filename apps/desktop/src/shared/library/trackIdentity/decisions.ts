import type { TrackIdentityDecisionCommandResult as ContractTrackIdentityDecisionCommandResult } from '@dekzer/library-boundary-contract'

export type TrackIdentityDecisionCommandState =
  | 'completed'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'commandFailed'

export type TrackIdentityDecisionCommandErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'commandFailed'

export type TrackIdentityDecisionCommandError = {
  readonly code: TrackIdentityDecisionCommandErrorCode
  readonly message: string
  readonly detail?: string
}

export type TrackIdentityDecisionCommandErrorState = Exclude<
  TrackIdentityDecisionCommandState,
  'completed'
>

export type TrackIdentityDecisionRequest = {
  readonly candidateId: string
  readonly reason?: string
}

export type TrackIdentityDecisionCommandResult =
  | {
      readonly state: 'completed'
      readonly result: ContractTrackIdentityDecisionCommandResult
    }
  | {
      readonly state: TrackIdentityDecisionCommandErrorState
      readonly error: TrackIdentityDecisionCommandError
    }
