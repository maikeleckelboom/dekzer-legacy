import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'
import type { ReadSourceIntegrityReply } from '@dekzer/library-boundary-contract'

import type {
  SourceIntegrityReadError,
  SourceIntegrityReadResult
} from '../../../shared/library/source/integrity'
import type { RendererApi } from '../../../shared/rendererApi'

export type ReadApi = Pick<RendererApi['library'], 'sourceIntegrity'>

export type Controller = {
  readonly snapshotBySourceId: Ref<ReadonlyMap<string, ReadSourceIntegrityReply>>
  readonly readErrorsBySourceId: Ref<ReadonlyMap<string, SourceIntegrityReadError>>
  readonly read: (sourceId: string, options?: ReadOptions) => Promise<boolean>
  readonly refresh: (sourceIds: Iterable<string>) => Promise<boolean>
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
  const snapshotBySourceId = shallowRef<ReadonlyMap<string, ReadSourceIntegrityReply>>(new Map())
  const readErrorsBySourceId = shallowRef<ReadonlyMap<string, SourceIntegrityReadError>>(new Map())
  const inFlightReadsBySourceId = new Map<string, InFlightRead>()
  const latestReadSequenceBySourceId = new Map<string, number>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
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
    let result: SourceIntegrityReadResult

    try {
      result = await libraryApi.sourceIntegrity.readSourceIntegrity({ sourceId })
    } catch {
      result = {
        state: 'readFailed',
        error: {
          code: 'readFailed',
          message: 'Unable to read source integrity.'
        }
      }
    }

    if (stopped || !isCurrentRead(sourceId, sequence)) {
      return false
    }

    if (result.state === 'ready') {
      const next = new Map(snapshotBySourceId.value)
      next.set(sourceId, result.integrity)
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
    start,
    stop
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
