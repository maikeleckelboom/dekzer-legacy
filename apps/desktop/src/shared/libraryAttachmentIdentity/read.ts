import type {
  ReadAttachmentSourceFilesReply,
  ReadSourceAttachmentSummaryReply,
  ReadSourceFileAttachmentReply
} from '@dekzer/library-boundary-contract'

export type AttachmentIdentityReadState =
  | 'ok'
  | 'notFound'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'readFailed'

export type AttachmentIdentityReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type AttachmentIdentityReadError = {
  readonly code: AttachmentIdentityReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type AttachmentIdentityReadErrorState = Exclude<
  AttachmentIdentityReadState,
  'ok' | 'notFound'
>

export type ReadSourceFileAttachmentRequest = {
  readonly sourceFileId: string
}

export type ReadAttachmentSourceFilesRequest = {
  readonly attachmentId: string
  readonly limit?: number
}

export type ReadSourceAttachmentSummaryRequest = {
  readonly sourceId: string
}

export type AttachmentIdentityReadResult<Reply> =
  | {
      readonly state: 'ok'
      readonly reply: Reply
    }
  | {
      readonly state: 'notFound'
      readonly reply: Reply
    }
  | {
      readonly state: AttachmentIdentityReadErrorState
      readonly error: AttachmentIdentityReadError
    }

export type ReadSourceFileAttachmentResult =
  AttachmentIdentityReadResult<ReadSourceFileAttachmentReply>

export type ReadAttachmentSourceFilesResult =
  AttachmentIdentityReadResult<ReadAttachmentSourceFilesReply>

export type ReadSourceAttachmentSummaryResult =
  AttachmentIdentityReadResult<ReadSourceAttachmentSummaryReply>
