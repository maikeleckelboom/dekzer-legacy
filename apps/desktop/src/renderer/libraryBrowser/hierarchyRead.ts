import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundary/status'
import type {
  LibraryHierarchyReadChildrenEntryPoint,
  LibraryHierarchyReadChildrenRequest,
  LibraryHierarchyReadChildrenResult,
  LibraryHierarchyReadChildrenRoot,
  LibraryHierarchyReadChildrenWindow
} from '../../shared/libraryHierarchy/readChildren'
import type {
  LibraryNavigationReadRowsResult,
  LibraryNavigationRow
} from '../../shared/libraryNavigation/readRows'
import type { RendererApi } from '../../shared/rendererApi'
import { projectState, type HierarchyProjection } from './hierarchyProjection'
import type {
  DirectoryReadState,
  DirectoryReadTarget,
  SourceReadState,
  SourceReadTarget
} from './hierarchyState'
import type { BrowserTreeNodeId } from './tree/types'

const readLimit = 50
const safeNavigationReadRequestFailure = 'Unable to request library navigation rows.'
const safeSourceReadRequestFailure = 'Unable to request library source hierarchy children.'
const safeChildReadRequestFailure = 'Unable to request library hierarchy directory children.'
const safePartialChildReadFailure = 'The hierarchy read returned a partial child window.'
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type LibraryBrowserApi = RendererApi['library']

