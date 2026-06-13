import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'
import type {
  ReadSourceMaintenanceReply,
  RunSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

import type {
  ReadSourceMaintenanceResult,
  RunSourceMaintenanceResult,
  SourceMaintenanceError
} from '../../../shared/library/source/maintenance'
import type { RendererApi } from '../../../shared/rendererApi'

export type SourceMaintenanceReadApi = Pick<RendererApi['library'], 'sourceMaintenance'>

export type SourceMaintenanceRunState = 'idle' | 'running' | 'completed' | 'failed'

export type SourceMaintenanceReadController = {
  readonly sourceMaintenanceBySourceId: Ref<ReadonlyMap<string, ReadSourceMaintenanceReply>>
  readonly sourceMaintenanceReadErrorsBySourceId: Ref<ReadonlyMap<string, SourceMaintenanceError>>
  readonly sourceMaintenanceRunStateBySourceId: Ref<ReadonlyMap<string, SourceMaintenanceRunState>>
  readonly sourceMaintenanceRunErrorsBySourceId: Ref<ReadonlyMap<string, SourceMaintenanceError>>
  readonly readSourceMaintenance: (sourceId: string) => Promise<boolean>
  readonly refreshSourceMaintenances: (sourceIds: Iterable<string>) => Promise<boolean>
  readonly runSourceMaintenance: (sourceId: string) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useSourceMaintenanceRead(
  libraryApi: SourceMaintenanceReadApi = getRendererApi().library
): SourceMaintenanceReadController {
  const controller = createSourceMaintenanceReadController(libraryApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createSourceMaintenanceReadController(
  libraryApi: SourceMaintenanceReadApi
): SourceMaintenanceReadController {
  const sourceMaintenanceBySourceId = shallowRef<ReadonlyMap<string, ReadSourceMaintenanceReply>>(
    new Map()
  )
  const sourceMaintenanceReadErrorsBySourceId = shallowRef<
    ReadonlyMap<string, SourceMaintenanceError>
  >(new Map())
  const sourceMaintenanceRunStateBySourceId = shallowRef<
    ReadonlyMap<string, SourceMaintenanceRunState>
  >(new Map())
  const sourceMaintenanceRunErrorsBySourceId = shallowRef<
    ReadonlyMap<string, SourceMaintenanceError>
  >(new Map())
  const inFlightReadsBySourceId = new Map<string, Promise<boolean>>()
  const inFlightRunsBySourceId = new Map<string, Promise<boolean>>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
  }

  function readSourceMaintenance(sourceId: string): Promise<boolean> {
    const current = inFlightReadsBySourceId.get(sourceId)
    if (current !== undefined) {
      return current
    }

    const read = requestSourceMaintenance(sourceId).finally(() => {
      inFlightReadsBySourceId.delete(sourceId)
    })
    inFlightReadsBySourceId.set(sourceId, read)
    return read
  }

  async function requestSourceMaintenance(sourceId: string): Promise<boolean> {
    let result: ReadSourceMaintenanceResult

    try {
      result = await libraryApi.sourceMaintenance.readSourceMaintenance({ sourceId })
    } catch {
      result = {
        state: 'readFailed',
        error: {
          code: 'readFailed',
          message: 'Unable to read source maintenance.'
        }
      }
    }

    if (stopped) {
      return false
    }

    if (result.state === 'ready') {
      setSnapshot(sourceId, result.snapshot)
      clearReadError(sourceId)
      return true
    }

    setReadError(sourceId, result.error)
    return false
  }

  async function refreshSourceMaintenances(sourceIds: Iterable<string>): Promise<boolean> {
    const uniqueSourceIds = [...new Set(sourceIds)].filter((sourceId) => sourceId.length > 0)
    if (uniqueSourceIds.length === 0) {
      return true
    }

    const results = await Promise.all(
      uniqueSourceIds.map((sourceId) => readSourceMaintenance(sourceId))
    )
    return results.every(Boolean)
  }

  function runSourceMaintenance(sourceId: string): Promise<boolean> {
    const current = inFlightRunsBySourceId.get(sourceId)
    if (current !== undefined) {
      return current
    }

    const run = requestRunSourceMaintenance(sourceId).finally(() => {
      inFlightRunsBySourceId.delete(sourceId)
    })
    inFlightRunsBySourceId.set(sourceId, run)
    return run
  }

  async function requestRunSourceMaintenance(sourceId: string): Promise<boolean> {
    setRunState(sourceId, 'running')
    clearRunError(sourceId)
    let result: RunSourceMaintenanceResult

    try {
      result = await libraryApi.sourceMaintenance.runSourceMaintenance({ sourceId })
    } catch {
      result = {
        state: 'maintenanceFailed',
        error: {
          code: 'maintenanceFailed',
          message: 'Unable to run source maintenance.'
        }
      }
    }

    if (stopped) {
      return false
    }

    if (result.state === 'completed') {
      setRunState(sourceId, 'completed')
      setSnapshot(sourceId, snapshotFromRun(result.result))
      return true
    }

    setRunState(sourceId, 'failed')
    setRunError(sourceId, result.error)
    return false
  }

  function setSnapshot(sourceId: string, snapshot: ReadSourceMaintenanceReply): void {
    const next = new Map(sourceMaintenanceBySourceId.value)
    next.set(sourceId, snapshot)
    sourceMaintenanceBySourceId.value = next
  }

  function setReadError(sourceId: string, error: SourceMaintenanceError): void {
    const next = new Map(sourceMaintenanceReadErrorsBySourceId.value)
    next.set(sourceId, error)
    sourceMaintenanceReadErrorsBySourceId.value = next
  }

  function clearReadError(sourceId: string): void {
    if (!sourceMaintenanceReadErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(sourceMaintenanceReadErrorsBySourceId.value)
    next.delete(sourceId)
    sourceMaintenanceReadErrorsBySourceId.value = next
  }

  function setRunState(sourceId: string, state: SourceMaintenanceRunState): void {
    const next = new Map(sourceMaintenanceRunStateBySourceId.value)
    next.set(sourceId, state)
    sourceMaintenanceRunStateBySourceId.value = next
  }

  function setRunError(sourceId: string, error: SourceMaintenanceError): void {
    const next = new Map(sourceMaintenanceRunErrorsBySourceId.value)
    next.set(sourceId, error)
    sourceMaintenanceRunErrorsBySourceId.value = next
  }

  function clearRunError(sourceId: string): void {
    if (!sourceMaintenanceRunErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(sourceMaintenanceRunErrorsBySourceId.value)
    next.delete(sourceId)
    sourceMaintenanceRunErrorsBySourceId.value = next
  }

  return {
    sourceMaintenanceBySourceId,
    sourceMaintenanceReadErrorsBySourceId,
    sourceMaintenanceRunStateBySourceId,
    sourceMaintenanceRunErrorsBySourceId,
    readSourceMaintenance,
    refreshSourceMaintenances,
    runSourceMaintenance,
    start,
    stop
  }
}

function snapshotFromRun(run: RunSourceMaintenanceReply): ReadSourceMaintenanceReply {
  return {
    sourceId: run.sourceId,
    status: run.status === 'failed' ? 'failed' : 'idle',
    remainingHashCandidates: run.remainingHashCandidates,
    remainingProbeCandidates: run.remainingProbeCandidates,
    remainingPlayableMediaPromotionCandidates: run.remainingPlayableMediaPromotionCandidates,
    remainingTrackIdentityCandidateProductionCandidates:
      run.remainingTrackIdentityCandidateProductionCandidates,
    remainingTrackIdentityDecisionProductionCandidates:
      run.remainingTrackIdentityDecisionProductionCandidates,
    ...(run.attachmentLinks === undefined ? {} : { attachmentLinks: run.attachmentLinks }),
    ...(run.sourceFailure === undefined ? {} : { sourceFailure: run.sourceFailure }),
    lastRun: {
      status: run.status,
      hash: run.hash,
      attachmentMaterialization: run.attachmentMaterialization,
      probe: run.probe,
      playableMediaPromotion: run.playableMediaPromotion,
      trackIdentityCandidates: run.trackIdentityCandidates,
      trackIdentityDecisions: run.trackIdentityDecisions,
      remainingHashCandidates: run.remainingHashCandidates,
      remainingProbeCandidates: run.remainingProbeCandidates,
      remainingPlayableMediaPromotionCandidates: run.remainingPlayableMediaPromotionCandidates,
      remainingTrackIdentityCandidateProductionCandidates:
        run.remainingTrackIdentityCandidateProductionCandidates,
      remainingTrackIdentityDecisionProductionCandidates:
        run.remainingTrackIdentityDecisionProductionCandidates,
      ...(run.sourceFailure === undefined ? {} : { sourceFailure: run.sourceFailure })
    }
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
