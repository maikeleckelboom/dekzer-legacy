import type {
  LocalBrowserChildRow as ContractLocalBrowserChildRow,
  ReadLocalBrowserChildrenRequest as ContractReadLocalBrowserChildrenRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  LocalBrowserChildRow,
  ReadLocalBrowserChildrenErrorCode,
  ReadLocalBrowserChildrenErrorState,
  ReadLocalBrowserChildrenOutcome,
  ReadLocalBrowserChildrenRequest
} from '../../../shared/library/localBrowser/children'

export async function readLocalBrowserChildrenThroughHost(
  host: LibraryBoundaryHost,
  request: ReadLocalBrowserChildrenRequest
): Promise<ReadLocalBrowserChildrenOutcome> {
  const client = getStartedClient(host)

  if (isReadLocalBrowserChildrenOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalBrowserChildren(
      request satisfies ContractReadLocalBrowserChildrenRequest
    )

    return {
      state: 'read',
      status: reply.status,
      windowIdentity: reply.windowIdentity,
      offset: reply.offset,
      limit: reply.limit,
      totalRows: reply.totalRows,
      rows: reply.rows.map(mapLocalBrowserChildRow),
      failure: reply.failure
    }
  } catch {
    return createReadLocalBrowserChildrenErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read local browser children.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalBrowserChildrenOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadLocalBrowserChildrenErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowserChildrenOutcome {
  return createReadLocalBrowserChildrenErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowserChildrenErrorCode {
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

function createReadLocalBrowserChildrenErrorResult(
  state: ReadLocalBrowserChildrenErrorState,
  code: ReadLocalBrowserChildrenErrorCode,
  message: string
): ReadLocalBrowserChildrenOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapLocalBrowserChildRow(row: ContractLocalBrowserChildRow): LocalBrowserChildRow {
  return {
    identity: row.identity,
    rowKind: row.rowKind,
    displayName: row.displayName,
    status: row.status,
    platform: row.platform,
    fileKind: row.fileKind,
    mediaRelevance: row.mediaRelevance,
    admissionHint: row.admissionHint,
    affordances: row.affordances,
    failure: row.failure
  }
}

function isReadLocalBrowserChildrenOutcome(
  value: unknown
): value is ReadLocalBrowserChildrenOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