export type LibraryHierarchyReadController = {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | undefined>
  readonly navigationReadResult: Ref<LibraryNavigationReadRowsResult | undefined>
  readonly hierarchyReadResult: Ref<LibraryHierarchyReadChildrenResult | undefined>
  readonly navigationReadRequestError: Ref<string | undefined>
  readonly hierarchyReadRequestError: Ref<string | undefined>
  readonly navigationReadIsLoading: Ref<boolean>
  readonly hierarchyReadIsLoading: Ref<boolean>
  readonly sourceReadStates: Ref<ReadonlyMap<string, SourceReadState>>
  readonly directoryReadStates: Ref<ReadonlyMap<string, DirectoryReadState>>
  readonly browserProjection: ComputedRef<HierarchyProjection | undefined>
  readonly currentRoot: ComputedRef<LibraryHierarchyReadChildrenRoot | undefined>
  readonly refreshHierarchy: () => Promise<boolean>
  readonly readFirstAvailableSourceHierarchy: () => Promise<boolean>
  readonly requestNodeChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean>
  readonly requestDirectoryChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useLibraryHierarchyRead(
  libraryApi: LibraryBrowserApi = getRendererApi().library
): LibraryHierarchyReadController {
  const controller = createLibraryHierarchyReadController(libraryApi)

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

export function createLibraryHierarchyReadController(
  libraryApi: LibraryBrowserApi
): LibraryHierarchyReadController {
  const hostStatus = ref<LibraryBoundaryHostStatus>()
  const navigationReadResult = ref<LibraryNavigationReadRowsResult>()
  const hierarchyReadResult = ref<LibraryHierarchyReadChildrenResult>()
  const navigationReadRequestError = ref<string>()
  const hierarchyReadRequestError = ref<string>()
  const navigationReadIsLoading = ref(false)
  const hierarchyReadIsLoading = ref(false)
  const sourceReadStates = shallowRef<ReadonlyMap<string, SourceReadState>>(new Map())
  const directoryReadStates = shallowRef<ReadonlyMap<string, DirectoryReadState>>(new Map())
  let hasRequestedNavigationRead = false
  let unsubscribeFromHostStatus: (() => void) | undefined
  let navigationReadSequence = 0
  let sourceReadSequence = 0
  let directoryReadSequence = 0

  const currentRoot = computed(() => {
    const result = hierarchyReadResult.value
    return result?.state === 'ready' ? result.window.root : undefined
  })

  const browserProjection = computed(() =>
    projectState({
      ...(navigationReadResult.value === undefined
        ? {}
        : { navigationReadResult: navigationReadResult.value }),
      sourceReadStates: sourceReadStates.value,
      directoryReadStates: directoryReadStates.value
    })
  )

  function start(): void {
    void libraryApi.host
      .getStatus()
      .then((status) => {
        hostStatus.value = status
        navigationReadRequestError.value = undefined
        hierarchyReadRequestError.value = undefined
        requestNavigationReadIfStarted(status)
      })
      .catch(() => {
        navigationReadRequestError.value = 'Unable to read library boundary host status.'
      })

    unsubscribeFromHostStatus = libraryApi.host.onStatusChanged((status) => {
      hostStatus.value = status
      navigationReadRequestError.value = undefined
      hierarchyReadRequestError.value = undefined
      requestNavigationReadIfStarted(status)
    })
  }

  function stop(): void {
    unsubscribeFromHostStatus?.()
    unsubscribeFromHostStatus = undefined
  }

  function requestNavigationReadIfStarted(status: LibraryBoundaryHostStatus): void {
    if (status.state !== 'started' || hasRequestedNavigationRead) {
      return
    }

    hasRequestedNavigationRead = true
    void refreshHierarchy()
  }

  async function refreshHierarchy(): Promise<boolean> {
    const readNavigationSucceeded = await refreshNavigationRows()

    if (!readNavigationSucceeded) {
      return false
    }

    return readFirstAvailableSourceHierarchy()
  }

  async function refreshNavigationRows(): Promise<boolean> {
    const sequence = ++navigationReadSequence
    navigationReadIsLoading.value = true
    navigationReadRequestError.value = undefined
    hierarchyReadRequestError.value = undefined

    try {
      const result = await libraryApi.navigation.readRows({
        parentNavigationRowId: null
      })

      if (sequence !== navigationReadSequence) {
        return false
      }

      navigationReadResult.value = result
      hierarchyReadResult.value = undefined
      sourceReadStates.value =
        result.state === 'ready' ? withDiscoveredUnloadedSourceStates(result.rows) : new Map()
      directoryReadStates.value = new Map()

      return result.state === 'ready'
    } catch {
      if (sequence === navigationReadSequence) {
        navigationReadRequestError.value = safeNavigationReadRequestFailure
      }

      return false
    } finally {
      if (sequence === navigationReadSequence) {
        navigationReadIsLoading.value = false
      }
    }
  }

  async function readFirstAvailableSourceHierarchy(): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    const firstSourceTarget = projection.sourceReadTargetsByNodeId.entries().next()

    if (firstSourceTarget.done === true) {
      return projection.nodes.length > 0
    }

    return readSourceChildren(firstSourceTarget.value[0], firstSourceTarget.value[1])
  }

  async function requestNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    const sourceTarget = projection.sourceReadTargetsByNodeId.get(nodeId)

    if (sourceTarget !== undefined) {
      return readSourceChildren(nodeId, sourceTarget)
    }

    const directoryTarget = projection.directoryReadTargetsByNodeId.get(nodeId)

    if (directoryTarget !== undefined) {
      return readDirectoryChildren(directoryTarget)
    }

    return false
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

  async function readSourceChildren(
    nodeId: BrowserTreeNodeId,
    target: SourceReadTarget
  ): Promise<boolean> {
    const requestKey = createEntryPointRequestKey(target.entryPoint)
    const currentState = sourceReadStates.value.get(nodeId)

    if (currentState?.kind === 'loading' && currentState.requestKey === requestKey) {
      return false
    }

    const sequence = ++sourceReadSequence
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined
    directoryReadStates.value = new Map()
    setSourceReadState(nodeId, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading literal hierarchy children.'
    })

    try {
      const result = await libraryApi.hierarchy.readChildren(sourceReadRequest(target))

      if (!isCurrentSourceLoading(nodeId, requestKey, sequence)) {
        return false
      }

      hierarchyReadResult.value = result

      if (result.state !== 'ready') {
        setSourceReadState(nodeId, {
          kind: 'failed',
          detail: result.error.message
        })
        return true
      }

      setSourceReadState(
        nodeId,
        {
          kind: 'loaded',
          window: result.window
        },
        result.window,
        target
      )
      return true
    } catch {
      if (isCurrentSourceLoading(nodeId, requestKey, sequence)) {
        hierarchyReadRequestError.value = safeSourceReadRequestFailure
        setSourceReadState(nodeId, {
          kind: 'failed',
          detail: safeSourceReadRequestFailure
        })
        return true
      }

      return false
    } finally {
      if (sequence === sourceReadSequence) {
        hierarchyReadIsLoading.value = false
      }
    }
  }

  async function readDirectoryChildren(target: DirectoryReadTarget): Promise<boolean> {
    const requestKey = createDirectoryRequestKey(target.entryPoint, target.sourceDirectoryId)
    const currentState = directoryReadStates.value.get(target.sourceDirectoryId)

    if (currentState?.kind === 'loading' && currentState.requestKey === requestKey) {
      return false
    }

    const sequence = ++directoryReadSequence
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined
    setDirectoryReadState(target.sourceDirectoryId, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading children.'
    })

    try {
      const result = await libraryApi.hierarchy.readChildren(directoryReadRequest(target))

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
        result.window,
        target
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
    } finally {
      if (sequence === directoryReadSequence) {
        hierarchyReadIsLoading.value = false
      }
    }
  }

  function setSourceReadState(
    nodeId: string,
    state: SourceReadState,
    discoveredWindow?: LibraryHierarchyReadChildrenWindow,
    target?: SourceReadTarget
  ): void {
    const nextStates = new Map(sourceReadStates.value)
    nextStates.set(nodeId, state)
    sourceReadStates.value = nextStates

    if (discoveredWindow !== undefined && target !== undefined) {
      const nextDirectoryStates = new Map(directoryReadStates.value)
      addDiscoveredUnloadedDirectoryStates(nextDirectoryStates, discoveredWindow)
      directoryReadStates.value = nextDirectoryStates
    }
  }

  function setDirectoryReadState(
    sourceDirectoryId: string,
    state: DirectoryReadState,
    discoveredWindow?: LibraryHierarchyReadChildrenWindow,
    target?: DirectoryReadTarget
  ): void {
    const nextStates = new Map(directoryReadStates.value)
    nextStates.set(sourceDirectoryId, state)

    if (discoveredWindow !== undefined && target !== undefined) {
      addDiscoveredUnloadedDirectoryStates(nextStates, discoveredWindow)
    }

    directoryReadStates.value = nextStates
  }

  function isCurrentSourceLoading(nodeId: string, requestKey: string, sequence: number): boolean {
    const state = sourceReadStates.value.get(nodeId)
    return (
      state?.kind === 'loading' && state.requestKey === requestKey && state.sequence === sequence
    )
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
    navigationReadResult,
    hierarchyReadResult,
    navigationReadRequestError,
    hierarchyReadRequestError,
    navigationReadIsLoading,
    hierarchyReadIsLoading,
    sourceReadStates,
    directoryReadStates,
    browserProjection,
    currentRoot,
    refreshHierarchy,
    readFirstAvailableSourceHierarchy,
    requestNodeChildren,
    requestDirectoryChildren,
    start,
    stop
  }
}

