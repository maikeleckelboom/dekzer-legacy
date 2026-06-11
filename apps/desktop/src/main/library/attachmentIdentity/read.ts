import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'
import type {
  ReadAttachmentSourceFilesRequest as ContractReadAttachmentSourceFilesRequest,
  ReadSourceAttachmentSummaryRequest as ContractReadSourceAttachmentSummaryRequest,
  ReadSourceFileAttachmentRequest as ContractReadSourceFileAttachmentRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'

import type {
  AttachmentIdentityReadResult,
  AttachmentIdentityReadErrorCode,
  AttachmentIdentityReadErrorState,
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from '../../../shared/library/attachmentIdentity/read'

const positiveOpaqueIdPattern = /^[1-9]\d*$/
const maxAttachmentSourceFilesLimit = 200

export async function readSourceFileAttachmentThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadSourceFileAttachmentResult> {
  const normalizedRequest = normalizeSourceFileRequest(request)

  if ('state' in normalizedRequest) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if ('state' in client) {
    return client
  }

  try {
    const reply = await client.readSourceFileAttachment({
      sourceFileId: normalizedRequest.sourceFileId
    } satisfies ContractReadSourceFileAttachmentRequest)

    return resultFromReplyStatus(reply)
  } catch (error: unknown) {
    return readFailedResult(error)
  }
}

export async function readAttachmentSourceFilesThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadAttachmentSourceFilesResult> {
  const normalizedRequest = normalizeAttachmentRequest(request)

  if ('state' in normalizedRequest) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if ('state' in client) {
    return client
  }

  try {
    const reply = await client.readAttachmentSourceFiles({
      attachmentId: normalizedRequest.attachmentId,
      ...(normalizedRequest.limit === undefined ? {} : { limit: normalizedRequest.limit })
    } satisfies ContractReadAttachmentSourceFilesRequest)

    return resultFromReplyStatus(reply)
  } catch (error: unknown) {
    return readFailedResult(error)
  }
}

export async function readSourceAttachmentSummaryThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadSourceAttachmentSummaryResult> {
  const normalizedRequest = normalizeSourceRequest(request)

  if ('state' in normalizedRequest) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if ('state' in client) {
    return client
  }

  try {
    const reply = await client.readSourceAttachmentSummary({
      sourceId: normalizedRequest.sourceId
    } satisfies ContractReadSourceAttachmentSummaryRequest)

    return resultFromReplyStatus(reply)
  } catch (error: unknown) {
    return readFailedResult(error)
  }
}

function normalizeSourceFileRequest(
  request: unknown
): ReadSourceFileAttachmentRequest | ReadSourceFileAttachmentResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceFileId)) {
    return invalidRequestResult('Attachment identity sourceFileId is invalid.')
  }

  return {
    sourceFileId: request.sourceFileId
  }
}

function normalizeAttachmentRequest(
  request: unknown
): ReadAttachmentSourceFilesRequest | ReadAttachmentSourceFilesResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.attachmentId)) {
    return invalidRequestResult('Attachment identity attachmentId is invalid.')
  }

  const limit = normalizeLimit(request.limit)

  if (limit !== undefined && typeof limit !== 'number') {
    return limit
  }

  return {
    attachmentId: request.attachmentId,
    ...(limit === undefined ? {} : { limit })
  }
}

function normalizeSourceRequest(
  request: unknown
): ReadSourceAttachmentSummaryRequest | ReadSourceAttachmentSummaryResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceId)) {
    return invalidRequestResult('Attachment identity sourceId is invalid.')
  }

  return {
    sourceId: request.sourceId
  }
}

function normalizeLimit(value: unknown): number | undefined | ReadAttachmentSourceFilesResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (
    typeof value === 'number' &&
    Number.isInteger(value) &&
    value > 0 &&
    value <= maxAttachmentSourceFilesLimit
  ) {
    return value
  }

  return invalidRequestResult('Attachment identity limit is invalid.')
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | AttachmentIdentityReadResult<never> {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createAttachmentReadErrorResult<never>(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): AttachmentIdentityReadResult<never> {
  return createAttachmentReadErrorResult<never>(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): AttachmentIdentityReadErrorCode {
  if (host.state === 'failed') {
    return 'hostFailed'
  }

  switch (error.code) {
    case 'invalidUserDataPath':
    case 'notStarted':
    case 'alreadyStarted':
    case 'missingDevelopmentBinary':
    case 'packagedBinaryUnavailable':
    case 'stdioTransportStartupFailure':
      return 'hostNotStarted'
    case 'stopping':
      return 'hostStopping'
    case 'stopped':
      return 'hostStopped'
  }
}

function hostErrorMessage(host: LibraryBoundaryHost, error: LibraryBoundaryHostError): string {
  if (host.state === 'failed') {
    return 'The library boundary host is unavailable after startup failure.'
  }

  switch (error.code) {
    case 'stopping':
      return 'The library boundary host is stopping.'
    case 'stopped':
      return 'The library boundary host is stopped.'
    default:
      return 'The library boundary host has not started yet.'
  }
}

function resultFromReplyStatus<Reply extends { readonly status: string }>(
  reply: Reply
): AttachmentIdentityReadResult<Reply> {
  if (reply.status === 'ok') {
    return {
      state: 'ok',
      reply
    }
  }

  if (reply.status === 'notFound') {
    return {
      state: 'notFound',
      reply
    }
  }

  return createAttachmentReadErrorResult<Reply>(
    reply.status === 'invalidRequest' ? 'invalidRequest' : 'readFailed',
    reply.status === 'invalidRequest' ? 'invalidRequest' : 'readFailed',
    'Unable to read attachment identity state.'
  )
}

function invalidRequestResult(message: string): AttachmentIdentityReadResult<never> {
  return createAttachmentReadErrorResult<never>('invalidRequest', 'invalidRequest', message)
}

function readFailedResult(error: unknown): AttachmentIdentityReadResult<never> {
  const { code, message, detail } = classifyReadError(error)
  return createAttachmentReadErrorResult<never>('readFailed', code, message, detail)
}

function classifyReadError(error: unknown): {
  readonly code: AttachmentIdentityReadErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: error.protocolError.type === 'invalidRequest' ? 'invalidRequest' : 'readFailed',
      message: 'Attachment identity read failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'readFailed',
      message: 'Attachment identity read received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'readFailed',
      message: 'Attachment identity read failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'readFailed',
      message: 'Unable to read attachment identity state.',
      detail: error.message
    }
  }

  return {
    code: 'readFailed',
    message: 'Unable to read attachment identity state.'
  }
}

function createAttachmentReadErrorResult<Reply = unknown>(
  state: AttachmentIdentityReadErrorState,
  code: AttachmentIdentityReadErrorCode,
  message: string,
  detail?: string
): AttachmentIdentityReadResult<Reply> {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
