import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'
import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

import type {
  SourceActivityReadError,
  SourceActivityReadResult
} from '../../../shared/library/source/activity'
import type { RendererApi } from '../../../shared/rendererApi'

export type ReadApi = Pick<RendererApi['library'], 'sourceActivity'>

export type Controller = {
  readonly snapshotBySourceId: Ref<ReadonlyMap<string, ReadSourceActivityReply>>
  readonly readErrorsBySourceId: Ref<ReadonlyMap<string, SourceActivityReadError>>
  readonly read: (sourceId: string, options?: ReadOptions) => Promise<boolean>
  readonly refresh: (sourceIds: Iterable<string>) => Promise<boolean>
  readonly invalidateSource: (sourceId: string) => void
  readonly start: () => void
  readonly stop: () => void
}

type ReadOptions = {
  readonly force?: boolean
}

type InFlightRead = {
  readonly sequence: number
  readonly promise: Promise<boolean>
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
  const snapshotBySourceId = shallowRef<ReadonlyMap<string, ReadSourceActivityReply>>(new Map())
  const readErrorsBySourceId = shallowRef<ReadonlyMap<string, SourceActivityReadError>>(new Map())
  const inFlightReadsBySourceId = new Map<string, InFlightRead>()
  const latestReadSequenceBySourceId = new Map<string, number>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
  }

  function invalidateSource(sourceId: string): void {
    latestReadSequenceBySourceId.set(
      sourceId,
      (latestReadSequenceBySourceId.get(sourceId) ?? 0) + 1
    )
    inFlightReadsBySourceId.delete(sourceId)
    deleteSnapshot(sourceId)
    clearReadError(sourceId)
  }

  function read(sourceId: string, options: ReadOptions = {}): Promise<boolean> {
    const current = inFlightReadsBySourceId.get(sourceId)
    if (!options.force && current !== undefined) {
      return current.promise
    }

    const sequence = nextReadSequence(sourceId)
    const nextRead = requestRead(sourceId, sequence).finally(() => {
      if (inFlightReadsBySourceId.get(sourceId)?.sequence === sequence) {
        inFlightReadsBySourceId.delete(sourceId)
      }
    })
    inFlightReadsBySourceId.set(sourceId, { sequence, promise: nextRead })
    return nextRead
  }

  async function requestRead(sourceId: string, sequence: number): Promise<boolean> {
    let result: SourceActivityReadResult

    try {
      result = await libraryApi.sourceActivity.readSourceActivity({ sourceId })
    } catch {
      result = {
        state: 'readFailed',
        error: {
          code: 'readFailed',
          message: 'Unable to read source activity.'
        }
      }
    }

    if (stopped || !isCurrentRead(sourceId, sequence)) {
      return false
    }

    if (result.state === 'ready') {
      const next = new Map(snapshotBySourceId.value)
      next.set(sourceId, result.activity)
      snapshotBySourceId.value = next
      clearReadError(sourceId)
      return true
    }

    const nextErrors = new Map(readErrorsBySourceId.value)
    nextErrors.set(sourceId, result.error)
    readErrorsBySourceId.value = nextErrors
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

  function clearReadError(sourceId: string): void {
    if (!readErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(readErrorsBySourceId.value)
    next.delete(sourceId)
    readErrorsBySourceId.value = next
  }

  function deleteSnapshot(sourceId: string): void {
    if (!snapshotBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(snapshotBySourceId.value)
    next.delete(sourceId)
    snapshotBySourceId.value = next
  }

  function nextReadSequence(sourceId: string): number {
    const sequence = (latestReadSequenceBySourceId.get(sourceId) ?? 0) + 1
    latestReadSequenceBySourceId.set(sourceId, sequence)
    return sequence
  }

  function isCurrentRead(sourceId: string, sequence: number): boolean {
    return latestReadSequenceBySourceId.get(sourceId) === sequence
  }

  return {
    snapshotBySourceId,
    readErrorsBySourceId,
    read,
    refresh,
    invalidateSource,
    start,
    stop
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
