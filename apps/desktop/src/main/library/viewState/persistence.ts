import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from '../../../shared/library/viewState/persistence'
import type { LibraryBoundaryHost } from '../boundary/host'

const viewStateFileName = 'library-view-state.json'
const viewStateKeys = new Set([
  'version',
  'activeSurface',
  'selectedLibraryNodeId',
  'selectedAddSourceNodeId',
  'expandedLibraryNodeIds',
  'expandedAddSourceNodeIds',
  'libraryBrowseProfile',
  'localPreviewMode'
])

export async function readViewStateFromHost(
  host: LibraryBoundaryHost
): Promise<LibraryViewStateReadResult> {
  try {
    const raw = await readFile(viewStateFilePath(host), 'utf-8')
    const parsed: unknown = JSON.parse(raw)

    if (!isValidViewState(parsed)) {
      return { state: 'empty' }
    }

    return {
      state: 'ready',
      viewState: {
        version: parsed.version,
        activeSurface: parsed.activeSurface,
        ...(parsed.selectedLibraryNodeId === undefined
          ? {}
          : { selectedLibraryNodeId: parsed.selectedLibraryNodeId }),
        ...(parsed.selectedAddSourceNodeId === undefined
          ? {}
          : { selectedAddSourceNodeId: parsed.selectedAddSourceNodeId }),
        expandedLibraryNodeIds: deduplicateStringIds(parsed.expandedLibraryNodeIds),
        expandedAddSourceNodeIds: deduplicateStringIds(parsed.expandedAddSourceNodeIds),
        ...(parsed.libraryBrowseProfile === undefined
          ? {}
          : { libraryBrowseProfile: parsed.libraryBrowseProfile }),
        ...(parsed.localPreviewMode === undefined
          ? {}
          : { localPreviewMode: parsed.localPreviewMode })
      }
    }
  } catch {
    return { state: 'empty' }
  }
}

export async function writeViewStateToHost(
  host: LibraryBoundaryHost,
  viewState: PersistedLibraryViewState
): Promise<LibraryViewStateWriteResult> {
  try {
    const dir = userDataPath(host)
    await mkdir(dir, { recursive: true })
    const payload = JSON.stringify({
      version: viewState.version,
      activeSurface: viewState.activeSurface,
      ...(viewState.selectedLibraryNodeId === undefined
        ? {}
        : { selectedLibraryNodeId: viewState.selectedLibraryNodeId }),
      ...(viewState.selectedAddSourceNodeId === undefined
        ? {}
        : { selectedAddSourceNodeId: viewState.selectedAddSourceNodeId }),
      expandedLibraryNodeIds: deduplicateStringIds(viewState.expandedLibraryNodeIds),
      expandedAddSourceNodeIds: deduplicateStringIds(viewState.expandedAddSourceNodeIds),
      ...(viewState.libraryBrowseProfile === undefined
        ? {}
        : { libraryBrowseProfile: viewState.libraryBrowseProfile }),
      ...(viewState.localPreviewMode === undefined
        ? {}
        : { localPreviewMode: viewState.localPreviewMode })
    })
    const filePath = viewStateFilePath(host)
    const tempPath = `${filePath}.tmp`
    await writeFile(tempPath, payload, 'utf-8')
    await rename(tempPath, filePath)
    return { state: 'written' }
  } catch {
    return {
      state: 'failed',
      detail: 'Unable to write library view state.'
    }
  }
}

export function writeViewStateThroughHost(
  host: LibraryBoundaryHost,
  viewState: unknown
): Promise<LibraryViewStateWriteResult> | LibraryViewStateWriteResult {
  if (!isValidViewState(viewState)) {
    return {
      state: 'failed',
      detail: 'Invalid view state payload.'
    }
  }

  return writeViewStateToHost(host, viewState)
}

function userDataPath(host: LibraryBoundaryHost): string {
  return host.config.storageEnvironment.userDataPath
}

function viewStateFilePath(host: LibraryBoundaryHost): string {
  return join(userDataPath(host), viewStateFileName)
}

export function isValidViewState(value: unknown): value is PersistedLibraryViewState {
  if (value === null || typeof value !== 'object') return false
  const obj = value as Record<string, unknown>
  if (!Object.keys(obj).every((key) => viewStateKeys.has(key))) return false
  if (obj.version !== 2) return false
  if (obj.activeSurface !== 'libraryBrowse' && obj.activeSurface !== 'addSource') return false
  if ('selectedLibraryNodeId' in obj && obj.selectedLibraryNodeId !== undefined) {
    if (typeof obj.selectedLibraryNodeId !== 'string') return false
  }
  if ('selectedAddSourceNodeId' in obj && obj.selectedAddSourceNodeId !== undefined) {
    if (typeof obj.selectedAddSourceNodeId !== 'string') return false
  }
  if (!Array.isArray(obj.expandedLibraryNodeIds)) return false
  if (!obj.expandedLibraryNodeIds.every((id: unknown) => typeof id === 'string')) return false
  if (!Array.isArray(obj.expandedAddSourceNodeIds)) return false
  if (!obj.expandedAddSourceNodeIds.every((id: unknown) => typeof id === 'string')) return false
  if ('libraryBrowseProfile' in obj && obj.libraryBrowseProfile !== undefined) {
    if (
      obj.libraryBrowseProfile !== 'audio' &&
      obj.libraryBrowseProfile !== 'playable' &&
      obj.libraryBrowseProfile !== 'allFiles'
    ) {
      return false
    }
  }
  if ('localPreviewMode' in obj && obj.localPreviewMode !== undefined) {
    return obj.localPreviewMode === 'musicEvidence' || obj.localPreviewMode === 'advancedInventory'
  }
  return true
}

function deduplicateStringIds(ids: readonly string[]): readonly string[] {
  return [...new Set(ids)]
}
