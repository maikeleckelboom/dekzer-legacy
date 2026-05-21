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
import {
  registerLocalRootScanIpc,
  runLocalRootScanThroughHost
} from '../src/main/libraryRoots/runScan'
import { libraryRootsIpcChannels } from '../src/shared/libraryRoots/channels'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from './support/libraryBoundary'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-root-scan-'))

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

  await validatesLocalRootScanHandler(config)
  validatesLocalRootScanIpcRegistration(config)
}

async function validatesLocalRootScanHandler(config: LibraryBoundaryHostConfig): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await runLocalRootScanThroughHost(idleHost, {
    rootId: 'root-1'
  })

  assert.equal(hostUnavailable.state, 'hostUnavailable')
  assertScanError(hostUnavailable, 'hostNotStarted')

  const invalidRequest = await runLocalRootScanThroughHost(
    await startedHostWithClient(config, createFakeClient()),
    null
  )

  assert.equal(invalidRequest.state, 'invalidRequest')
  assertScanError(invalidRequest, 'invalidRequest')

  const blankRootId = await runLocalRootScanThroughHost(
    await startedHostWithClient(config, createFakeClient()),
    {
      rootId: '   '
    }
  )

  assert.equal(blankRootId.state, 'invalidRequest')
  assertScanError(blankRootId, 'invalidRequest')

  let receivedRootId = ''
  const success = await runLocalRootScanThroughHost(
    await startedHostWithClient(
      config,
      createFakeClient({
        runRootScan: async (request) => {
          receivedRootId = request.rootId
          return {
            rootId: 'root-1',
            scanRunId: 'scan-1',
            discoveredFileCount: 12,
            queuedSourceWorkItems: 8
          }
        }
      })
    ),
    {
      rootId: 'root-1'
    }
  )

  assert.equal(receivedRootId, 'root-1')
  assert.deepEqual(success, {
    state: 'scanned',
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  } satisfies LocalRootScanResult)

  const failure = await runLocalRootScanThroughHost(
    await startedHostWithClient(
      config,
      createFakeClient({
        runRootScan: async () => {
          throw new Error('fixture scan failure')
        }
      })
    ),
    {
      rootId: 'root-1'
    }
  )

  assert.equal(failure.state, 'scanFailed')
  assertScanError(failure, 'scanFailed')
}

function validatesLocalRootScanIpcRegistration(config: LibraryBoundaryHostConfig): void {
  const host = new LibraryBoundaryHost(config, silentLogger())

  const registration: {
    channel?: string
    handler?: (request: unknown) => Promise<LocalRootScanResult>
  } = {}

  registerLocalRootScanIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (request) => listener({}, request)
      }
    },
    host
  )

  assert.equal(registration.channel, libraryRootsIpcChannels.runScan)
  assert.equal(typeof registration.handler, 'function')
}

function assertScanError(
  result: LocalRootScanResult,
  code: Exclude<LocalRootScanResult, { state: 'scanned' }>['error']['code']
): void {
  if (result.state === 'scanned') {
    assert.fail(`expected local root scan error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}
