import type { LiteralHierarchyEntryPoint } from '@dekzer/library-boundary-contract'

import type {
  LibraryHierarchyReadEntryPoint,
  LibraryHierarchyReadResult,
  LibraryHierarchyReadRoot,
  LibraryHierarchyReadTarget
} from '../shared/libraryHierarchyRead'
import type { LibraryBoundaryHostClient } from './libraryBoundaryHost'
import {
  mapLibraryHierarchyReadEntryPoint,
  rootIdForLibraryHierarchyReadEntryPoint
} from './libraryHierarchyReadMapping'
import { createHierarchyReadErrorResult, isPositiveOpaqueId } from './libraryHierarchyReadRequest'

export type ResolvedLibraryHierarchyReadTarget = {
  readonly root: LibraryHierarchyReadRoot
  readonly entryPoint: LiteralHierarchyEntryPoint
}

export async function resolveLibraryHierarchyReadTarget(
  client: LibraryBoundaryHostClient,
  target: LibraryHierarchyReadTarget
): Promise<ResolvedLibraryHierarchyReadTarget | LibraryHierarchyReadResult> {
  if (target.kind === 'entryPoint') {
    return resolveEntryPointTarget(target.entryPoint, target.label)
  }

  return resolveFirstAvailableSourceTarget(client)
}

function resolveEntryPointTarget(
  entryPoint: LibraryHierarchyReadEntryPoint,
  label: string | null | undefined
): ResolvedLibraryHierarchyReadTarget {
  return {
    root: {
      id: rootIdForLibraryHierarchyReadEntryPoint(entryPoint),
      label: normalizeOptionalLabel(label),
      entryPoint
    },
    entryPoint: mapLibraryHierarchyReadEntryPoint(entryPoint)
  }
}

async function resolveFirstAvailableSourceTarget(
  client: LibraryBoundaryHostClient
): Promise<ResolvedLibraryHierarchyReadTarget | LibraryHierarchyReadResult> {
  const navigationRows = await client.readNavigationRows({
    parentNavigationRowId: null
  })
  const sourceRow = navigationRows.rows.find(
    (row) => row.selectorKind === 'source' && isPositiveOpaqueId(row.selectorPayload)
  )

  if (sourceRow === undefined || sourceRow.selectorPayload === null) {
    return createHierarchyReadErrorResult(
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
      id: rootIdForLibraryHierarchyReadEntryPoint(entryPoint),
      label: sourceRow.displayName,
      entryPoint
    },
    entryPoint: mapLibraryHierarchyReadEntryPoint(entryPoint)
  }
}

function normalizeOptionalLabel(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null
  }

  const label = value.trim()
  return label.length > 0 ? label : null
}
