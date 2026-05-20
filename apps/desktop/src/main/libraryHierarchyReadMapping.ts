import type {
  LiteralHierarchyEntryPoint,
  LiteralHierarchyNode
} from '@dekzer/library-boundary-contract'

import type {
  LibraryHierarchyReadEntryPoint,
  LibraryHierarchyReadNode
} from '../shared/libraryHierarchyRead'

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
): LibraryHierarchyReadNode | null {
  if (row.nodeKind === 'directory' && row.sourceDirectoryId === null) {
    return null
  }

  if (row.nodeKind === 'file' && row.sourceFileId === null) {
    return null
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
