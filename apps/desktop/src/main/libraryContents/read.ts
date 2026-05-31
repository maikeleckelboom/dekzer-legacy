import type {
  ContentsFileRow as ContractContentsFileRow,
  ContentsReadPolicy as ContractContentsReadPolicy,
  ContentsReadRequest as ContractContentsReadRequest,
  ContentsResult as ContractContentsResult,
  ContentsScope as ContractContentsScope,
  PrimaryMediaSummary as ContractPrimaryMediaSummary
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  contentsReadChannels,
  type ContentsCoverage,
  type ContentsFileRow,
  type ContentsMediaClass,
  type ContentsReadErrorCode,
  type ContentsReadErrorState,
  type ContentsReadPolicy,
  type ContentsReadResult,
  type ContentsRecursion,
  type ContentsResult,
  type ContentsRowProfile,
  type ContentsScope,
  type PrimaryMediaSummary
} from '../../shared/libraryContents/read'

export type LibraryContentsReadIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<ContentsReadResult>
  ): void
}

const defaultContentsLimit = 100
const maxContentsLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/
const canonicalMediaClassOrder: readonly ContentsMediaClass[] = [
  'audio',
  'video',
  'image',
  'unsupported'
]

export function registerContentsReadIpc(
  ipcMain: LibraryContentsReadIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(contentsReadChannels.read, (_event, request) =>
    readContentsThroughHost(host, request)
  )
}

export async function readContentsThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ContentsReadResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isContentsReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isContentsReadResult(client)) {
    return client
  }

  try {
    const reply = await client.readContents({
      scope: mapScopeToContract(normalizedRequest.scope),
      policy: mapPolicyToContract(normalizedRequest.policy),
      recursion: normalizedRequest.recursion,
      limit: normalizedRequest.limit,
      ...(normalizedRequest.cursor === undefined ? {} : { cursor: normalizedRequest.cursor })
    } satisfies ContractContentsReadRequest)

    const stateError = contentsStateError(reply.result)

    if (stateError !== undefined) {
      return stateError
    }

    const result = mapContentsResult(reply.result)

    if (result === undefined) {
      return createContentsErrorResult(
        'readFailed',
        'readFailed',
        'Unable to read library contents.'
      )
    }

    return {
      state: 'ready',
      result
    }
  } catch {
    return createContentsErrorResult('readFailed', 'readFailed', 'Unable to read library contents.')
  }
}

type NormalizedRequest = {
  readonly scope: ContentsScope
  readonly policy: ContentsReadPolicy
  readonly recursion: ContentsRecursion
  readonly limit: number
  readonly cursor?: string
}

function normalizeRequest(request: unknown): NormalizedRequest | ContentsReadResult {
  if (!isRecord(request)) {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents reads require a request object.'
    )
  }

  const scope = normalizeScope(request.scope)

  if (isContentsReadResult(scope)) {
    return scope
  }

  const policy = normalizePolicy(request.policy)

  if (isContentsReadResult(policy)) {
    return policy
  }

  const recursion = normalizeRecursion(request.recursion)

  if (isContentsReadResult(recursion)) {
    return recursion
  }

  const limit = normalizeLimit(request.limit)

  if (isContentsReadResult(limit)) {
    return limit
  }

  const cursor = normalizeCursor(request.cursor)

  if (isContentsReadResult(cursor)) {
    return cursor
  }

  return {
    scope,
    policy,
    recursion,
    limit,
    ...(cursor === undefined ? {} : { cursor })
  }
}

function normalizeScope(value: unknown): ContentsScope | ContentsReadResult {
  if (value === null || value === undefined) {
    return createContentsErrorResult('noTarget', 'noTarget', 'No contents scope was provided.')
  }

  if (!isRecord(value) || typeof value.kind !== 'string') {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents scope is invalid.'
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

  return createContentsErrorResult('invalidRequest', 'invalidRequest', 'Contents scope is invalid.')
}

function normalizePolicy(value: unknown): ContentsReadPolicy | ContentsReadResult {
  if (!isRecord(value)) {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents policy is invalid.'
    )
  }

  const mediaClasses = normalizeMediaClasses(value.mediaClasses)

  if (isContentsReadResult(mediaClasses)) {
    return mediaClasses
  }

  const rowProfile = normalizeRowProfile(value.rowProfile)

  if (isContentsReadResult(rowProfile)) {
    return rowProfile
  }

  return {
    mediaClasses,
    rowProfile
  }
}

