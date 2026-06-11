import { libraryControlChannels } from '../../../shared/library/boundary/controlPlane'
import type { HashSourceFilesBlake3Request as ContractHashSourceFilesBlake3Request } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import {
  type HashSourceFilesBlake3ErrorCode,
  type HashSourceFilesBlake3ErrorState,
  type HashSourceFilesBlake3Request,
  type HashSourceFilesBlake3Result
} from '../../../shared/library/source/fileHashing'

export type SourceFileHashingLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type SourceFileHashingIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<HashSourceFilesBlake3Result>
  ): void
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export function registerSourceFileHashingIpc(
  ipcMain: SourceFileHashingIpcMain,
  host: LibraryBoundaryHost,
  logger: SourceFileHashingLogger = console
): void {
  ipcMain.handle(libraryControlChannels.source.fileHashing, (_event, request) =>
    hashSourceFilesBlake3ThroughHost(host, request, logger)
  )
}

export async function hashSourceFilesBlake3ThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: SourceFileHashingLogger = console
): Promise<HashSourceFilesBlake3Result> {
  const normalizedRequest = normalizeRequest(request)

  if (isHashSourceFilesBlake3Result(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isHashSourceFilesBlake3Result(client)) {
    return client
  }

  try {
    const reply = await client.hashSourceFilesBlake3({
      sourceId: normalizedRequest.sourceId,
      ...(normalizedRequest.limit === undefined ? {} : { limit: normalizedRequest.limit })
    } satisfies ContractHashSourceFilesBlake3Request)

    return {
      state: 'completed',
      result: reply
    }
  } catch (error: unknown) {
    logger.error('[source-file-hashing] failed', {
      sourceId: normalizedRequest.sourceId,
      error
    })

    const { code, message, detail } = classifyHashError(error)

    return createHashErrorResult('hashFailed', code, message, detail)
  }
}

function normalizeRequest(
  request: unknown
): HashSourceFilesBlake3Request | HashSourceFilesBlake3Result {
  if (!isRecord(request)) {
    return createHashErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source file hashing requires a request object.'
    )
  }

  if (!isPositiveOpaqueId(request.sourceId)) {
    return createHashErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source file hashing sourceId is invalid.'
    )
  }

  const limit = normalizeLimit(request.limit)

  if (isHashSourceFilesBlake3Result(limit)) {
    return limit
  }

  return {
    sourceId: request.sourceId,
    ...(limit === undefined ? {} : { limit })
  }
}

function normalizeLimit(value: unknown): number | undefined | HashSourceFilesBlake3Result {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value === 'number' && Number.isInteger(value) && value > 0) {
    return value
  }

  return createHashErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Source file hashing limit is invalid.'
  )
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | HashSourceFilesBlake3Result {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createHashErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): HashSourceFilesBlake3Result {
  return createHashErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): HashSourceFilesBlake3ErrorCode {
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

function classifyHashError(error: unknown): {
  readonly code: HashSourceFilesBlake3ErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'hashFailed',
      message: 'Source file hashing failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'hashFailed',
      message: 'Source file hashing received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'hashFailed',
      message: 'Source file hashing failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'hashFailed',
      message: 'Unable to hash source files.',
      detail: error.message
    }
  }

  return {
    code: 'hashFailed',
    message: 'Unable to hash source files.'
  }
}

function createHashErrorResult(
  state: HashSourceFilesBlake3ErrorState,
  code: HashSourceFilesBlake3ErrorCode,
  message: string,
  detail?: string
): HashSourceFilesBlake3Result {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isHashSourceFilesBlake3Result(value: unknown): value is HashSourceFilesBlake3Result {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
