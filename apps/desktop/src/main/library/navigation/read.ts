import type {
  NavigationRow as ContractNavigationRow,
  ReadNavigationRowsRequest
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import {
  type NavigationReadRowsErrorCode,
  type NavigationReadRowsErrorState,
  type NavigationReadRowsResult,
  type NavigationRow
} from '../../../shared/library/navigation/read'

const signedOpaqueIdPattern = /^-?[1-9]\d*$/

export async function readNavigationRowsThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<NavigationReadRowsResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isReadRowsResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isReadRowsResult(client)) {
    return client
  }

  try {
    const reply = await client.readNavigationRows({
      parentNavigationRowId: normalizedRequest.parentNavigationRowId
    } satisfies ReadNavigationRowsRequest)

    return {
      state: 'ready',
      rows: reply.rows.map(mapNavigationRow)
    }
  } catch {
    return createReadRowsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read library navigation rows.'
    )
  }
}

function normalizeRequest(request: unknown): ReadNavigationRowsRequest | NavigationReadRowsResult {
  if (!isRecord(request)) {
    return createReadRowsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Library navigation reads require a request object.'
    )
  }

  const parentNavigationRowId = request.parentNavigationRowId

  if (parentNavigationRowId === null || parentNavigationRowId === undefined) {
    return {
      parentNavigationRowId: null
    }
  }

  if (
    typeof parentNavigationRowId === 'string' &&
    signedOpaqueIdPattern.test(parentNavigationRowId)
  ) {
    return {
      parentNavigationRowId
    }
  }

  return createReadRowsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library navigation parentNavigationRowId is invalid.'
  )
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | NavigationReadRowsResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createReadRowsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): NavigationReadRowsResult {
  return createReadRowsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): NavigationReadRowsErrorCode {
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

function createReadRowsErrorResult(
  state: NavigationReadRowsErrorState,
  code: NavigationReadRowsErrorCode,
  message: string
): NavigationReadRowsResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapNavigationRow(row: ContractNavigationRow): NavigationRow {
  return {
    navigationRowId: row.navigationRowId,
    stableKey: row.stableKey,
    parentNavigationRowId: row.parentNavigationRowId,
    family: row.family,
    rowKind: row.rowKind,
    displayName: row.displayName,
    siblingPosition: row.siblingPosition,
    selectable: row.selectable,
    selectorKind: row.selectorKind,
    selectorPayload: row.selectorPayload,
    updatedAtMs: row.updatedAtMs,
    rowVersion: row.rowVersion
  }
}

function isReadRowsResult(value: unknown): value is NavigationReadRowsResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
