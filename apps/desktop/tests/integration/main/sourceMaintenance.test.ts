import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  ReadSourceMaintenanceRequest,
  RunSourceMaintenanceRequest
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  readSourceMaintenanceThroughHost,
  registerSourceMaintenanceIpc,
  runSourceMaintenanceThroughHost
} from '../../../src/main/librarySourceMaintenance/sourceMaintenance'
import {
  sourceMaintenanceChannels,
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceResult
} from '../../../src/shared/librarySourceMaintenance/sourceMaintenance'
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

describe('source maintenance through the host', () => {
  it('validates source scope, forwards only ids and limits, and returns typed summaries', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      runSourceMaintenanceThroughHost(idleHost, { sourceId: '7', hashLimit: 4 })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    let receivedRunRequest: RunSourceMaintenanceRequest | undefined
    let receivedReadRequest: ReadSourceMaintenanceRequest | undefined
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        runSourceMaintenance: async (request) => {
          receivedRunRequest = request
          return {
            sourceId: '7',
            status: 'completed',
            effectiveLimits: {
              hashLimit: 4,
              attachmentLimit: 3,
              probeLimit: 2
            },
            hash: {
              effectiveLimit: 4,
              hashedCount: 1,
              skippedCount: 0,
              failedCount: 0,
              remainingCandidates: 0
            },
            attachmentMaterialization: {
              effectiveLimit: 3,
              attachmentsCreated: 1,
              attachmentsRefreshed: 0,
              linksCreated: 1,
              linksReplaced: 0,
              linksRefreshed: 0,
              skippedStaleFacts: 0,
              skippedNoBlake3: 0,
              skippedNoFacts: 0,
              remainingCandidates: 0
            },
            probe: {
              effectiveLimit: 2,
              probedCount: 1,
              skippedCount: 0,
              failedCount: 0,
              remainingCandidates: 0
            },
            remainingHashCandidates: 0,
            remainingProbeCandidates: 0
          }
        },
        readSourceMaintenance: async (request) => {
          receivedReadRequest = request
          return {
            sourceId: '7',
            status: 'idle',
            remainingHashCandidates: 0,
            remainingProbeCandidates: 0,
            attachmentLinks: {
              currentLinksCount: 1,
              staleLinksCount: 0,
              sourceFilesWithCurrentBlake3FactsCount: 1,
              sourceFilesWithAttachmentLinksCount: 1,
              sourceFilesMissingAttachmentLinksCount: 0,
              unmaterializedBlake3FactsCount: 0
            }
          }
        }
      })
    )

    await expect(
      runSourceMaintenanceThroughHost(successHost, {
        sourceId: '7',
        hashLimit: 4,
        attachmentLimit: 3,
        probeLimit: 2,
        absolutePath: 'C:/RendererMustNotControlThis'
      })
    ).resolves.toMatchObject({
      state: 'completed',
      result: {
        sourceId: '7',
        hash: { hashedCount: 1 },
        attachmentMaterialization: { linksCreated: 1 },
        probe: { probedCount: 1 }
      }
    })
    expect(receivedRunRequest).toEqual({
      sourceId: '7',
      hashLimit: 4,
      attachmentLimit: 3,
      probeLimit: 2
    })

    await expect(readSourceMaintenanceThroughHost(successHost, { sourceId: '7' })).resolves.toEqual(
      {
        state: 'ready',
        snapshot: {
          sourceId: '7',
          status: 'idle',
          remainingHashCandidates: 0,
          remainingProbeCandidates: 0,
          attachmentLinks: {
            currentLinksCount: 1,
            staleLinksCount: 0,
            sourceFilesWithCurrentBlake3FactsCount: 1,
            sourceFilesWithAttachmentLinksCount: 1,
            sourceFilesMissingAttachmentLinksCount: 0,
            unmaterializedBlake3FactsCount: 0
          }
        }
      }
    )
    expect(receivedReadRequest).toEqual({ sourceId: '7' })

    await expect(
      runSourceMaintenanceThroughHost(successHost, { sourceId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      runSourceMaintenanceThroughHost(successHost, { sourceId: '7', probeLimit: 0 })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readSourceMaintenanceThroughHost(successHost, { sourceId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('registers source maintenance IPC channels', () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())
    const registration = new Map<
      string,
      (request: unknown) => Promise<RunSourceMaintenanceResult | ReadSourceMaintenanceResult>
    >()

    registerSourceMaintenanceIpc(
      {
        handle(channel, listener): void {
          registration.set(channel, (request) => listener({}, request))
        }
      },
      idleHost
    )

    expect(registration.has(sourceMaintenanceChannels.runSourceMaintenance)).toBe(true)
    expect(registration.has(sourceMaintenanceChannels.readSourceMaintenance)).toBe(true)
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-source-maintenance-'))
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