function normalizeMediaClasses(value: unknown): readonly ContentsMediaClass[] | ContentsReadResult {
  if (!Array.isArray(value) || value.length === 0) {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents mediaClasses must be a non-empty array.'
    )
  }

  const mediaClasses = new Set<ContentsMediaClass>()

  for (const mediaClass of value) {
    if (
      mediaClass !== 'audio' &&
      mediaClass !== 'video' &&
      mediaClass !== 'image' &&
      mediaClass !== 'unsupported'
    ) {
      return createContentsErrorResult(
        'invalidRequest',
        'invalidRequest',
        'Contents mediaClasses contains an unsupported value.'
      )
    }

    mediaClasses.add(mediaClass)
  }

  return canonicalMediaClassOrder.filter((mediaClass) => mediaClasses.has(mediaClass))
}

function normalizeRowProfile(value: unknown): ContentsRowProfile | ContentsReadResult {
  if (isRecord(value) && value.kind === 'sourceFile') {
    return { kind: 'sourceFile' }
  }

  if (isRecord(value) && value.kind === 'primaryMedia') {
    return { kind: 'primaryMedia' }
  }

  return createContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Contents rowProfile is invalid.'
  )
}

function normalizeRecursion(value: unknown): ContentsRecursion | ContentsReadResult {
  if (value === 'immediate' || value === 'recursive') {
    return value
  }

  return createContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Contents recursion is invalid.'
  )
}

function normalizeLimit(value: unknown): number | ContentsReadResult {
  if (value === undefined) {
    return defaultContentsLimit
  }

  if (
    Number.isInteger(value) &&
    typeof value === 'number' &&
    value > 0 &&
    value <= maxContentsLimit
  ) {
    return value
  }

  return createContentsErrorResult('invalidRequest', 'invalidRequest', 'Contents limit is invalid.')
}

function normalizeCursor(value: unknown): string | undefined | ContentsReadResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value === 'string' && value.trim().length > 0) {
    return value
  }

  return createContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Contents cursor is invalid.'
  )
}

