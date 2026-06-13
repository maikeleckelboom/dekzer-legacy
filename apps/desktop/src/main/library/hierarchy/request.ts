import {
  type EntryPoint,
  type LibraryTreeRowPolicy,
  type ReadErrorCode,
  type ReadErrorState,
  type ReadResult,
  type ReadTarget
} from '../../../shared/library/hierarchy/read'

const defaultLiteralHierarchyReadLimit = 50
const maxLiteralHierarchyReadLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type NormalizedRequest = {
  readonly target: ReadTarget
  readonly parentDirectoryId?: string
  readonly rowPolicy: LibraryTreeRowPolicy
  readonly offset: number
  readonly limit: number
}

export function normalizeRequest(request: unknown): NormalizedRequest | ReadResult {
  if (!isRecord(request)) {
    return createHierarchyReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Library hierarchy reads require a request object.'
    )
  }

  const target = normalizeTarget(request.target)

  if (isReadResult(target)) {
    return target
  }

  const parentDirectoryId = normalizeNullableOpaqueId(
    request.parentDirectoryId,
    'parentDirectoryId'
  )

  if (isReadResult(parentDirectoryId)) {
    return parentDirectoryId
  }

  const offset = normalizeOffset(request.offset)

  if (isReadResult(offset)) {
    return offset
  }

  const limit = normalizeLimit(request.limit)

  if (isReadResult(limit)) {
    return limit
  }

  const rowPolicy = normalizeRowPolicy(request.rowPolicy)

  if (isReadResult(rowPolicy)) {
    return rowPolicy
  }

  return {
    target,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    rowPolicy,
    offset,
    limit
  }
}

export function createHierarchyReadErrorResult(
  state: ReadErrorState,
  code: ReadErrorCode,
  message: string
): ReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

export function isReadResult(value: unknown): value is ReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

export function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function normalizeTarget(value: unknown): ReadTarget | ReadResult {
  if (value === null || value === undefined) {
    return createHierarchyReadErrorResult(
      'noTarget',
      'noTarget',
      'No library hierarchy read target was provided.'
    )
  }

  if (!isRecord(value) || typeof value.kind !== 'string') {
    return createHierarchyReadErrorResult(
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
    return createHierarchyReadErrorResult(
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
    ...(typeof value.label === 'string' ? { label: value.label } : {})
  }
}

function normalizeEntryPoint(value: Record<string, unknown>): EntryPoint | ReadResult {
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

function invalidEntryPoint(): ReadResult {
  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read entry point is invalid.'
  )
}

function normalizeNullableOpaqueId(
  value: unknown,
  fieldName: string
): string | undefined | ReadResult {
  if (value === null || value === undefined) {
    return undefined
  }

  if (isPositiveOpaqueId(value)) {
    return value
  }

  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    `Library hierarchy ${fieldName} is invalid.`
  )
}

function normalizeOffset(value: unknown): number | ReadResult {
  if (value === undefined) {
    return 0
  }

  if (Number.isInteger(value) && typeof value === 'number' && value >= 0) {
    return value
  }

  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read offset is invalid.'
  )
}

function normalizeLimit(value: unknown): number | ReadResult {
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

  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read limit is invalid.'
  )
}

function normalizeRowPolicy(value: unknown): LibraryTreeRowPolicy | ReadResult {
  if (value === undefined) {
    return 'playableMediaBrowse'
  }

  if (
    value === 'audioBrowse' ||
    value === 'playableMediaBrowse' ||
    value === 'sourceFileInventory'
  ) {
    return value
  }

  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy row policy is invalid.'
  )
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
