import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type {
  DekzerRendererApi,
  LibraryBoundaryHostStatus
} from '../../shared/libraryBoundary/status'
import type {
  LibraryHierarchyReadRequest,
  LibraryHierarchyReadResult,
  LibraryHierarchyReadRoot,
  LibraryHierarchyReadWindow
} from '../../shared/libraryHierarchy/read'
import {
  projectLibraryHierarchyBrowserStateToBrowserTree,
  type LibraryHierarchyBrowserProjection
} from './hierarchyProjection'
import type {
  LibraryHierarchyDirectoryReadState,
  LibraryHierarchyDirectoryReadTarget
} from './hierarchyState'
import type { BrowserTreeNodeId } from './tree/types'

const literalHierarchyReadLimit = 50
const safeRootReadRequestFailure = 'Unable to request library hierarchy children.'
const safeChildReadRequestFailure = 'Unable to request library hierarchy directory children.'
const safePartialChildReadFailure = 'The hierarchy read returned a partial child window.'

export type LibraryHierarchyBoundaryApi = DekzerRendererApi['libraryBoundary']

export type LibraryHierarchyReadController = {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | undefined>
  readonly hierarchyReadResult: Ref<LibraryHierarchyReadResult | undefined>
  readonly hierarchyReadRequestError: Ref<string | undefined>
  readonly hierarchyReadIsLoading: Ref<boolean>
  readonly directoryReadStates: Ref<ReadonlyMap<string, LibraryHierarchyDirectoryReadState>>
  readonly browserProjection: ComputedRef<LibraryHierarchyBrowserProjection | undefined>
  readonly currentRoot: ComputedRef<LibraryHierarchyReadRoot | undefined>
  readonly readFirstAvailableSourceHierarchy: () => Promise<void>
  readonly requestDirectoryChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useLibraryHierarchyRead(
  libraryBoundary: LibraryHierarchyBoundaryApi = getDekzerRendererApi().libraryBoundary
): LibraryHierarchyReadController {
  const controller = createLibraryHierarchyReadController(libraryBoundary)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

function getDekzerRendererApi(): DekzerRendererApi {
  return (window as unknown as { readonly dekzer: DekzerRendererApi }).dekzer
}

export function createLibraryHierarchyReadController(
  libraryBoundary: LibraryHierarchyBoundaryApi
): LibraryHierarchyReadController {
  const hostStatus = ref<LibraryBoundaryHostStatus>()
  const hierarchyReadResult = ref<LibraryHierarchyReadResult>()
  const hierarchyReadRequestError = ref<string>()
  const hierarchyReadIsLoading = ref(false)
  const directoryReadStates = shallowRef<ReadonlyMap<string, LibraryHierarchyDirectoryReadState>>(
    new Map()
  )
  let hasRequestedHierarchyRead = false
  let unsubscribeFromHostStatus: (() => void) | undefined
  let rootReadSequence = 0
  let directoryReadSequence = 0

  const currentRoot = computed(() => {
    const result = hierarchyReadResult.value
    return result?.state === 'ready' ? result.window.root : undefined
  })

  const browserProjection = computed(() =>
    projectLibraryHierarchyBrowserStateToBrowserTree({
      ...(hierarchyReadResult.value === undefined
        ? {}
        : { rootReadResult: hierarchyReadResult.value }),
      directoryReadStates: directoryReadStates.value
    })
  )

  function start(): void {
    void libraryBoundary
      .getStatus()
      .then((status) => {
        hostStatus.value = status
        hierarchyReadRequestError.value = undefined
        requestHierarchyReadIfStarted(status)
      })
      .catch(() => {
        hierarchyReadRequestError.value = 'Unable to read library boundary host status.'
      })

    unsubscribeFromHostStatus = libraryBoundary.onStatusChanged((status) => {
      hostStatus.value = status
      hierarchyReadRequestError.value = undefined
      requestHierarchyReadIfStarted(status)
    })
  }

  function stop(): void {
    unsubscribeFromHostStatus?.()
    unsubscribeFromHostStatus = undefined
  }

  function requestHierarchyReadIfStarted(status: LibraryBoundaryHostStatus): void {
    if (status.state !== 'started' || hasRequestedHierarchyRead) {
      return
    }

    hasRequestedHierarchyRead = true
    void readFirstAvailableSourceHierarchy()
  }

  async function readFirstAvailableSourceHierarchy(): Promise<void> {
    const sequence = ++rootReadSequence
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined

    try {
      const result = await libraryBoundary.readLiteralHierarchyChildren({
        target: {
          kind: 'firstAvailableSource'
        },
        offset: 0,
        limit: literalHierarchyReadLimit
      })

      if (sequence !== rootReadSequence) {
        return
      }

      hierarchyReadResult.value = result
      directoryReadStates.value =
        result.state === 'ready' ? withDiscoveredUnloadedDirectoryStates(result.window) : new Map()
    } catch {
      if (sequence === rootReadSequence) {
        hierarchyReadRequestError.value = safeRootReadRequestFailure
      }
    } finally {
      if (sequence === rootReadSequence) {
        hierarchyReadIsLoading.value = false
      }
    }
  }

  async function requestDirectoryChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    const target = projection.directoryReadTargetsByNodeId.get(nodeId)

    if (target === undefined) {
      return false
    }

    return readDirectoryChildren(target)
  }

  async function readDirectoryChildren(
    target: LibraryHierarchyDirectoryReadTarget
  ): Promise<boolean> {
    const root = currentRoot.value

    if (root === undefined) {
      return false
    }

    const requestKey = createDirectoryRequestKey(root, target.sourceDirectoryId)
    const currentState = directoryReadStates.value.get(target.sourceDirectoryId)

    if (currentState?.kind === 'loading' && currentState.requestKey === requestKey) {
      return false
    }

    const sequence = ++directoryReadSequence
    setDirectoryReadState(target.sourceDirectoryId, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading children.'
    })

    try {
      const result = await libraryBoundary.readLiteralHierarchyChildren(
        directoryReadRequest(root, target.sourceDirectoryId)
      )

      if (!isCurrentDirectoryLoading(target.sourceDirectoryId, requestKey, sequence)) {
        return false
      }

      if (result.state !== 'ready') {
        setDirectoryReadState(target.sourceDirectoryId, {
          kind: 'failed',
          detail: result.error.message
        })
        return true
      }

      if (!isCompleteWindow(result.window)) {
        setDirectoryReadState(target.sourceDirectoryId, {
          kind: 'failed',
          detail: safePartialChildReadFailure
        })
        return true
      }

      setDirectoryReadState(
        target.sourceDirectoryId,
        {
          kind: 'loaded',
          window: result.window
        },
        result.window
      )
      return true
    } catch {
      if (isCurrentDirectoryLoading(target.sourceDirectoryId, requestKey, sequence)) {
        setDirectoryReadState(target.sourceDirectoryId, {
          kind: 'failed',
          detail: safeChildReadRequestFailure
        })
        return true
      }

      return false
    }
  }

  function setDirectoryReadState(
    sourceDirectoryId: string,
    state: LibraryHierarchyDirectoryReadState,
    discoveredWindow?: LibraryHierarchyReadWindow
  ): void {
    const nextStates = new Map(directoryReadStates.value)
    nextStates.set(sourceDirectoryId, state)

    if (discoveredWindow !== undefined) {
      addDiscoveredUnloadedDirectoryStates(nextStates, discoveredWindow)
    }

    directoryReadStates.value = nextStates
  }

  function isCurrentDirectoryLoading(
    sourceDirectoryId: string,
    requestKey: string,
    sequence: number
  ): boolean {
    const state = directoryReadStates.value.get(sourceDirectoryId)
    return (
      state?.kind === 'loading' && state.requestKey === requestKey && state.sequence === sequence
    )
  }

  return {
    hostStatus,
    hierarchyReadResult,
    hierarchyReadRequestError,
    hierarchyReadIsLoading,
    directoryReadStates,
    browserProjection,
    currentRoot,
    readFirstAvailableSourceHierarchy,
    requestDirectoryChildren,
    start,
    stop
  }
}

