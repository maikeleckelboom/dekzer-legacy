import { libraryControlChannels } from '../../../src/shared/library/boundary/controlPlane'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  ReadTrackIdentityReviewCandidatesReply as ContractReadTrackIdentityReviewCandidatesReply,
  ReadTrackIdentityReviewCandidatesRequest as ContractReadTrackIdentityReviewCandidatesRequest
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import {
  readCandidatesThroughHost,
  registerTrackIdentityReviewCandidatesIpc
} from '../../../src/main/library/trackIdentity/candidates'
import { type ReadCandidatesResult } from '../../../src/shared/library/trackIdentity/candidates'
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

describe('track identity review candidate reads through the host', () => {
  it('validates read scope and forwards no paths or metadata', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readCandidatesThroughHost(idleHost, { sourceId: '7', limit: 25 })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    let receivedRequest: ContractReadTrackIdentityReviewCandidatesRequest | undefined
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readTrackIdentityReviewCandidates: async (request) => {
          receivedRequest = request
          return reviewReply()
        }
      })
    )

    await expect(
      readCandidatesThroughHost(successHost, {
        sourceId: '7',
        reviewState: 'userRejected',
        limit: 25,
        sourcePath: 'C:/RendererMustNotControlThis/track.wav',
        filePath: 'C:/RendererMustNotControlThis/track.wav',
        title: 'Renderer title',
        artist: 'Renderer artist',
        album: 'Renderer album',
        metadata: { title: 'Renderer title' }
      })
    ).resolves.toMatchObject({
      state: 'ready',
      result: {
        status: 'ok',
        candidates: [{ candidateId: '11', reviewState: 'userRejected' }]
      }
    })
    expect(receivedRequest).toEqual({
      sourceId: '7',
      reviewState: 'userRejected',
      limit: 25
    })

    await expect(
      readCandidatesThroughHost(successHost, {
        sourceId: '0',
        limit: 25
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readCandidatesThroughHost(successHost, {
        sourceId: '7',
        reviewState: 'blockedByUserDecision',
        limit: 25
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readCandidatesThroughHost(successHost, {
        sourceId: '7',
        reviewState: 'all',
        limit: 25
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readCandidatesThroughHost(successHost, {
        sourceId: '7',
        limit: 0
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('registers track identity review candidate read IPC channel', () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())
    const registration = new Map<string, (request: unknown) => Promise<ReadCandidatesResult>>()

    registerTrackIdentityReviewCandidatesIpc(
      {
        handle(channel, listener): void {
          registration.set(channel, (request) => listener({}, request))
        }
      },
      idleHost
    )

    expect(registration.has(libraryControlChannels.trackIdentity.candidates.read)).toBe(true)
  })
})

function reviewReply(): ContractReadTrackIdentityReviewCandidatesReply {
  return {
    status: 'ok',
    candidates: [
      {
        candidateId: '11',
        candidateKind: 'exact_primary_media_content',
        candidateEvidenceBasis: 'current_primary_media_exact_blake3',
        candidateStatus: 'active',
        evidenceKeyAlgorithm: 'blake3',
        evidenceKeyValue: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        evidenceSummary: {
          memberCount: 1,
          evidenceCount: 1,
          currentEvidenceCount: 1
        },
        sourceSummary: {
          sourceCount: 1,
          sourceSamples: [{ sourceId: '7', displayName: 'Local' }]
        },
        reviewState: 'userRejected',
        effectiveDecision: {
          decisionId: '12',
          decisionState: 'rejected',
          decisionSource: 'user_local_v0',
          decisionBasis: 'explicit_user_local_decision_v0',
          currentStatus: 'current',
          createdAtMs: 100,
          userBlockingDecisionState: 'rejected',
          maskedSystemDecisionId: '10'
        },
        createdAtMs: 80,
        updatedAtMs: 90
      }
    ]
  }
}

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-track-identity-review-candidates-'))
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
