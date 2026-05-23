import type { UnregisterLocalRootRequest as ContractRequest } from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { rootChannels } from '../../shared/libraryRoots/channels'
import type {
  UnregisterLocalRootErrorCode,
  UnregisterLocalRootErrorState,
  UnregisterLocalRootRequest,
  UnregisterLocalRootResult
} from '../../shared/libraryRoots/unregisterLocalRoot'

export type UnregisterLocalRootIpcMain = {
  handle(
    channel: string,
    listener: (
      event: unknown,
      request: UnregisterLocalRootRequest
    ) => Promise<UnregisterLocalRootResult>
  ): void
}

export function registerUnregisterLocalRootIpc(
  ipcMain: UnregisterLocalRootIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(rootChannels.unregisterLocalRoot, (_event, request) =>
    unregisterLocalRootThroughHost(host, request)
  )
}

export async function unregisterLocalRootThroughHost(
  host: LibraryBoundaryHost,
  request: UnregisterLocalRootRequest
): Promise<UnregisterLocalRootResult> {
  const client = getStartedClient(host)

  if (isUnregisterLocalRootResult(client)) {
    return client
  }

  try {
    const reply = await client.unregisterLocalRoot({
      rootId: request.rootId
    } satisfies ContractRequest)

    return {
      state: 'unregistered',
      unregistered: reply.unregistered
    }
  } catch {
    return createUnregisterLocalRootErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Unable to unregister local root.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | UnregisterLocalRootResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createUnregisterLocalRootErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): UnregisterLocalRootResult {
  return createUnregisterLocalRootErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): UnregisterLocalRootErrorCode {
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

function createUnregisterLocalRootErrorResult(
  state: UnregisterLocalRootErrorState,
  code: UnregisterLocalRootErrorCode,
  message: string
): UnregisterLocalRootResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isUnregisterLocalRootResult(value: unknown): value is UnregisterLocalRootResult {
  return typeof value === 'object' && value !== null && 'state' in value
}
