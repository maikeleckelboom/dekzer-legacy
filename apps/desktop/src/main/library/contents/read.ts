import type {
  ContentsFileRow as ContractContentsFileRow,
  ContentsReadPolicy as ContractContentsReadPolicy,
  ContentsReadRequest as ContractContentsReadRequest,
  ContentsResult as ContractContentsResult,
  ContentsScope as ContractContentsScope,
  PrimaryMediaSummary as ContractPrimaryMediaSummary
} from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import {
  type ContentsScopeCoverage,
  type ContentsFileClass,
  type ContentsFileRow,
  type ContentsReadErrorCode,
  type ContentsReadErrorState,
  type ContentsReadPolicy,
  type ContentsReadResult,
  type ContentsScopeDepth,
  type ContentsResult,
  type ContentsScope,
  type PrimaryMediaKind,
  type PrimaryMediaSummary
} from '../../../shared/library/contents/read'

const defaultContentsLimit = 100
const maxContentsLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/
const canonicalFileClassOrder: readonly ContentsFileClass[] = [
  'audio',
  'video',
  'image',
  'unsupported'
]
const canonicalPrimaryMediaKindOrder: readonly PrimaryMediaKind[] = ['audio', 'video']

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
      scopeDepth: normalizedRequest.scopeDepth,
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
  readonly scopeDepth: ContentsScopeDepth
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

  const scopeDepth = normalizeScopeDepth(request.scopeDepth)

  if (isContentsReadResult(scopeDepth)) {
    return scopeDepth
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
    scopeDepth,
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
  if (!isRecord(value) || typeof value.kind !== 'string') {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents policy is invalid.'
    )
  }

  if (value.kind === 'audioBrowse') {
    return { kind: 'audioBrowse' }
  }

  if (value.kind === 'playableMediaBrowse') {
    return { kind: 'playableMediaBrowse' }
  }

  if (value.kind === 'sourceFileInventory') {
    const fileClasses = normalizeFileClasses(value.fileClasses)
    return isContentsReadResult(fileClasses)
      ? fileClasses
      : { kind: 'sourceFileInventory', fileClasses }
  }

  if (value.kind === 'primaryMedia') {
    const mediaKinds = normalizePrimaryMediaKinds(value.mediaKinds)
    return isContentsReadResult(mediaKinds) ? mediaKinds : { kind: 'primaryMedia', mediaKinds }
  }

  return createContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Contents policy kind is invalid.'
  )
}

function normalizeFileClasses(value: unknown): readonly ContentsFileClass[] | ContentsReadResult {
  if (!Array.isArray(value) || value.length === 0) {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents sourceFileInventory fileClasses must be a non-empty array.'
    )
  }

  const fileClasses = new Set<ContentsFileClass>()

  for (const fileClass of value) {
    if (
      fileClass !== 'audio' &&
      fileClass !== 'video' &&
      fileClass !== 'image' &&
      fileClass !== 'unsupported'
    ) {
      return createContentsErrorResult(
        'invalidRequest',
        'invalidRequest',
        'Contents fileClasses contains an unsupported value.'
      )
    }

    fileClasses.add(fileClass)
  }

  return canonicalFileClassOrder.filter((fileClass) => fileClasses.has(fileClass))
}

function normalizePrimaryMediaKinds(
  value: unknown
): readonly PrimaryMediaKind[] | ContentsReadResult {
  if (!Array.isArray(value) || value.length === 0) {
    return createContentsErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Contents primaryMedia mediaKinds must be a non-empty array.'
    )
  }

  const mediaKinds = new Set<PrimaryMediaKind>()
  for (const mediaKind of value) {
    if (mediaKind !== 'audio' && mediaKind !== 'video') {
      return createContentsErrorResult(
        'invalidRequest',
        'invalidRequest',
        'Contents mediaKinds contains an unsupported value.'
      )
    }
    mediaKinds.add(mediaKind)
  }
  return canonicalPrimaryMediaKindOrder.filter((mediaKind) => mediaKinds.has(mediaKind))
}

function normalizeScopeDepth(value: unknown): ContentsScopeDepth | ContentsReadResult {
  if (value === 'immediate' || value === 'recursive') {
    return value
  }

  return createContentsErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Contents scope depth is invalid.'
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
  switch (policy.kind) {
    case 'playableMediaBrowse':
    case 'audioBrowse':
      return policy
    case 'sourceFileInventory':
      return {
        kind: policy.kind,
        fileClasses: [...policy.fileClasses]
      }
    case 'primaryMedia':
      return {
        kind: policy.kind,
        mediaKinds: [...policy.mediaKinds]
      }
  }
}

function mapPolicyFromContract(policy: ContractContentsReadPolicy): ContentsReadPolicy {
  switch (policy.kind) {
    case 'playableMediaBrowse':
    case 'audioBrowse':
      return policy
    case 'sourceFileInventory':
      return {
        kind: policy.kind,
        fileClasses: policy.fileClasses
      }
    case 'primaryMedia':
      return {
        kind: policy.kind,
        mediaKinds: policy.mediaKinds
      }
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

  if (state === undefined || typeof result.hasPolicyOmittedRows !== 'boolean') {
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
    scopeDepth: result.scopeDepth,
    rows,
    scopeCoverage: mapCoverage(result),
    hasPolicyOmittedRows: result.hasPolicyOmittedRows,
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

function mapCoverage(result: ContractContentsResult): ContentsScopeCoverage {
  return {
    state: result.scopeCoverage.state,
    subtreeCoverageComplete: result.scopeCoverage.subtreeCoverageComplete,
    emptyResultAuthoritative: result.scopeCoverage.emptyResultAuthoritative,
    ...(result.scopeCoverage.detail === undefined ? {} : { detail: result.scopeCoverage.detail })
  }
}

function mapContentsRow(row: ContractContentsFileRow): ContentsFileRow | undefined {
  if (
    row.primaryMedia !== undefined &&
    (row.fileClass === 'image' || row.fileClass === 'unsupported')
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
    fileClass: row.fileClass,
    fileKind: row.fileKind,
    presence: row.presence,
    ...(row.primaryMedia === undefined ? {} : { primaryMedia: mapPrimaryMedia(row.primaryMedia) }),
    ...(row.updatedAtMs === undefined ? {} : { updatedAtMs: row.updatedAtMs })
  }
}

function mapPrimaryMedia(summary: ContractPrimaryMediaSummary): PrimaryMediaSummary {
  return {
    ...(summary.primaryMediaFactId === null
      ? {}
      : { primaryMediaFactId: summary.primaryMediaFactId }),
    ...(summary.attachmentId === null ? {} : { attachmentId: summary.attachmentId }),
    ...(summary.contentHashAlgorithm === null
      ? {}
      : { contentHashAlgorithm: summary.contentHashAlgorithm }),
    ...(summary.contentHashValue === null ? {} : { contentHashValue: summary.contentHashValue }),
    ...(summary.evidenceSourceFileId === null
      ? {}
      : { evidenceSourceFileId: summary.evidenceSourceFileId }),
    ...(summary.mediaKind === null ? {} : { mediaKind: summary.mediaKind }),
    ...(summary.mimeType === null ? {} : { mimeType: summary.mimeType }),
    ...(summary.durationMs === null ? {} : { durationMs: summary.durationMs }),
    ...(summary.sampleRateHz === null ? {} : { sampleRateHz: summary.sampleRateHz }),
    ...(summary.channels === null ? {} : { channels: summary.channels }),
    ...(summary.bitDepth === null ? {} : { bitDepth: summary.bitDepth }),
    ...(summary.codec === null ? {} : { codec: summary.codec })
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
