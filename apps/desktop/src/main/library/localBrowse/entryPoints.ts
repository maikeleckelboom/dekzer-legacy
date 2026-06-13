import type {
  LocalBrowseEntryPoint as ContractLocalBrowseEntryPoint,
  ReadLocalBrowseEntryPointsRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  LocalBrowseEntryPoint,
  ReadLocalBrowseEntryPointsErrorCode,
  ReadLocalBrowseEntryPointsErrorState,
  ReadLocalBrowseEntryPointsOutcome
} from '../../../shared/library/localBrowse/entryPoints'

export async function readEntryPoints(
  host: LibraryBoundaryHost
): Promise<ReadLocalBrowseEntryPointsOutcome> {
  const client = startedClient(host)

  if (isOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalBrowseEntryPoints(
      null satisfies ReadLocalBrowseEntryPointsRequest
    )

    return {
      state: 'read',
      status: reply.status,
      entries: reply.entries.map(mapEntryPoint),
      failure: reply.failure
    }
  } catch {
    return errorResult('readFailed', 'readFailed', 'Unable to read local browse entry points.')
  }
}

function startedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalBrowseEntryPointsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostError(host, error)
    }

    return errorResult('hostUnavailable', 'hostFailed', 'The library boundary host is unavailable.')
  }
}

function hostError(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowseEntryPointsOutcome {
  return errorResult('hostUnavailable', hostErrorCode(host, error), hostErrorMessage(host, error))
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowseEntryPointsErrorCode {
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

function errorResult(
  state: ReadLocalBrowseEntryPointsErrorState,
  code: ReadLocalBrowseEntryPointsErrorCode,
  message: string
): ReadLocalBrowseEntryPointsOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapEntryPoint(entry: ContractLocalBrowseEntryPoint): LocalBrowseEntryPoint {
  return {
    identity: entry.identity,
    displayName: entry.displayName,
    status: entry.status,
    platform: entry.platform,
    availableOperations: entry.availableOperations,
    failure: entry.failure
  }
}

function isOutcome(value: unknown): value is ReadLocalBrowseEntryPointsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
