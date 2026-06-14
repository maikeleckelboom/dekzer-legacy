import { describe, expect, it, vi } from 'vitest'
import type {
  ReadSourceMaintenanceReply,
  RunSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

import {
  createController,
  type ReadApi
} from '../../../../src/renderer/library/sourceMaintenance/read'
import type { ReadSourceMaintenanceResult } from '../../../../src/shared/library/source/maintenance'

describe('source maintenance read controller', () => {
  it('does not let a stale in-flight read restore backlog after maintenance completes', async () => {
    const pendingRead = deferred<ReadSourceMaintenanceResult>()
    const api = maintenanceApi({
      readSourceMaintenance: vi.fn(() => pendingRead.promise),
      runSourceMaintenance: vi.fn(async () => ({
        state: 'completed',
        result: runResult({
          remainingHashCandidates: 0
        })
      }))
    })
    const controller = createController(api)

    controller.start()
    const staleRead = controller.read('7')
    await expect(controller.run('7')).resolves.toBe(true)

    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      remainingHashCandidates: 0
    })

    pendingRead.resolve({
      state: 'ready',
      snapshot: maintenanceSnapshot({
        remainingHashCandidates: 4
      })
    })

    await expect(staleRead).resolves.toBe(false)
    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      remainingHashCandidates: 0
    })
  })

  it('force reads bypass older in-flight reads and keep the freshest snapshot', async () => {
    const staleRead = deferred<ReadSourceMaintenanceResult>()
    const freshRead = deferred<ReadSourceMaintenanceResult>()
    const api = maintenanceApi({
      readSourceMaintenance: vi
        .fn()
        .mockReturnValueOnce(staleRead.promise)
        .mockReturnValueOnce(freshRead.promise),
      runSourceMaintenance: vi.fn()
    })
    const controller = createController(api)

    controller.start()
    const stale = controller.read('7')
    const fresh = controller.read('7', { force: true })

    freshRead.resolve({
      state: 'ready',
      snapshot: maintenanceSnapshot({
        remainingHashCandidates: 0
      })
    })
    await expect(fresh).resolves.toBe(true)

    staleRead.resolve({
      state: 'ready',
      snapshot: maintenanceSnapshot({
        remainingHashCandidates: 4
      })
    })
    await expect(stale).resolves.toBe(false)

    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      remainingHashCandidates: 0
    })
  })

  it('invalidates one source maintenance snapshot and rejects late reads', async () => {
    const staleRead = deferred<ReadSourceMaintenanceResult>()
    const api = maintenanceApi({
      readSourceMaintenance: vi
        .fn()
        .mockResolvedValueOnce({
          state: 'ready',
          snapshot: maintenanceSnapshot({ sourceId: '7', remainingHashCandidates: 2 })
        })
        .mockResolvedValueOnce({
          state: 'ready',
          snapshot: maintenanceSnapshot({ sourceId: '8', remainingHashCandidates: 4 })
        })
        .mockResolvedValueOnce({
          state: 'readFailed',
          error: { code: 'readFailed', message: 'Unable to read source maintenance.' }
        })
        .mockReturnValueOnce(staleRead.promise)
        .mockResolvedValue({
          state: 'ready',
          snapshot: maintenanceSnapshot({ sourceId: '7', remainingHashCandidates: 0 })
        }),
      runSourceMaintenance: vi.fn(async () => ({
        state: 'maintenanceFailed',
        error: { code: 'maintenanceFailed', message: 'Unable to run source maintenance.' }
      }))
    })
    const controller = createController(api)

    controller.start()
    await expect(controller.read('7')).resolves.toBe(true)
    await expect(controller.read('8')).resolves.toBe(true)
    await expect(controller.read('7')).resolves.toBe(false)
    await expect(controller.run('7')).resolves.toBe(false)

    const stale = controller.read('7')
    controller.invalidateSource('7')

    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()
    expect(controller.readErrorsBySourceId.value.get('7')).toBeUndefined()
    expect(controller.runStateBySourceId.value.get('7')).toBeUndefined()
    expect(controller.runErrorsBySourceId.value.get('7')).toBeUndefined()
    expect(controller.snapshotBySourceId.value.get('8')).toMatchObject({
      sourceId: '8',
      remainingHashCandidates: 4
    })

    staleRead.resolve({
      state: 'ready',
      snapshot: maintenanceSnapshot({ sourceId: '7', remainingHashCandidates: 1 })
    })

    await expect(stale).resolves.toBe(false)
    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()

    await expect(controller.refresh(['7'])).resolves.toBe(true)
    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      remainingHashCandidates: 0
    })
  })

  it('rejects a late in-flight maintenance run after invalidation', async () => {
    const pendingRun = deferred<Awaited<ReturnType<ReadApi['sourceMaintenance']['runSourceMaintenance']>>>()
    const api = maintenanceApi({
      readSourceMaintenance: vi.fn(),
      runSourceMaintenance: vi.fn().mockReturnValueOnce(pendingRun.promise)
    })
    const controller = createController(api)

    controller.start()
    const run = controller.run('7')
    expect(controller.runStateBySourceId.value.get('7')).toBe('running')

    controller.invalidateSource('7')
    expect(controller.runStateBySourceId.value.get('7')).toBeUndefined()

    pendingRun.resolve({
      state: 'completed',
      result: runResult({ sourceId: '7', remainingHashCandidates: 3 })
    })

    await expect(run).resolves.toBe(false)
    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()
    expect(controller.runStateBySourceId.value.get('7')).toBeUndefined()
  })
})

