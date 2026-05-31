import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  readSourceLifecycleThroughHost,
  registerReadSourceLifecycleIpc
} from '../../../src/main/librarySourceLifecycle/readSourceLifecycle'
import { sourceLifecycleReadChannels } from '../../../src/shared/librarySourceLifecycle/channels'
import type { ReadSourceLifecycleResult } from '../../../src/shared/librarySourceLifecycle/readSourceLifecycle'
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

describe('source lifecycle reads through the host', () => {
  it('maps host, invalid request, not found, and semantic lifecycle outcomes', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readSourceLifecycleThroughHost(idleHost, { sourceId: '7' })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readSourceLifecycle: async (request) => {
          expect(request).toEqual({ sourceId: '7' })
          return {
            lifecycle: {
              sourceId: '7',
              sourceClass: 'externalMounted',
              isUserVisible: true,
              mountStatus: 'unmounted',
              accessState: 'blocked',
              accessIssueKind: 'unavailableMount',
              scanPhase: 'blocked',
              scanIssueKind: 'permissionDenied',
              lastScanStartedAtMs: 10,
              lastScanFinishedAtMs: 20,
              lastSeenAtMs: 9,
              updatedAtMs: 21
            }
          }
        }
      })
    )

    await expect(readSourceLifecycleThroughHost(successHost, { sourceId: '7' })).resolves.toEqual({
      state: 'ready',
      lifecycle: {
        sourceId: '7',
        sourceClass: 'externalMounted',
        isUserVisible: true,
        mountStatus: 'unmounted',
        accessState: 'blocked',
        accessIssueKind: 'unavailableMount',
        scanPhase: 'blocked',
        scanIssueKind: 'permissionDenied',
        lastScanStartedAtMs: 10,
        lastScanFinishedAtMs: 20,
        lastSeenAtMs: 9,
        updatedAtMs: 21
      }
    })

    await expect(
      readSourceLifecycleThroughHost(successHost, { sourceId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    const missingHost = await startedHostWithClient(
      config,
      createFakeClient({
        readSourceLifecycle: async () => ({ lifecycle: null })
      })
    )

    await expect(
      readSourceLifecycleThroughHost(missingHost, { sourceId: '999' })
    ).resolves.toMatchObject({
      state: 'notFound',
      error: { code: 'notFound' }
    })

    const registration: {
      channel?: string
      handler?: (request: unknown) => Promise<ReadSourceLifecycleResult>
    } = {}
    registerReadSourceLifecycleIpc(
      {
        handle(channel, listener): void {
          registration.channel = channel
          registration.handler = (request) => listener({}, request)
        }
      },
      idleHost
    )
    expect(registration.channel).toBe(sourceLifecycleReadChannels.readSourceLifecycle)
    expect(typeof registration.handler).toBe('function')
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-source-lifecycle-read-'))
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
