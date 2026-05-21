import type { LiteralHierarchyEntryPoint } from '@dekzer/library-boundary-contract'

import type {
  EntryPoint,
  ReadResult,
  ReadRoot,
  ReadTarget
} from '../../shared/libraryHierarchy/readChildren'
import type { LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { mapReadEntryPointToLiteralEntryPoint, rootNodeIdForReadEntryPoint } from './mapping'
import { createHierarchyReadErrorResult, isPositiveOpaqueId } from './request'

export type ResolvedTarget = {
  readonly root: ReadRoot
  readonly entryPoint: LiteralHierarchyEntryPoint
}

export async function resolveTarget(
  client: LibraryBoundaryHostClient,
  target: ReadTarget
): Promise<ResolvedTarget | ReadResult> {
  if (target.kind === 'entryPoint') {
    return resolveEntryPointTarget(target.entryPoint, target.label)
  }

  return resolveFirstAvailableSourceTarget(client)
}

function resolveEntryPointTarget(
  entryPoint: EntryPoint,
  label: string | undefined
): ResolvedTarget {
  return {
    root: {
      id: rootNodeIdForReadEntryPoint(entryPoint),
      ...optionalLabelProperty(label),
      entryPoint
    },
    entryPoint: mapReadEntryPointToLiteralEntryPoint(entryPoint)
  }
}

async function resolveFirstAvailableSourceTarget(
  client: LibraryBoundaryHostClient
): Promise<ResolvedTarget | ReadResult> {
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

  const entryPoint: EntryPoint = {
    kind: 'source',
    sourceId: sourceRow.selectorPayload
  }

  return {
    root: {
      id: rootNodeIdForReadEntryPoint(entryPoint),
      label: sourceRow.displayName,
      entryPoint
    },
    entryPoint: mapReadEntryPointToLiteralEntryPoint(entryPoint)
  }
}

function optionalLabelProperty(value: string | undefined): { readonly label?: string } {
  if (typeof value !== 'string') {
    return {}
  }

  const label = value.trim()
  return label.length > 0 ? { label } : {}
}
