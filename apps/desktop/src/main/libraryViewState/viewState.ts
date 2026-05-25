import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { libraryViewStateChannels } from '../../shared/libraryViewState/viewState'
import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from '../../shared/libraryViewState/viewState'
import type { LibraryBoundaryHost } from '../libraryBoundary/host'

export type LibraryViewStateIpcMain = {
  handle(channel: string, listener: (...args: readonly unknown[]) => unknown): void
}

const viewStateFileName = 'library-view-state.json'

export function registerLibraryViewStateIpc(
  ipcMain: LibraryViewStateIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(libraryViewStateChannels.readViewState, () => readViewStateFromHost(host))

  ipcMain.handle(libraryViewStateChannels.writeViewState, (_event: unknown, viewState: unknown) => {
    if (!isValidViewState(viewState)) {
      const result: LibraryViewStateWriteResult = {
        state: 'failed',
        detail: 'Invalid view state payload.'
      }
      return result
    }

    return writeViewStateToHost(host, viewState)
  })
}

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
        expandedNodeIds: deduplicateStringIds(parsed.expandedNodeIds)
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
      expandedNodeIds: deduplicateStringIds(viewState.expandedNodeIds)
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
  return obj.expandedNodeIds.every((id: unknown) => typeof id === 'string')
}

function deduplicateStringIds(ids: readonly string[]): readonly string[] {
  return [...new Set(ids)]
}
