import type {
  LocalRootAvailability,
  ReadLocalRootsRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'

import type {
  ReadLocalRootsErrorCode,
  ReadLocalRootsErrorState,
  ReadLocalRootsOutcome,
  LocalRoot
} from '../../../shared/library/roots/read'

export async function readLocalRootsThroughHost(
  host: LibraryBoundaryHost
): Promise<ReadLocalRootsOutcome> {
  const client = getStartedClient(host)

  if (isReadLocalRootsOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalRoots(null satisfies ReadLocalRootsRequest)

    return {
      state: 'read',
      roots: reply.roots.map(
        (root): LocalRoot => ({
          rootId: root.rootId,
          admittedRootPath: root.admittedRootPath,
          availability: mapContractAvailability(root.availability)
        })
      )
    }
  } catch {
    return createReadLocalRootsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read local roots.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalRootsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadLocalRootsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalRootsOutcome {
  return createReadLocalRootsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalRootsErrorCode {
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
    return 'The library boundary host is unavailable after startup failure'
  }

  switch (error.code) {
    case 'stopping':
      return 'The library boundary host is stopping'
    case 'stopped':
      return 'The library boundary host is stopped'
    default:
      return 'The library boundary host has not started yet'
  }
}

function createReadLocalRootsErrorResult(
  state: ReadLocalRootsErrorState,
  code: ReadLocalRootsErrorCode,
  message: string
): ReadLocalRootsOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isReadLocalRootsOutcome(value: unknown): value is ReadLocalRootsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}

function mapContractAvailability(availability: LocalRootAvailability): LocalRoot['availability'] {
  switch (availability) {
    case 'available':
      return 'available'
    case 'unavailable':
      return 'unavailable'
  }
}
