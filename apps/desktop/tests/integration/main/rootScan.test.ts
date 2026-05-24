import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  registerLocalRootScanIpc,
  runLocalRootScanThroughHost
} from '../../../src/main/libraryRoots/runScan'
import { rootChannels } from '../../../src/shared/libraryRoots/channels'
import type { LocalRootScanResult } from '../../../src/shared/libraryRoots/runScan'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/libraryBoundary'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('local root scan boundary', () => {
  it('runs scans through a started host and maps invalid or unavailable requests', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      runLocalRootScanThroughHost(idleHost, { rootId: 'root-1' })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    await expect(
      runLocalRootScanThroughHost(await startedHostWithClient(config, createFakeClient()), null)
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    await expect(
      runLocalRootScanThroughHost(await startedHostWithClient(config, createFakeClient()), {
        rootId: '   '
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    let receivedRootId = ''
    await expect(
      runLocalRootScanThroughHost(
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
        { rootId: 'root-1' }
      )
    ).resolves.toEqual({
      state: 'scanned',
      rootId: 'root-1',
      scanRunId: 'scan-1',
      discoveredFileCount: 12,
      queuedSourceWorkItems: 8
    } satisfies LocalRootScanResult)
    expect(receivedRootId).toBe('root-1')

    await expect(
      runLocalRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            runRootScan: async () => {
              throw new Error('fixture scan failure')
            }
          })
        ),
        { rootId: 'root-1' }
      )
    ).resolves.toMatchObject({
      state: 'scanFailed',
      error: { code: 'scanFailed' }
    })
  })

  it('registers the scan IPC channel', () => {
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
      new LibraryBoundaryHost(hostConfig(), silentLogger())
    )

    expect(registration.channel).toBe(rootChannels.runScan)
    expect(typeof registration.handler).toBe('function')
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-root-scan-'))
  tempRoots.push(tempRoot)
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
