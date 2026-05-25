import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'
import { LibraryBoundaryStdioProcessExitError } from '@dekzer/library-boundary-stdio-transport'

import {
  desktopLibraryUserDataEnvironmentVariable,
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  selectLibraryBoundaryHostEnvironment,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHostError } from '../../../src/main/libraryBoundary/errors'
import {
  LibraryBoundaryHost,
  type LibraryBoundaryHostTransport,
  type LibraryBoundaryHostTransportOptions
} from '../../../src/main/libraryBoundary/host'
import {
  createLibraryBoundaryHostStatus,
  LibraryBoundaryHostStatusController,
  registerLibraryBoundaryHostStatusIpc
} from '../../../src/main/libraryBoundary/status'
import { hostStatusChannels } from '../../../src/shared/libraryBoundary/status'
import {
  createFakeClient,
  deferred,
  silentLogger,
  silentStatusLogger,
  testApp
} from '../../support/libraryBoundary'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('library boundary host', () => {
  it('resolves development host config and rejects unsafe paths', () => {
    const tempRoot = tempRootFor('dekzer-desktop-host-config-')
    const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
    writeFileSync(fakeBinaryPath, '')

    expect(selectLibraryBoundaryHostEnvironment(true)).toBe('development')
    expect(selectLibraryBoundaryHostEnvironment(false)).toBe('production')

    const devConfig = resolveLibraryBoundaryHostConfig({
      app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
      isDev: true,
      env: {
        [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
      },
      platform: 'linux'
    })

    expect(devConfig).toMatchObject({
      environment: 'development',
      storageEnvironment: {
        kind: 'userDataRoot',
        userDataPath: join(tempRoot, '.dev-user-data', 'default'),
        source: 'developmentDefault'
      },
      binaryPolicy: {
        kind: 'developmentBinary',
        binaryPath: resolve(fakeBinaryPath),
        source: 'environmentOverride'
      }
    })
    expect(resolveLibraryBoundaryStdioBinaryPath(devConfig.binaryPolicy)).toBe(
      resolve(fakeBinaryPath)
    )

    const overriddenUserDataPath = join(tempRoot, 'diagnostic-user-data')
    expect(
      resolveLibraryBoundaryHostConfig({
        app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
        isDev: true,
        env: {
          [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath,
          [desktopLibraryUserDataEnvironmentVariable]: overriddenUserDataPath
        },
        platform: 'linux'
      }).storageEnvironment
    ).toEqual({
      kind: 'userDataRoot',
      userDataPath: overriddenUserDataPath,
      source: 'environmentOverride'
    })

    expectHostError(
      () =>
        resolveLibraryBoundaryHostConfig({
          app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
          isDev: true,
          env: {
            [desktopLibraryUserDataEnvironmentVariable]: 'relative-user-data'
          },
          platform: 'linux'
        }),
      'invalidUserDataPath'
    )

    expect(existsSync(fakeBinaryPath)).toBe(true)
  })

  it('starts only after transport readiness and maps readiness failure safely', async () => {
    const config = hostConfig()
    const ready = deferred<void>()
    const fakeClient = createFakeClient()
    let createdClient = false
    const fakeTransport = {
      ready: ready.promise,
      close: async () => undefined,
      execute: async () => {
        throw new Error('execute should not be called by host tests')
      }
    } satisfies LibraryBoundaryHostTransport
    const host = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: () => fakeTransport,
      createClient: (transport) => {
        expect(transport).toBe(fakeTransport)
        createdClient = true
        return fakeClient
      }
    })

    const started = host.start()
    await Promise.resolve()

    expect(host.state).toBe('starting')
    expect(createdClient).toBe(false)
    expectHostError(() => {
      void host.client
    }, 'notStarted')

    ready.resolve()
    await expect(started).resolves.toBe(fakeClient)
    expect(createdClient).toBe(true)
    expect(host.state).toBe('started')
    expect(host.client).toBe(fakeClient)

    const startupFailure = new Error('fixture readiness failure')
    const failedReady = deferred<void>()
    let closeCalls = 0
    const failingHost = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: () =>
        ({
          ready: failedReady.promise,
          close: async () => {
            closeCalls += 1
          },
          execute: async () => {
            throw new Error('execute should not be called by host tests')
          }
        }) satisfies LibraryBoundaryHostTransport,
      createClient: () => {
        throw new Error('client should not be created before readiness')
      }
    })

    const failedStart = failingHost.start()
    failedReady.reject(startupFailure)

    await expect(failedStart).rejects.toMatchObject({
      code: 'stdioTransportStartupFailure',
      cause: startupFailure
    })
    expect(closeCalls).toBe(1)
    expect(failingHost.state).toBe('failed')
  })

  it('publishes status transitions and sanitizes startup failures', async () => {
    const config = hostConfig()
    const ready = deferred<void>()
    const host = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: () =>
        ({
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by host status tests')
          }
        }) satisfies LibraryBoundaryHostTransport,
      createClient: () => createFakeClient()
    })
    const controller = new LibraryBoundaryHostStatusController(host, silentStatusLogger())
    const publishedStates: string[] = []
    const unsubscribe = controller.onStatusChanged((status) => {
      publishedStates.push(status.state)
    })

    const started = controller.start()
    await Promise.resolve()
    ready.resolve()
    await started

    expect(publishedStates).toEqual(['starting', 'started'])
    unsubscribe()
    await controller.start()
    expect(publishedStates).toEqual(['starting', 'started'])

    const secretBinaryPath = join(tempRootFor('dekzer-desktop-host-failure-'), 'missing-secret')
    const failingHost = new LibraryBoundaryHost(config, silentLogger(), {
      resolveStdioBinaryPath: () => {
        throw new LibraryBoundaryHostError(
          'missingDevelopmentBinary',
          `Missing development library boundary stdio binary at ${secretBinaryPath}.`
        )
      }
    })
    const failingController = new LibraryBoundaryHostStatusController(
      failingHost,
      silentStatusLogger()
    )

    await failingController.start()

    expect(failingController.getStatus()).toMatchObject({
      state: 'failed',
      lastError: {
        code: 'missingDevelopmentBinary',
        message: 'The development library boundary stdio binary is missing.'
      }
    })
    expect(failingController.getStatus().lastError?.message.includes(secretBinaryPath)).toBe(false)
  })

  it('registers host status IPC on the status channel', () => {
    const host = new LibraryBoundaryHost(hostConfig(), silentLogger())
    const controller = new LibraryBoundaryHostStatusController(host, silentStatusLogger())
    const registration: {
      channel?: string
      handler?: () => ReturnType<typeof createLibraryBoundaryHostStatus>
    } = {}

    registerLibraryBoundaryHostStatusIpc(
      {
        handle(channel, listener): void {
          registration.channel = channel
          registration.handler = () => listener({})
        }
      },
      controller
    )

    expect(registration.channel).toBe(hostStatusChannels.getStatus)
    expect(registration.handler?.().state).toBe('idle')
  })

  it('classifies schema mismatch startup failures from startup diagnostics', async () => {
    const config = hostConfig()
    const schemaDiagnostic =
      'database schema state is malformed: database schema does not match the canonical substrate baseline: table source_directories column count mismatch: canonical=17 live=16'
    const schemaMismatchHost = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: (options) => {
        const ready = deferred<void>()
        options.diagnostics?.({ stream: 'stderr', line: schemaDiagnostic })
        ready.reject(new LibraryBoundaryStdioProcessExitError(1, null))
        return {
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by schema mismatch tests')
          }
        } satisfies LibraryBoundaryHostTransport
      },
      createClient: () => createFakeClient()
    })

    const controller = new LibraryBoundaryHostStatusController(
      schemaMismatchHost,
      silentStatusLogger()
    )

    await controller.start()

    expect(controller.getStatus()).toMatchObject({
      state: 'failed',
      lastError: {
        code: 'stdioTransportStartupFailure',
        message: 'The library database is incompatible with the current schema.'
      }
    })
    expect(controller.getStatus().lastError?.detail).toBe(schemaDiagnostic)
  })

  it('does not reclassify process exits without schema diagnostics', async () => {
    const config = hostConfig()
    const unrelatedHost = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: () => {
        const ready = deferred<void>()
        ready.reject(new LibraryBoundaryStdioProcessExitError(1, null))
        return {
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by unrelated failure tests')
          }
        } satisfies LibraryBoundaryHostTransport
      },
      createClient: () => createFakeClient()
    })

    const controller = new LibraryBoundaryHostStatusController(unrelatedHost, silentStatusLogger())

    await controller.start()

    expect(controller.getStatus()).toMatchObject({
      state: 'failed',
      lastError: {
        code: 'stdioTransportStartupFailure',
        message: 'Failed to start the library boundary stdio transport.'
      }
    })
  })

  it('does not reclassify unrelated stderr diagnostics with process exits', async () => {
    const config = hostConfig()
    const unrelatedHost = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: (options) => {
        const ready = deferred<void>()
        options.diagnostics?.({ stream: 'stderr', line: 'opened development database' })
        ready.reject(new LibraryBoundaryStdioProcessExitError(1, null))
        return {
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by unrelated diagnostic tests')
          }
        } satisfies LibraryBoundaryHostTransport
      },
      createClient: () => createFakeClient()
    })

    const controller = new LibraryBoundaryHostStatusController(unrelatedHost, silentStatusLogger())

    await controller.start()

    expect(controller.getStatus()).toMatchObject({
      state: 'failed',
      lastError: {
        code: 'stdioTransportStartupFailure',
        message: 'Failed to start the library boundary stdio transport.',
        detail: 'library boundary stdio process exited unexpectedly: code=1 signal=null'
      }
    })
  })

  it('bounds startup diagnostics captured before readiness', async () => {
    const config = hostConfig()
    const host = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: (options: LibraryBoundaryHostTransportOptions) => {
        const ready = deferred<void>()
        for (let i = 0; i < 12; i += 1) {
          options.diagnostics?.({ stream: 'stderr', line: `diagnostic ${i}` })
        }
        ready.reject(new LibraryBoundaryStdioProcessExitError(1, null))
        return {
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by bounded diagnostics tests')
          }
        } satisfies LibraryBoundaryHostTransport
      },
      createClient: () => createFakeClient()
    })

    await expect(host.start()).rejects.toMatchObject({
      code: 'stdioTransportStartupFailure',
      details: {
        startupDiagnostics: [
          'diagnostic 4',
          'diagnostic 5',
          'diagnostic 6',
          'diagnostic 7',
          'diagnostic 8',
          'diagnostic 9',
          'diagnostic 10',
          'diagnostic 11'
        ]
      }
    })
  })

  it('preserves schema diagnostic beyond recent diagnostics buffer', async () => {
    const config = hostConfig()
    const schemaLine =
      'database schema state is malformed: database schema does not match the canonical substrate baseline: table source_directories column count mismatch: canonical=17 live=16'
    const schemaMismatchHost = new LibraryBoundaryHost(config, silentLogger(), {
      createTransport: (options) => {
        const ready = deferred<void>()
        options.diagnostics?.({ stream: 'stderr', line: schemaLine })
        for (let i = 0; i < 12; i += 1) {
          options.diagnostics?.({ stream: 'stderr', line: `noise ${i}` })
        }
        ready.reject(new LibraryBoundaryStdioProcessExitError(1, null))
        return {
          ready: ready.promise,
          close: async () => undefined,
          execute: async () => {
            throw new Error('execute should not be called by schema diagnostic preservation tests')
          }
        } satisfies LibraryBoundaryHostTransport
      },
      createClient: () => createFakeClient()
    })

    const controller = new LibraryBoundaryHostStatusController(
      schemaMismatchHost,
      silentStatusLogger()
    )

    await controller.start()

    expect(controller.getStatus()).toMatchObject({
      state: 'failed',
      lastError: {
        code: 'stdioTransportStartupFailure',
        message: 'The library database is incompatible with the current schema.',
        detail: schemaLine
      }
    })
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = tempRootFor('dekzer-desktop-host-')
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  return resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })
}

function tempRootFor(prefix: string): string {
  const tempRoot = mkdtempSync(join(tmpdir(), prefix))
  tempRoots.push(tempRoot)
  return tempRoot
}

function expectHostError(action: () => void, code: LibraryBoundaryHostError['code']): void {
  try {
    action()
  } catch (error: unknown) {
    expect(error).toBeInstanceOf(LibraryBoundaryHostError)
    expect((error as LibraryBoundaryHostError).code).toBe(code)
    return
  }

  throw new Error(`Expected LibraryBoundaryHostError ${code}.`)
}
