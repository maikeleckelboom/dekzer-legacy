import {
  type LibraryHierarchyReadChildrenEntryPoint,
  type LibraryHierarchyReadChildrenErrorCode,
  type LibraryHierarchyReadChildrenErrorState,
  type LibraryHierarchyReadChildrenResult,
  type LibraryHierarchyReadChildrenTarget
} from '../../shared/libraryHierarchy/readChildren'

const defaultLiteralHierarchyReadLimit = 50
const maxLiteralHierarchyReadLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type NormalizedRequest = {
  readonly target: LibraryHierarchyReadChildrenTarget
  readonly parentSourceDirectoryId?: string
  readonly offset: number
  readonly limit: number
}

export function normalizeRequest(
  request: unknown
): NormalizedRequest | LibraryHierarchyReadChildrenResult {
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
    ...(parentSourceDirectoryId === undefined ? {} : { parentSourceDirectoryId }),
    offset,
    limit
  }
}

export function createHierarchyReadErrorResult(
  state: LibraryHierarchyReadChildrenErrorState,
  code: LibraryHierarchyReadChildrenErrorCode,
  message: string
): LibraryHierarchyReadChildrenResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

export function isReadResult(value: unknown): value is LibraryHierarchyReadChildrenResult {
  return isRecord(value) && typeof value.state === 'string'
}

export function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function normalizeTarget(
  value: unknown
): LibraryHierarchyReadChildrenTarget | LibraryHierarchyReadChildrenResult {
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

function normalizeEntryPoint(
  value: Record<string, unknown>
): LibraryHierarchyReadChildrenEntryPoint | LibraryHierarchyReadChildrenResult {
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

function invalidEntryPoint(): LibraryHierarchyReadChildrenResult {
  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read entry point is invalid.'
  )
}

function normalizeNullableOpaqueId(
  value: unknown,
  fieldName: string
): string | undefined | LibraryHierarchyReadChildrenResult {
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

function normalizeOffset(value: unknown): number | LibraryHierarchyReadChildrenResult {
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

function normalizeLimit(value: unknown): number | LibraryHierarchyReadChildrenResult {
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
