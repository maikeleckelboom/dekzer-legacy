import { strict as assert } from 'node:assert'
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import {
  LibraryBoundaryHost,
  type LibraryBoundaryHostClient,
  type LibraryBoundaryHostTransport
} from '../src/main/libraryBoundaryHost'
import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  selectLibraryBoundaryHostEnvironment,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundaryHostConfig'
import { LibraryBoundaryHostError } from '../src/main/libraryBoundaryHostErrors'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-host-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  assert.equal(selectLibraryBoundaryHostEnvironment(true), 'development')
  assert.equal(selectLibraryBoundaryHostEnvironment(false), 'production')

  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  const devConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })

  assert.equal(devConfig.environment, 'development')
  assert.equal(devConfig.userDataPath, join(tempRoot, 'user-data'))
  assert.deepEqual(devConfig.binaryPolicy, {
    kind: 'developmentBinary',
    binaryPath: resolve(fakeBinaryPath),
    source: 'environmentOverride'
  })
  assert.equal(
    resolveLibraryBoundaryStdioBinaryPath(devConfig.binaryPolicy),
    resolve(fakeBinaryPath)
  )

  const defaultDevConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {},
    platform: 'win32'
  })

  assert.deepEqual(defaultDevConfig.binaryPolicy, {
    kind: 'developmentBinary',
    binaryPath: join(tempRoot, 'target', 'debug', 'library-boundary-stdio.exe'),
    source: 'repoDebugTarget'
  })

  assertHostError(
    () => resolveLibraryBoundaryStdioBinaryPath(defaultDevConfig.binaryPolicy, () => false),
    'missingDevelopmentBinary'
  )

  const prodConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: false,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux',
    resourcesPath: join(tempRoot, 'resources')
  })

  assert.equal(prodConfig.environment, 'production')
  assert.deepEqual(prodConfig.binaryPolicy, {
    kind: 'packagedBinaryUnavailable',
    executableName: 'library-boundary-stdio',
    resourceRoot: join(tempRoot, 'resources')
  })
  assertHostError(
    () => resolveLibraryBoundaryStdioBinaryPath(prodConfig.binaryPolicy),
    'packagedBinaryUnavailable'
  )

  assert.equal(existsSync(fakeBinaryPath), true)

  await validatesHostStartWaitsForTransportReadiness(devConfig)
  await validatesReadinessFailureMapsToStartupFailure(devConfig)
  validatesHostConstructionDoesNotStartTransport(devConfig)
}

async function validatesHostStartWaitsForTransportReadiness(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const ready = deferred<void>()
  const fakeClient = createFakeClient()
  let createdClient = false
  const fakeTransport = {
    ready: ready.promise,
    close: async () => undefined,
    execute: async () => {
      throw new Error('execute should not be called by host validation')
    }
  } satisfies LibraryBoundaryHostTransport
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () => fakeTransport,
    createClient: (transport) => {
      assert.equal(transport, fakeTransport)
      createdClient = true
      return fakeClient
    }
  })

  const started = host.start()
  await Promise.resolve()

  assert.equal(host.state, 'starting')
  assert.equal(createdClient, false)
  assertHostError(() => {
    void host.client
  }, 'notStarted')

  ready.resolve()
  assert.equal(await started, fakeClient)
  assert.equal(createdClient, true)
  assert.equal(host.state, 'started')
  assert.equal(host.client, fakeClient)
}

async function validatesReadinessFailureMapsToStartupFailure(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const startupFailure = new Error('fixture readiness failure')
  const ready = deferred<void>()
  let closeCalls = 0
  const fakeTransport = {
    ready: ready.promise,
    close: async () => {
      closeCalls += 1
    },
    execute: async () => {
      throw new Error('execute should not be called by host validation')
    }
  } satisfies LibraryBoundaryHostTransport
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () => fakeTransport,
    createClient: () => {
      throw new Error('client should not be created before readiness')
    }
  })

  const started = host.start()
  ready.reject(startupFailure)

  await assert.rejects(started, (error: unknown) => {
    assert.equal(error instanceof LibraryBoundaryHostError, true)
    assert.equal((error as LibraryBoundaryHostError).code, 'stdioTransportStartupFailure')
    assert.equal((error as Error).cause, startupFailure)
    return true
  })
  assert.equal(closeCalls, 1)
  assert.equal(host.state, 'failed')
}

function validatesHostConstructionDoesNotStartTransport(config: LibraryBoundaryHostConfig): void {
  let transportCreations = 0
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () => {
      transportCreations += 1
      throw new Error('transport should be lazy')
    }
  })

  assert.equal(host.state, 'idle')
  assert.equal(host.hasStarted, false)
  assert.equal(transportCreations, 0)
}

function testApp(options: { readonly appPath: string }): {
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

function assertHostError(action: () => void, code: LibraryBoundaryHostError['code']): void {
  assert.throws(action, (error: unknown) => {
    assert.equal(error instanceof LibraryBoundaryHostError, true)
    assert.equal((error as LibraryBoundaryHostError).code, code)
    return true
  })
}

function deferred<T>(): {
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

function silentLogger(): {
  warn(): void
} {
  return {
    warn: () => undefined
  }
}

function createFakeClient(): LibraryBoundaryHostClient {
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
    readPendingBoundaryEvents: rejectUnexpectedClientCall
  } satisfies LibraryBoundaryHostClient
}

function rejectUnexpectedClientCall(): Promise<never> {
  return Promise.reject(new Error('client methods should not be called by host validation'))
}