function sourceReadRequest(target: SourceReadTarget): LibraryHierarchyReadChildrenRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyReadEntryPoint(target.entryPoint),
      label: target.label
    },
    offset: 0,
    limit: readLimit
  }
}

function directoryReadRequest(target: DirectoryReadTarget): LibraryHierarchyReadChildrenRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyReadEntryPoint(target.entryPoint),
      ...(target.label === undefined ? {} : { label: target.label })
    },
    parentSourceDirectoryId: target.sourceDirectoryId,
    offset: 0,
    limit: readLimit
  }
}

function copyReadEntryPoint(
  entryPoint: LibraryHierarchyReadChildrenEntryPoint
): LibraryHierarchyReadChildrenEntryPoint {
  switch (entryPoint.kind) {
    case 'source':
      return {
        kind: 'source',
        sourceId: entryPoint.sourceId
      }
    case 'sourceLocation':
      return {
        kind: 'sourceLocation',
        sourceLocationId: entryPoint.sourceLocationId
      }
  }
}

function withDiscoveredUnloadedSourceStates(
  rows: readonly LibraryNavigationRow[]
): ReadonlyMap<string, SourceReadState> {
  const states = new Map<string, SourceReadState>()

  for (const row of rows) {
    if (sourceReadEntryPointFor(row) !== undefined) {
      states.set(`navigation-row:${row.navigationRowId}`, {
        kind: 'unloaded',
        detail: 'Literal hierarchy not loaded yet.'
      })
    }
  }

  return states
}

function addDiscoveredUnloadedDirectoryStates(
  states: Map<string, DirectoryReadState>,
  window: LibraryHierarchyReadChildrenWindow
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

function sourceReadEntryPointFor(
  row: LibraryNavigationRow
): LibraryHierarchyReadChildrenEntryPoint | undefined {
  if (row.selectorKind === 'source' && isPositiveOpaqueId(row.selectorPayload)) {
    return {
      kind: 'source',
      sourceId: row.selectorPayload
    }
  }

  if (row.selectorKind === 'sourceLocation' && isPositiveOpaqueId(row.selectorPayload)) {
    return {
      kind: 'sourceLocation',
      sourceLocationId: row.selectorPayload
    }
  }

  return undefined
}

function createEntryPointRequestKey(entryPoint: LibraryHierarchyReadChildrenEntryPoint): string {
  switch (entryPoint.kind) {
    case 'source':
      return `source:${entryPoint.sourceId}`
    case 'sourceLocation':
      return `source-location:${entryPoint.sourceLocationId}`
  }
}

function createDirectoryRequestKey(
  entryPoint: LibraryHierarchyReadChildrenEntryPoint,
  sourceDirectoryId: string
): string {
  return `${createEntryPointRequestKey(entryPoint)}/directory:${sourceDirectoryId}`
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isCompleteWindow(window: {
  readonly offset: number
  readonly nodes: readonly unknown[]
  readonly totalRows: number
}): boolean {
  return window.offset === 0 && window.nodes.length === window.totalRows
}
