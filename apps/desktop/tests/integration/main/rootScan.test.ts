import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { LibraryBoundaryProtocolError } from '@dekzer/library-boundary-client'

import { afterEach, describe, expect, it } from 'vitest'

import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  type ScanLogger,
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

const noLog: ScanLogger = {
  // eslint-disable-next-line @typescript-eslint/no-empty-function
  error() {}
}

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
      runLocalRootScanThroughHost(idleHost, { rootId: 'root-1' }, noLog)
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    await expect(
      runLocalRootScanThroughHost(
        await startedHostWithClient(config, createFakeClient()),
        null,
        noLog
      )
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    await expect(
      runLocalRootScanThroughHost(
        await startedHostWithClient(config, createFakeClient()),
        {
          rootId: '   '
        },
        noLog
      )
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
            startRootScan: async (request) => {
              receivedRootId = request.rootId
              return {
                scanRunId: 'scan-1'
              }
            }
          })
        ),
        { rootId: 'root-1' },
        noLog
      )
    ).resolves.toEqual({
      state: 'started',
      scanRunId: 'scan-1'
    } satisfies LocalRootScanResult)
    expect(receivedRootId).toBe('root-1')

    await expect(
      runLocalRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            startRootScan: async () => {
              throw new Error('fixture scan failure')
            }
          })
        ),
        { rootId: 'root-1' },
        noLog
      )
    ).resolves.toMatchObject({
      state: 'scanFailed',
      error: { code: 'scanFailed', detail: 'fixture scan failure' }
    })
  })

  it('preserves scan failure detail from thrown errors', async () => {
    const config = hostConfig()

    const protocolErrorResult = await runLocalRootScanThroughHost(
      await startedHostWithClient(
        config,
        createFakeClient({
          startRootScan: async () => {
            throw new LibraryBoundaryProtocolError({
              type: 'durableStoreFailure',
              payload: { detail: 'database is locked' }
            })
          }
        })
      ),
      { rootId: 'root-2' },
      noLog
    )
    expect(protocolErrorResult.state).toBe('scanFailed')
    if (protocolErrorResult.state === 'scanFailed') {
      expect(protocolErrorResult.error.message).toContain('protocol error')
      expect(protocolErrorResult.error.detail).toContain('database is locked')
    }

    const unknownThrownResult = await runLocalRootScanThroughHost(
      await startedHostWithClient(
        config,
        createFakeClient({
          startRootScan: async () => {
            throw 'unexpected string error'
          }
        })
      ),
      { rootId: 'root-3' },
      noLog
    )
    expect(unknownThrownResult.state).toBe('scanFailed')
    if (unknownThrownResult.state === 'scanFailed') {
      expect(unknownThrownResult.error.detail).toBe('unexpected string error')
    }
  })

  it('logs the full error when the scan client throws', async () => {
    const config = hostConfig()
    const logEntries: unknown[][] = []

    await runLocalRootScanThroughHost(
      await startedHostWithClient(
        config,
        createFakeClient({
          startRootScan: async () => {
            throw new Error('diagnostic fixture')
          }
        })
      ),
      { rootId: 'root-diag' },
      {
        error: (...args: unknown[]) => {
          logEntries.push(args)
        }
      }
    )

    expect(logEntries.length).toBe(1)
    expect(logEntries[0]![0]).toBe('[local-root-scan] failed')
    expect(logEntries[0]![1]).toMatchObject({ rootId: 'root-diag' })
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

  return resolveHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [boundaryStdioBinaryPathEnvVar]: fakeBinaryPath
    },
    platform: 'linux'
  })
}
