import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { LibraryBoundaryProtocolError } from '@dekzer/library-boundary-client'

import { afterEach, describe, expect, it } from 'vitest'

import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import { type ScanLogger, runLocalRootScanThroughHost } from '../../../src/main/library/roots/scan'
import { cancelRootScanThroughHost } from '../../../src/main/library/roots/cancel'

import type { CancelRootScanResult } from '../../../src/shared/library/roots/cancel'
import type { LocalRootScanResult } from '../../../src/shared/library/roots/scan'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/library/boundary'

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
})

describe('local root scan cancellation boundary', () => {
  it('returns notFound for unknown scanRunId', async () => {
    const config = hostConfig()

    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            cancelRootScan: async () => ({
              status: 'notFound'
            })
          })
        ),
        { scanRunId: 'unknown-999' },
        noLog
      )
    ).resolves.toMatchObject({
      state: 'notFound',
      status: 'notFound'
    } satisfies Partial<CancelRootScanResult>)
  })

  it('returns invalidRequest for invalid scanRunId', async () => {
    const config = hostConfig()

    await expect(
      cancelRootScanThroughHost(await startedHostWithClient(config, createFakeClient()), {}, noLog)
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: {
        code: 'invalidRequest',
        message: 'Cancel root scan scanRunId is invalid.'
      }
    } satisfies Partial<CancelRootScanResult>)

    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(config, createFakeClient()),
        { scanRunId: '   ' },
        noLog
      )
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: {
        code: 'invalidRequest',
        message: 'Cancel root scan scanRunId is invalid.'
      }
    } satisfies Partial<CancelRootScanResult>)

    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(config, createFakeClient()),
        null,
        noLog
      )
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: {
        code: 'invalidRequest',
        message: 'Cancel root scan requires a request object.'
      }
    } satisfies Partial<CancelRootScanResult>)
  })

  it('returns alreadyTerminal for already-finished scan run', async () => {
    const config = hostConfig()

    let receivedScanRunId = ''
    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            cancelRootScan: async (request) => {
              receivedScanRunId = request.scanRunId
              return { status: 'alreadyTerminal' }
            }
          })
        ),
        { scanRunId: 'scan-1' },
        noLog
      )
    ).resolves.toEqual({
      state: 'alreadyTerminal',
      status: 'alreadyTerminal'
    } satisfies CancelRootScanResult)
    expect(receivedScanRunId).toBe('scan-1')
  })

  it('returns accepted for active scan cancellation', async () => {
    const config = hostConfig()

    let receivedScanRunId = ''
    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            cancelRootScan: async (request) => {
              receivedScanRunId = request.scanRunId
              return { status: 'accepted' }
            }
          })
        ),
        { scanRunId: 'scan-active' },
        noLog
      )
    ).resolves.toEqual({
      state: 'accepted',
      status: 'accepted'
    } satisfies CancelRootScanResult)
    expect(receivedScanRunId).toBe('scan-active')
  })

  it('returns notCancelable when the service signals notCancelable', async () => {
    const config = hostConfig()

    await expect(
      cancelRootScanThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            cancelRootScan: async () => ({
              status: 'notCancelable'
            })
          })
        ),
        { scanRunId: 'scan-nc-1' },
        noLog
      )
    ).resolves.toEqual({
      state: 'notCancelable',
      status: 'notCancelable'
    } satisfies CancelRootScanResult)
  })

  it('handles protocol errors as cancelFailed', async () => {
    const config = hostConfig()

    const result = await cancelRootScanThroughHost(
      await startedHostWithClient(
        config,
        createFakeClient({
          cancelRootScan: async () => {
            throw new LibraryBoundaryProtocolError({
              type: 'durableStoreFailure',
              payload: { detail: 'database is locked' }
            })
          }
        })
      ),
      { scanRunId: 'scan-err' },
      noLog
    )

    expect(result).toMatchObject({
      state: 'cancelFailed',
      error: { code: 'cancelFailed' }
    })
  })

  it('returns hostUnavailable for unstarted host', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      cancelRootScanThroughHost(idleHost, { scanRunId: 'scan-1' }, noLog)
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })
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
