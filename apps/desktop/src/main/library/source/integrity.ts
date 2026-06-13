import type { ReadSourceIntegrityRequest as ContractReadSourceIntegrityRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  ReadSourceIntegrityRequest,
  SourceIntegrityReadErrorCode,
  SourceIntegrityReadResult
} from '../../../shared/library/source/integrity'

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export async function readSourceIntegrityThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<SourceIntegrityReadResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isSourceIntegrityReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isSourceIntegrityReadResult(client)) {
    return client
  }

  try {
    const reply = await client.readSourceIntegrity({
      sourceId: normalizedRequest.sourceId
    } satisfies ContractReadSourceIntegrityRequest)

    return {
      state: 'ready',
      integrity: reply
    }
  } catch (error: unknown) {
    const { code, message, detail } = classifyReadError(error)
    return createErrorResult('readFailed', code, message, detail)
  }
}

function normalizeRequest(
  request: unknown
): ReadSourceIntegrityRequest | SourceIntegrityReadResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceId)) {
    return createErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source integrity reads require a positive sourceId.'
    )
  }

  return {
    sourceId: request.sourceId
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | SourceIntegrityReadResult {
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
): SourceIntegrityReadErrorCode {
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
  readonly code: SourceIntegrityReadErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    return {
      code: 'readFailed',
      message: 'Source integrity read failed due to a protocol error.',
      detail: error.protocolError.payload?.detail ?? error.protocolError.type
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'readFailed',
      message: 'Source integrity received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof Error) {
    return {
      code: 'readFailed',
      message: 'Unable to read source integrity.',
      detail: error.message
    }
  }

  return {
    code: 'readFailed',
    message: 'Unable to read source integrity.'
  }
}

function createErrorResult(
  state: Exclude<SourceIntegrityReadResult['state'], 'ready'>,
  code: SourceIntegrityReadErrorCode,
  message: string,
  detail?: string
): SourceIntegrityReadResult {
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

function isSourceIntegrityReadResult(value: unknown): value is SourceIntegrityReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
