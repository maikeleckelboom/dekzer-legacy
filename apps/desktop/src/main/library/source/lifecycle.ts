import { libraryControlChannels } from '../../../shared/library/boundary/controlPlane'
import type {
  ReadSourceLifecycleRequest as ContractReadSourceLifecycleRequest,
  SourceLifecycle
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'

import type {
  ReadSourceLifecycleErrorCode,
  ReadSourceLifecycleErrorState,
  ReadSourceLifecycleRequest,
  ReadSourceLifecycleResult,
  SourceLifecycleRecord
} from '../../../shared/library/source/lifecycle'

export type ReadSourceLifecycleIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<ReadSourceLifecycleResult>
  ): void
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export function registerReadSourceLifecycleIpc(
  ipcMain: ReadSourceLifecycleIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(libraryControlChannels.source.lifecycle, (_event, request) =>
    readSourceLifecycleThroughHost(host, request)
  )
}

export async function readSourceLifecycleThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadSourceLifecycleResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isReadSourceLifecycleResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isReadSourceLifecycleResult(client)) {
    return client
  }

  try {
    const reply = await client.readSourceLifecycle({
      sourceId: normalizedRequest.sourceId
    } satisfies ContractReadSourceLifecycleRequest)

    if (reply.lifecycle === null) {
      return createReadSourceLifecycleErrorResult(
        'notFound',
        'notFound',
        'The requested source lifecycle record was not found.'
      )
    }

    return {
      state: 'ready',
      lifecycle: mapLifecycle(reply.lifecycle)
    }
  } catch {
    return createReadSourceLifecycleErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read source lifecycle.'
    )
  }
}

function normalizeRequest(
  request: unknown
): ReadSourceLifecycleRequest | ReadSourceLifecycleResult {
  if (
    typeof request !== 'object' ||
    request === null ||
    !('sourceId' in request) ||
    !isPositiveOpaqueId(request.sourceId)
  ) {
    return createReadSourceLifecycleErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source lifecycle reads require a positive sourceId.'
    )
  }

  return {
    sourceId: request.sourceId
  }
}

function mapLifecycle(lifecycle: SourceLifecycle): SourceLifecycleRecord {
  return {
    sourceId: lifecycle.sourceId,
    sourceClass: lifecycle.sourceClass,
    isUserVisible: lifecycle.isUserVisible,
    mountStatus: lifecycle.mountStatus,
    accessState: lifecycle.accessState,
    ...(lifecycle.accessIssueKind === undefined
      ? {}
      : { accessIssueKind: lifecycle.accessIssueKind }),
    scanPhase: lifecycle.scanPhase,
    ...(lifecycle.scanIssueKind === undefined ? {} : { scanIssueKind: lifecycle.scanIssueKind }),
    ...(lifecycle.lastScanStartedAtMs === undefined
      ? {}
      : { lastScanStartedAtMs: lifecycle.lastScanStartedAtMs }),
    ...(lifecycle.lastScanFinishedAtMs === undefined
      ? {}
      : { lastScanFinishedAtMs: lifecycle.lastScanFinishedAtMs }),
    ...(lifecycle.lastSuccessfulScanAtMs === undefined
      ? {}
      : { lastSuccessfulScanAtMs: lifecycle.lastSuccessfulScanAtMs }),
    ...(lifecycle.lastSeenAtMs === undefined ? {} : { lastSeenAtMs: lifecycle.lastSeenAtMs }),
    updatedAtMs: lifecycle.updatedAtMs
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadSourceLifecycleResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadSourceLifecycleErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadSourceLifecycleResult {
  return createReadSourceLifecycleErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadSourceLifecycleErrorCode {
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

function createReadSourceLifecycleErrorResult(
  state: ReadSourceLifecycleErrorState,
  code: ReadSourceLifecycleErrorCode,
  message: string
): ReadSourceLifecycleResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isReadSourceLifecycleResult(value: unknown): value is ReadSourceLifecycleResult {
  return typeof value === 'object' && value !== null && 'state' in value
}
