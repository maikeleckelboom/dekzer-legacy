import type {
  ReadTrackIdentityReviewCandidatesReply as ContractReadTrackIdentityReviewCandidatesReply,
  TrackIdentityReviewStateFilter
} from '@dekzer/library-boundary-contract'

export const trackIdentityReviewCandidateChannels = {
  readTrackIdentityReviewCandidates:
    'desktop:library-track-identity-review-candidates:read-track-identity-review-candidates'
} as const

export type ReadTrackIdentityReviewCandidatesRequest = {
  readonly sourceId?: string
  readonly reviewState?: TrackIdentityReviewStateFilter
  readonly limit: number
}

export type TrackIdentityReviewCandidatesReadState =
  | 'ready'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'readFailed'

export type TrackIdentityReviewCandidatesReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type TrackIdentityReviewCandidatesReadError = {
  readonly code: TrackIdentityReviewCandidatesReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type TrackIdentityReviewCandidatesReadErrorState = Exclude<
  TrackIdentityReviewCandidatesReadState,
  'ready'
>

export type ReadTrackIdentityReviewCandidatesResult =
  | {
      readonly state: 'ready'
      readonly result: ContractReadTrackIdentityReviewCandidatesReply
    }
  | {
      readonly state: TrackIdentityReviewCandidatesReadErrorState
      readonly error: TrackIdentityReviewCandidatesReadError
    }
