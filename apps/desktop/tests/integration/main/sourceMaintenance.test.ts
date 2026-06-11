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
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import {
  readSourceMaintenanceThroughHost,
  runSourceMaintenanceThroughHost
} from '../../../src/main/library/source/maintenance'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/library/boundary'

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
              probeLimit: 2,
              promotionLimit: 5,
              identityCandidateLimit: 6,
              identityDecisionLimit: 7
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
            primaryMediaPromotion: {
              effectiveLimit: 5,
              promotedCount: 1,
              refreshedCount: 0,
              skippedUnusableSource: 0,
              skippedUnsupportedMediaKind: 0,
              skippedNoFacts: 0,
              skippedStaleFacts: 0,
              skippedNoBlake3: 0,
              skippedNoProbeFacts: 0,
              skippedMissingAttachmentLink: 0,
              skippedStaleAttachmentLink: 0,
              remainingCandidates: 0
            },
            trackIdentityCandidates: {
              effectiveLimit: 6,
              candidatesCreated: 1,
              candidatesRefreshed: 0,
              membersCreated: 1,
              membersRefreshed: 0,
              evidenceCreated: 1,
              evidenceRefreshed: 0,
              candidatesMarkedStale: 0,
              skippedStalePrimaryMediaCandidates: 0,
              remainingCandidates: 0
            },
            trackIdentityDecisions: {
              effectiveLimit: 7,
              decisionsCreated: 1,
              decisionEvidenceCreated: 1,
              skippedStaleCandidates: 0,
              skippedExistingCurrentDecisions: 0,
              skippedUserBlockedCandidates: 0,
              remainingCandidates: 0
            },
            remainingHashCandidates: 0,
            remainingProbeCandidates: 0,
            remainingPrimaryMediaPromotionCandidates: 0,
            remainingTrackIdentityCandidateProductionCandidates: 0,
            remainingTrackIdentityDecisionProductionCandidates: 0
          }
        },
        readSourceMaintenance: async (request) => {
          receivedReadRequest = request
          return {
            sourceId: '7',
            status: 'idle',
            remainingHashCandidates: 0,
            remainingProbeCandidates: 0,
            remainingPrimaryMediaPromotionCandidates: 0,
            remainingTrackIdentityCandidateProductionCandidates: 0,
            remainingTrackIdentityDecisionProductionCandidates: 0,
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
        promotionLimit: 5,
        identityCandidateLimit: 6,
        identityDecisionLimit: 7,
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
      probeLimit: 2,
      promotionLimit: 5,
      identityCandidateLimit: 6,
      identityDecisionLimit: 7
    })

    await expect(readSourceMaintenanceThroughHost(successHost, { sourceId: '7' })).resolves.toEqual(
      {
        state: 'ready',
        snapshot: {
          sourceId: '7',
          status: 'idle',
          remainingHashCandidates: 0,
          remainingProbeCandidates: 0,
          remainingPrimaryMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0,
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
      runSourceMaintenanceThroughHost(successHost, { sourceId: '7', promotionLimit: 0 })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      runSourceMaintenanceThroughHost(successHost, { sourceId: '7', identityCandidateLimit: 0 })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      runSourceMaintenanceThroughHost(successHost, { sourceId: '7', identityDecisionLimit: 0 })
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
