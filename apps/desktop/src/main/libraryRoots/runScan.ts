import type { RunRootScanRequest } from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { rootChannels } from '../../shared/libraryRoots/channels'
import type {
  LocalRootScanErrorCode,
  LocalRootScanErrorState,
  LocalRootScanRequest,
  LocalRootScanResult
} from '../../shared/libraryRoots/runScan'

export type LocalRootScanIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LocalRootScanResult>
  ): void
}

export function registerLocalRootScanIpc(
  ipcMain: LocalRootScanIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(rootChannels.runScan, (_event, request) =>
    runLocalRootScanThroughHost(host, request)
  )
}

export async function runLocalRootScanThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
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
    const reply = await client.runRootScan({
      rootId: normalizedRequest.rootId
    } satisfies RunRootScanRequest)

    return {
      state: 'scanned',
      rootId: reply.rootId,
      scanRunId: reply.scanRunId,
      discoveredFileCount: reply.discoveredFileCount,
      queuedSourceWorkItems: reply.queuedSourceWorkItems
    }
  } catch {
    return createLocalRootScanErrorResult(
      'scanFailed',
      'scanFailed',
      'Unable to run local library root scan.'
    )
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
  message: string
): LocalRootScanResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isLocalRootScanResult(value: unknown): value is LocalRootScanResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}
