import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../../shared/library/boundary/status'
import type {
  EntryPoint,
  ChildRow,
  LibraryTreeRowPolicy,
  ReadRequest,
  ReadResult,
  ReadRoot,
  ChildWindow
} from '../../../shared/library/hierarchy/read'
import type { NavigationReadRowsResult } from '../../../shared/library/navigation/read'
import type { RendererApi } from '../../../shared/rendererApi'
import { projectState, type BrowserProjection } from '../tree/projection'
import type {
  MoreTarget,
  DirectoryState,
  DirectoryTarget,
  MoreState,
  LoadedChildren,
  RowBinding,
  SourceState,
  SourceTarget
} from '../state'
import type { BrowserTreeNodeId } from '../tree/types'
import { copyEntryPoint, sameEntryPoint } from '../runtime/entryPoint'
import {
  defaultLibraryBrowseProfile,
  type LibraryBrowseProfile
} from '../libraryBrowseProfile/types'

const readLimit = 50
const branchWarmupDepth = 2
const branchWarmupBreadth = 24
const maxConcurrentBranchWarmReads = 2
const safeNavigationReadRequestFailure = 'Unable to request library navigation rows.'
const safeSourceReadRequestFailure = 'Unable to request library source hierarchy children.'
const safeChildReadRequestFailure = 'Unable to request library hierarchy directory children.'
const safeUnexpectedChildWindowFailure = 'The hierarchy read returned an unexpected child window.'

export type LibraryHierarchyReadApi = Pick<
  RendererApi['library'],
  'host' | 'navigation' | 'hierarchy'
>

export type LibraryBranchWarmupTrace = {
  readonly parentNodeId: BrowserTreeNodeId
  readonly parentWindowIdentity: string
  readonly scheduledChildCount: number
  readonly skippedTerminalCount: number
  readonly staleIgnoredCount: number
  readonly reason?: string
}

