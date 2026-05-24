import type {
  ReadSelectedContentsRequest,
  SelectedContentsResult as ContractSelectedContentsResult,
  SelectedContentsScope as ContractSelectedContentsScope,
  SelectedContentsRow as ContractSelectedContentsRow
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  selectedContentsReadChannels,
  type SelectedContentsCoverage,
  type SelectedContentsReadErrorCode,
  type SelectedContentsReadErrorState,
  type SelectedContentsReadResult,
  type SelectedContentsRequest,
  type SelectedContentsResult,
  type SelectedContentsRow,
  type SelectedContentsScope
} from '../../shared/librarySelectedContents/read'

export type LibrarySelectedContentsReadIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<SelectedContentsReadResult>
  ): void
}

const defaultSelectedContentsLimit = 100
const maxSelectedContentsLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export function registerSelectedContentsReadIpc(
  ipcMain: LibrarySelectedContentsReadIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(selectedContentsReadChannels.read, (_event, request) =>
    readSelectedContentsThroughHost(host, request)
  )
}

export async function readSelectedContentsThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<SelectedContentsReadResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isSelectedContentsReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isSelectedContentsReadResult(client)) {
    return client
  }

  try {
    const reply = await client.readSelectedContents({
      scope: mapScopeToContract(normalizedRequest.scope),
      limit: normalizedRequest.limit,
      ...(normalizedRequest.cursor === undefined ? {} : { cursor: normalizedRequest.cursor })
    } satisfies ReadSelectedContentsRequest)

    const result = mapSelectedContentsResult(reply.result)

    if (result === undefined) {
      return createSelectedContentsErrorResult(
        'readFailed',
        'readFailed',
        'Unable to read selected library contents.'
      )
    }

    return {
      state: 'ready',
      result
    }
  } catch {
    return createSelectedContentsErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read selected library contents.'
    )
  }
}

type NormalizedRequest = {
  readonly scope: SelectedContentsScope
  readonly limit: number
  readonly cursor?: string
}

function normalizeRequest(request: unknown): NormalizedRequest | SelectedContentsReadResult {
  if (!isRecord(request)) {
    return createSelectedContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Selected contents reads require a request object.'
    )
  }

  const scope = normalizeScope(request.scope)

  if (isSelectedContentsReadResult(scope)) {
    return scope
  }

  const limit = normalizeLimit(request.limit)

  if (isSelectedContentsReadResult(limit)) {
    return limit
  }

  const cursor = normalizeCursor(request.cursor)

  if (isSelectedContentsReadResult(cursor)) {
    return cursor
  }

  return {
    scope,
    limit,
    ...(cursor === undefined ? {} : { cursor })
  }
}

function normalizeScope(value: unknown): SelectedContentsScope | SelectedContentsReadResult {
  if (value === null || value === undefined) {
    return createSelectedContentsErrorResult(
      'noTarget',
      'noTarget',
      'No selected contents scope was provided.'
    )
  }

  if (!isRecord(value) || typeof value.kind !== 'string') {
    return createSelectedContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Selected contents scope is invalid.'
    )
  }

  if (value.kind === 'source' && isPositiveOpaqueId(value.sourceId)) {
    return {
      kind: 'source',
      sourceId: value.sourceId
    }
  }

  if (value.kind === 'sourceLocation' && isPositiveOpaqueId(value.sourceLocationId)) {
    return {
      kind: 'sourceLocation',
      sourceLocationId: value.sourceLocationId
    }
  }

  if (
    value.kind === 'directory' &&
    isPositiveOpaqueId(value.sourceId) &&
    isPositiveOpaqueId(value.sourceDirectoryId)
  ) {
    return {
      kind: 'directory',
      sourceId: value.sourceId,
      sourceDirectoryId: value.sourceDirectoryId
    }
  }

  return createSelectedContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Selected contents scope is invalid.'
  )
}

function normalizeLimit(value: unknown): number | SelectedContentsReadResult {
  if (value === undefined) {
    return defaultSelectedContentsLimit
  }

  if (
    Number.isInteger(value) &&
    typeof value === 'number' &&
    value > 0 &&
    value <= maxSelectedContentsLimit
  ) {
    return value
  }

  return createSelectedContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Selected contents limit is invalid.'
  )
}

