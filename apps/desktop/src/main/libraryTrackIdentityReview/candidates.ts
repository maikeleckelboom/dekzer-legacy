import type { ReadTrackIdentityReviewCandidatesRequest as ContractReadTrackIdentityReviewCandidatesRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  channels,
  type ReadCandidatesRequest,
  type ReadCandidatesResult,
  type ReadErrorCode,
  type ReadErrorState
} from '../../shared/libraryTrackIdentityReview/candidates'

export type TrackIdentityReviewCandidatesIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<ReadCandidatesResult>
  ): void
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/
const reviewStateValues = new Set([
  'needsUserDecision',
  'systemAccepted',
  'userAccepted',
  'userRejected',
  'userDeferred',
  'staleDecision'
])

export function registerTrackIdentityReviewCandidatesIpc(
  ipcMain: TrackIdentityReviewCandidatesIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(channels.readCandidates, (_event, request) =>
    readCandidatesThroughHost(host, request)
  )
}

export async function readCandidatesThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadCandidatesResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isReadResult(client)) {
    return client
  }

  try {
    const reply = await client.readTrackIdentityReviewCandidates({
      ...(normalizedRequest.sourceId === undefined ? {} : { sourceId: normalizedRequest.sourceId }),
      ...(normalizedRequest.reviewState === undefined
        ? {}
        : { reviewState: normalizedRequest.reviewState }),
      limit: normalizedRequest.limit
    } satisfies ContractReadTrackIdentityReviewCandidatesRequest)

    return {
      state: 'ready',
      result: reply
    }
  } catch (error: unknown) {
    const { code, message, detail } = classifyReadError(error)
    return createReadErrorResult('readFailed', code, message, detail)
  }
}

function normalizeRequest(request: unknown): ReadCandidatesRequest | ReadCandidatesResult {
  if (!isRecord(request)) {
    return createReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity review reads require a request object.'
    )
  }

  if (
    request.sourceId !== undefined &&
    request.sourceId !== null &&
    !isPositiveOpaqueId(request.sourceId)
  ) {
    return createReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity review sourceId must be a positive id.'
    )
  }

  if (!isPositiveInteger(request.limit)) {
    return createReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity review reads require a positive limit.'
    )
  }

  if (
    request.reviewState !== undefined &&
    request.reviewState !== null &&
    !isReviewState(request.reviewState)
  ) {
    return createReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity reviewState is invalid.'
    )
  }

  return {
    ...(request.sourceId === undefined || request.sourceId === null
      ? {}
      : { sourceId: request.sourceId }),
    ...(request.reviewState === undefined || request.reviewState === null
      ? {}
      : { reviewState: request.reviewState }),
    limit: request.limit
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadCandidatesResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return createReadErrorResult(
        'hostUnavailable',
        hostErrorCode(host, error),
        hostErrorMessage(host, error)
      )
    }

    return createReadErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostErrorCode(host: LibraryBoundaryHost, error: LibraryBoundaryHostError): ReadErrorCode {
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

function classifyReadError(error: unknown): {
  readonly code: ReadErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'readFailed',
      message: 'Track identity review read failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'readFailed',
      message: 'Track identity review read received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'readFailed',
      message: 'Track identity review read failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'readFailed',
      message: 'Unable to read track identity review candidates.',
      detail: error.message
    }
  }

  return {
    code: 'readFailed',
    message: 'Unable to read track identity review candidates.'
  }
}

function createReadErrorResult(
  state: ReadErrorState,
  code: ReadErrorCode,
  message: string,
  detail?: string
): ReadCandidatesResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isReviewState(value: unknown): value is ReadCandidatesRequest['reviewState'] {
  return typeof value === 'string' && reviewStateValues.has(value)
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0
}

function isReadResult(value: unknown): value is ReadCandidatesResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
