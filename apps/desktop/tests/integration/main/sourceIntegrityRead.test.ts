import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import { readSourceIntegrityThroughHost } from '../../../src/main/library/source/integrity'
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

describe('source integrity reads through the host', () => {
  it('validates source scope and forwards the backend-owned integrity read', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readSourceIntegrityThroughHost(idleHost, { sourceId: '7' })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readSourceIntegrity: async (request) => {
          expect(request).toEqual({ sourceId: '7' })
          return {
            sourceId: '7',
            sourceAvailability: {
              state: 'mounted'
            },
            coverageIntegrity: {
              state: 'complete',
              subtreeCoverageComplete: true,
              emptyResultAuthoritative: true,
              totalDirectoriesCount: 1,
              missingDirectoriesCount: 0,
              pendingDirectoriesCount: 0,
              scanningDirectoriesCount: 0,
              blockedDirectoriesCount: 0,
              failedDirectoriesCount: 0
            },
            evidenceAndMaintenance: {
              remainingHashCandidates: 0,
              remainingProbeCandidates: 0,
              remainingPlayableMediaPromotionCandidates: 0,
              remainingTrackIdentityCandidateProductionCandidates: 0,
              remainingTrackIdentityDecisionProductionCandidates: 0
            },
            runtimeMaintenance: {
              state: 'idle'
            }
          }
        }
      })
    )

    await expect(readSourceIntegrityThroughHost(successHost, { sourceId: '7' })).resolves.toEqual({
      state: 'ready',
      integrity: {
        sourceId: '7',
        sourceAvailability: {
          state: 'mounted'
        },
        coverageIntegrity: {
          state: 'complete',
          subtreeCoverageComplete: true,
          emptyResultAuthoritative: true,
          totalDirectoriesCount: 1,
          missingDirectoriesCount: 0,
          pendingDirectoriesCount: 0,
          scanningDirectoriesCount: 0,
          blockedDirectoriesCount: 0,
          failedDirectoriesCount: 0
        },
        evidenceAndMaintenance: {
          remainingHashCandidates: 0,
          remainingProbeCandidates: 0,
          remainingPlayableMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0
        },
        runtimeMaintenance: {
          state: 'idle'
        }
      }
    })

    await expect(
      readSourceIntegrityThroughHost(successHost, { sourceId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-source-integrity-read-'))
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
