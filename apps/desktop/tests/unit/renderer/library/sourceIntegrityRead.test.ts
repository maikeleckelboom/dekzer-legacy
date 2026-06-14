import { describe, expect, it, vi } from 'vitest'
import type { ReadSourceIntegrityReply } from '@dekzer/library-boundary-contract'

import { createController, type ReadApi } from '../../../../src/renderer/library/sourceIntegrity/read'
import type { SourceIntegrityReadResult } from '../../../../src/shared/library/source/integrity'

describe('source integrity read controller', () => {
  it('invalidates one source integrity snapshot and rejects late reads', async () => {
    const staleRead = deferred<SourceIntegrityReadResult>()
    const api = integrityApi(
      vi
        .fn()
        .mockResolvedValueOnce({
          state: 'ready',
          integrity: sourceIntegrity({ sourceId: '7', availability: 'blocked' })
        })
        .mockResolvedValueOnce({
          state: 'ready',
          integrity: sourceIntegrity({ sourceId: '8', availability: 'mounted' })
        })
        .mockResolvedValueOnce({
          state: 'readFailed',
          error: { code: 'readFailed', message: 'Unable to read source integrity.' }
        })
        .mockReturnValueOnce(staleRead.promise)
        .mockResolvedValue({
          state: 'ready',
          integrity: sourceIntegrity({ sourceId: '7', availability: 'mounted' })
        })
    )
    const controller = createController(api)

    await expect(controller.read('7')).resolves.toBe(true)
    await expect(controller.read('8')).resolves.toBe(true)
    await expect(controller.read('7')).resolves.toBe(false)

    const stale = controller.read('7')
    controller.invalidateSource('7')

    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()
    expect(controller.readErrorsBySourceId.value.get('7')).toBeUndefined()
    expect(controller.snapshotBySourceId.value.get('8')).toMatchObject({
      sourceId: '8',
      sourceAvailability: { state: 'mounted' }
    })

    staleRead.resolve({
      state: 'ready',
      integrity: sourceIntegrity({ sourceId: '7', availability: 'mounted' })
    })

    await expect(stale).resolves.toBe(false)
    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()

    await expect(controller.refresh(['7'])).resolves.toBe(true)
    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      sourceAvailability: { state: 'mounted' }
    })
  })
})

function integrityApi(readSourceIntegrity: ReadApi['sourceIntegrity']['readSourceIntegrity']): ReadApi {
  return {
    sourceIntegrity: {
      readSourceIntegrity
    }
  }
}

function sourceIntegrity(
  options: {
    readonly sourceId?: string
    readonly availability?: ReadSourceIntegrityReply['sourceAvailability']['state']
  } = {}
): ReadSourceIntegrityReply {
  const sourceId = options.sourceId ?? '7'
  const availability = options.availability ?? 'mounted'

  return {
    sourceId,
    sourceAvailability: {
      state: availability
    },
    coverageIntegrity: {
      state: 'complete',
      subtreeCoverageComplete: true,
      emptyResultAuthoritative: true,
      totalDirectoriesCount: 1,
      missingDirectoriesCount: availability === 'missing' ? 1 : 0,
      pendingDirectoriesCount: 0,
      scanningDirectoriesCount: 0,
      blockedDirectoriesCount: availability === 'blocked' ? 1 : 0,
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

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolveDeferred: (value: T) => void = () => undefined
  const promise = new Promise<T>((resolve) => {
    resolveDeferred = resolve
  })

  return {
    promise,
    resolve: resolveDeferred
  }
}
