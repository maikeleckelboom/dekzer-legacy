import {
  type LibraryHierarchyReadEntryPoint,
  type LibraryHierarchyReadErrorCode,
  type LibraryHierarchyReadErrorState,
  type LibraryHierarchyReadResult,
  type LibraryHierarchyReadTarget
} from '../../shared/libraryHierarchy/read'

const defaultLiteralHierarchyReadLimit = 50
const maxLiteralHierarchyReadLimit = 200
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type NormalizedLibraryHierarchyReadRequest = {
  readonly target: LibraryHierarchyReadTarget
  readonly parentSourceDirectoryId?: string
  readonly offset: number
  readonly limit: number
}

export function normalizeLibraryHierarchyReadRequest(
  request: unknown
): NormalizedLibraryHierarchyReadRequest | LibraryHierarchyReadResult {
  if (!isRecord(request)) {
    return createHierarchyReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Library hierarchy reads require a request object.'
    )
  }

  const target = normalizeLibraryHierarchyReadTarget(request.target)

  if (isLibraryHierarchyReadResult(target)) {
    return target
  }

  const parentSourceDirectoryId = normalizeNullableOpaqueId(
    request.parentSourceDirectoryId,
    'parentSourceDirectoryId'
  )

  if (isLibraryHierarchyReadResult(parentSourceDirectoryId)) {
    return parentSourceDirectoryId
  }

  const offset = normalizeOffset(request.offset)

  if (isLibraryHierarchyReadResult(offset)) {
    return offset
  }

  const limit = normalizeLimit(request.limit)

  if (isLibraryHierarchyReadResult(limit)) {
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
  state: LibraryHierarchyReadErrorState,
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

export function isLibraryHierarchyReadResult(value: unknown): value is LibraryHierarchyReadResult {
  return isRecord(value) && typeof value.state === 'string'
}

export function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function normalizeLibraryHierarchyReadTarget(
  value: unknown
): LibraryHierarchyReadTarget | LibraryHierarchyReadResult {
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

  if (isLibraryHierarchyReadResult(entryPoint)) {
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
  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read entry point is invalid.'
  )
}

function normalizeNullableOpaqueId(
  value: unknown,
  fieldName: string
): string | undefined | LibraryHierarchyReadResult {
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

function normalizeOffset(value: unknown): number | LibraryHierarchyReadResult {
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

  return createHierarchyReadErrorResult(
    'invalidRequest',
    'invalidRequest',
    'Library hierarchy read limit is invalid.'
  )
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
