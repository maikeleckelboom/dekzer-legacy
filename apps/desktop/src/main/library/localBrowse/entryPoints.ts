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

export async function readLocalBrowseEntryPointsThroughHost(
  host: LibraryBoundaryHost
): Promise<ReadLocalBrowseEntryPointsOutcome> {
  const client = getStartedClient(host)

  if (isReadLocalBrowseEntryPointsOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalBrowseEntryPoints(
      null satisfies ReadLocalBrowseEntryPointsRequest
    )

    return {
      state: 'read',
      status: reply.status,
      entries: reply.entries.map(mapLocalBrowseEntryPoint),
      failure: reply.failure
    }
  } catch {
    return createReadLocalBrowseEntryPointsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read local browse entry points.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalBrowseEntryPointsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadLocalBrowseEntryPointsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowseEntryPointsOutcome {
  return createReadLocalBrowseEntryPointsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
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

function createReadLocalBrowseEntryPointsErrorResult(
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

function mapLocalBrowseEntryPoint(entry: ContractLocalBrowseEntryPoint): LocalBrowseEntryPoint {
  return {
    identity: entry.identity,
    displayName: entry.displayName,
    status: entry.status,
    platform: entry.platform,
    admissionAction: entry.admissionAction,
    availableActions: entry.availableActions,
    failure: entry.failure
  }
}

function isReadLocalBrowseEntryPointsOutcome(
  value: unknown
): value is ReadLocalBrowseEntryPointsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
