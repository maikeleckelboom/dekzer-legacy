import { libraryControlChannels } from '../../../shared/library/boundary/controlPlane'
import type { CancelRootScanRequest as ContractCancelRequest } from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'

import type {
  CancelRootScanErrorCode,
  CancelRootScanErrorState,
  CancelRootScanRequest,
  CancelRootScanResult
} from '../../../shared/library/roots/cancel'

export type CancelScanLogger = {
  error(message?: unknown, ...optionalParams: unknown[]): void
}

export type CancelRootScanIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<CancelRootScanResult>
  ): void
}

export function registerCancelRootScanIpc(
  ipcMain: CancelRootScanIpcMain,
  host: LibraryBoundaryHost,
  logger: CancelScanLogger = console
): void {
  ipcMain.handle(libraryControlChannels.roots.cancel, (_event, request) =>
    cancelRootScanThroughHost(host, request, logger)
  )
}

export async function cancelRootScanThroughHost(
  host: LibraryBoundaryHost,
  request: unknown,
  logger: CancelScanLogger = console
): Promise<CancelRootScanResult> {
  const normalizedRequest = normalizeCancelRootScanRequest(request)

  if (isCancelRootScanErrorResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isCancelRootScanErrorResult(client)) {
    return client
  }

  try {
    const reply = await client.cancelRootScan({
      scanRunId: normalizedRequest.scanRunId
    } satisfies ContractCancelRequest)

    return {
      state: reply.status,
      status: reply.status
    }
  } catch (error: unknown) {
    logger.error('[local-root-cancel-scan] failed', {
      scanRunId: normalizedRequest.scanRunId,
      error
    })

    const { code, message, detail } = classifyCancelScanError(error)

    return createCancelRootScanErrorResult('cancelFailed', code, message, detail)
  }
}

function normalizeCancelRootScanRequest(
  request: unknown
): CancelRootScanRequest | CancelRootScanResult {
  if (!isRecord(request)) {
    return createCancelRootScanErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Cancel root scan requires a request object.'
    )
  }

  if (typeof request.scanRunId !== 'string' || request.scanRunId.trim().length === 0) {
    return createCancelRootScanErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Cancel root scan scanRunId is invalid.'
    )
  }

  return {
    scanRunId: request.scanRunId
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | CancelRootScanResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createCancelRootScanErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): CancelRootScanResult {
  return createCancelRootScanErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): CancelRootScanErrorCode {
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

function createCancelRootScanErrorResult(
  state: CancelRootScanErrorState,
  code: CancelRootScanErrorCode,
  message: string,
  detail?: string
): CancelRootScanResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function classifyCancelScanError(error: unknown): {
  readonly code: CancelRootScanErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'cancelFailed',
      message: 'Root scan cancellation failed due to a protocol error.',
      ...(payload?.detail !== undefined
        ? { detail: payload.detail }
        : { detail: error.protocolError.type })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'cancelFailed',
      message: 'Root scan cancellation received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'cancelFailed',
      message: 'Root scan cancellation failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'cancelFailed',
      message: 'Unable to cancel local root scan.',
      detail: error.message
    }
  }

  if (typeof error === 'string' && error.length > 0) {
    return {
      code: 'cancelFailed',
      message: 'Unable to cancel local root scan.',
      detail: error.length > 200 ? error.slice(0, 200) : error
    }
  }

  return {
    code: 'cancelFailed',
    message: 'Unable to cancel local root scan.'
  }
}

function isCancelRootScanErrorResult(value: unknown): value is CancelRootScanResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}