function normalizeCursor(value: unknown): string | undefined | SelectedContentsReadResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value === 'string' && value.trim().length > 0) {
    return value
  }

  return createSelectedContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Selected contents cursor is invalid.'
  )
}

function mapScopeToContract(scope: SelectedContentsScope): ContractSelectedContentsScope {
  switch (scope.kind) {
    case 'source':
      return {
        type: 'source',
        payload: {
          sourceId: scope.sourceId
        }
      }
    case 'sourceLocation':
      return {
        type: 'sourceLocation',
        payload: {
          sourceLocationId: scope.sourceLocationId
        }
      }
    case 'directory':
      return {
        type: 'directory',
        payload: {
          sourceId: scope.sourceId,
          sourceDirectoryId: scope.sourceDirectoryId
        }
      }
  }
}

function mapScopeFromContract(scope: ContractSelectedContentsScope): SelectedContentsScope {
  switch (scope.type) {
    case 'source':
      return {
        kind: 'source',
        sourceId: scope.payload.sourceId
      }
    case 'sourceLocation':
      return {
        kind: 'sourceLocation',
        sourceLocationId: scope.payload.sourceLocationId
      }
    case 'directory':
      return {
        kind: 'directory',
        sourceId: scope.payload.sourceId,
        sourceDirectoryId: scope.payload.sourceDirectoryId
      }
  }
}

function mapSelectedContentsResult(
  result: ContractSelectedContentsResult
): SelectedContentsResult | undefined {
  const rows: SelectedContentsRow[] = []

  for (const row of result.rows) {
    const mapped = mapSelectedContentsRow(row)

    if (mapped === undefined) {
      return undefined
    }

    rows.push(mapped)
  }

  return {
    state: result.state,
    scope: mapScopeFromContract(result.scope),
    rows,
    coverage: mapCoverage(result),
    ...(result.nextCursor === undefined ? {} : { nextCursor: result.nextCursor }),
    ...(result.detail === undefined ? {} : { detail: result.detail })
  }
}

function mapCoverage(result: ContractSelectedContentsResult): SelectedContentsCoverage {
  return {
    state: result.coverage.state,
    recursiveScopeComplete: result.coverage.recursiveScopeComplete,
    emptyResultAuthoritative: result.coverage.emptyResultAuthoritative,
    ...(result.coverage.detail === undefined ? {} : { detail: result.coverage.detail })
  }
}

function mapSelectedContentsRow(
  row: ContractSelectedContentsRow
): SelectedContentsRow | undefined {
  return {
    stableId: row.stableId,
    label: row.label,
    libraryAssetId: row.libraryAssetId,
    rowVersion: row.rowVersion,
    ...(row.primarySourceFileId === null ? {} : { primarySourceFileId: row.primarySourceFileId }),
    scopedSourceFileId: row.scopedSourceFileId,
    sourceId: row.sourceId,
    relativePath: row.relativePath,
    fileName: row.fileName,
    mediaClass: row.mediaClass,
    availabilityState: row.availabilityState,
    ...(row.title === null ? {} : { title: row.title }),
    ...(row.artist === null ? {} : { artist: row.artist }),
    ...(row.album === null ? {} : { album: row.album }),
    ...(row.durationMs === null ? {} : { durationMs: row.durationMs }),
    ...(row.musicalKey === null ? {} : { musicalKey: row.musicalKey }),
    ...(row.tempoBpm === null ? {} : { tempoBpm: row.tempoBpm }),
    ...(row.waveformQualityCurrent === null
      ? {}
      : { waveformQualityCurrent: row.waveformQualityCurrent }),
    ...(row.waveformQualityTarget === null
      ? {}
      : { waveformQualityTarget: row.waveformQualityTarget }),
    ...(row.stemsStateSummary === null ? {} : { stemsStateSummary: row.stemsStateSummary }),
    prepReadinessSummary: row.prepReadinessSummary,
    updatedAtMs: row.updatedAtMs
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | SelectedContentsReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createSelectedContentsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SelectedContentsReadResult {
  return createSelectedContentsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SelectedContentsReadErrorCode {
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

function createSelectedContentsErrorResult(
  state: SelectedContentsReadErrorState,
  code: SelectedContentsReadErrorCode,
  message: string
): SelectedContentsReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isSelectedContentsReadResult(value: unknown): value is SelectedContentsReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