function directoryReadRequest(
  root: LibraryHierarchyReadRoot,
  parentSourceDirectoryId: string
): LibraryHierarchyReadRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: root.entryPoint,
      ...(root.label === undefined ? {} : { label: root.label })
    },
    parentSourceDirectoryId,
    offset: 0,
    limit: literalHierarchyReadLimit
  }
}

function withDiscoveredUnloadedDirectoryStates(
  window: LibraryHierarchyReadWindow
): ReadonlyMap<string, LibraryHierarchyDirectoryReadState> {
  const states = new Map<string, LibraryHierarchyDirectoryReadState>()
  addDiscoveredUnloadedDirectoryStates(states, window)
  return states
}

function addDiscoveredUnloadedDirectoryStates(
  states: Map<string, LibraryHierarchyDirectoryReadState>,
  window: LibraryHierarchyReadWindow
): void {
  for (const node of window.nodes) {
    if (node.kind === 'directory' && !states.has(node.sourceDirectoryId)) {
      states.set(node.sourceDirectoryId, {
        kind: 'unloaded',
        detail: 'Children not loaded yet.'
      })
    }
  }
}

function createDirectoryRequestKey(
  root: LibraryHierarchyReadRoot,
  sourceDirectoryId: string
): string {
  const rootKey =
    root.entryPoint.kind === 'source'
      ? `source:${root.entryPoint.sourceId}`
      : `source-location:${root.entryPoint.sourceLocationId}`

  return `${rootKey}/directory:${sourceDirectoryId}`
}

function isCompleteWindow(window: LibraryHierarchyReadWindow): boolean {
  return window.offset === 0 && window.nodes.length === window.totalRows
}
