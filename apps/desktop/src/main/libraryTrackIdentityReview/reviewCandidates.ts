import type { ReadTrackIdentityReviewCandidatesRequest as ContractReadTrackIdentityReviewCandidatesRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  trackIdentityReviewCandidateChannels,
  type ReadTrackIdentityReviewCandidatesRequest,
  type ReadTrackIdentityReviewCandidatesResult,
  type TrackIdentityReviewCandidatesReadErrorCode,
  type TrackIdentityReviewCandidatesReadErrorState
} from '../../shared/libraryTrackIdentityReview/reviewCandidates'

export type TrackIdentityReviewCandidatesIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<ReadTrackIdentityReviewCandidatesResult>
  ): void
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/
const reviewStateValues = new Set([
  'all',
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
  ipcMain.handle(
    trackIdentityReviewCandidateChannels.readTrackIdentityReviewCandidates,
    (_event, request) => readTrackIdentityReviewCandidatesThroughHost(host, request)
  )
}

export async function readTrackIdentityReviewCandidatesThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadTrackIdentityReviewCandidatesResult> {
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

function normalizeRequest(
  request: unknown
): ReadTrackIdentityReviewCandidatesRequest | ReadTrackIdentityReviewCandidatesResult {
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
    !isReviewStateFilter(request.reviewState)
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
): LibraryBoundaryHostClient | ReadTrackIdentityReviewCandidatesResult {
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

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): TrackIdentityReviewCandidatesReadErrorCode {
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
  readonly code: TrackIdentityReviewCandidatesReadErrorCode
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
  state: TrackIdentityReviewCandidatesReadErrorState,
  code: TrackIdentityReviewCandidatesReadErrorCode,
  message: string,
  detail?: string
): ReadTrackIdentityReviewCandidatesResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isReviewStateFilter(
  value: unknown
): value is ReadTrackIdentityReviewCandidatesRequest['reviewState'] {
  return typeof value === 'string' && reviewStateValues.has(value)
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value > 0
}

function isReadResult(value: unknown): value is ReadTrackIdentityReviewCandidatesResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
