import type {
  LocalBrowserEntryPoint as ContractLocalBrowserEntryPoint,
  ReadLocalBrowserEntryPointsRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  LocalBrowserEntryPoint,
  ReadLocalBrowserEntryPointsErrorCode,
  ReadLocalBrowserEntryPointsErrorState,
  ReadLocalBrowserEntryPointsOutcome
} from '../../../shared/library/localBrowser/entryPoints'

export async function readLocalBrowserEntryPointsThroughHost(
  host: LibraryBoundaryHost
): Promise<ReadLocalBrowserEntryPointsOutcome> {
  const client = getStartedClient(host)

  if (isReadLocalBrowserEntryPointsOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalBrowserEntryPoints(
      null satisfies ReadLocalBrowserEntryPointsRequest
    )

    return {
      state: 'read',
      status: reply.status,
      entries: reply.entries.map(mapLocalBrowserEntryPoint),
      failure: reply.failure
    }
  } catch {
    return createReadLocalBrowserEntryPointsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read local browser entry points.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalBrowserEntryPointsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadLocalBrowserEntryPointsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowserEntryPointsOutcome {
  return createReadLocalBrowserEntryPointsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowserEntryPointsErrorCode {
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

function createReadLocalBrowserEntryPointsErrorResult(
  state: ReadLocalBrowserEntryPointsErrorState,
  code: ReadLocalBrowserEntryPointsErrorCode,
  message: string
): ReadLocalBrowserEntryPointsOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapLocalBrowserEntryPoint(entry: ContractLocalBrowserEntryPoint): LocalBrowserEntryPoint {
  return {
    identity: entry.identity,
    displayName: entry.displayName,
    status: entry.status,
    platform: entry.platform,
    admissionHint: entry.admissionHint,
    affordances: entry.affordances,
    failure: entry.failure
  }
}

function isReadLocalBrowserEntryPointsOutcome(
  value: unknown
): value is ReadLocalBrowserEntryPointsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
