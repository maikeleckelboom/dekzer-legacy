import type { NavigationRow, ReadNavigationRowsRequest } from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  navigationReadChannels,
  type LibraryNavigationReadRowsErrorCode,
  type LibraryNavigationReadRowsErrorState,
  type LibraryNavigationReadRowsResult,
  type LibraryNavigationRow
} from '../../shared/libraryNavigation/readRows'

export type LibraryNavigationReadRowsIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LibraryNavigationReadRowsResult>
  ): void
}

const signedOpaqueIdPattern = /^-?[1-9]\d*$/

export function registerReadNavigationRowsIpc(
  ipcMain: LibraryNavigationReadRowsIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(navigationReadChannels.readRows, (_event, request) =>
    readNavigationRowsThroughHost(host, request)
  )
}

export async function readNavigationRowsThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<LibraryNavigationReadRowsResult> {
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

function normalizeRequest(
  request: unknown
): ReadNavigationRowsRequest | LibraryNavigationReadRowsResult {
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
): LibraryBoundaryHostClient | LibraryNavigationReadRowsResult {
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
): LibraryNavigationReadRowsResult {
  return createReadRowsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryNavigationReadRowsErrorCode {
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
  state: LibraryNavigationReadRowsErrorState,
  code: LibraryNavigationReadRowsErrorCode,
  message: string
): LibraryNavigationReadRowsResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function mapNavigationRow(row: NavigationRow): LibraryNavigationRow {
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

function isReadRowsResult(value: unknown): value is LibraryNavigationReadRowsResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
