import type { ReadRegisteredLocalRootsRequest } from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { rootChannels } from '../../shared/libraryRoots/channels'
import type {
  ReadRegisteredLocalRootsErrorCode,
  ReadRegisteredLocalRootsErrorState,
  ReadRegisteredLocalRootsOutcome,
  RegisteredLocalRoot
} from '../../shared/libraryRoots/readRegisteredRoots'

export type ReadRegisteredLocalRootsIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown) => Promise<ReadRegisteredLocalRootsOutcome>
  ): void
}

export function registerReadRegisteredLocalRootsIpc(
  ipcMain: ReadRegisteredLocalRootsIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(rootChannels.readRegisteredRoots, () => readRegisteredLocalRootsThroughHost(host))
}

export async function readRegisteredLocalRootsThroughHost(
  host: LibraryBoundaryHost
): Promise<ReadRegisteredLocalRootsOutcome> {
  const client = getStartedClient(host)

  if (isReadRegisteredLocalRootsOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readRegisteredLocalRoots(
      null satisfies ReadRegisteredLocalRootsRequest
    )

    return {
      state: 'read',
      roots: reply.roots.map(
        (root): RegisteredLocalRoot => ({
          rootId: root.rootId,
          canonicalPath: root.canonicalPath
        })
      )
    }
  } catch {
    return createReadRegisteredLocalRootsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read registered local roots.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadRegisteredLocalRootsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadRegisteredLocalRootsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadRegisteredLocalRootsOutcome {
  return createReadRegisteredLocalRootsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadRegisteredLocalRootsErrorCode {
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

function createReadRegisteredLocalRootsErrorResult(
  state: ReadRegisteredLocalRootsErrorState,
  code: ReadRegisteredLocalRootsErrorCode,
  message: string
): ReadRegisteredLocalRootsOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isReadRegisteredLocalRootsOutcome(
  value: unknown
): value is ReadRegisteredLocalRootsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
