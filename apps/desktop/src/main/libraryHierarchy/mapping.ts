import type {
  LiteralHierarchyEntryPoint,
  LiteralHierarchyNode
} from '@dekzer/library-boundary-contract'

import type {
  LibraryHierarchyReadEntryPoint,
  LibraryHierarchyReadNode
} from '../../shared/libraryHierarchy/read'

export function mapLibraryHierarchyReadEntryPoint(
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

export function rootIdForLibraryHierarchyReadEntryPoint(
  entryPoint: LibraryHierarchyReadEntryPoint
): string {
  if (entryPoint.kind === 'source') {
    return `source:${entryPoint.sourceId}`
  }

  return `source-location:${entryPoint.sourceLocationId}`
}

export function mapLiteralHierarchyNode(
  row: LiteralHierarchyNode
): LibraryHierarchyReadNode | undefined {
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
      sourceDirectoryId,
      ...(row.parentSourceDirectoryId === null
        ? {}
        : { parentSourceDirectoryId: row.parentSourceDirectoryId }),
      presenceState: row.presenceState,
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
    sourceFileId,
    ...(row.parentSourceDirectoryId === null
      ? {}
      : { parentSourceDirectoryId: row.parentSourceDirectoryId }),
    presenceState: row.presenceState,
    updatedAtMs: row.updatedAtMs
  }
}
