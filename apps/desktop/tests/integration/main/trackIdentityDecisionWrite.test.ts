import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  AcceptTrackIdentityCandidateRequest,
  DeferTrackIdentityCandidateRequest,
  RejectTrackIdentityCandidateRequest,
  TrackIdentityDecisionWriteResult as ContractTrackIdentityDecisionWriteResult
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  acceptTrackIdentityCandidateThroughHost,
  deferTrackIdentityCandidateThroughHost,
  registerTrackIdentityDecisionWriteIpc,
  rejectTrackIdentityCandidateThroughHost
} from '../../../src/main/libraryTrackIdentityDecisionWrite/decisionWrite'
import {
  trackIdentityDecisionWriteChannels,
  type TrackIdentityDecisionWriteResult
} from '../../../src/shared/libraryTrackIdentityDecisionWrite/decisionWrite'
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

describe('track identity decision writes through the host', () => {
  it('validates candidate scope and forwards no paths or metadata', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      acceptTrackIdentityCandidateThroughHost(idleHost, { candidateId: '7' }, silentDecisionLogger)
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    let receivedAcceptRequest: AcceptTrackIdentityCandidateRequest | undefined
    let receivedRejectRequest: RejectTrackIdentityCandidateRequest | undefined
    let receivedDeferRequest: DeferTrackIdentityCandidateRequest | undefined
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        acceptTrackIdentityCandidate: async (request) => {
          receivedAcceptRequest = request
          return writtenDecision('11', request.candidateId, 'accepted', 1, 'current')
        },
        rejectTrackIdentityCandidate: async (request) => {
          receivedRejectRequest = request
          return {
            type: 'failed',
            payload: { type: 'candidateNotFound' }
          }
        },
        deferTrackIdentityCandidate: async (request) => {
          receivedDeferRequest = request
          return writtenDecision('12', request.candidateId, 'deferred', 0, 'stale')
        }
      })
    )

    await expect(
      acceptTrackIdentityCandidateThroughHost(
        successHost,
        {
          candidateId: '7',
          reason: '  same identity  ',
          absolutePath: 'C:/RendererMustNotControlThis',
          sourcePath: 'C:/RendererMustNotControlThis/track.wav',
          title: 'Renderer title',
          artist: 'Renderer artist',
          album: 'Renderer album',
          trackId: '13'
        },
        silentDecisionLogger
      )
    ).resolves.toMatchObject({
      state: 'completed',
      result: {
        type: 'written',
        payload: {
          candidateId: '7',
          decisionState: 'accepted',
          decisionSource: 'user_local_v0',
          evidenceSnapshotCount: 1
        }
      }
    })
    expect(receivedAcceptRequest).toEqual({
      candidateId: '7',
      reason: 'same identity'
    })

    await expect(
      rejectTrackIdentityCandidateThroughHost(
        successHost,
        {
          candidateId: '999',
          sourcePath: 'C:/RendererMustNotControlThis/other.wav',
          title: 'Renderer title'
        },
        silentDecisionLogger
      )
    ).resolves.toMatchObject({
      state: 'completed',
      result: { type: 'failed', payload: { type: 'candidateNotFound' } }
    })
    expect(receivedRejectRequest).toEqual({ candidateId: '999' })

    await expect(
      deferTrackIdentityCandidateThroughHost(
        successHost,
        {
          candidateId: '8',
          reason: 'decide later',
          canonicalTrackId: 'not-allowed'
        },
        silentDecisionLogger
      )
    ).resolves.toMatchObject({
      state: 'completed',
      result: {
        type: 'written',
        payload: {
          candidateId: '8',
          decisionState: 'deferred',
          evidenceSnapshotCount: 0
        }
      }
    })
    expect(receivedDeferRequest).toEqual({
      candidateId: '8',
      reason: 'decide later'
    })

    await expect(
      acceptTrackIdentityCandidateThroughHost(
        successHost,
        { candidateId: '0' },
        silentDecisionLogger
      )
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      deferTrackIdentityCandidateThroughHost(
        successHost,
        { candidateId: '8', reason: 7 },
        silentDecisionLogger
      )
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('registers track identity decision write IPC channels', () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())
    const registration = new Map<
      string,
      (request: unknown) => Promise<TrackIdentityDecisionWriteResult>
    >()

    registerTrackIdentityDecisionWriteIpc(
      {
        handle(channel, listener): void {
          registration.set(channel, (request) => listener({}, request))
        }
      },
      idleHost,
      silentDecisionLogger
    )

    expect(registration.has(trackIdentityDecisionWriteChannels.acceptTrackIdentityCandidate)).toBe(
      true
    )
    expect(registration.has(trackIdentityDecisionWriteChannels.rejectTrackIdentityCandidate)).toBe(
      true
    )
    expect(registration.has(trackIdentityDecisionWriteChannels.deferTrackIdentityCandidate)).toBe(
      true
    )
  })
})

const silentDecisionLogger = {
  error: () => undefined
}

function writtenDecision(
  decisionId: string,
  candidateId: string,
  decisionState: 'accepted' | 'rejected' | 'deferred',
  evidenceSnapshotCount: number,
  effectiveDecisionCurrentStatus: 'current' | 'stale'
): ContractTrackIdentityDecisionWriteResult {
  return {
    type: 'written',
    payload: {
      decisionId,
      candidateId,
      decisionState,
      decisionSource: 'user_local_v0',
      evidenceSnapshotCount,
      decisionCreated: true,
      effectiveDecision: {
        effectiveDecisionId: decisionId,
        effectiveDecisionState: decisionState,
        effectiveDecisionSource: 'user_local_v0',
        effectiveDecisionCurrentStatus,
        effectiveDecisionPrecedence: 'user'
      }
    }
  }
}

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-track-identity-decision-write-'))
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
