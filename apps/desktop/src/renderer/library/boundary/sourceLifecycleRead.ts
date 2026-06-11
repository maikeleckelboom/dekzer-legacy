import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'

import type {
  ReadSourceLifecycleError,
  ReadSourceLifecycleErrorState,
  SourceLifecycleRecord
} from '../../../shared/library/source/lifecycle'
import type { RendererApi } from '../../../shared/rendererApi'
import type { RowBinding } from '../state'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'

export type SourceLifecycleReadApi = Pick<RendererApi['library'], 'sourceLifecycle'>

export type SourceLifecycleReadError = {
  readonly state: ReadSourceLifecycleErrorState
  readonly error: ReadSourceLifecycleError
}

export type SourceLifecycleReadController = {
  readonly sourceLifecycleBySourceId: Ref<ReadonlyMap<string, SourceLifecycleRecord>>
  readonly sourceLifecycleReadErrorsBySourceId: Ref<ReadonlyMap<string, SourceLifecycleReadError>>
  readonly readSourceLifecycle: (sourceId: string) => Promise<boolean>
  readonly refreshSourceLifecycles: (sourceIds: Iterable<string>) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useSourceLifecycleRead(
  libraryApi: SourceLifecycleReadApi = getRendererApi().library
): SourceLifecycleReadController {
  const controller = createSourceLifecycleReadController(libraryApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createSourceLifecycleReadController(
  libraryApi: SourceLifecycleReadApi
): SourceLifecycleReadController {
  const sourceLifecycleBySourceId = shallowRef<ReadonlyMap<string, SourceLifecycleRecord>>(
    new Map()
  )
  const sourceLifecycleReadErrorsBySourceId = shallowRef<
    ReadonlyMap<string, SourceLifecycleReadError>
  >(new Map())
  const inFlightReadsBySourceId = new Map<string, Promise<boolean>>()
  let stopped = false

  function start(): void {
    stopped = false
  }

  function stop(): void {
    stopped = true
  }

  function readSourceLifecycle(sourceId: string): Promise<boolean> {
    const currentRead = inFlightReadsBySourceId.get(sourceId)
    if (currentRead !== undefined) {
      return currentRead
    }

    const read = requestSourceLifecycle(sourceId).finally(() => {
      inFlightReadsBySourceId.delete(sourceId)
    })
    inFlightReadsBySourceId.set(sourceId, read)
    return read
  }

  async function requestSourceLifecycle(sourceId: string): Promise<boolean> {
    try {
      const result = await libraryApi.sourceLifecycle.readSourceLifecycle({ sourceId })

      if (stopped) {
        return false
      }

      if (result.state === 'ready') {
        setLifecycleRecord(result.lifecycle)
        clearReadError(sourceId)
        return true
      }

      setReadError(sourceId, {
        state: result.state,
        error: result.error
      })
      return false
    } catch {
      if (!stopped) {
        setReadError(sourceId, {
          state: 'readFailed',
          error: {
            code: 'readFailed',
            message: 'Unable to read source lifecycle.'
          }
        })
      }
      return false
    }
  }

  async function refreshSourceLifecycles(sourceIds: Iterable<string>): Promise<boolean> {
    const uniqueSourceIds = [...new Set(sourceIds)].filter((sourceId) => sourceId.length > 0)
    if (uniqueSourceIds.length === 0) {
      return true
    }

    const results = await Promise.all(
      uniqueSourceIds.map((sourceId) => readSourceLifecycle(sourceId))
    )
    return results.every(Boolean)
  }

  function setLifecycleRecord(lifecycle: SourceLifecycleRecord): void {
    const next = new Map(sourceLifecycleBySourceId.value)
    next.set(lifecycle.sourceId, lifecycle)
    sourceLifecycleBySourceId.value = next
  }

  function setReadError(sourceId: string, error: SourceLifecycleReadError): void {
    const next = new Map(sourceLifecycleReadErrorsBySourceId.value)
    next.set(sourceId, error)
    sourceLifecycleReadErrorsBySourceId.value = next
  }

  function clearReadError(sourceId: string): void {
    if (!sourceLifecycleReadErrorsBySourceId.value.has(sourceId)) {
      return
    }

    const next = new Map(sourceLifecycleReadErrorsBySourceId.value)
    next.delete(sourceId)
    sourceLifecycleReadErrorsBySourceId.value = next
  }

  return {
    sourceLifecycleBySourceId,
    sourceLifecycleReadErrorsBySourceId,
    readSourceLifecycle,
    refreshSourceLifecycles,
    start,
    stop
  }
}

export function sourceLifecycleIdsForBrowserContext(input: {
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
}): ReadonlySet<string> {
  const sourceIds = new Set<string>()
  const projection = input.projection

  if (projection === undefined) {
    return sourceIds
  }

  for (const binding of projection.bindingsById.values()) {
    if (binding.kind === 'source') {
      addSourceId(sourceIds, binding)
    }
  }

  const selectedBinding =
    input.selectedNodeId === undefined
      ? undefined
      : projection.bindingsById.get(input.selectedNodeId)
  if (selectedBinding?.kind === 'source') {
    addSourceId(sourceIds, selectedBinding)
  }

  for (const nodeId of input.expandedNodeIds) {
    const binding = projection.bindingsById.get(nodeId)
    if (binding?.kind === 'source') {
      addSourceId(sourceIds, binding)
    }
  }

  return sourceIds
}

function addSourceId(
  sourceIds: Set<string>,
  binding: Extract<RowBinding, { readonly kind: 'source' }>
): void {
  if (binding.target.entryPoint.kind === 'source') {
    sourceIds.add(binding.target.entryPoint.sourceId)
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
