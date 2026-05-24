import { onMounted, onUnmounted, ref } from 'vue'
import type { Ref } from 'vue'

import type { SelectedContentsReadResult } from '../../../shared/librarySelectedContents/read'
import type { RendererApi } from '../../../shared/rendererApi'
import type { RowBinding } from '../state'

export type SelectedContentsBoundaryState =
  | {
      readonly kind: 'idle'
      readonly detail?: string
    }
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'ready'
      readonly requestKey: string
      readonly result: SelectedContentsReadResult
    }
  | {
      readonly kind: 'failed'
      readonly requestKey: string
      readonly detail: string
    }

export type SelectedContentsReadController = {
  readonly state: Ref<SelectedContentsBoundaryState>
  readonly readForBinding: (binding: RowBinding | undefined, options?: ReadOptions) => Promise<boolean>
  readonly clear: () => void
  readonly start: () => void
  readonly stop: () => void
}

type ReadOptions = {
  readonly force?: boolean
}

type LibrarySelectedContentsApi = RendererApi['library']['selectedContents']

const readLimit = 100
const safeSelectedContentsRequestFailure = 'Unable to request selected library contents.'

export function useSelectedContentsRead(
  selectedContentsApi: LibrarySelectedContentsApi = getRendererApi().library.selectedContents
): SelectedContentsReadController {
  const controller = createSelectedContentsReadController(selectedContentsApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

export function createSelectedContentsReadController(
  selectedContentsApi: LibrarySelectedContentsApi
): SelectedContentsReadController {
  const state = ref<SelectedContentsBoundaryState>({
    kind: 'idle',
    detail: 'No selected contents scope has been requested.'
  })
  let readSequence = 0
  let started = false

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
  }

  function clear(): void {
    state.value = {
      kind: 'idle',
      detail: 'No selected contents scope is active.'
    }
  }

  async function readForBinding(
    binding: RowBinding | undefined,
    options: ReadOptions = {}
  ): Promise<boolean> {
    const scope = selectedContentsScopeForBinding(binding)

    if (scope === undefined) {
      clear()
      return false
    }

    const requestKey = selectedContentsRequestKey(scope)
    const currentState = state.value

    if (
      !options.force &&
      (currentState.kind === 'loading' || currentState.kind === 'ready') &&
      currentState.requestKey === requestKey
    ) {
      return false
    }

    const sequence = ++readSequence
    state.value = {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading selected contents.'
    }

    try {
      const result = await selectedContentsApi.read({
        scope,
        limit: readLimit
      })

      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      state.value = {
        kind: 'ready',
        requestKey,
        result
      }
      return true
    } catch {
      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      state.value = {
        kind: 'failed',
        requestKey,
        detail: safeSelectedContentsRequestFailure
      }
      return true
    }
  }

  function isCurrentLoading(requestKey: string, sequence: number): boolean {
    const currentState = state.value
    return (
      currentState.kind === 'loading' &&
      currentState.requestKey === requestKey &&
      currentState.sequence === sequence
    )
  }

  return {
    state,
    readForBinding,
    clear,
    start,
    stop
  }
}

function selectedContentsScopeForBinding(
  binding: RowBinding | undefined
): Parameters<LibrarySelectedContentsApi['read']>[0]['scope'] | undefined {
  if (binding === undefined) {
    return undefined
  }

  switch (binding.kind) {
    case 'source':
      if (binding.target.entryPoint.kind === 'source') {
        return {
          kind: 'source',
          sourceId: binding.target.entryPoint.sourceId
        }
      }

      return {
        kind: 'sourceLocation',
        sourceLocationId: binding.target.entryPoint.sourceLocationId
      }
    case 'directory':
      return {
        kind: 'directory',
        sourceId: binding.sourceId,
        sourceDirectoryId: binding.directoryId
      }
    case 'file':
    case 'navigation':
    case 'readState':
    case 'more':
      return undefined
  }
}

function selectedContentsRequestKey(
  scope: NonNullable<Parameters<LibrarySelectedContentsApi['read']>[0]['scope']>
): string {
  switch (scope.kind) {
    case 'source':
      return `source:${scope.sourceId}`
    case 'sourceLocation':
      return `source-location:${scope.sourceLocationId}`
    case 'directory':
      return `directory:${scope.sourceId}:${scope.sourceDirectoryId}`
  }
}
