import type { LibraryTreeEntryPoint, LibraryTreeNode } from '@dekzer/library-boundary-contract'

import type { EntryPoint, ChildRow } from '../../shared/libraryHierarchy/readChildren'

export function mapReadEntryPointToLibraryTreeEntryPoint(
  entryPoint: EntryPoint
): LibraryTreeEntryPoint {
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

export function mapLibraryTreeNode(row: LibraryTreeNode): ChildRow | undefined {
  if (row.nodeKind === 'directory' && row.sourceDirectoryId === null) {
    return undefined
  }

  if (row.nodeKind === 'file' && row.sourceFileId === null) {
    return undefined
  }

  if (row.nodeKind === 'directory') {
    const sourceDirectoryId = row.sourceDirectoryId

    if (
      sourceDirectoryId === null ||
      row.directoryPrimaryMediaState === undefined ||
      row.directoryImageMediaState === undefined ||
      row.directoryScanState === undefined ||
      row.navigableChildScopeState === undefined
    ) {
      return undefined
    }

    return {
      id: `source-directory:${sourceDirectoryId}`,
      kind: 'directory',
      label: row.displayName,
      sourceId: row.sourceId,
      directoryId: sourceDirectoryId,
      ...(row.parentSourceDirectoryId === null
        ? {}
        : { parentDirectoryId: row.parentSourceDirectoryId }),
      presence: row.presenceState,
      hasChildDirectories: row.hasChildDirectories ?? false,
      directoryPrimaryMediaState: row.directoryPrimaryMediaState,
      directoryImageMediaState: row.directoryImageMediaState,
      directoryScanState: row.directoryScanState,
      navigableChildScopeState: row.navigableChildScopeState,
      updatedAtMs: row.updatedAtMs
    }
  }

  const sourceFileId = row.sourceFileId
  const fileClass = row.fileClass ?? 'none'

  if (sourceFileId === null) {
    return undefined
  }

  return {
    id: `source-file:${sourceFileId}`,
    kind: 'file',
    label: row.displayName,
    sourceId: row.sourceId,
    fileId: sourceFileId,
    fileClass,
    ...(row.parentSourceDirectoryId === null
      ? {}
      : { parentDirectoryId: row.parentSourceDirectoryId }),
    presence: row.presenceState,
    updatedAtMs: row.updatedAtMs
  }
}