function mapScopeToContract(scope: ContentsScope): ContractContentsScope {
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

function mapScopeFromContract(scope: ContractContentsScope): ContentsScope {
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

function mapPolicyToContract(policy: ContentsReadPolicy): ContractContentsReadPolicy {
  return {
    mediaClasses: [...policy.mediaClasses],
    rowProfile: policy.rowProfile
  }
}

function mapPolicyFromContract(policy: ContractContentsReadPolicy): ContentsReadPolicy {
  return {
    mediaClasses: policy.mediaClasses,
    rowProfile: policy.rowProfile
  }
}

function contentsStateError(result: ContractContentsResult): ContentsReadResult | undefined {
  if (result.state === 'policyConflict') {
    return createContentsErrorResult(
      'policyConflict',
      'policyConflict',
      result.detail ?? 'The contents policy cannot be read.'
    )
  }

  if (result.state === 'cursorInvalid') {
    return createContentsErrorResult(
      'cursorInvalid',
      'cursorInvalid',
      result.detail ?? 'The contents cursor is invalid.'
    )
  }

  return undefined
}

function mapContentsResult(result: ContractContentsResult): ContentsResult | undefined {
  const state = mapReadyContentsState(result.state)

  if (state === undefined) {
    return undefined
  }

  const rows: ContentsFileRow[] = []

  for (const row of result.rows) {
    const mapped = mapContentsRow(row)

    if (mapped === undefined) {
      return undefined
    }

    rows.push(mapped)
  }

  return {
    state,
    scope: mapScopeFromContract(result.scope),
    policy: mapPolicyFromContract(result.policy),
    recursion: result.recursion,
    rows,
    coverage: mapCoverage(result),
    ...(result.nextCursor === undefined ? {} : { nextCursor: result.nextCursor }),
    ...(result.detail === undefined ? {} : { detail: result.detail })
  }
}

function mapReadyContentsState(
  state: ContractContentsResult['state']
): ContentsResult['state'] | undefined {
  switch (state) {
    case 'ready':
    case 'empty':
    case 'partial':
    case 'sourceUnavailable':
    case 'locationMissing':
    case 'blocked':
    case 'failed':
      return state
    case 'policyConflict':
    case 'cursorInvalid':
      return undefined
  }
}

function mapCoverage(result: ContractContentsResult): ContentsCoverage {
  return {
    state: result.coverage.state,
    recursiveScopeComplete: result.coverage.recursiveScopeComplete,
    emptyResultAuthoritative: result.coverage.emptyResultAuthoritative,
    ...(result.coverage.detail === undefined ? {} : { detail: result.coverage.detail })
  }
}

function mapContentsRow(row: ContractContentsFileRow): ContentsFileRow | undefined {
  if (
    row.primaryMedia !== undefined &&
    (row.mediaClass === 'image' || row.mediaClass === 'unsupported')
  ) {
    return undefined
  }

  return {
    id: row.id,
    sourceId: row.sourceId,
    sourceFileId: row.sourceFileId,
    ...(row.parentDirectoryId === null ? {} : { parentDirectoryId: row.parentDirectoryId }),
    label: row.label,
    ...(row.relativePath === undefined ? {} : { relativePath: row.relativePath }),
    fileName: row.fileName,
    mediaClass: row.mediaClass,
    fileKind: row.fileKind,
    presence: row.presence,
    ...(row.availabilityState === undefined ? {} : { availabilityState: row.availabilityState }),
    ...(row.primaryMedia === undefined ? {} : { primaryMedia: mapPrimaryMedia(row.primaryMedia) }),
    ...(row.updatedAtMs === undefined ? {} : { updatedAtMs: row.updatedAtMs })
  }
}

function mapPrimaryMedia(summary: ContractPrimaryMediaSummary): PrimaryMediaSummary {
  return {
    origin: summary.origin,
    ...(summary.libraryAssetId === null ? {} : { libraryAssetId: summary.libraryAssetId }),
    ...(summary.rowVersion === null ? {} : { rowVersion: summary.rowVersion }),
    ...(summary.primarySourceFileId === null
      ? {}
      : { primarySourceFileId: summary.primarySourceFileId }),
    ...(summary.title === null ? {} : { title: summary.title }),
    ...(summary.artist === null ? {} : { artist: summary.artist }),
    ...(summary.album === null ? {} : { album: summary.album }),
    ...(summary.durationMs === null ? {} : { durationMs: summary.durationMs }),
    ...(summary.musicalKey === null ? {} : { musicalKey: summary.musicalKey }),
    ...(summary.tempoBpm === null ? {} : { tempoBpm: summary.tempoBpm }),
    ...(summary.waveformQualityCurrent === null
      ? {}
      : { waveformQualityCurrent: summary.waveformQualityCurrent }),
    ...(summary.waveformQualityTarget === null
      ? {}
      : { waveformQualityTarget: summary.waveformQualityTarget }),
    ...(summary.stemsStateSummary === null ? {} : { stemsStateSummary: summary.stemsStateSummary }),
    ...(summary.prepReadinessSummary === null
      ? {}
      : { prepReadinessSummary: summary.prepReadinessSummary })
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ContentsReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createContentsErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ContentsReadResult {
  return createContentsErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ContentsReadErrorCode {
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

function createContentsErrorResult(
  state: ContentsReadErrorState,
  code: ContentsReadErrorCode,
  message: string
): ContentsReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isContentsReadResult(value: unknown): value is ContentsReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