function maintenanceApi(overrides: ReadApi['sourceMaintenance']): ReadApi {
  return {
    sourceMaintenance: overrides
  }
}

function maintenanceSnapshot(
  overrides: Partial<ReadSourceMaintenanceReply> = {}
): ReadSourceMaintenanceReply {
  return {
    sourceId: '7',
    status: 'idle',
    remainingHashCandidates: 0,
    remainingProbeCandidates: 0,
    remainingPlayableMediaPromotionCandidates: 0,
    remainingTrackIdentityCandidateProductionCandidates: 0,
    remainingTrackIdentityDecisionProductionCandidates: 0,
    ...overrides
  }
}

function runResult(overrides: Partial<RunSourceMaintenanceReply> = {}): RunSourceMaintenanceReply {
  return {
    sourceId: '7',
    status: 'completed',
    effectiveLimits: {
      hashLimit: 0,
      attachmentLimit: 0,
      probeLimit: 0,
      promotionLimit: 0,
      identityCandidateLimit: 0,
      identityDecisionLimit: 0
    },
    hash: {
      effectiveLimit: 0,
      hashedCount: 0,
      skippedCount: 0,
      failedCount: 0,
      remainingCandidates: 0
    },
    attachmentMaterialization: {
      effectiveLimit: 0,
      attachmentsCreated: 0,
      attachmentsRefreshed: 0,
      linksCreated: 0,
      linksReplaced: 0,
      linksRefreshed: 0,
      skippedStaleObservations: 0,
      skippedNoBlake3: 0,
      skippedNoObservations: 0,
      remainingCandidates: 0
    },
    probe: {
      effectiveLimit: 0,
      probedCount: 0,
      skippedCount: 0,
      failedCount: 0,
      remainingCandidates: 0
    },
    playableMediaPromotion: {
      effectiveLimit: 0,
      promotedCount: 0,
      refreshedCount: 0,
      skippedUnusableSource: 0,
      skippedUnsupportedMediaKind: 0,
      skippedNoObservations: 0,
      skippedStaleObservations: 0,
      skippedNoBlake3: 0,
      skippedNoProbeObservations: 0,
      skippedMissingAttachmentLink: 0,
      skippedStaleAttachmentLink: 0,
      remainingCandidates: 0
    },
    trackIdentityCandidates: {
      effectiveLimit: 0,
      candidatesCreated: 0,
      candidatesRefreshed: 0,
      membersCreated: 0,
      membersRefreshed: 0,
      evidenceCreated: 0,
      evidenceRefreshed: 0,
      candidatesMarkedStale: 0,
      skippedStalePlayableMedia: 0,
      remainingCandidates: 0
    },
    trackIdentityDecisions: {
      effectiveLimit: 0,
      decisionsCreated: 0,
      decisionEvidenceCreated: 0,
      skippedStaleCandidates: 0,
      skippedExistingCurrentDecisions: 0,
      skippedUserBlockedCandidates: 0,
      remainingCandidates: 0
    },
    remainingHashCandidates: 0,
    remainingProbeCandidates: 0,
    remainingPlayableMediaPromotionCandidates: 0,
    remainingTrackIdentityCandidateProductionCandidates: 0,
    remainingTrackIdentityDecisionProductionCandidates: 0,
    ...overrides
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
