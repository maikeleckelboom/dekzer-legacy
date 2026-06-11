import type {
  SearchFilterReadRequest,
  SearchFilterReadResult,
  SearchFilterReadErrorCode,
  SearchFilterReadErrorState
} from '../../../shared/library/searchFilter/read'
import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'

export async function readSearchFilterThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<SearchFilterReadResult> {
  if (!isSearchFilterReadRequest(request)) {
    return createSearchFilterErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Search/filter reads require a protocol request object.'
    )
  }

  const client = getStartedClient(host)

  if (isSearchFilterReadResult(client)) {
    return client
  }

  try {
    return {
      state: 'ready',
      reply: await client.readSearchFilter(request)
    }
  } catch {
    return createSearchFilterErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read library search/filter results.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | SearchFilterReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createSearchFilterErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SearchFilterReadResult {
  return createSearchFilterErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SearchFilterReadErrorCode {
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

function createSearchFilterErrorResult(
  state: SearchFilterReadErrorState,
  code: SearchFilterReadErrorCode,
  message: string
): SearchFilterReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isSearchFilterReadResult(value: unknown): value is SearchFilterReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isSearchFilterReadRequest(value: unknown): value is SearchFilterReadRequest {
  return isRecord(value) && isRecord(value.scope) && value.filters !== undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
