import type {
  LocalBrowseItem as ContractLocalBrowseItem,
  ReadLocalBrowseItemsRequest as ContractReadLocalBrowseItemsRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type {
  LocalBrowseItem,
  ReadLocalBrowseItemsErrorCode,
  ReadLocalBrowseItemsErrorState,
  ReadLocalBrowseItemsOutcome,
  ReadLocalBrowseItemsRequest
} from '../../../shared/library/localBrowse/items'

export async function readLocalBrowseItemsThroughHost(
  host: LibraryBoundaryHost,
  request: ReadLocalBrowseItemsRequest
): Promise<ReadLocalBrowseItemsOutcome> {
  const client = getStartedClient(host)

  if (isReadLocalBrowseItemsOutcome(client)) {
    return client
  }

  try {
    const reply = await client.readLocalBrowseItems(
      request satisfies ContractReadLocalBrowseItemsRequest
    )

    return {
      state: 'read',
      status: reply.status,
      windowIdentity: reply.windowIdentity,
      offset: reply.offset,
      limit: reply.limit,
      totalItems: reply.totalItems,
      items: reply.items.map(mapLocalBrowseItem),
      failure: reply.failure
    }
  } catch {
    return createReadLocalBrowseItemsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read local browse items.'
    )
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadLocalBrowseItemsOutcome {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadLocalBrowseItemsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowseItemsOutcome {
  return createReadLocalBrowseItemsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadLocalBrowseItemsErrorCode {
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

function createReadLocalBrowseItemsErrorResult(
  state: ReadLocalBrowseItemsErrorState,
  code: ReadLocalBrowseItemsErrorCode,
  message: string
): ReadLocalBrowseItemsOutcome {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapLocalBrowseItem(item: ContractLocalBrowseItem): LocalBrowseItem {
  return {
    identity: item.identity,
    itemKind: item.itemKind,
    displayName: item.displayName,
    status: item.status,
    platform: item.platform,
    fileKind: item.fileKind,
    mediaRelevance: item.mediaRelevance,
    admissionAction: item.admissionAction,
    availableActions: item.availableActions,
    failure: item.failure
  }
}

function isReadLocalBrowseItemsOutcome(value: unknown): value is ReadLocalBrowseItemsOutcome {
  return typeof value === 'object' && value !== null && 'state' in value
}
