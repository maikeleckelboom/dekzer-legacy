import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'
import type { ReadSourceIntegrityReply } from '@dekzer/library-boundary-contract'

import type {
  SourceIntegrityReadError,
  SourceIntegrityReadResult
} from '../../../shared/library/source/integrity'
import type { RendererApi } from '../../../shared/rendererApi'

export type SourceIntegrityReadApi = Pick<RendererApi['library'], 'sourceIntegrity'>

export type SourceIntegrityReadController = {
  readonly sourceIntegrityBySourceId: Ref<ReadonlyMap<string, ReadSourceIntegrityReply>>
  readonly sourceIntegrityReadErrorsBySourceId: Ref<ReadonlyMap<string, SourceIntegrityReadError>>
  readonly readSourceIntegrity: (sourceId: string) => Promise<boolean>
  readonly refreshSourceIntegrities: (sourceIds: Iterable<string>) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useSourceIntegrityRead(
  libraryApi: SourceIntegrityReadApi = getRendererApi().library
): SourceIntegrityReadController {
  const controller = createSourceIntegrityReadController(libraryApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createSourceIntegrityReadController(
  libraryApi: SourceIntegrityReadApi
): SourceIntegrityReadController {
  const sourceIntegrityBySourceId = shallowRef<ReadonlyMap<string, ReadSourceIntegrityReply>>(
    new Map()
  )
  const sourceIntegrityReadErrorsBySourceId = shallowRef<
    ReadonlyMap<string, SourceIntegrityReadError>
  >(new Map())
  const inFlightReadsBySourceId = new Map<string, Promise<boolean>>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
  }

  function readSourceIntegrity(sourceId: string): Promise<boolean> {
    const current = inFlightReadsBySourceId.get(sourceId)
    if (current !== undefined) {
      return current
    }

    const read = requestSourceIntegrity(sourceId).finally(() => {
      inFlightReadsBySourceId.delete(sourceId)
    })
    inFlightReadsBySourceId.set(sourceId, read)
    return read
  }

  async function requestSourceIntegrity(sourceId: string): Promise<boolean> {
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

    if (stopped) {
      return false
    }

    if (result.state === 'ready') {
      const next = new Map(sourceIntegrityBySourceId.value)
      next.set(sourceId, result.integrity)
      sourceIntegrityBySourceId.value = next
      clearReadError(sourceId)
      return true
    }

    const nextErrors = new Map(sourceIntegrityReadErrorsBySourceId.value)
    nextErrors.set(sourceId, result.error)
    sourceIntegrityReadErrorsBySourceId.value = nextErrors
    return false
  }

  async function refreshSourceIntegrities(sourceIds: Iterable<string>): Promise<boolean> {
    const uniqueSourceIds = [...new Set(sourceIds)].filter((sourceId) => sourceId.length > 0)
    if (uniqueSourceIds.length === 0) {
      return true
    }

    const results = await Promise.all(
      uniqueSourceIds.map((sourceId) => readSourceIntegrity(sourceId))
    )
    return results.every(Boolean)
  }

  function clearReadError(sourceId: string): void {
    if (!sourceIntegrityReadErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(sourceIntegrityReadErrorsBySourceId.value)
    next.delete(sourceId)
    sourceIntegrityReadErrorsBySourceId.value = next
  }

  return {
    sourceIntegrityBySourceId,
    sourceIntegrityReadErrorsBySourceId,
    readSourceIntegrity,
    refreshSourceIntegrities,
    start,
    stop
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
