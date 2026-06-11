import type { HashSourceFilesBlake3Reply } from '@dekzer/library-boundary-contract'

export type HashSourceFilesBlake3State =
  | 'completed'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'hashFailed'

export type HashSourceFilesBlake3ErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'hashFailed'

export type HashSourceFilesBlake3Error = {
  readonly code: HashSourceFilesBlake3ErrorCode
  readonly message: string
  readonly detail?: string
}

export type HashSourceFilesBlake3ErrorState = Exclude<HashSourceFilesBlake3State, 'completed'>

export type HashSourceFilesBlake3Request = {
  readonly sourceId: string
  readonly limit?: number
}

export type HashSourceFilesBlake3Result =
  | {
      readonly state: 'completed'
      readonly result: HashSourceFilesBlake3Reply
    }
  | {
      readonly state: HashSourceFilesBlake3ErrorState
      readonly error: HashSourceFilesBlake3Error
    }
