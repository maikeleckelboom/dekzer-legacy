import type { ReadSourceIntegrityReply } from '@dekzer/library-boundary-contract'

export type ReadSourceIntegrityRequest = {
  readonly sourceId: string
}

export type SourceIntegrityReadState = 'ready' | 'hostUnavailable' | 'invalidRequest' | 'readFailed'

export type SourceIntegrityReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type SourceIntegrityReadError = {
  readonly code: SourceIntegrityReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type SourceIntegrityReadResult =
  | {
      readonly state: 'ready'
      readonly integrity: ReadSourceIntegrityReply
    }
  | {
      readonly state: Exclude<SourceIntegrityReadState, 'ready'>
      readonly error: SourceIntegrityReadError
    }
