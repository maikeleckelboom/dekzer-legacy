import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundary/status'
import type {
  EntryPoint,
  ChildRow,
  ReadRequest,
  ReadResult,
  ReadRoot,
  ChildWindow
} from '../../shared/libraryHierarchy/readChildren'
import type {
  LibraryNavigationReadRowsResult,
  LibraryNavigationRow
} from '../../shared/libraryNavigation/readRows'
import type { RendererApi } from '../../shared/rendererApi'
import { projectState, type BrowserProjection } from './hierarchyProjection'
import type {
  MoreTarget,
  DirectoryState,
  DirectoryTarget,
  MoreState,
  LoadedChildren,
  SourceState,
  SourceTarget
} from './hierarchyState'
import type { BrowserTreeNodeId } from './tree/types'

const readLimit = 50
const safeNavigationReadRequestFailure = 'Unable to request library navigation rows.'
const safeSourceReadRequestFailure = 'Unable to request library source hierarchy children.'
const safeChildReadRequestFailure = 'Unable to request library hierarchy directory children.'
const safeUnexpectedChildWindowFailure = 'The hierarchy read returned an unexpected child window.'
const positiveOpaqueIdPattern = /^[1-9]\d*$/

export type LibraryBrowserApi = RendererApi['library']

export type LibraryHierarchyReadController = {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | undefined>
  readonly navigationReadResult: Ref<LibraryNavigationReadRowsResult | undefined>
  readonly hierarchyReadResult: Ref<ReadResult | undefined>
  readonly navigationReadRequestError: Ref<string | undefined>
  readonly hierarchyReadRequestError: Ref<string | undefined>
  readonly navigationReadIsLoading: Ref<boolean>
  readonly hierarchyReadIsLoading: Ref<boolean>
  readonly sourceReadStates: Ref<ReadonlyMap<string, SourceState>>
  readonly directoryReadStates: Ref<ReadonlyMap<string, DirectoryState>>
  readonly browserProjection: ComputedRef<BrowserProjection | undefined>
  readonly currentRoot: ComputedRef<ReadRoot | undefined>
  readonly refresh: () => Promise<boolean>
  readonly loadFirstSource: () => Promise<boolean>
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
  const hierarchyReadResult = ref<ReadResult>()
  const navigationReadRequestError = ref<string>()
  const hierarchyReadRequestError = ref<string>()
  const navigationReadIsLoading = ref(false)
  const hierarchyReadIsLoading = ref(false)
  const sourceReadStates = shallowRef<ReadonlyMap<string, SourceState>>(new Map())
  const directoryReadStates = shallowRef<ReadonlyMap<string, DirectoryState>>(new Map())
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
    void refresh()
  }

  async function refresh(): Promise<boolean> {
    const readNavigationSucceeded = await refreshNavigationRows()

    if (!readNavigationSucceeded) {
      return false
    }

    return loadFirstSource()
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

  async function loadFirstSource(): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    for (const [nodeId, binding] of projection.bindingsById) {
      if (binding.kind === 'source') {
        return readSource(nodeId, binding.target)
      }
    }

    return projection.nodes.length > 0
  }

  async function requestNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    const binding = projection.bindingsById.get(nodeId)

    if (binding === undefined) {
      return false
    }

    switch (binding.kind) {
      case 'source':
        return readSource(nodeId, binding.target)
      case 'directory':
        return readDirectory({
          entryPoint: binding.entryPoint,
          ...(binding.label === undefined ? {} : { label: binding.label }),
          sourceDirectoryId: binding.sourceDirectoryId
        })
      case 'more':
        return readMore(binding.target)
    }

    return false
  }

  async function requestDirectoryChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return false
    }

    const binding = projection.bindingsById.get(nodeId)

    if (binding === undefined || binding.kind !== 'directory') {
      return false
    }

    return readDirectory({
      entryPoint: binding.entryPoint,
      ...(binding.label === undefined ? {} : { label: binding.label }),
      sourceDirectoryId: binding.sourceDirectoryId
    })
  }

  async function readSource(nodeId: BrowserTreeNodeId, target: SourceTarget): Promise<boolean> {
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

      if (!isExpectedWindow(result.window, 0, undefined, target.entryPoint)) {
        setSourceReadState(nodeId, {
          kind: 'failed',
          detail: safeUnexpectedChildWindowFailure
        })
        return true
      }

      setSourceReadState(
        nodeId,
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(result.window, sourceLoadedTarget(target))
        },
        result.window
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

  async function readDirectory(target: DirectoryTarget): Promise<boolean> {
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

      if (!isExpectedWindow(result.window, 0, target.sourceDirectoryId, target.entryPoint)) {
        setDirectoryReadState(target.sourceDirectoryId, {
          kind: 'failed',
          detail: safeUnexpectedChildWindowFailure
        })
        return true
      }

      setDirectoryReadState(
        target.sourceDirectoryId,
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(result.window, directoryLoadedTarget(target))
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
    } finally {
      if (sequence === directoryReadSequence) {
        hierarchyReadIsLoading.value = false
      }
    }
  }

  async function readMore(target: MoreTarget): Promise<boolean> {
    if (target.parentSourceDirectoryId === undefined) {
      return readSourceMore(target)
    }

    return readDirectoryMore(target)
  }

  async function readSourceMore(target: MoreTarget): Promise<boolean> {
    const requestKey = createMoreRequestKey(target)
    const currentState = sourceReadStates.value.get(target.ownerNodeId)

    if (!canReadMore(currentState, target)) {
      return false
    }

    if (
      currentState.children.more?.kind === 'loading' &&
      currentState.children.more.requestKey === requestKey
    ) {
      return false
    }

    const sequence = ++sourceReadSequence
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined
    setSourceMoreState(target, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading more literal hierarchy rows.'
    })

    try {
      const result = await libraryApi.hierarchy.readChildren(moreReadRequest(target))

      if (!isCurrentSourceMoreLoading(target, requestKey, sequence)) {
        return false
      }

      if (result.state !== 'ready') {
        setSourceMoreState(target, {
          kind: 'failed',
          detail: result.error.message
        })
        return true
      }

      if (
        !isExpectedWindow(
          result.window,
          target.offset,
          target.parentSourceDirectoryId,
          target.entryPoint
        )
      ) {
        setSourceMoreState(target, {
          kind: 'failed',
          detail: safeUnexpectedChildWindowFailure
        })
        return true
      }

      const state = sourceReadStates.value.get(target.ownerNodeId)

      if (!canReadMore(state, target)) {
        return false
      }

      setSourceReadState(
        target.ownerNodeId,
        {
          kind: 'loaded',
          children: appendHierarchyChildrenWindow(state.children, result.window)
        },
        result.window
      )
      return true
    } catch {
      if (isCurrentSourceMoreLoading(target, requestKey, sequence)) {
        hierarchyReadRequestError.value = safeSourceReadRequestFailure
        setSourceMoreState(target, {
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

  async function readDirectoryMore(target: MoreTarget): Promise<boolean> {
    const sourceDirectoryId = target.parentSourceDirectoryId

    if (sourceDirectoryId === undefined) {
      return false
    }

    const requestKey = createMoreRequestKey(target)
    const currentState = directoryReadStates.value.get(sourceDirectoryId)

    if (!canReadMore(currentState, target)) {
      return false
    }

    if (
      currentState.children.more?.kind === 'loading' &&
      currentState.children.more.requestKey === requestKey
    ) {
      return false
    }

    const sequence = ++directoryReadSequence
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined
    setDirectoryMoreState(target, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading more children.'
    })

    try {
      const result = await libraryApi.hierarchy.readChildren(moreReadRequest(target))

      if (!isCurrentDirectoryMoreLoading(target, requestKey, sequence)) {
        return false
      }

      if (result.state !== 'ready') {
        setDirectoryMoreState(target, {
          kind: 'failed',
          detail: result.error.message
        })
        return true
      }

      if (!isExpectedWindow(result.window, target.offset, sourceDirectoryId, target.entryPoint)) {
        setDirectoryMoreState(target, {
          kind: 'failed',
          detail: safeUnexpectedChildWindowFailure
        })
        return true
      }

      const state = directoryReadStates.value.get(sourceDirectoryId)

      if (!canReadMore(state, target)) {
        return false
      }

      setDirectoryReadState(
        sourceDirectoryId,
        {
          kind: 'loaded',
          children: appendHierarchyChildrenWindow(state.children, result.window)
        },
        result.window
      )
      return true
    } catch {
      if (isCurrentDirectoryMoreLoading(target, requestKey, sequence)) {
        setDirectoryMoreState(target, {
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
    state: SourceState,
    discoveredWindow?: ChildWindow
  ): void {
    const nextStates = new Map(sourceReadStates.value)
    nextStates.set(nodeId, state)
    sourceReadStates.value = nextStates

    if (discoveredWindow !== undefined) {
      const nextDirectoryStates = new Map(directoryReadStates.value)
      addDiscoveredUnloadedDirectoryStates(nextDirectoryStates, discoveredWindow)
      directoryReadStates.value = nextDirectoryStates
    }
  }

  function setDirectoryReadState(
    sourceDirectoryId: string,
    state: DirectoryState,
    discoveredWindow?: ChildWindow
  ): void {
    const nextStates = new Map(directoryReadStates.value)
    nextStates.set(sourceDirectoryId, state)

    if (discoveredWindow !== undefined) {
      addDiscoveredUnloadedDirectoryStates(nextStates, discoveredWindow)
    }

    directoryReadStates.value = nextStates
  }

  function setSourceMoreState(target: MoreTarget, more: MoreState): void {
    const state = sourceReadStates.value.get(target.ownerNodeId)

    if (!canReadMore(state, target)) {
      return
    }

    setSourceReadState(target.ownerNodeId, {
      kind: 'loaded',
      children: withMoreState(state.children, more)
    })
  }

  function setDirectoryMoreState(target: MoreTarget, more: MoreState): void {
    const sourceDirectoryId = target.parentSourceDirectoryId

    if (sourceDirectoryId === undefined) {
      return
    }

    const state = directoryReadStates.value.get(sourceDirectoryId)

    if (!canReadMore(state, target)) {
      return
    }

    setDirectoryReadState(sourceDirectoryId, {
      kind: 'loaded',
      children: withMoreState(state.children, more)
    })
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

  function isCurrentSourceMoreLoading(
    target: MoreTarget,
    requestKey: string,
    sequence: number
  ): boolean {
    const state = sourceReadStates.value.get(target.ownerNodeId)
    const more = state?.kind === 'loaded' ? state.children.more : undefined

    return more?.kind === 'loading' && more.requestKey === requestKey && more.sequence === sequence
  }

  function isCurrentDirectoryMoreLoading(
    target: MoreTarget,
    requestKey: string,
    sequence: number
  ): boolean {
    const sourceDirectoryId = target.parentSourceDirectoryId

    if (sourceDirectoryId === undefined) {
      return false
    }

    const state = directoryReadStates.value.get(sourceDirectoryId)
    const more = state?.kind === 'loaded' ? state.children.more : undefined

    return more?.kind === 'loading' && more.requestKey === requestKey && more.sequence === sequence
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
    refresh,
    loadFirstSource,
    requestNodeChildren,
    requestDirectoryChildren,
    start,
    stop
  }
}

function sourceReadRequest(target: SourceTarget): ReadRequest {
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

function directoryReadRequest(target: DirectoryTarget): ReadRequest {
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

function moreReadRequest(target: MoreTarget): ReadRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyReadEntryPoint(target.entryPoint),
      ...(target.label === undefined ? {} : { label: target.label })
    },
    ...(target.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: target.parentSourceDirectoryId }),
    offset: target.offset,
    limit: target.limit
  }
}

function sourceLoadedTarget(target: SourceTarget): {
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly parentSourceDirectoryId?: string
} {
  return {
    entryPoint: copyReadEntryPoint(target.entryPoint),
    label: target.label
  }
}

function directoryLoadedTarget(target: DirectoryTarget): {
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly parentSourceDirectoryId?: string
} {
  return {
    entryPoint: copyReadEntryPoint(target.entryPoint),
    ...(target.label === undefined ? {} : { label: target.label }),
    parentSourceDirectoryId: target.sourceDirectoryId
  }
}

function loadedChildrenFromWindow(
  window: ChildWindow,
  target: {
    readonly entryPoint: EntryPoint
    readonly label?: string
    readonly parentSourceDirectoryId?: string
  }
): LoadedChildren {
  return makeLoadedChildren({
    entryPoint: target.entryPoint,
    ...(target.label === undefined ? {} : { label: target.label }),
    ...(target.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: target.parentSourceDirectoryId }),
    rows: window.nodes,
    totalRows: window.totalRows,
    limit: window.limit
  })
}

function appendHierarchyChildrenWindow(
  children: LoadedChildren,
  window: ChildWindow
): LoadedChildren {
  return makeLoadedChildren({
    entryPoint: children.entryPoint,
    ...(children.label === undefined ? {} : { label: children.label }),
    ...(children.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: children.parentSourceDirectoryId }),
    rows: [...children.rows, ...window.nodes],
    totalRows: window.totalRows,
    limit: window.limit
  })
}

function withMoreState(children: LoadedChildren, more: MoreState): LoadedChildren {
  return makeLoadedChildren({
    entryPoint: children.entryPoint,
    ...(children.label === undefined ? {} : { label: children.label }),
    ...(children.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: children.parentSourceDirectoryId }),
    rows: children.rows,
    totalRows: children.totalRows,
    limit: children.limit,
    more
  })
}

function makeLoadedChildren(options: {
  readonly entryPoint: EntryPoint
  readonly parentSourceDirectoryId?: string
  readonly label?: string
  readonly rows: readonly ChildRow[]
  readonly totalRows: number
  readonly limit: number
  readonly more?: MoreState
}): LoadedChildren {
  const nextOffset = options.rows.length < options.totalRows ? options.rows.length : undefined

  return {
    entryPoint: copyReadEntryPoint(options.entryPoint),
    ...(options.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: options.parentSourceDirectoryId }),
    ...(options.label === undefined ? {} : { label: options.label }),
    rows: options.rows,
    totalRows: options.totalRows,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: Math.min(options.limit, readLimit),
    ...(options.more === undefined ? {} : { more: options.more })
  }
}

function isExpectedWindow(
  window: ChildWindow,
  expectedOffset: number,
  expectedParentSourceDirectoryId: string | undefined,
  expectedEntryPoint: EntryPoint
): boolean {
  if (window.offset !== expectedOffset) {
    return false
  }

  if ((window.parentSourceDirectoryId ?? undefined) !== expectedParentSourceDirectoryId) {
    return false
  }

  if (!sameEntryPoint(window.root.entryPoint, expectedEntryPoint)) {
    return false
  }

  if (window.nodes.length > window.limit) {
    return false
  }

  if (window.offset + window.nodes.length > window.totalRows) {
    return false
  }

  return window.offset >= window.totalRows || window.nodes.length > 0
}

function canReadMore(
  state: SourceState | DirectoryState | undefined,
  target: MoreTarget
): state is Extract<SourceState | DirectoryState, { readonly kind: 'loaded' }> {
  return (
    state?.kind === 'loaded' &&
    state.children.nextOffset === target.offset &&
    sameEntryPoint(state.children.entryPoint, target.entryPoint) &&
    (state.children.parentSourceDirectoryId ?? undefined) === target.parentSourceDirectoryId
  )
}

function sameEntryPoint(left: EntryPoint, right: EntryPoint): boolean {
  if (left.kind !== right.kind) {
    return false
  }

  if (left.kind === 'source') {
    return right.kind === 'source' && left.sourceId === right.sourceId
  }

  return right.kind === 'sourceLocation' && left.sourceLocationId === right.sourceLocationId
}

function copyReadEntryPoint(entryPoint: EntryPoint): EntryPoint {
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
): ReadonlyMap<string, SourceState> {
  const states = new Map<string, SourceState>()

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
  states: Map<string, DirectoryState>,
  window: ChildWindow
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

function sourceReadEntryPointFor(row: LibraryNavigationRow): EntryPoint | undefined {
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

function createEntryPointRequestKey(entryPoint: EntryPoint): string {
  switch (entryPoint.kind) {
    case 'source':
      return `source:${entryPoint.sourceId}`
    case 'sourceLocation':
      return `source-location:${entryPoint.sourceLocationId}`
  }
}

function createDirectoryRequestKey(entryPoint: EntryPoint, sourceDirectoryId: string): string {
  return `${createEntryPointRequestKey(entryPoint)}/directory:${sourceDirectoryId}`
}

function createMoreRequestKey(target: MoreTarget): string {
  return `${createEntryPointRequestKey(target.entryPoint)}/directory:${
    target.parentSourceDirectoryId ?? 'root'
  }/offset:${target.offset}`
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}
