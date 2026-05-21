import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../src/main/libraryBoundary/host'
import { registerLocalRoot } from '../src/main/libraryRoots/registerLocalRoot'
import type { LocalRootRegistrationResult } from '../src/shared/libraryRoots/registerLocalRoot'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from './support/libraryBoundary'

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
}

async function validatesLocalRootRegistrationHandler(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await registerLocalRoot(idleHost, {
    absolutePath: 'C:/Music'
  })

  assert.equal(hostUnavailable.state, 'hostUnavailable')
  assertRegistrationError(hostUnavailable, 'hostNotStarted')

  const invalidRequest = await registerLocalRoot(
    await startedHostWithClient(config, createFakeClient()),
    {
      absolutePath: '   '
    }
  )

  assert.equal(invalidRequest.state, 'invalidRequest')
  assertRegistrationError(invalidRequest, 'invalidRequest')

  let receivedAbsolutePath = ''
  const success = await registerLocalRoot(
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

  const failure = await registerLocalRoot(
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

function assertRegistrationError(
  result: LocalRootRegistrationResult,
  code: Exclude<LocalRootRegistrationResult, { state: 'registered' }>['error']['code']
): void {
  if (result.state === 'registered') {
    assert.fail(`expected local root registration error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}
