import type { TrackIdentityDecisionCommandResult as ContractTrackIdentityDecisionCommandResult } from '@dekzer/library-boundary-contract'

export const trackIdentityDecisionChannels = {
  acceptTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:accept-track-identity-candidate',
  rejectTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:reject-track-identity-candidate',
  deferTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:defer-track-identity-candidate'
} as const

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