export type LibraryHierarchyReadController = {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | undefined>
  readonly navigationReadResult: Ref<NavigationReadRowsResult | undefined>
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
  readonly refreshNavigationRows: () => Promise<boolean>
  readonly refreshBrowserWindows: (
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  ) => Promise<boolean>
  readonly loadFirstSource: () => Promise<boolean>
  readonly requestNodeChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean>
  readonly requestDirectoryChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useLibraryHierarchyRead(
  libraryApi: LibraryHierarchyReadApi = getRendererApi().library,
  options: {
    readonly profile?: Ref<LibraryBrowseProfile>
    readonly warmup?: LibraryBranchWarmupOptions
  } = {}
): LibraryHierarchyReadController {
  const controller = createLibraryHierarchyReadController(libraryApi, options)

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
  libraryApi: LibraryHierarchyReadApi,
  options: {
    readonly profile?: Ref<LibraryBrowseProfile>
    readonly warmup?: LibraryBranchWarmupOptions
  } = {}
): LibraryHierarchyReadController {
  const hostStatus = ref<LibraryBoundaryHostStatus>()
  const navigationReadResult = shallowRef<NavigationReadRowsResult>()
  const hierarchyReadResult = shallowRef<ReadResult>()
  const navigationReadRequestError = ref<string>()
  const hierarchyReadRequestError = ref<string>()
  const navigationReadIsLoading = ref(false)
  const hierarchyReadIsLoading = ref(false)
  const sourceReadStates = shallowRef<ReadonlyMap<string, SourceState>>(new Map())
  const directoryReadStates = shallowRef<ReadonlyMap<string, DirectoryState>>(new Map())
  const profile = options.profile ?? ref<LibraryBrowseProfile>(defaultLibraryBrowseProfile)
  let hasRequestedNavigationRead = false
  let unsubscribeFromHostStatus: (() => void) | undefined
  let navigationReadSequence = 0
  let sourceReadSequence = 0
  let directoryReadSequence = 0
  let warmupGeneration = 0
  let activeWarmReads = 0
  let staleIgnoredWarmReads = 0
  const queuedWarmReadKeys = new Set<string>()
  const activeWarmReadKeys = new Set<string>()
  const warmReadQueue: LibraryBranchWarmupTask[] = []

  const currentRoot = computed(() => {
    const result = hierarchyReadResult.value
    return result?.state === 'ready' ? result.window.root : undefined
  })

  const browserProjection = computed(() =>
    projectState({
      sourceReadStates: sourceReadStates.value,
      directoryReadStates: directoryReadStates.value,
      ...(hostStatus.value === undefined ? {} : { hostStatus: hostStatus.value }),
      ...(navigationReadResult.value === undefined
        ? {}
        : { navigationReadResult: navigationReadResult.value })
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
    cancelBranchWarmups()
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

    await loadFirstSource()
    return true
  }

  async function refreshNavigationRows(): Promise<boolean> {
    const sequence = ++navigationReadSequence
    const priorAcceptedNavigationResult = acceptedNavigationResult(navigationReadResult.value)
    cancelBranchWarmups()
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

      if (result.state !== 'ready' && priorAcceptedNavigationResult !== undefined) {
        navigationReadRequestError.value = result.error.message
        return true
      }

      navigationReadResult.value = result

      if (priorAcceptedNavigationResult === undefined) {
        hierarchyReadResult.value = undefined
      }

      return result.state === 'ready'
    } catch {
      if (sequence === navigationReadSequence) {
        navigationReadRequestError.value = safeNavigationReadRequestFailure
        if (priorAcceptedNavigationResult !== undefined) {
          return true
        }
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
        return readDirectoryWindow(
          {
            entryPoint: binding.entryPoint,
            ...(binding.label === undefined ? {} : { label: binding.label }),
            directoryId: binding.directoryId
          },
          { ownerNodeId: nodeId }
        )
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

    return readDirectoryWindow(
      {
        entryPoint: binding.entryPoint,
        ...(binding.label === undefined ? {} : { label: binding.label }),
        directoryId: binding.directoryId
      },
      { ownerNodeId: nodeId }
    )
  }

  async function refreshBrowserWindows(
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  ): Promise<boolean> {
    const targets = browserWindowRefreshTargets(expandedNodeIds)

    if (targets === undefined) {
      return false
    }

    let refreshedAny = false
    let allSucceeded = true

    for (const [nodeId, target] of targets.sourceTargets) {
      refreshedAny = true
      allSucceeded = (await readSource(nodeId, target)) && allSucceeded
    }

    for (const { nodeId, target } of targets.directoryTargets.values()) {
      refreshedAny = true
      allSucceeded = (await readDirectoryWindow(target, { ownerNodeId: nodeId })) && allSucceeded
    }

    return refreshedAny ? allSucceeded : true
  }

  function browserWindowRefreshTargets(expandedNodeIds: ReadonlySet<BrowserTreeNodeId>):
    | {
        readonly sourceTargets: ReadonlyMap<string, SourceTarget>
        readonly directoryTargets: ReadonlyMap<
          string,
          { readonly nodeId: BrowserTreeNodeId; readonly target: DirectoryTarget }
        >
      }
    | undefined {
    const projection = browserProjection.value

    if (projection?.kind !== 'tree') {
      return undefined
    }

    const sourceTargets = new Map<string, SourceTarget>()
    const directoryTargets = new Map<
      string,
      { readonly nodeId: BrowserTreeNodeId; readonly target: DirectoryTarget }
    >()

    function addSourceTarget(nodeId: string, target: SourceTarget): void {
      sourceTargets.set(nodeId, target)
    }

    function addDirectoryTarget(
      nodeId: BrowserTreeNodeId,
      binding: Extract<RowBinding, { readonly kind: 'directory' }>
    ): void {
      const target = {
        entryPoint: binding.entryPoint,
        ...(binding.label === undefined ? {} : { label: binding.label }),
        directoryId: binding.directoryId
      }
      const requestKey = createDirectoryRequestKey(
        target.entryPoint,
        target.directoryId,
        profile.value
      )

      directoryTargets.set(requestKey, { nodeId, target })
    }

    for (const [nodeId, binding] of projection.bindingsById) {
      if (binding.kind === 'source') {
        const state = sourceReadStates.value.get(nodeId)
        if (state?.kind === 'loaded') {
          addSourceTarget(nodeId, binding.target)
        }
      } else if (binding.kind === 'directory') {
        const state = directoryReadStates.value.get(binding.directoryId)
        if (state?.kind === 'loaded') {
          addDirectoryTarget(nodeId, binding)
        }
      }
    }

    for (const nodeId of expandedNodeIds) {
      const binding = projection.bindingsById.get(nodeId)

      if (binding?.kind === 'source') {
        addSourceTarget(nodeId, binding.target)
      } else if (binding?.kind === 'directory') {
        addDirectoryTarget(nodeId, binding)
      }
    }

    return { sourceTargets, directoryTargets }
  }

  async function readSource(nodeId: BrowserTreeNodeId, target: SourceTarget): Promise<boolean> {
    const requestKey = createEntryPointRequestKey(target.entryPoint, profile.value)
    const currentState = resolveSourceState(nodeId)

    if (currentState?.kind === 'loading' && currentState.requestKey === requestKey) {
      return false
    }

    if (currentState?.kind === 'refreshing' && currentState.requestKey === requestKey) {
      return false
    }

    const hadPriorChildren = currentState?.kind === 'loaded'
    const sequence = ++sourceReadSequence

    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined

    if (hadPriorChildren) {
      setSourceReadState(nodeId, {
        kind: 'refreshing',
        children: currentState.children,
        requestKey,
        sequence,
        detail: 'Refreshing hierarchy children.'
      })
    } else {
      setSourceReadState(nodeId, {
        kind: 'loading',
        requestKey,
        sequence,
        detail: 'Loading literal hierarchy children.'
      })
    }

    try {
      const result = await libraryApi.hierarchy.readChildren(
        sourceReadRequest(target, profile.value)
      )

      if (!isCurrentSourceLoading(nodeId, requestKey, sequence)) {
        return false
      }

      hierarchyReadResult.value = result

      if (result.state !== 'ready') {
        if (hadPriorChildren) {
          setSourceReadState(nodeId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          setSourceReadState(nodeId, {
            kind: 'failed',
            detail: result.error.message,
            errorCode: result.error.code
          })
        }
        return true
      }

      if (!isExpectedWindow(result.window, 0, undefined, target.entryPoint)) {
        if (hadPriorChildren) {
          setSourceReadState(nodeId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          setSourceReadState(nodeId, {
            kind: 'failed',
            detail: safeUnexpectedChildWindowFailure,
            errorCode: 'windowMismatch'
          })
        }
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
      scheduleBranchWarmupFromWindow({
        anchorNodeId: nodeId,
        parent: {
          kind: 'source',
          nodeId,
          entryPoint: target.entryPoint
        },
        target: sourceLoadedTarget(target),
        window: result.window,
        remainingDepth: branchWarmupDepth
      })
      return true
    } catch {
      if (isCurrentSourceLoading(nodeId, requestKey, sequence)) {
        if (hadPriorChildren) {
          hierarchyReadRequestError.value = safeSourceReadRequestFailure
          setSourceReadState(nodeId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          hierarchyReadRequestError.value = safeSourceReadRequestFailure
          setSourceReadState(nodeId, {
            kind: 'failed',
            detail: safeSourceReadRequestFailure,
            errorCode: 'readFailed'
          })
        }
        return true
      }

      return false
    } finally {
      if (sequence === sourceReadSequence) {
        hierarchyReadIsLoading.value = false
      }
    }
  }

  async function readDirectoryWindow(
    target: DirectoryTarget,
    options: { readonly ownerNodeId?: BrowserTreeNodeId } = {}
  ): Promise<boolean> {
    const requestKey = createDirectoryRequestKey(
      target.entryPoint,
      target.directoryId,
      profile.value
    )
    const currentState = resolveDirectoryState(target.directoryId)

    if (currentState?.kind === 'loading' && currentState.requestKey === requestKey) {
      return false
    }

    if (currentState?.kind === 'refreshing' && currentState.requestKey === requestKey) {
      return false
    }

    const hadPriorChildren = currentState?.kind === 'loaded'
    const sequence = ++directoryReadSequence

    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = undefined

    if (hadPriorChildren) {
      setDirectoryReadState(target.directoryId, {
        kind: 'refreshing',
        children: currentState.children,
        requestKey,
        sequence,
        detail: 'Refreshing children.'
      })
    } else {
      setDirectoryReadState(target.directoryId, {
        kind: 'loading',
        requestKey,
        sequence,
        detail: 'Loading children.'
      })
    }

    try {
      const result = await libraryApi.hierarchy.readChildren(
        directoryReadRequest(target, profile.value)
      )

      if (!isCurrentDirectoryLoading(target.directoryId, requestKey, sequence)) {
        return false
      }

      if (result.state !== 'ready') {
        if (hadPriorChildren) {
          setDirectoryReadState(target.directoryId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          setDirectoryReadState(target.directoryId, {
            kind: 'failed',
            detail: result.error.message,
            errorCode: result.error.code
          })
        }
        return true
      }

      if (!isExpectedWindow(result.window, 0, target.directoryId, target.entryPoint)) {
        if (hadPriorChildren) {
          setDirectoryReadState(target.directoryId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          setDirectoryReadState(target.directoryId, {
            kind: 'failed',
            detail: safeUnexpectedChildWindowFailure,
            errorCode: 'windowMismatch'
          })
        }
        return true
      }

      setDirectoryReadState(
        target.directoryId,
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(result.window, directoryLoadedTarget(target))
        },
        result.window
      )
      if (options.ownerNodeId !== undefined) {
        scheduleBranchWarmupFromWindow({
          anchorNodeId: options.ownerNodeId,
          parent: {
            kind: 'directory',
            nodeId: options.ownerNodeId,
            directoryId: target.directoryId,
            entryPoint: target.entryPoint
          },
          target: directoryLoadedTarget(target),
          window: result.window,
          remainingDepth: branchWarmupDepth
        })
      }
      return true
    } catch {
      if (isCurrentDirectoryLoading(target.directoryId, requestKey, sequence)) {
        if (hadPriorChildren) {
          setDirectoryReadState(target.directoryId, {
            kind: 'loaded',
            children: currentState.children
          })
        } else {
          setDirectoryReadState(target.directoryId, {
            kind: 'failed',
            detail: safeChildReadRequestFailure,
            errorCode: 'readFailed'
          })
        }
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
    if (target.parentDirectoryId === undefined) {
      return readSourceMore(target)
    }

    return readDirectoryMore(target)
  }

  async function readSourceMore(target: MoreTarget): Promise<boolean> {
    const requestKey = createMoreRequestKey(target, profile.value)
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
      const result = await libraryApi.hierarchy.readChildren(moreReadRequest(target, profile.value))

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
        !isExpectedWindow(result.window, target.offset, target.parentDirectoryId, target.entryPoint)
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
    const directoryId = target.parentDirectoryId

    if (directoryId === undefined) {
      return false
    }

    const requestKey = createMoreRequestKey(target, profile.value)
    const currentState = directoryReadStates.value.get(directoryId)

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
      const result = await libraryApi.hierarchy.readChildren(moreReadRequest(target, profile.value))

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

      if (!isExpectedWindow(result.window, target.offset, directoryId, target.entryPoint)) {
        setDirectoryMoreState(target, {
          kind: 'failed',
          detail: safeUnexpectedChildWindowFailure
        })
        return true
      }

      const state = directoryReadStates.value.get(directoryId)

      if (!canReadMore(state, target)) {
        return false
      }

      setDirectoryReadState(
        directoryId,
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
    directoryId: string,
    state: DirectoryState,
    discoveredWindow?: ChildWindow
  ): void {
    const nextStates = new Map(directoryReadStates.value)
    nextStates.set(directoryId, state)

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
    const directoryId = target.parentDirectoryId

    if (directoryId === undefined) {
      return
    }

    const state = directoryReadStates.value.get(directoryId)

    if (!canReadMore(state, target)) {
      return
    }

    setDirectoryReadState(directoryId, {
      kind: 'loaded',
      children: withMoreState(state.children, more)
    })
  }

  function isCurrentSourceLoading(nodeId: string, requestKey: string, sequence: number): boolean {
    const state = resolveSourceState(nodeId)
    return (
      (state?.kind === 'loading' || state?.kind === 'refreshing') &&
      state.requestKey === requestKey &&
      state.sequence === sequence
    )
  }

  function isCurrentDirectoryLoading(
    directoryId: string,
    requestKey: string,
    sequence: number
  ): boolean {
    const state = resolveDirectoryState(directoryId)
    return (
      (state?.kind === 'loading' || state?.kind === 'refreshing') &&
      state.requestKey === requestKey &&
      state.sequence === sequence
    )
  }

  function isCurrentSourceMoreLoading(
    target: MoreTarget,
    requestKey: string,
    sequence: number
  ): boolean {
    const state = resolveSourceState(target.ownerNodeId)
    const more = state?.kind === 'loaded' ? state.children.more : undefined

    return more?.kind === 'loading' && more.requestKey === requestKey && more.sequence === sequence
  }

  function isCurrentDirectoryMoreLoading(
    target: MoreTarget,
    requestKey: string,
    sequence: number
  ): boolean {
    const directoryId = target.parentDirectoryId

    if (directoryId === undefined) {
      return false
    }

    const state = resolveDirectoryState(directoryId)
    const more = state?.kind === 'loaded' ? state.children.more : undefined

    return more?.kind === 'loading' && more.requestKey === requestKey && more.sequence === sequence
  }

  function resolveSourceState(nodeId: string): SourceState | undefined {
    return sourceReadStates.value.get(nodeId)
  }

  function resolveDirectoryState(directoryId: string): DirectoryState | undefined {
    return directoryReadStates.value.get(directoryId)
  }

  function scheduleBranchWarmupFromWindow(options: {
    readonly anchorNodeId: BrowserTreeNodeId
    readonly parent: LibraryBranchWarmupParent
    readonly target: {
      readonly entryPoint: EntryPoint
      readonly label?: string
      readonly parentDirectoryId?: string
    }
    readonly window: ChildWindow
    readonly remainingDepth: number
  }): void {
    if (!isBranchWarmupEnabled() || options.remainingDepth <= 0) {
      return
    }

    if (!shouldContinueBranchWarmup(options.anchorNodeId)) {
      traceBranchWarmup({
        parentNodeId: options.parent.nodeId,
        parentWindowIdentity: branchWarmupParentIdentity(options.parent, profile.value),
        scheduledChildCount: 0,
        skippedTerminalCount: 0,
        staleIgnoredCount: staleIgnoredWarmReads,
        reason: 'inactive'
      })
      return
    }

    let skippedTerminalCount = 0
    const childTargets: WarmableDirectoryTarget[] = []

    for (const row of options.window.nodes) {
      if (childTargets.length >= branchWarmupBreadth) {
        break
      }

      if (row.kind !== 'directory' || row.navigableChildScopeState !== 'hasNavigableChildScopes') {
        skippedTerminalCount += 1
        continue
      }

      childTargets.push({
        nodeId: row.id,
        entryPoint: copyEntryPoint(options.target.entryPoint),
        ...(options.target.label === undefined ? {} : { label: options.target.label }),
        directoryId: row.directoryId
      })
    }

    traceBranchWarmup({
      parentNodeId: options.parent.nodeId,
      parentWindowIdentity: branchWarmupParentIdentity(options.parent, profile.value),
      scheduledChildCount: childTargets.length,
      skippedTerminalCount,
      staleIgnoredCount: staleIgnoredWarmReads
    })

    for (const target of childTargets) {
      enqueueBranchWarmup({
        generation: warmupGeneration,
        profile: profile.value,
        anchorNodeId: options.anchorNodeId,
        parent: options.parent,
        target,
        remainingDepth: options.remainingDepth
      })
    }

    drainBranchWarmupQueue()
  }

  function enqueueBranchWarmup(task: Omit<LibraryBranchWarmupTask, 'requestKey'>): void {
    const requestKey = createDirectoryRequestKey(
      task.target.entryPoint,
      task.target.directoryId,
      task.profile
    )

    if (queuedWarmReadKeys.has(requestKey) || activeWarmReadKeys.has(requestKey)) {
      return
    }

    if (!canWarmDirectory(task.target.directoryId)) {
      return
    }

    queuedWarmReadKeys.add(requestKey)
    warmReadQueue.push({
      ...task,
      requestKey
    })
  }

  function drainBranchWarmupQueue(): void {
    if (!isBranchWarmupEnabled()) {
      return
    }

    while (activeWarmReads < maxConcurrentBranchWarmReads && warmReadQueue.length > 0) {
      const task = warmReadQueue.shift()

      if (task === undefined) {
        return
      }

      queuedWarmReadKeys.delete(task.requestKey)

      if (!canStartWarmRead(task)) {
        continue
      }

      activeWarmReads += 1
      activeWarmReadKeys.add(task.requestKey)
      void runBranchWarmupTask(task).finally(() => {
        activeWarmReads -= 1
        activeWarmReadKeys.delete(task.requestKey)
        drainBranchWarmupQueue()
      })
    }
  }

  async function runBranchWarmupTask(task: LibraryBranchWarmupTask): Promise<void> {
    if (!canStartWarmRead(task)) {
      return
    }

    try {
      const result = await libraryApi.hierarchy.readChildren(
        directoryReadRequest(task.target, task.profile)
      )

      if (!canCommitWarmRead(task)) {
        noteStaleWarmRead(task)
        return
      }

      if (result.state !== 'ready') {
        return
      }

      if (!isExpectedWindow(result.window, 0, task.target.directoryId, task.target.entryPoint)) {
        return
      }

      if (!canCommitWarmRead(task)) {
        noteStaleWarmRead(task)
        return
      }

      setDirectoryReadState(
        task.target.directoryId,
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(result.window, directoryLoadedTarget(task.target))
        },
        result.window
      )

      scheduleBranchWarmupFromWindow({
        anchorNodeId: task.anchorNodeId,
        parent: {
          kind: 'directory',
          nodeId: task.target.nodeId,
          directoryId: task.target.directoryId,
          entryPoint: task.target.entryPoint
        },
        target: directoryLoadedTarget(task.target),
        window: result.window,
        remainingDepth: task.remainingDepth - 1
      })
    } catch {
      return
    }
  }

  function canStartWarmRead(task: LibraryBranchWarmupTask): boolean {
    return (
      task.generation === warmupGeneration &&
      task.profile === profile.value &&
      shouldContinueBranchWarmup(task.anchorNodeId) &&
      isWarmParentCurrent(task) &&
      canWarmDirectory(task.target.directoryId)
    )
  }

  function canCommitWarmRead(task: LibraryBranchWarmupTask): boolean {
    return canStartWarmRead(task)
  }

  function canWarmDirectory(directoryId: string): boolean {
    const state = directoryReadStates.value.get(directoryId)
    return state === undefined || state.kind === 'unloaded'
  }

  function isWarmParentCurrent(task: LibraryBranchWarmupTask): boolean {
    if (task.parent.kind === 'source') {
      const state = sourceReadStates.value.get(task.parent.nodeId)

      return (
        state?.kind === 'loaded' &&
        sameEntryPoint(state.children.entryPoint, task.parent.entryPoint) &&
        state.children.parentDirectoryId === undefined &&
        state.children.rows.some(
          (row) => row.kind === 'directory' && row.directoryId === task.target.directoryId
        )
      )
    }

    const state = directoryReadStates.value.get(task.parent.directoryId)

    return (
      state?.kind === 'loaded' &&
      sameEntryPoint(state.children.entryPoint, task.parent.entryPoint) &&
      state.children.parentDirectoryId === task.parent.directoryId &&
      state.children.rows.some(
        (row) => row.kind === 'directory' && row.directoryId === task.target.directoryId
      )
    )
  }

  function noteStaleWarmRead(task: LibraryBranchWarmupTask): void {
    staleIgnoredWarmReads += 1
    traceBranchWarmup({
      parentNodeId: task.parent.nodeId,
      parentWindowIdentity: branchWarmupParentIdentity(task.parent, task.profile),
      scheduledChildCount: 0,
      skippedTerminalCount: 0,
      staleIgnoredCount: staleIgnoredWarmReads,
      reason: 'stale'
    })
  }

  function cancelBranchWarmups(): void {
    warmupGeneration += 1
    warmReadQueue.length = 0
    queuedWarmReadKeys.clear()
  }

  function isBranchWarmupEnabled(): boolean {
    return options.warmup !== undefined && options.warmup.enabled !== false
  }

  function shouldContinueBranchWarmup(anchorNodeId: BrowserTreeNodeId): boolean {
    return options.warmup?.shouldContinue?.(anchorNodeId) ?? true
  }

  function traceBranchWarmup(trace: LibraryBranchWarmupTrace): void {
    options.warmup?.trace?.(trace)
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
    refreshNavigationRows,
    refreshBrowserWindows,
    loadFirstSource,
    requestNodeChildren,
    requestDirectoryChildren,
    start,
    stop
  }
}

type LibraryBranchWarmupOptions = {
  readonly enabled?: boolean
  readonly shouldContinue?: (anchorNodeId: BrowserTreeNodeId) => boolean
  readonly trace?: (trace: LibraryBranchWarmupTrace) => void
}

type WarmableDirectoryTarget = DirectoryTarget & {
  readonly nodeId: BrowserTreeNodeId
}

type LibraryBranchWarmupParent =
  | {
      readonly kind: 'source'
      readonly nodeId: BrowserTreeNodeId
      readonly entryPoint: EntryPoint
    }
  | {
      readonly kind: 'directory'
      readonly nodeId: BrowserTreeNodeId
      readonly directoryId: string
      readonly entryPoint: EntryPoint
    }

type LibraryBranchWarmupTask = {
  readonly generation: number
  readonly profile: LibraryBrowseProfile
  readonly anchorNodeId: BrowserTreeNodeId
  readonly parent: LibraryBranchWarmupParent
  readonly target: WarmableDirectoryTarget
  readonly remainingDepth: number
  readonly requestKey: string
}

function acceptedNavigationResult(
  result: NavigationReadRowsResult | undefined
): Extract<NavigationReadRowsResult, { readonly state: 'ready' }> | undefined {
  return result?.state === 'ready' ? result : undefined
}

function sourceReadRequest(target: SourceTarget, profile: LibraryBrowseProfile): ReadRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyEntryPoint(target.entryPoint),
      label: target.label
    },
    rowPolicy: rowPolicyForProfile(profile),
    offset: 0,
    limit: readLimit
  }
}

function directoryReadRequest(target: DirectoryTarget, profile: LibraryBrowseProfile): ReadRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyEntryPoint(target.entryPoint),
      ...(target.label === undefined ? {} : { label: target.label })
    },
    parentDirectoryId: target.directoryId,
    rowPolicy: rowPolicyForProfile(profile),
    offset: 0,
    limit: readLimit
  }
}

function moreReadRequest(target: MoreTarget, profile: LibraryBrowseProfile): ReadRequest {
  return {
    target: {
      kind: 'entryPoint',
      entryPoint: copyEntryPoint(target.entryPoint),
      ...(target.label === undefined ? {} : { label: target.label })
    },
    ...(target.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: target.parentDirectoryId }),
    rowPolicy: rowPolicyForProfile(profile),
    offset: target.offset,
    limit: target.limit
  }
}

function sourceLoadedTarget(target: SourceTarget): {
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly parentDirectoryId?: string
} {
  return {
    entryPoint: copyEntryPoint(target.entryPoint),
    label: target.label
  }
}

function directoryLoadedTarget(target: DirectoryTarget): {
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly parentDirectoryId?: string
} {
  return {
    entryPoint: copyEntryPoint(target.entryPoint),
    ...(target.label === undefined ? {} : { label: target.label }),
    parentDirectoryId: target.directoryId
  }
}

function loadedChildrenFromWindow(
  window: ChildWindow,
  target: {
    readonly entryPoint: EntryPoint
    readonly label?: string
    readonly parentDirectoryId?: string
  }
): LoadedChildren {
  return makeLoadedChildren({
    entryPoint: target.entryPoint,
    ...(target.label === undefined ? {} : { label: target.label }),
    ...(target.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: target.parentDirectoryId }),
    rows: window.nodes,
    totalRows: window.totalRows,
    coverage: window.coverage,
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
    ...(children.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: children.parentDirectoryId }),
    rows: [...children.rows, ...window.nodes],
    totalRows: window.totalRows,
    coverage: window.coverage,
    limit: window.limit
  })
}

function withMoreState(children: LoadedChildren, more: MoreState): LoadedChildren {
  return makeLoadedChildren({
    entryPoint: children.entryPoint,
    ...(children.label === undefined ? {} : { label: children.label }),
    ...(children.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: children.parentDirectoryId }),
    rows: children.rows,
    totalRows: children.totalRows,
    coverage: children.coverage,
    limit: children.limit,
    more
  })
}

function makeLoadedChildren(options: {
  readonly entryPoint: EntryPoint
  readonly parentDirectoryId?: string
  readonly label?: string
  readonly rows: readonly ChildRow[]
  readonly totalRows: number
  readonly coverage: LoadedChildren['coverage']
  readonly limit: number
  readonly more?: MoreState
}): LoadedChildren {
  const nextOffset = options.rows.length < options.totalRows ? options.rows.length : undefined

  return {
    entryPoint: copyEntryPoint(options.entryPoint),
    ...(options.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: options.parentDirectoryId }),
    ...(options.label === undefined ? {} : { label: options.label }),
    rows: options.rows,
    totalRows: options.totalRows,
    coverage: options.coverage,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: Math.min(options.limit, readLimit),
    ...(options.more === undefined ? {} : { more: options.more })
  }
}

