import type {
  AcceptTrackIdentityCandidateRequest as ContractAcceptTrackIdentityCandidateRequest,
  DeferTrackIdentityCandidateRequest as ContractDeferTrackIdentityCandidateRequest,
  RejectTrackIdentityCandidateRequest as ContractRejectTrackIdentityCandidateRequest,
  TrackIdentityDecisionCommandResult as ContractTrackIdentityDecisionCommandResult
} from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  trackIdentityDecisionChannels,
  type TrackIdentityDecisionCommandErrorCode,
  type TrackIdentityDecisionCommandErrorState,
  type TrackIdentityDecisionRequest,
  type TrackIdentityDecisionCommandResult
} from '../../shared/libraryTrackIdentityDecisions/decisionCommands'

export type TrackIdentityDecisionCommandLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type TrackIdentityDecisionCommandIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<TrackIdentityDecisionCommandResult>
  ): void
}

type DecisionCommandIntent = 'accept' | 'reject' | 'defer'

const positiveOpaqueIdPattern = /^[1-9]\d*$/
const maxReasonCharacters = 512

export function registerTrackIdentityDecisionIpc(
  ipcMain: TrackIdentityDecisionCommandIpcMain,
  host: LibraryBoundaryHost,
  logger: TrackIdentityDecisionCommandLogger = console
): void {
  ipcMain.handle(trackIdentityDecisionChannels.acceptTrackIdentityCandidate, (_event, request) =>
    acceptTrackIdentityCandidateThroughHost(host, request, logger)
  )
  ipcMain.handle(trackIdentityDecisionChannels.rejectTrackIdentityCandidate, (_event, request) =>
    rejectTrackIdentityCandidateThroughHost(host, request, logger)
  )
  ipcMain.handle(trackIdentityDecisionChannels.deferTrackIdentityCandidate, (_event, request) =>
    deferTrackIdentityCandidateThroughHost(host, request, logger)
  )
}

export async function acceptTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionCommandLogger = console
): Promise<TrackIdentityDecisionCommandResult> {
  return sendTrackIdentityDecisionCommandThroughHost(host, 'accept', request, logger)
}

export async function rejectTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionCommandLogger = console
): Promise<TrackIdentityDecisionCommandResult> {
  return sendTrackIdentityDecisionCommandThroughHost(host, 'reject', request, logger)
}

export async function deferTrackIdentityCandidateThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: TrackIdentityDecisionCommandLogger = console
): Promise<TrackIdentityDecisionCommandResult> {
  return sendTrackIdentityDecisionCommandThroughHost(host, 'defer', request, logger)
}

async function sendTrackIdentityDecisionCommandThroughHost(
  host: LibraryBoundaryHost,
  intent: DecisionCommandIntent,
  request: unknown,
  logger: TrackIdentityDecisionCommandLogger
): Promise<TrackIdentityDecisionCommandResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isTrackIdentityDecisionCommandResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isTrackIdentityDecisionCommandResult(client)) {
    return client
  }

  try {
    const contractRequest = contractRequestFromNormalized(normalizedRequest)
    const result = await sendDecisionCommand(client, intent, contractRequest)

    return {
      state: 'completed',
      result
    }
  } catch (error: unknown) {
    logger.error('[track-identity-decision-authority] failed', {
      intent,
      candidateId: normalizedRequest.candidateId,
      error
    })

    const { code, message, detail } = classifyCommandError(error)

    return createCommandErrorResult('commandFailed', code, message, detail)
  }
}

function normalizeRequest(
  request: unknown
): TrackIdentityDecisionRequest | TrackIdentityDecisionCommandResult {
  if (!isRecord(request)) {
    return createCommandErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision commands require a request object.'
    )
  }

  if (!isPositiveOpaqueId(request.candidateId)) {
    return createCommandErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision commands require a positive candidateId.'
    )
  }

  const reason = normalizeReason(request.reason)

  if (isTrackIdentityDecisionCommandResult(reason)) {
    return reason
  }

  return {
    candidateId: request.candidateId,
    ...(reason === undefined ? {} : { reason })
  }
}

function normalizeReason(value: unknown): string | undefined | TrackIdentityDecisionCommandResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value !== 'string') {
    return createCommandErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Track identity decision command reason must be text.'
    )
  }

  const trimmed = value.trim()
  if (trimmed.length === 0) {
    return undefined
  }

  return [...trimmed].slice(0, maxReasonCharacters).join('')
}

function contractRequestFromNormalized(
  request: TrackIdentityDecisionRequest
):
  | ContractAcceptTrackIdentityCandidateRequest
  | ContractRejectTrackIdentityCandidateRequest
  | ContractDeferTrackIdentityCandidateRequest {
  return {
    candidateId: request.candidateId,
    ...(request.reason === undefined ? {} : { reason: request.reason })
  }
}

async function sendDecisionCommand(
  client: LibraryBoundaryHostClient,
  intent: DecisionCommandIntent,
  request:
    | ContractAcceptTrackIdentityCandidateRequest
    | ContractRejectTrackIdentityCandidateRequest
    | ContractDeferTrackIdentityCandidateRequest
): Promise<ContractTrackIdentityDecisionCommandResult> {
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
): LibraryBoundaryHostClient | TrackIdentityDecisionCommandResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createCommandErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): TrackIdentityDecisionCommandResult {
  return createCommandErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): TrackIdentityDecisionCommandErrorCode {
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

function classifyCommandError(error: unknown): {
  readonly code: TrackIdentityDecisionCommandErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'commandFailed',
      message: 'Track identity decision command failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'commandFailed',
      message: 'Track identity decision command received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'commandFailed',
      message: 'Track identity decision command failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'commandFailed',
      message: 'Unable to run track identity decision command.',
      detail: error.message
    }
  }

  return {
    code: 'commandFailed',
    message: 'Unable to run track identity decision command.'
  }
}

function createCommandErrorResult(
  state: TrackIdentityDecisionCommandErrorState,
  code: TrackIdentityDecisionCommandErrorCode,
  message: string,
  detail?: string
): TrackIdentityDecisionCommandResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isTrackIdentityDecisionCommandResult(
  value: unknown
): value is TrackIdentityDecisionCommandResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
