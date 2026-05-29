import { strict as assert } from 'node:assert'
import { join } from 'node:path'

import {
  LibraryBoundaryHost,
  type LibraryBoundaryHostClient,
  type LibraryBoundaryHostTransport
} from '../../src/main/libraryBoundary/host'
import type { LibraryBoundaryHostConfig } from '../../src/main/libraryBoundary/config'
import { LibraryBoundaryHostError } from '../../src/main/libraryBoundary/errors'
import {
  hostStatusChannels,
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
  for (const listener of listeners.get(hostStatusChannels.statusChanged) ?? []) {
    listener({}, status)
  }
}

export function createFakeClient(
  overrides: Partial<LibraryBoundaryHostClient> = {}
): LibraryBoundaryHostClient {
  return {
    registerLocalRoot: rejectUnexpectedClientCall,
    startRootScan: rejectUnexpectedClientCall,
    readNavigationRows: rejectUnexpectedClientCall,
    loadNavigationRow: rejectUnexpectedClientCall,
    loadNavigationRowByStableKey: rejectUnexpectedClientCall,
    readLibraryTreeChildren: rejectUnexpectedClientCall,
    readNavigationNodeLibraryBrowserWindow: rejectUnexpectedClientCall,
    searchNavigationNodeLibraryBrowserWindow: rejectUnexpectedClientCall,
    createPlaylist: rejectUnexpectedClientCall,
    renamePlaylist: rejectUnexpectedClientCall,
    deletePlaylist: rejectUnexpectedClientCall,
    ...overrides
  } as LibraryBoundaryHostClient
}

export async function startedHostWithClient(
  config: LibraryBoundaryHostConfig,
  client: LibraryBoundaryHostClient
): Promise<LibraryBoundaryHost> {
  const ready = deferred<void>()
  ready.resolve()

  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () =>
      ({
        ready: ready.promise,
        close: async () => undefined,
        execute: async () => {
          throw new Error('transport execute should not be called when using a fake client')
        }
      }) satisfies LibraryBoundaryHostTransport,
    createClient: () => client
  })

  await host.start()
  return host
}

function rejectUnexpectedClientCall(): Promise<never> {
  return Promise.reject(new Error('client methods should not be called by this test'))
}