function isExpectedWindow(
  window: ChildWindow,
  expectedOffset: number,
  expectedParentDirectoryId: string | undefined,
  expectedEntryPoint: EntryPoint
): boolean {
  if (window.offset !== expectedOffset) {
    return false
  }

  if ((window.parentDirectoryId ?? undefined) !== expectedParentDirectoryId) {
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
    (state.children.parentDirectoryId ?? undefined) === target.parentDirectoryId
  )
}

function addDiscoveredUnloadedDirectoryStates(
  states: Map<string, DirectoryState>,
  window: ChildWindow
): void {
  for (const node of window.nodes) {
    if (node.kind === 'directory' && !states.has(node.directoryId)) {
      states.set(node.directoryId, {
        kind: 'unloaded',
        detail: 'Children not loaded yet.'
      })
    }
  }
}

function createEntryPointRequestKey(entryPoint: EntryPoint, profile: LibraryBrowseProfile): string {
  const policyKey = rowPolicyForProfile(profile)

  switch (entryPoint.kind) {
    case 'source':
      return `source:${entryPoint.sourceId}:${policyKey}`
    case 'sourceLocation':
      return `source-location:${entryPoint.sourceLocationId}:${policyKey}`
  }
}

function rowPolicyForProfile(profile: LibraryBrowseProfile): LibraryTreeRowPolicy {
  switch (profile) {
    case 'audio':
      return 'audioBrowse'
    case 'playable':
      return 'playableMediaBrowse'
    case 'allFiles':
      return 'sourceFileInventory'
  }
}

function createDirectoryRequestKey(
  entryPoint: EntryPoint,
  directoryId: string,
  profile: LibraryBrowseProfile
): string {
  return `${createEntryPointRequestKey(entryPoint, profile)}/directory:${directoryId}`
}

function createMoreRequestKey(target: MoreTarget, profile: LibraryBrowseProfile): string {
  return `${createEntryPointRequestKey(target.entryPoint, profile)}/directory:${
    target.parentDirectoryId ?? 'root'
  }/offset:${target.offset}`
}

function branchWarmupParentIdentity(
  parent: LibraryBranchWarmupParent,
  profile: LibraryBrowseProfile
): string {
  if (parent.kind === 'source') {
    return createEntryPointRequestKey(parent.entryPoint, profile)
  }

  return createDirectoryRequestKey(parent.entryPoint, parent.directoryId, profile)
}
