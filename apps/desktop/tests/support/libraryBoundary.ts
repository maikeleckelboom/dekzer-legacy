import { strict as assert } from 'node:assert'
import { join } from 'node:path'

import type { LibraryBoundaryHostClient } from '../../src/main/libraryBoundary/host'
import { LibraryBoundaryHostError } from '../../src/main/libraryBoundary/errors'
import {
  libraryBoundaryHostStatusIpcChannels,
  type LibraryBoundaryHostStatus
} from '../../src/shared/libraryBoundary/status'

export function testApp(
  tempRoot: string,
  options: { readonly appPath: string }
): {
  getPath(name: 'userData'): string
  getAppPath(): string
} {
  return {
    getPath(name: 'userData'): string {
      assert.equal(name, 'userData')
      return join(tempRoot, 'user-data')
    },
    getAppPath(): string {
      return options.appPath
    }
  }
}

export function assertHostError(action: () => void, code: LibraryBoundaryHostError['code']): void {
  assert.throws(action, (error: unknown) => {
    assert.equal(error instanceof LibraryBoundaryHostError, true)
    assert.equal((error as LibraryBoundaryHostError).code, code)
    return true
  })
}

export function deferred<T>(): {
  readonly promise: Promise<T>
  readonly resolve: (value: T | PromiseLike<T>) => void
  readonly reject: (reason?: unknown) => void
} {
  let resolve: ((value: T | PromiseLike<T>) => void) | null = null
  let reject: ((reason?: unknown) => void) | null = null
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve
    reject = innerReject
  })

  if (resolve === null || reject === null) {
    throw new Error('deferred promise was not initialized')
  }

  return { promise, resolve, reject }
}

export function silentLogger(): {
  warn(): void
} {
  return {
    warn: () => undefined
  }
}

export function silentStatusLogger(): {
  error(): void
} {
  return {
    error: () => undefined
  }
}

export function testStatus(): LibraryBoundaryHostStatus {
  return {
    state: 'idle',
    environment: 'development',
    binaryPolicy: {
      kind: 'developmentBinary',
      source: 'environmentOverride'
    },
    lastError: null
  }
}

export function emitStatus(
  listeners: ReadonlyMap<
    string,
    ReadonlySet<(event: unknown, changedStatus: LibraryBoundaryHostStatus) => void>
  >,
  status: LibraryBoundaryHostStatus
): void {
  for (const listener of listeners.get(libraryBoundaryHostStatusIpcChannels.statusChanged) ?? []) {
    listener({}, status)
  }
}

export function createFakeClient(
  overrides: Partial<LibraryBoundaryHostClient> = {}
): LibraryBoundaryHostClient {
  return {
    registerLocalRoot: rejectUnexpectedClientCall,
    runRootScan: rejectUnexpectedClientCall,
    readNavigationRows: rejectUnexpectedClientCall,
    loadNavigationRow: rejectUnexpectedClientCall,
    loadNavigationRowByStableKey: rejectUnexpectedClientCall,
    readLiteralHierarchyChildren: rejectUnexpectedClientCall,
    readNavigationNodeLibraryBrowserWindow: rejectUnexpectedClientCall,
    searchNavigationNodeLibraryBrowserWindow: rejectUnexpectedClientCall,
    createPlaylist: rejectUnexpectedClientCall,
    renamePlaylist: rejectUnexpectedClientCall,
    deletePlaylist: rejectUnexpectedClientCall,
    readPendingBoundaryEvents: rejectUnexpectedClientCall,
    ...overrides
  } as LibraryBoundaryHostClient
}

function rejectUnexpectedClientCall(): Promise<never> {
  return Promise.reject(new Error('client methods should not be called by host validation'))
}
