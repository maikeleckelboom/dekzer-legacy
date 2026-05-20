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
  createLibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusController,
  registerLibraryBoundaryHostStatusIpc
} from '../src/main/libraryBoundaryHostStatus'
import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  selectLibraryBoundaryHostEnvironment,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundaryHostConfig'
import { LibraryBoundaryHostError } from '../src/main/libraryBoundaryHostErrors'
import {
  createDekzerRendererApi,
  exposeDekzerRendererApi
} from '../src/preload/libraryBoundaryPreload'
import {
  libraryBoundaryHostStatusIpcChannels,
  type LibraryBoundaryHostStatus
} from '../src/shared/libraryBoundaryStatus'

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
  await validatesHostStatusProjection(devConfig)
  await validatesStatusControllerPublishesAndUnsubscribes(devConfig)
  await validatesMissingDevelopmentBinaryPublishesFailure(devConfig)
  validatesStatusIpcRegistration(devConfig)
  await validatesPreloadApiSurface()
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

async function validatesHostStatusProjection(config: LibraryBoundaryHostConfig): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const idleStatus = createLibraryBoundaryHostStatus(idleHost)

  assert.equal(idleStatus.state, 'idle')
  assert.equal(idleStatus.environment, 'development')
  assert.deepEqual(idleStatus.binaryPolicy, {
    kind: 'developmentBinary',
    source: 'environmentOverride'
  })
  assert.equal(idleStatus.lastError, null)

  const ready = deferred<void>()
  ready.resolve()

  const fakeTransport = {
    ready: ready.promise,
    close: async () => undefined,
    execute: async () => {
      throw new Error('execute should not be called by host status validation')
    }
  } satisfies LibraryBoundaryHostTransport
  const startedHost = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () => fakeTransport,
    createClient: () => createFakeClient()
  })
  const controller = new LibraryBoundaryHostStatusController(startedHost, silentStatusLogger())

  await controller.start()

  const startedStatus = controller.getStatus()
  assert.equal(startedStatus.state, 'started')
  assert.equal(startedStatus.lastError, null)
}

async function validatesStatusControllerPublishesAndUnsubscribes(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const ready = deferred<void>()
  const fakeTransport = {
    ready: ready.promise,
    close: async () => undefined,
    execute: async () => {
      throw new Error('execute should not be called by host status validation')
    }
  } satisfies LibraryBoundaryHostTransport
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () => fakeTransport,
    createClient: () => createFakeClient()
  })
  const controller = new LibraryBoundaryHostStatusController(host, silentStatusLogger())
  const publishedStates: LibraryBoundaryHostStatus['state'][] = []
  const unsubscribe = controller.onStatusChanged((status) => {
    publishedStates.push(status.state)
  })

  const started = controller.start()
  await Promise.resolve()
  assert.deepEqual(publishedStates, ['starting'])

  ready.resolve()
  await started
  assert.deepEqual(publishedStates, ['starting', 'started'])

  unsubscribe()
  await controller.start()
  assert.deepEqual(publishedStates, ['starting', 'started'])
}

async function validatesMissingDevelopmentBinaryPublishesFailure(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const secretBinaryPath = join(tempRoot, 'missing-secret-binary')
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    resolveStdioBinaryPath: () => {
      throw new LibraryBoundaryHostError(
        'missingDevelopmentBinary',
        `Missing development library boundary stdio binary at ${secretBinaryPath}.`,
        {
          details: {
            binaryPath: secretBinaryPath,
            binarySource: 'environmentOverride'
          }
        }
      )
    }
  })
  const controller = new LibraryBoundaryHostStatusController(host, silentStatusLogger())

  await controller.start()

  const status = controller.getStatus()
  assert.equal(status.state, 'failed')
  assert.equal(status.lastError?.code, 'missingDevelopmentBinary')
  assert.equal(
    status.lastError?.message,
    'The development library boundary stdio binary is missing.'
  )
  assert.equal(status.lastError?.message.includes(secretBinaryPath), false)
}

function validatesStatusIpcRegistration(config: LibraryBoundaryHostConfig): void {
  const host = new LibraryBoundaryHost(config, silentLogger())
  const controller = new LibraryBoundaryHostStatusController(host, silentStatusLogger())
  let registeredChannel: string | null = null
  let registeredHandler: (() => LibraryBoundaryHostStatus) | null = null

  registerLibraryBoundaryHostStatusIpc(
    {
      handle(channel, listener): void {
        registeredChannel = channel
        registeredHandler = () => listener({})
      }
    },
    controller
  )

  assert.equal(registeredChannel, libraryBoundaryHostStatusIpcChannels.getStatus)
  assert.equal(registeredHandler?.().state, 'idle')
}

async function validatesPreloadApiSurface(): Promise<void> {
  const status = testStatus()
  const listeners = new Map<
    string,
    Set<(event: unknown, changedStatus: LibraryBoundaryHostStatus) => void>
  >()
  const ipcRenderer = {
    invoke: async (channel) => {
      assert.equal(channel, libraryBoundaryHostStatusIpcChannels.getStatus)
      return status
    },
    on: (channel, listener) => {
      const channelListeners = listeners.get(channel) ?? new Set()
      channelListeners.add(listener)
      listeners.set(channel, channelListeners)
    },
    off: (channel, listener) => {
      listeners.get(channel)?.delete(listener)
    }
  }
  const exposedApis = new Map<string, unknown>()

  exposeDekzerRendererApi(
    {
      exposeInMainWorld(apiKey, api): void {
        exposedApis.set(apiKey, api)
      }
    },
    ipcRenderer
  )

  assert.deepEqual([...exposedApis.keys()], ['dekzer'])
  assert.equal(exposedApis.has('desktop'), false)

  const api = createDekzerRendererApi(ipcRenderer)

  assert.deepEqual(Object.keys(api), ['libraryBoundary'])
  assert.deepEqual(Object.keys(api.libraryBoundary).sort(), ['getStatus', 'onStatusChanged'])
  assert.equal('ipcRenderer' in api, false)
  assert.equal('client' in api, false)
  assert.equal('transport' in api, false)
  assert.equal('client' in api.libraryBoundary, false)
  assert.equal('transport' in api.libraryBoundary, false)
  assert.equal('ipcRenderer' in api.libraryBoundary, false)
  assert.equal(await api.libraryBoundary.getStatus(), status)

  let receivedStatus: LibraryBoundaryHostStatus | null = null
  const unsubscribe = api.libraryBoundary.onStatusChanged((changedStatus) => {
    receivedStatus = changedStatus
  })

  emitStatus(listeners, status)
  assert.equal(receivedStatus, status)

  receivedStatus = null
  unsubscribe()
  emitStatus(listeners, status)
  assert.equal(receivedStatus, null)
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

function silentStatusLogger(): {
  error(): void
} {
  return {
    error: () => undefined
  }
}

function testStatus(): LibraryBoundaryHostStatus {
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

function emitStatus(
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
  } as LibraryBoundaryHostClient
}

function rejectUnexpectedClientCall(): Promise<never> {
  return Promise.reject(new Error('client methods should not be called by host validation'))
}
