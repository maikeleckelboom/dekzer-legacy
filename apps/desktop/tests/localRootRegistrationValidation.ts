import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  LibraryBoundaryHost,
  type LibraryBoundaryHostClient,
  type LibraryBoundaryHostTransport
} from '../src/main/libraryBoundary/host'
import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import {
  registerLocalRootRegistrationIpc,
  registerLocalRootThroughHost
} from '../src/main/libraryRoots/registerLocalRoot'
import {
  libraryRootRegistrationIpcChannels,
  type LocalRootRegistrationResult
} from '../src/shared/libraryRoots/registerLocalRoot'
import { createFakeClient, deferred, silentLogger, testApp } from './support/libraryBoundary'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-root-registration-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  const config = resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })

  await validatesLocalRootRegistrationHandler(config)
  validatesLocalRootRegistrationIpcRegistration(config)
}

async function validatesLocalRootRegistrationHandler(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await registerLocalRootThroughHost(idleHost, {
    absolutePath: 'C:/Music'
  })

  assert.equal(hostUnavailable.state, 'hostUnavailable')
  assertRegistrationError(hostUnavailable, 'hostNotStarted')

  const invalidRequest = await registerLocalRootThroughHost(
    await startedHostWithClient(config, createFakeClient()),
    {
      absolutePath: '   '
    }
  )

  assert.equal(invalidRequest.state, 'invalidRequest')
  assertRegistrationError(invalidRequest, 'invalidRequest')

  let receivedAbsolutePath = ''
  const success = await registerLocalRootThroughHost(
    await startedHostWithClient(
      config,
      createFakeClient({
        registerLocalRoot: async (request) => {
          receivedAbsolutePath = request.absolutePath
          return {
            rootId: '7',
            canonicalPath: 'C:/Music'
          }
        }
      })
    ),
    {
      absolutePath: 'C:/Music'
    }
  )

  assert.equal(receivedAbsolutePath, 'C:/Music')
  assert.deepEqual(success, {
    state: 'registered',
    root: {
      rootId: '7',
      canonicalPath: 'C:/Music'
    }
  } satisfies LocalRootRegistrationResult)

  const failure = await registerLocalRootThroughHost(
    await startedHostWithClient(
      config,
      createFakeClient({
        registerLocalRoot: async () => {
          throw new Error('fixture registration failure')
        }
      })
    ),
    {
      absolutePath: 'C:/Missing'
    }
  )

  assert.equal(failure.state, 'registrationFailed')
  assertRegistrationError(failure, 'registrationFailed')
}

function validatesLocalRootRegistrationIpcRegistration(config: LibraryBoundaryHostConfig): void {
  const host = new LibraryBoundaryHost(config, silentLogger())

  const registration: {
    channel?: string
    handler?: (request: unknown) => Promise<LocalRootRegistrationResult>
  } = {}

  registerLocalRootRegistrationIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (request) => listener({}, request)
      }
    },
    host
  )

  assert.equal(registration.channel, libraryRootRegistrationIpcChannels.registerLocalRoot)
  assert.equal(typeof registration.handler, 'function')
}

async function startedHostWithClient(
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
          throw new Error('execute should not be called by local root registration validation')
        }
      }) satisfies LibraryBoundaryHostTransport,
    createClient: () => client
  })

  await host.start()
  return host
}

function assertRegistrationError(
  result: LocalRootRegistrationResult,
  code: Exclude<LocalRootRegistrationResult, { state: 'registered' }>['error']['code']
): void {
  if (result.state === 'registered') {
    assert.fail(`expected local root registration error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}
