import type { StartRootScanRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { rootChannels } from '../../shared/libraryRoots/channels'
import type {
  LocalRootScanErrorCode,
  LocalRootScanErrorState,
  LocalRootScanRequest,
  LocalRootScanResult
} from '../../shared/libraryRoots/runScan'

export type ScanLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type LocalRootScanIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LocalRootScanResult>
  ): void
}

export function registerLocalRootScanIpc(
  ipcMain: LocalRootScanIpcMain,
  host: LibraryBoundaryHost,
  logger: ScanLogger = console
): void {
  ipcMain.handle(rootChannels.runScan, (_event, request) =>
    runLocalRootScanThroughHost(host, request, logger)
  )
}

export async function runLocalRootScanThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: ScanLogger = console
): Promise<LocalRootScanResult> {
  const normalizedRequest = normalizeLocalRootScanRequest(request)

  if (isLocalRootScanResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isLocalRootScanResult(client)) {
    return client
  }

  try {
    const reply = await client.startRootScan({
      rootId: normalizedRequest.rootId
    } satisfies StartRootScanRequest)

    return {
      state: 'started',
      scanRunId: reply.scanRunId
    }
  } catch (error: unknown) {
    logger.error('[local-root-scan] failed', { rootId: normalizedRequest.rootId, error })

    const { code, message, detail } = classifyScanError(error)

    return createLocalRootScanErrorResult('scanFailed', code, message, detail)
  }
}

function normalizeLocalRootScanRequest(
  request: unknown
): LocalRootScanRequest | LocalRootScanResult {
  if (!isRecord(request)) {
    return createLocalRootScanErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Local root scan requires a request object.'
    )
  }

  if (typeof request.rootId !== 'string' || request.rootId.trim().length === 0) {
    return createLocalRootScanErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Local root scan rootId is invalid.'
    )
  }

  return {
    rootId: request.rootId
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | LocalRootScanResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createLocalRootScanErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LocalRootScanResult {
  return createLocalRootScanErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LocalRootScanErrorCode {
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

function createLocalRootScanErrorResult(
  state: LocalRootScanErrorState,
  code: LocalRootScanErrorCode,
  message: string,
  detail?: string
): LocalRootScanResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function classifyScanError(error: unknown): {
  readonly code: LocalRootScanErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'scanFailed',
      message: 'Library root scan failed due to a protocol error.',
      ...(payload?.detail !== undefined
        ? { detail: payload.detail }
        : { detail: error.protocolError.type })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'scanFailed',
      message: 'Library root scan received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'scanFailed',
      message: 'Library root scan failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'scanFailed',
      message: 'Unable to run local library root scan.',
      detail: error.message
    }
  }

  if (typeof error === 'string' && error.length > 0) {
    return {
      code: 'scanFailed',
      message: 'Unable to run local library root scan.',
      detail: error.length > 200 ? error.slice(0, 200) : error
    }
  }

  return {
    code: 'scanFailed',
    message: 'Unable to run local library root scan.'
  }
}

function isLocalRootScanResult(value: unknown): value is LocalRootScanResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}
