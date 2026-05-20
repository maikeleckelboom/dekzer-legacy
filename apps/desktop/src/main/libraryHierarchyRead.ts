import type {
  LiteralHierarchyEntryPoint,
  LiteralHierarchyNode,
  ReadLiteralHierarchyChildrenRequest
} from '@dekzer/library-boundary-contract'

import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadEntryPoint,
  type LibraryHierarchyReadErrorCode,
  type LibraryHierarchyReadNode,
  type LibraryHierarchyReadResult,
  type LibraryHierarchyReadRoot,
  type LibraryHierarchyReadTarget
} from '../shared/libraryHierarchyRead'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from './libraryBoundaryHost'
import { LibraryBoundaryHostError } from './libraryBoundaryHostErrors'

const defaultLiteralHierarchyReadLimit = 50
const maxLiteralHierarchyReadLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type LibraryHierarchyReadIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LibraryHierarchyReadResult>
  ): void
}

type NormalizedReadRequest = {
  readonly target: LibraryHierarchyReadTarget
  readonly parentSourceDirectoryId: string | null
  readonly offset: number
  readonly limit: number
}

type ResolvedReadTarget = {
  readonly root: LibraryHierarchyReadRoot
  readonly entryPoint: LiteralHierarchyEntryPoint
}

export function registerLibraryHierarchyReadIpc(
  ipcMain: LibraryHierarchyReadIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(libraryHierarchyReadIpcChannels.readLiteralHierarchyChildren, (_event, request) =>
    readLiteralHierarchyChildrenThroughHost(host, request)
  )
}

export async function readLiteralHierarchyChildrenThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<LibraryHierarchyReadResult> {
  const normalizedRequest = normalizeReadRequest(request)

  if (isReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isReadResult(client)) {
    return client
  }

  try {
    const resolvedTarget = await resolveReadTarget(client, normalizedRequest.target)

    if (isReadResult(resolvedTarget)) {
      return resolvedTarget
    }

    const reply = await client.readLiteralHierarchyChildren({
      entryPoint: resolvedTarget.entryPoint,
      parentSourceDirectoryId: normalizedRequest.parentSourceDirectoryId,
      offset: normalizedRequest.offset,
      limit: normalizedRequest.limit
    } satisfies ReadLiteralHierarchyChildrenRequest)

    if (reply.window === null) {
      return errorResult(
        'notFound',
        'notFound',
        'The requested library hierarchy target is not available.'
      )
    }

    return {
      state: 'ready',
      window: {
        root: resolvedTarget.root,
        parentSourceDirectoryId: reply.window.parentSourceDirectoryId,
        offset: reply.window.offset,
        limit: reply.window.limit,
        totalRows: reply.window.totalRows,
        nodes: reply.window.rows.map(mapLiteralHierarchyNode)
      }
    }
  } catch {
    return errorResult('readFailed', 'readFailed', 'Unable to read library hierarchy children.')
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | LibraryHierarchyReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return errorResult('hostUnavailable', 'hostFailed', 'The library boundary host is unavailable.')
  }
}

async function resolveReadTarget(
  client: LibraryBoundaryHostClient,
  target: LibraryHierarchyReadTarget
): Promise<ResolvedReadTarget | LibraryHierarchyReadResult> {
  if (target.kind === 'entryPoint') {
    return {
      root: {
        id: rootIdForEntryPoint(target.entryPoint),
        label: normalizeOptionalLabel(target.label),
        entryPoint: target.entryPoint
      },
      entryPoint: toProtocolEntryPoint(target.entryPoint)
    }
  }

  const navigationRows = await client.readNavigationRows({
    parentNavigationRowId: null
  })
  const sourceRow = navigationRows.rows.find(
    (row) => row.selectorKind === 'source' && isPositiveOpaqueId(row.selectorPayload)
  )

  if (sourceRow === undefined || sourceRow.selectorPayload === null) {
    return errorResult(
      'noTarget',
      'noTarget',
      'No library source is available for a literal hierarchy read.'
    )
  }

  const entryPoint: LibraryHierarchyReadEntryPoint = {
    kind: 'source',
    sourceId: sourceRow.selectorPayload
  }

  return {
    root: {
      id: rootIdForEntryPoint(entryPoint),
      label: sourceRow.displayName,
      entryPoint
    },
    entryPoint: toProtocolEntryPoint(entryPoint)
  }
}

function normalizeReadRequest(
  request: unknown
): NormalizedReadRequest | LibraryHierarchyReadResult {
  if (!isRecord(request)) {
    return errorResult(
      'invalidRequest',
      'invalidRequest',
      'Library hierarchy reads require a request object.'
    )
  }

  const target = normalizeReadTarget(request.target)

  if (isReadResult(target)) {
    return target
  }

  const parentSourceDirectoryId = normalizeNullableOpaqueId(
    request.parentSourceDirectoryId,
    'parentSourceDirectoryId'
  )

  if (isReadResult(parentSourceDirectoryId)) {
    return parentSourceDirectoryId
  }

  const offset = normalizeOffset(request.offset)

  if (isReadResult(offset)) {
    return offset
  }

  const limit = normalizeLimit(request.limit)

  if (isReadResult(limit)) {
    return limit
  }

  return {
    target,
    parentSourceDirectoryId,
    offset,
    limit
  }
}

