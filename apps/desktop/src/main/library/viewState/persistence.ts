import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { join } from 'node:path'

import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from '../../../shared/library/viewState/persistence'
import type { LibraryBoundaryHost } from '../boundary/host'

const viewStateFileName = 'library-view-state.json'

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
        ...(parsed.selectedNodeId === undefined ? {} : { selectedNodeId: parsed.selectedNodeId }),
        expandedNodeIds: deduplicateStringIds(parsed.expandedNodeIds),
        ...(parsed.profile === undefined ? {} : { profile: parsed.profile })
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
      ...(viewState.selectedNodeId === undefined
        ? {}
        : { selectedNodeId: viewState.selectedNodeId }),
      expandedNodeIds: deduplicateStringIds(viewState.expandedNodeIds),
      ...(viewState.profile === undefined ? {} : { profile: viewState.profile })
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
  if (obj.version !== 1) return false
  if ('selectedNodeId' in obj && obj.selectedNodeId !== undefined) {
    if (typeof obj.selectedNodeId !== 'string') return false
  }
  if (!Array.isArray(obj.expandedNodeIds)) return false
  if (!obj.expandedNodeIds.every((id: unknown) => typeof id === 'string')) return false
  if ('profile' in obj && obj.profile !== undefined) {
    return obj.profile === 'audio' || obj.profile === 'playable' || obj.profile === 'allFiles'
  }
  return true
}

function deduplicateStringIds(ids: readonly string[]): readonly string[] {
  return [...new Set(ids)]
}
