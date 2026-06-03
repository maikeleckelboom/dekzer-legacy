import type { TrackIdentityDecisionWriteResult as ContractTrackIdentityDecisionWriteResult } from '@dekzer/library-boundary-contract'

export const trackIdentityDecisionWriteChannels = {
  acceptTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:accept-track-identity-candidate',
  rejectTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:reject-track-identity-candidate',
  deferTrackIdentityCandidate:
    'desktop:library-track-identity-decisions:defer-track-identity-candidate'
} as const

export type TrackIdentityDecisionWriteState =
  | 'completed'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'writeFailed'

export type TrackIdentityDecisionWriteErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'writeFailed'

export type TrackIdentityDecisionWriteError = {
  readonly code: TrackIdentityDecisionWriteErrorCode
  readonly message: string
  readonly detail?: string
}

export type TrackIdentityDecisionWriteErrorState = Exclude<
  TrackIdentityDecisionWriteState,
  'completed'
>

export type TrackIdentityDecisionWriteRequest = {
  readonly candidateId: string
  readonly reason?: string
}

export type TrackIdentityDecisionWriteResult =
  | {
      readonly state: 'completed'
      readonly result: ContractTrackIdentityDecisionWriteResult
    }
  | {
      readonly state: TrackIdentityDecisionWriteErrorState
      readonly error: TrackIdentityDecisionWriteError
    }
