import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

export type ReadSourceActivityRequest = {
  readonly sourceId: string
}

export type SourceActivityReadState = 'ready' | 'hostUnavailable' | 'invalidRequest' | 'readFailed'

export type SourceActivityReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type SourceActivityReadError = {
  readonly code: SourceActivityReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type SourceActivityReadResult =
  | {
      readonly state: 'ready'
      readonly activity: ReadSourceActivityReply
    }
  | {
      readonly state: Exclude<SourceActivityReadState, 'ready'>
      readonly error: SourceActivityReadError
    }