function normalizeReadTarget(
  value: unknown
): LibraryHierarchyReadTarget | LibraryHierarchyReadResult {
  if (value === null || value === undefined) {
    return errorResult('noTarget', 'noTarget', 'No library hierarchy read target was provided.')
  }

  if (!isRecord(value) || typeof value.kind !== 'string') {
    return errorResult(
      'invalidRequest',
      'invalidRequest',
      'Library hierarchy read target is invalid.'
    )
  }

  if (value.kind === 'firstAvailableSource') {
    return {
      kind: 'firstAvailableSource'
    }
  }

  if (value.kind !== 'entryPoint' || !isRecord(value.entryPoint)) {
    return errorResult(
      'invalidRequest',
      'invalidRequest',
      'Library hierarchy read target entry point is invalid.'
    )
  }

  const entryPoint = normalizeEntryPoint(value.entryPoint)

  if (isReadResult(entryPoint)) {
    return entryPoint
  }

  return {
    kind: 'entryPoint',
    entryPoint,
    label: value.label === null || typeof value.label === 'string' ? value.label : undefined
  }
}

function normalizeEntryPoint(
  value: Record<string, unknown>
): LibraryHierarchyReadEntryPoint | LibraryHierarchyReadResult {
  if (value.kind === 'source') {
    if (!isPositiveOpaqueId(value.sourceId)) {
      return invalidEntryPoint()
    }

    return {
      kind: 'source',
      sourceId: value.sourceId
    }
  }

  if (value.kind === 'sourceLocation') {
    if (!isPositiveOpaqueId(value.sourceLocationId)) {
      return invalidEntryPoint()
    }

    return {
      kind: 'sourceLocation',
      sourceLocationId: value.sourceLocationId
    }
  }

  return invalidEntryPoint()
}

function invalidEntryPoint(): LibraryHierarchyReadResult {
  return errorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read entry point is invalid.'
  )
}

function normalizeNullableOpaqueId(
  value: unknown,
  fieldName: string
): string | null | LibraryHierarchyReadResult {
  if (value === null || value === undefined) {
    return null
  }

  if (isPositiveOpaqueId(value)) {
    return value
  }

  return errorResult(
    'invalidRequest',
    'invalidRequest',
    `Library hierarchy ${fieldName} is invalid.`
  )
}

function normalizeOffset(value: unknown): number | LibraryHierarchyReadResult {
  if (value === undefined) {
    return 0
  }

  if (Number.isInteger(value) && typeof value === 'number' && value >= 0) {
    return value
  }

  return errorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read offset is invalid.'
  )
}

function normalizeLimit(value: unknown): number | LibraryHierarchyReadResult {
  if (value === undefined) {
    return defaultLiteralHierarchyReadLimit
  }

  if (
    Number.isInteger(value) &&
    typeof value === 'number' &&
    value > 0 &&
    value <= maxLiteralHierarchyReadLimit
  ) {
    return value
  }

  return errorResult('invalidRequest', 'invalidRequest', 'Library hierarchy read limit is invalid.')
}

function mapLiteralHierarchyNode(row: LiteralHierarchyNode): LibraryHierarchyReadNode {
  if (row.nodeKind === 'directory' && row.sourceDirectoryId === null) {
    throw new Error('literal hierarchy directory row is missing sourceDirectoryId')
  }

  if (row.nodeKind === 'file' && row.sourceFileId === null) {
    throw new Error('literal hierarchy file row is missing sourceFileId')
  }

  return {
    id:
      row.nodeKind === 'directory'
        ? `source-directory:${row.sourceDirectoryId}`
        : `source-file:${row.sourceFileId}`,
    kind: row.nodeKind,
    label: row.displayName,
    parentSourceDirectoryId: row.parentSourceDirectoryId,
    sourceDirectoryId: row.sourceDirectoryId,
    sourceFileId: row.sourceFileId,
    presenceState: row.presenceState,
    updatedAtMs: row.updatedAtMs
  }
}

function toProtocolEntryPoint(
  entryPoint: LibraryHierarchyReadEntryPoint
): LiteralHierarchyEntryPoint {
  if (entryPoint.kind === 'source') {
    return {
      type: 'source',
      payload: {
        sourceId: entryPoint.sourceId
      }
    }
  }

  return {
    type: 'sourceLocation',
    payload: {
      sourceLocationId: entryPoint.sourceLocationId
    }
  }
}

function rootIdForEntryPoint(entryPoint: LibraryHierarchyReadEntryPoint): string {
  if (entryPoint.kind === 'source') {
    return `source:${entryPoint.sourceId}`
  }

  return `source-location:${entryPoint.sourceLocationId}`
}

function normalizeOptionalLabel(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const label = value.trim()
  return label.length > 0 ? label : null
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryHierarchyReadResult {
  return errorResult('hostUnavailable', hostErrorCode(host, error), hostErrorMessage(host, error))
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryHierarchyReadErrorCode {
  if (host.state === 'failed') {
    return 'hostFailed'
  }

  switch (error.code) {
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
  state: Exclude<LibraryHierarchyReadResult['state'], 'ready'>,
  code: LibraryHierarchyReadErrorCode,
  message: string
): LibraryHierarchyReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isReadResult(value: unknown): value is LibraryHierarchyReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}
