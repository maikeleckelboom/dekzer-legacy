import type { ReadSourceActivityRequest as ContractReadSourceActivityRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  ReadSourceActivityRequest,
  SourceActivityReadErrorCode,
  SourceActivityReadResult
} from '../../../shared/library/source/activity'

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export async function readSourceActivityThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<SourceActivityReadResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isSourceActivityReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isSourceActivityReadResult(client)) {
    return client
  }

  try {
    const reply = await client.readSourceActivity({
      sourceId: normalizedRequest.sourceId
    } satisfies ContractReadSourceActivityRequest)

    return {
      state: 'ready',
      activity: reply
    }
  } catch (error: unknown) {
    const { code, message, detail } = classifyReadError(error)
    return createErrorResult('readFailed', code, message, detail)
  }
}

function normalizeRequest(
  request: unknown
): ReadSourceActivityRequest | SourceActivityReadResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceId)) {
    return createErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source activity reads require a positive sourceId.'
    )
  }

  return {
    sourceId: request.sourceId
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | SourceActivityReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return createErrorResult(
        'hostUnavailable',
        hostErrorCode(host, error),
        hostErrorMessage(host, error)
      )
    }

    return createErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SourceActivityReadErrorCode {
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
  readonly code: SourceActivityReadErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    return {
      code: 'readFailed',
      message: 'Source activity read failed due to a protocol error.',
      detail: error.protocolError.payload?.detail ?? error.protocolError.type
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'readFailed',
      message: 'Source activity received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof Error) {
    return {
      code: 'readFailed',
      message: 'Unable to read source activity.',
      detail: error.message
    }
  }

  return {
    code: 'readFailed',
    message: 'Unable to read source activity.'
  }
}

function createErrorResult(
  state: Exclude<SourceActivityReadResult['state'], 'ready'>,
  code: SourceActivityReadErrorCode,
  message: string,
  detail?: string
): SourceActivityReadResult {
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

function isSourceActivityReadResult(value: unknown): value is SourceActivityReadResult {
  return (
    isRecord(value) &&
    typeof value.state === 'string' &&
    ['ready', 'hostUnavailable', 'invalidRequest', 'readFailed'].includes(value.state)
  )
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object'
}
