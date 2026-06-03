import type {
  AcceptTrackIdentityCandidateRequest as ContractAcceptTrackIdentityCandidateRequest,
  DeferTrackIdentityCandidateRequest as ContractDeferTrackIdentityCandidateRequest,
  RejectTrackIdentityCandidateRequest as ContractRejectTrackIdentityCandidateRequest,
  TrackIdentityDecisionWriteResult as ContractTrackIdentityDecisionWriteResult
} from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  trackIdentityDecisionWriteChannels,
  type TrackIdentityDecisionWriteErrorCode,
  type TrackIdentityDecisionWriteErrorState,
  type TrackIdentityDecisionWriteRequest,
  type TrackIdentityDecisionWriteResult
} from '../../shared/libraryTrackIdentityDecisionWrite/decisionWrite'

export type TrackIdentityDecisionWriteLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type TrackIdentityDecisionWriteIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<TrackIdentityDecisionWriteResult>
  ): void
}

type DecisionWriteIntent = 'accept' | 'reject' | 'defer'

const positiveOpaqueIdPattern = /^[1-9]\d*$/
const maxReasonCharacters = 512

export function registerTrackIdentityDecisionWriteIpc(
  ipcMain: TrackIdentityDecisionWriteIpcMain,
  host: LibraryBoundaryHost,
  logger: TrackIdentityDecisionWriteLogger = console
): void {
  ipcMain.handle(
    trackIdentityDecisionWriteChannels.acceptTrackIdentityCandidate,
    (_event, request) => acceptTrackIdentityCandidateThroughHost(host, request, logger)
  )
  ipcMain.handle(
    trackIdentityDecisionWriteChannels.rejectTrackIdentityCandidate,
    (_event, request) => rejectTrackIdentityCandidateThroughHost(host, request, logger)
  )
  ipcMain.handle(
    trackIdentityDecisionWriteChannels.deferTrackIdentityCandidate,
    (_event, request) => deferTrackIdentityCandidateThroughHost(host, request, logger)
  )
}

export async function acceptTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionWriteLogger = console
): Promise<TrackIdentityDecisionWriteResult> {
  return writeTrackIdentityDecisionThroughHost(host, 'accept', request, logger)
}

export async function rejectTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionWriteLogger = console
): Promise<TrackIdentityDecisionWriteResult> {
  return writeTrackIdentityDecisionThroughHost(host, 'reject', request, logger)
}

export async function deferTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionWriteLogger = console
): Promise<TrackIdentityDecisionWriteResult> {
  return writeTrackIdentityDecisionThroughHost(host, 'defer', request, logger)
}

async function writeTrackIdentityDecisionThroughHost(
  host: LibraryBoundaryHost,
  intent: DecisionWriteIntent,
  request: unknown,
  logger: TrackIdentityDecisionWriteLogger
): Promise<TrackIdentityDecisionWriteResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isTrackIdentityDecisionWriteResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isTrackIdentityDecisionWriteResult(client)) {
    return client
  }

  try {
    const contractRequest = contractRequestFromNormalized(normalizedRequest)
    const result = await sendDecisionWrite(client, intent, contractRequest)

    return {
      state: 'completed',
      result
    }
  } catch (error: unknown) {
    logger.error('[track-identity-decision-write] failed', {
      intent,
      candidateId: normalizedRequest.candidateId,
      error
    })

    const { code, message, detail } = classifyWriteError(error)

    return createWriteErrorResult('writeFailed', code, message, detail)
  }
}

function normalizeRequest(
  request: unknown
): TrackIdentityDecisionWriteRequest | TrackIdentityDecisionWriteResult {
  if (!isRecord(request)) {
    return createWriteErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision writes require a request object.'
    )
  }

  if (!isPositiveOpaqueId(request.candidateId)) {
    return createWriteErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision writes require a positive candidateId.'
    )
  }

  const reason = normalizeReason(request.reason)

  if (isTrackIdentityDecisionWriteResult(reason)) {
    return reason
  }

  return {
    candidateId: request.candidateId,
    ...(reason === undefined ? {} : { reason })
  }
}

function normalizeReason(value: unknown): string | undefined | TrackIdentityDecisionWriteResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value !== 'string') {
    return createWriteErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision write reason must be text.'
    )
  }

  const trimmed = value.trim()
  if (trimmed.length === 0) {
    return undefined
  }

  return [...trimmed].slice(0, maxReasonCharacters).join('')
}

function contractRequestFromNormalized(
  request: TrackIdentityDecisionWriteRequest
):
  | ContractAcceptTrackIdentityCandidateRequest
  | ContractRejectTrackIdentityCandidateRequest
  | ContractDeferTrackIdentityCandidateRequest {
  return {
    candidateId: request.candidateId,
    ...(request.reason === undefined ? {} : { reason: request.reason })
  }
}

async function sendDecisionWrite(
  client: LibraryBoundaryHostClient,
  intent: DecisionWriteIntent,
  request:
    | ContractAcceptTrackIdentityCandidateRequest
    | ContractRejectTrackIdentityCandidateRequest
    | ContractDeferTrackIdentityCandidateRequest
): Promise<ContractTrackIdentityDecisionWriteResult> {
  switch (intent) {
    case 'accept':
      return client.acceptTrackIdentityCandidate(request)
    case 'reject':
      return client.rejectTrackIdentityCandidate(request)
    case 'defer':
      return client.deferTrackIdentityCandidate(request)
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | TrackIdentityDecisionWriteResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createWriteErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): TrackIdentityDecisionWriteResult {
  return createWriteErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): TrackIdentityDecisionWriteErrorCode {
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

function classifyWriteError(error: unknown): {
  readonly code: TrackIdentityDecisionWriteErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'writeFailed',
      message: 'Track identity decision write failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'writeFailed',
      message: 'Track identity decision write received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'writeFailed',
      message: 'Track identity decision write failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'writeFailed',
      message: 'Unable to write track identity decision.',
      detail: error.message
    }
  }

  return {
    code: 'writeFailed',
    message: 'Unable to write track identity decision.'
  }
}

function createWriteErrorResult(
  state: TrackIdentityDecisionWriteErrorState,
  code: TrackIdentityDecisionWriteErrorCode,
  message: string,
  detail?: string
): TrackIdentityDecisionWriteResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isTrackIdentityDecisionWriteResult(
  value: unknown
): value is TrackIdentityDecisionWriteResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
