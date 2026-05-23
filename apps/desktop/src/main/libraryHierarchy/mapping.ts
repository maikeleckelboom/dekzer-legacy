import type {
  LiteralHierarchyEntryPoint,
  LiteralHierarchyNode
} from '@dekzer/library-boundary-contract'

import type { EntryPoint, ChildRow } from '../../shared/libraryHierarchy/readChildren'

export function mapReadEntryPointToLiteralEntryPoint(
  entryPoint: EntryPoint
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

export function rootNodeIdForReadEntryPoint(entryPoint: EntryPoint): string {
  if (entryPoint.kind === 'source') {
    return `source:${entryPoint.sourceId}`
  }

  return `source-location:${entryPoint.sourceLocationId}`
}

export function mapLiteralHierarchyNode(row: LiteralHierarchyNode): ChildRow | undefined {
  if (row.nodeKind === 'directory' && row.sourceDirectoryId === null) {
    return undefined
  }

  if (row.nodeKind === 'file' && row.sourceFileId === null) {
    return undefined
  }

  if (row.nodeKind === 'directory') {
    const sourceDirectoryId = row.sourceDirectoryId

    if (sourceDirectoryId === null) {
      return undefined
    }

    return {
      id: `source-directory:${sourceDirectoryId}`,
      kind: 'directory',
      label: row.displayName,
      directoryId: sourceDirectoryId,
      ...(row.parentSourceDirectoryId === null
        ? {}
        : { parentDirectoryId: row.parentSourceDirectoryId }),
      presence: row.presenceState,
      browseability: row.mediaBrowseability ?? 'unknown',
      updatedAtMs: row.updatedAtMs
    }
  }

  const sourceFileId = row.sourceFileId

  if (sourceFileId === null) {
    return undefined
  }

  return {
    id: `source-file:${sourceFileId}`,
    kind: 'file',
    label: row.displayName,
    fileId: sourceFileId,
    ...(row.parentSourceDirectoryId === null
      ? {}
      : { parentDirectoryId: row.parentSourceDirectoryId }),
    presence: row.presenceState,
    updatedAtMs: row.updatedAtMs
  }
}
