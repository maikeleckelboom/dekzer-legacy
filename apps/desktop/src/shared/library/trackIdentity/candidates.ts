import type {
  ReadTrackIdentityReviewCandidatesReply as ContractReadTrackIdentityReviewCandidatesReply,
  TrackIdentityReviewState
} from '@dekzer/library-boundary-contract'

export type ReadCandidatesRequest = {
  readonly sourceId?: string
  readonly reviewState?: TrackIdentityReviewState
  readonly limit: number
}

export type ReadState = 'ready' | 'hostUnavailable' | 'invalidRequest' | 'readFailed'

export type ReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type ReadError = {
  readonly code: ReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type ReadErrorState = Exclude<ReadState, 'ready'>

export type ReadCandidatesResult =
  | {
      readonly state: 'ready'
      readonly result: ContractReadTrackIdentityReviewCandidatesReply
    }
  | {
      readonly state: ReadErrorState
      readonly error: ReadError
    }
