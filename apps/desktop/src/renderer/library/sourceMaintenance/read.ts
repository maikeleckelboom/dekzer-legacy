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

export type ReadApi = Pick<RendererApi['library'], 'sourceMaintenance'>

export type RunState = 'idle' | 'running' | 'completed' | 'failed'

export type Controller = {
  readonly snapshotBySourceId: Ref<ReadonlyMap<string, ReadSourceMaintenanceReply>>
  readonly readErrorsBySourceId: Ref<ReadonlyMap<string, SourceMaintenanceError>>
  readonly runStateBySourceId: Ref<ReadonlyMap<string, RunState>>
  readonly runErrorsBySourceId: Ref<ReadonlyMap<string, SourceMaintenanceError>>
  readonly read: (sourceId: string) => Promise<boolean>
  readonly refresh: (sourceIds: Iterable<string>) => Promise<boolean>
  readonly run: (sourceId: string) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useRead(libraryApi: ReadApi = getRendererApi().library): Controller {
  const controller = createController(libraryApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createController(libraryApi: ReadApi): Controller {
  const snapshotBySourceId = shallowRef<ReadonlyMap<string, ReadSourceMaintenanceReply>>(new Map())
  const readErrorsBySourceId = shallowRef<ReadonlyMap<string, SourceMaintenanceError>>(new Map())
  const runStateBySourceId = shallowRef<ReadonlyMap<string, RunState>>(new Map())
  const runErrorsBySourceId = shallowRef<ReadonlyMap<string, SourceMaintenanceError>>(new Map())
  const inFlightReadsBySourceId = new Map<string, Promise<boolean>>()
  const inFlightRunsBySourceId = new Map<string, Promise<boolean>>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
  }

  function read(sourceId: string): Promise<boolean> {
    const current = inFlightReadsBySourceId.get(sourceId)
    if (current !== undefined) {
      return current
    }

    const nextRead = requestRead(sourceId).finally(() => {
      inFlightReadsBySourceId.delete(sourceId)
    })
    inFlightReadsBySourceId.set(sourceId, nextRead)
    return nextRead
  }

  async function requestRead(sourceId: string): Promise<boolean> {
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

  async function refresh(sourceIds: Iterable<string>): Promise<boolean> {
    const uniqueSourceIds = [...new Set(sourceIds)].filter((sourceId) => sourceId.length > 0)
    if (uniqueSourceIds.length === 0) {
      return true
    }

    const results = await Promise.all(uniqueSourceIds.map((sourceId) => read(sourceId)))
    return results.every(Boolean)
  }

  function run(sourceId: string): Promise<boolean> {
    const current = inFlightRunsBySourceId.get(sourceId)
    if (current !== undefined) {
      return current
    }

    const nextRun = requestRun(sourceId).finally(() => {
      inFlightRunsBySourceId.delete(sourceId)
    })
    inFlightRunsBySourceId.set(sourceId, nextRun)
    return nextRun
  }

  async function requestRun(sourceId: string): Promise<boolean> {
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
    const next = new Map(snapshotBySourceId.value)
    next.set(sourceId, snapshot)
    snapshotBySourceId.value = next
  }

  function setReadError(sourceId: string, error: SourceMaintenanceError): void {
    const next = new Map(readErrorsBySourceId.value)
    next.set(sourceId, error)
    readErrorsBySourceId.value = next
  }

  function clearReadError(sourceId: string): void {
    if (!readErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(readErrorsBySourceId.value)
    next.delete(sourceId)
    readErrorsBySourceId.value = next
  }

  function setRunState(sourceId: string, state: RunState): void {
    const next = new Map(runStateBySourceId.value)
    next.set(sourceId, state)
    runStateBySourceId.value = next
  }

  function setRunError(sourceId: string, error: SourceMaintenanceError): void {
    const next = new Map(runErrorsBySourceId.value)
    next.set(sourceId, error)
    runErrorsBySourceId.value = next
  }

  function clearRunError(sourceId: string): void {
    if (!runErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(runErrorsBySourceId.value)
    next.delete(sourceId)
    runErrorsBySourceId.value = next
  }

  return {
    snapshotBySourceId,
    readErrorsBySourceId,
    runStateBySourceId,
    runErrorsBySourceId,
    read,
    refresh,
    run,
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
