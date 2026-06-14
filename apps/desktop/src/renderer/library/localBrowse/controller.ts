import { onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { Ref } from 'vue'

import type {
  LocalBrowseItem,
  ReadLocalBrowseItemsRequest
} from '../../../shared/library/localBrowse/items'
import type { ReadLocalBrowseEntryPointsResult } from '../../../shared/library/localBrowse/entryPoints'
import type { RendererApi } from '../../../shared/rendererApi'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { RowBinding } from '../state'
import {
  localBrowseRootTarget,
  localBrowseItemFilterForTarget,
  localBrowseWindowKey,
  localBrowseWindowKeyFromIdentity,
  type LoadedLocalBrowseItems,
  type LocalBrowseDirectoryTarget,
  type LocalBrowseEntryPointsState,
  type LocalBrowseItemState,
  type LocalBrowseMoreTarget
} from './types'
import { defaultAddSourceView, type AddSourceView } from '../addSource/view'

const readLimit = 50
const branchWarmupDepth = 2
const branchWarmupBreadth = 24
const maxConcurrentBranchWarmReads = 2
const safeEntryPointsReadFailure = 'Unable to read local browse entry points.'
const safeItemsReadFailure = 'Unable to read local browse items.'
const safeUnexpectedWindowFailure = 'The local browse read returned an unexpected item window.'

export type LocalBrowseReadApi = Pick<RendererApi['library'], 'localBrowse'>

export type LocalBrowseBranchWarmupTrace = {
  readonly parentNodeId: BrowserTreeNodeId
  readonly parentWindowIdentity: string
  readonly scheduledChildCount: number
  readonly skippedTerminalCount: number
  readonly staleIgnoredCount: number
  readonly reason?: string
}

export type LocalBrowseController = {
  readonly entryPointsState: Ref<LocalBrowseEntryPointsState>
  readonly itemStates: Ref<ReadonlyMap<string, LocalBrowseItemState>>
  readonly clearItemWindows: () => void
  readonly refreshEntryPoints: () => Promise<boolean>
  readonly refreshBrowserWindows: (
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>,
    projection: BrowserProjection | undefined
  ) => Promise<boolean>
  readonly requestNodeChildren: (
    nodeId: BrowserTreeNodeId,
    projection: BrowserProjection | undefined
  ) => Promise<boolean>
  readonly requestNodeMore: (
    nodeId: BrowserTreeNodeId,
    projection: BrowserProjection | undefined
  ) => Promise<boolean>
  readonly start: () => void
  readonly stop: () => void
}

export function useLocalBrowseController(
  libraryApi: LocalBrowseReadApi = getRendererApi().library,
  options: {
    readonly addSourceView?: Ref<AddSourceView>
    readonly warmup?: LocalBrowseBranchWarmupOptions
  } = {}
): LocalBrowseController {
  const controller = createLocalBrowseController(libraryApi, options)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createLocalBrowseController(
  libraryApi: LocalBrowseReadApi,
  options: {
    readonly addSourceView?: Ref<AddSourceView>
    readonly warmup?: LocalBrowseBranchWarmupOptions
  } = {}
): LocalBrowseController {
  const entryPointsState = ref<LocalBrowseEntryPointsState>({ kind: 'unread' })
  const itemStates = shallowRef<ReadonlyMap<string, LocalBrowseItemState>>(new Map())
  const addSourceView = options.addSourceView ?? ref<AddSourceView>(defaultAddSourceView)
  let started = false
  let entryPointReadSequence = 0
  let itemReadSequence = 0
  let warmupGeneration = 0
  let activeWarmReads = 0
  let staleIgnoredWarmReads = 0
  const queuedWarmReadKeys = new Set<string>()
  const activeWarmReadKeys = new Set<string>()
  const warmReadQueue: LocalBrowseBranchWarmupTask[] = []

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
    cancelBranchWarmups()
  }

  function clearItemWindows(): void {
    itemReadSequence += 1
    cancelBranchWarmups()
    itemStates.value = new Map()
  }

  async function refreshEntryPoints(): Promise<boolean> {
    const sequence = ++entryPointReadSequence
    const priorResult = acceptedEntryPointsResult(entryPointsState.value)

    if (priorResult === undefined) {
      entryPointsState.value = {
        kind: 'loading',
        sequence,
        detail: 'Loading local browse entry points.'
      }
    } else {
      entryPointsState.value = {
        kind: 'refreshing',
        result: priorResult,
        sequence,
        detail: 'Refreshing local browse entry points.'
      }
    }

    try {
      const result = await libraryApi.localBrowse.readEntryPoints()

      if (!isCurrentEntryPointRead(sequence)) {
        return false
      }

      if (result.state !== 'read') {
        if (priorResult !== undefined) {
          entryPointsState.value = {
            kind: 'ready',
            result: priorResult,
            refreshError: result.error.message
          }
          return true
        }

        entryPointsState.value = {
          kind: 'failed',
          detail: result.error.message,
          errorCode: result.error.code
        }
        return false
      }

      entryPointsState.value = {
        kind: 'ready',
        result
      }
      return true
    } catch {
      if (!isCurrentEntryPointRead(sequence)) {
        return false
      }

      if (priorResult !== undefined) {
        entryPointsState.value = {
          kind: 'ready',
          result: priorResult,
          refreshError: safeEntryPointsReadFailure
        }
        return true
      }

      entryPointsState.value = {
        kind: 'failed',
        detail: safeEntryPointsReadFailure,
        errorCode: 'readFailed'
      }
      return false
    }
  }

  async function requestNodeChildren(
    nodeId: BrowserTreeNodeId,
    projection: BrowserProjection | undefined
  ): Promise<boolean> {
    if (projection?.kind !== 'tree') {
      return false
    }

    const binding = projection.bindingsById.get(nodeId)

    if (binding === undefined) {
      return false
    }

    switch (binding.kind) {
      case 'localBrowseEntryPoint':
        return readItems(localBrowseRootTarget(binding.target, addSourceView.value), {
          parentNodeId: nodeId
        })
      case 'localBrowseItem':
        return binding.target === undefined
          ? false
          : readItems(binding.target, { parentNodeId: nodeId })
      case 'localBrowseMore':
        return readMore(binding.target)
      default:
        return false
    }
  }

  async function requestNodeMore(
    nodeId: BrowserTreeNodeId,
    projection: BrowserProjection | undefined
  ): Promise<boolean> {
    if (projection?.kind !== 'tree') {
      return false
    }

    const binding = projection.bindingsById.get(nodeId)
    const target =
      binding === undefined ? undefined : windowTargetFromBinding(binding, addSourceView.value)

    if (target === undefined) {
      return false
    }

    const state = itemStates.value.get(localBrowseWindowKey(target))

    if (state?.kind !== 'loaded' || state.window.nextOffset === undefined) {
      return false
    }

    return readMore({
      ...target,
      parentNodeId: nodeId,
      offset: state.window.nextOffset,
      limit: state.window.limit
    })
  }

  async function refreshBrowserWindows(
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>,
    projection: BrowserProjection | undefined
  ): Promise<boolean> {
    const targets = browserWindowRefreshTargets(expandedNodeIds, projection)

    if (targets === undefined) {
      return false
    }

    let refreshedAny = false
    let allSucceeded = true

    for (const { nodeId, target } of targets.values()) {
      refreshedAny = true
      allSucceeded = (await readItems(target, { parentNodeId: nodeId })) && allSucceeded
    }

    return refreshedAny ? allSucceeded : true
  }

  function browserWindowRefreshTargets(
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>,
    projection: BrowserProjection | undefined
  ): ReadonlyMap<
    string,
    { readonly nodeId: BrowserTreeNodeId; readonly target: LocalBrowseDirectoryTarget }
  > | undefined {
    if (projection?.kind !== 'tree') {
      return undefined
    }

    const targets = new Map<
      string,
      { readonly nodeId: BrowserTreeNodeId; readonly target: LocalBrowseDirectoryTarget }
    >()

    function addTarget(nodeId: BrowserTreeNodeId, target: LocalBrowseDirectoryTarget): void {
      targets.set(localBrowseWindowKey(target), { nodeId, target })
    }

    for (const [nodeId, binding] of projection.bindingsById) {
      if (binding.kind === 'localBrowseEntryPoint') {
        const target = localBrowseRootTarget(binding.target, addSourceView.value)
        const state = itemStates.value.get(localBrowseWindowKey(target))

        if (state?.kind === 'loaded') {
          addTarget(nodeId, target)
        }
      } else if (binding.kind === 'localBrowseItem' && binding.target !== undefined) {
        const state = itemStates.value.get(localBrowseWindowKey(binding.target))

        if (state?.kind === 'loaded') {
          addTarget(nodeId, binding.target)
        }
      }
    }

    for (const nodeId of expandedNodeIds) {
      const binding = projection.bindingsById.get(nodeId)

      if (binding?.kind === 'localBrowseEntryPoint') {
        addTarget(nodeId, localBrowseRootTarget(binding.target, addSourceView.value))
      } else if (binding?.kind === 'localBrowseItem' && binding.target !== undefined) {
        addTarget(nodeId, binding.target)
      }
    }

    return targets
  }

  async function readItems(
    target: LocalBrowseDirectoryTarget,
    options: { readonly parentNodeId?: BrowserTreeNodeId } = {}
  ): Promise<boolean> {
    const requestKey = localBrowseWindowKey(target)
    const currentState = itemStates.value.get(requestKey)

    if (
      (currentState?.kind === 'loading' || currentState?.kind === 'refreshing') &&
      currentState.requestKey === requestKey
    ) {
      return false
    }

    const hadPriorWindow = currentState?.kind === 'loaded'
    const sequence = ++itemReadSequence

    if (hadPriorWindow) {
      setItemState(requestKey, {
        kind: 'refreshing',
        window: currentState.window,
        requestKey,
        sequence,
        detail: 'Refreshing local browse items.'
      })
    } else {
      setItemState(requestKey, {
        kind: 'loading',
        requestKey,
        sequence,
        detail: 'Loading local browse items.'
      })
    }

    try {
      const result = await libraryApi.localBrowse.readItems(readItemsRequest(target, 0))

      if (!isCurrentItemRead(requestKey, sequence)) {
        return false
      }

      if (result.state !== 'read') {
        if (hadPriorWindow) {
          setItemState(requestKey, {
            kind: 'loaded',
            window: currentState.window
          })
        } else {
          setItemState(requestKey, {
            kind: 'failed',
            detail: result.error.message,
            errorCode: result.error.code
          })
        }
        return true
      }

      if (!isExpectedWindow(result, target, 0)) {
        if (hadPriorWindow) {
          setItemState(requestKey, {
            kind: 'loaded',
            window: currentState.window
          })
        } else {
          setItemState(requestKey, {
            kind: 'failed',
            detail: safeUnexpectedWindowFailure,
            errorCode: 'windowMismatch'
          })
        }
        return true
      }

      const window = loadedWindowFromResult(result, target)
      setItemState(requestKey, {
        kind: 'loaded',
        window
      })
      if (options.parentNodeId !== undefined) {
        scheduleBranchWarmupFromWindow({
          anchorNodeId: options.parentNodeId,
          parentNodeId: options.parentNodeId,
          parentTarget: target,
          window,
          remainingDepth: branchWarmupDepth
        })
      }
      return true
    } catch {
      if (!isCurrentItemRead(requestKey, sequence)) {
        return false
      }

      if (hadPriorWindow) {
        setItemState(requestKey, {
          kind: 'loaded',
          window: currentState.window
        })
      } else {
        setItemState(requestKey, {
          kind: 'failed',
          detail: safeItemsReadFailure,
          errorCode: 'readFailed'
        })
      }
      return true
    }
  }

  async function readMore(target: LocalBrowseMoreTarget): Promise<boolean> {
    const requestKey = localBrowseWindowKey(target)
    const currentState = itemStates.value.get(requestKey)

    if (currentState?.kind !== 'loaded') {
      return false
    }

    if (currentState.window.nextOffset !== target.offset) {
      return false
    }

    if (currentState.window.more?.kind === 'loading') {
      return false
    }

    const sequence = ++itemReadSequence
    setWindowMoreState(requestKey, {
      kind: 'loading',
      requestKey,
      sequence,
      detail: 'Loading more local browse items.'
    })

    try {
      const result = await libraryApi.localBrowse.readItems(readItemsRequest(target, target.offset))

      if (!isCurrentMoreRead(requestKey, sequence)) {
        return false
      }

      if (result.state !== 'read') {
        setWindowMoreState(requestKey, {
          kind: 'failed',
          detail: result.error.message
        })
        return true
      }

      if (!isExpectedWindow(result, target, target.offset)) {
        setWindowMoreState(requestKey, {
          kind: 'failed',
          detail: safeUnexpectedWindowFailure
        })
        return true
      }

      const state = itemStates.value.get(requestKey)
      if (state?.kind !== 'loaded') {
        return false
      }

      setItemState(requestKey, {
        kind: 'loaded',
        window: appendLoadedWindow(state.window, loadedWindowFromResult(result, target))
      })
      return true
    } catch {
      if (isCurrentMoreRead(requestKey, sequence)) {
        setWindowMoreState(requestKey, {
          kind: 'failed',
          detail: safeItemsReadFailure
        })
        return true
      }

      return false
    }
  }

  function setItemState(key: string, state: LocalBrowseItemState): void {
    const nextStates = new Map(itemStates.value)
    nextStates.set(key, state)
    itemStates.value = nextStates
  }

  function setWindowMoreState(
    key: string,
    more: NonNullable<LoadedLocalBrowseItems['more']>
  ): void {
    const state = itemStates.value.get(key)

    if (state?.kind !== 'loaded') {
      return
    }

    setItemState(key, {
      kind: 'loaded',
      window: {
        ...state.window,
        more
      }
    })
  }

  function isCurrentEntryPointRead(sequence: number): boolean {
    return started && entryPointReadSequence === sequence
  }

  function isCurrentItemRead(requestKey: string, sequence: number): boolean {
    const state = itemStates.value.get(requestKey)

    return (
      started &&
      (state?.kind === 'loading' || state?.kind === 'refreshing') &&
      state.requestKey === requestKey &&
      state.sequence === sequence
    )
  }

  function isCurrentMoreRead(requestKey: string, sequence: number): boolean {
    const state = itemStates.value.get(requestKey)
    const more = state?.kind === 'loaded' ? state.window.more : undefined

    return (
      started &&
      more?.kind === 'loading' &&
      more.requestKey === requestKey &&
      more.sequence === sequence
    )
  }

  function scheduleBranchWarmupFromWindow(options: {
    readonly anchorNodeId: BrowserTreeNodeId
    readonly parentNodeId: BrowserTreeNodeId
    readonly parentTarget: LocalBrowseDirectoryTarget
    readonly window: LoadedLocalBrowseItems
    readonly remainingDepth: number
  }): void {
    if (!isBranchWarmupEnabled() || options.remainingDepth <= 0) {
      return
    }

    if (!shouldContinueBranchWarmup(options.anchorNodeId)) {
      traceBranchWarmup({
        parentNodeId: options.parentNodeId,
        parentWindowIdentity: localBrowseWindowKey(options.parentTarget),
        scheduledChildCount: 0,
        skippedTerminalCount: 0,
        staleIgnoredCount: staleIgnoredWarmReads,
        reason: 'inactive'
      })
      return
    }

    let skippedTerminalCount = 0
    const childTargets: WarmableLocalBrowseTarget[] = []

    for (const item of options.window.items) {
      if (childTargets.length >= branchWarmupBreadth) {
        break
      }

      if (!isWarmableLocalBrowseItem(item)) {
        skippedTerminalCount += 1
        continue
      }

      childTargets.push({
        nodeId: localBrowseItemNodeId(item),
        target: {
          addSourceView: options.window.addSourceView,
          entryPointKind: item.identity.entryPointKind,
          resolvedRootPath: item.identity.resolvedRootPath,
          resolvedParentPath: item.identity.resolvedItemPath,
          label: item.displayName
        }
      })
    }

    traceBranchWarmup({
      parentNodeId: options.parentNodeId,
      parentWindowIdentity: localBrowseWindowKey(options.parentTarget),
      scheduledChildCount: childTargets.length,
      skippedTerminalCount,
      staleIgnoredCount: staleIgnoredWarmReads
    })

    for (const child of childTargets) {
      enqueueBranchWarmup({
        generation: warmupGeneration,
        anchorNodeId: options.anchorNodeId,
        parentNodeId: options.parentNodeId,
        parentTarget: options.parentTarget,
        nodeId: child.nodeId,
        target: child.target,
        remainingDepth: options.remainingDepth
      })
    }

    drainBranchWarmupQueue()
  }

  function enqueueBranchWarmup(task: Omit<LocalBrowseBranchWarmupTask, 'requestKey'>): void {
    const requestKey = localBrowseWindowKey(task.target)

    if (queuedWarmReadKeys.has(requestKey) || activeWarmReadKeys.has(requestKey)) {
      return
    }

    if (!canWarmWindow(requestKey)) {
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

  async function runBranchWarmupTask(task: LocalBrowseBranchWarmupTask): Promise<void> {
    if (!canStartWarmRead(task)) {
      return
    }

    try {
      const result = await libraryApi.localBrowse.readItems(readItemsRequest(task.target, 0))

      if (!canCommitWarmRead(task)) {
        noteStaleWarmRead(task)
        return
      }

      if (result.state !== 'read') {
        return
      }

      if (!isExpectedWindow(result, task.target, 0)) {
        return
      }

      if (!canCommitWarmRead(task)) {
        noteStaleWarmRead(task)
        return
      }

      const window = loadedWindowFromResult(result, task.target)
      setItemState(task.requestKey, {
        kind: 'loaded',
        window
      })

      scheduleBranchWarmupFromWindow({
        anchorNodeId: task.anchorNodeId,
        parentNodeId: task.nodeId,
        parentTarget: task.target,
        window,
        remainingDepth: task.remainingDepth - 1
      })
    } catch {
      return
    }
  }

  function canStartWarmRead(task: LocalBrowseBranchWarmupTask): boolean {
    return (
      started &&
      task.generation === warmupGeneration &&
      task.target.addSourceView === addSourceView.value &&
      shouldContinueBranchWarmup(task.anchorNodeId) &&
      isWarmParentCurrent(task) &&
      canWarmWindow(task.requestKey)
    )
  }

  function canCommitWarmRead(task: LocalBrowseBranchWarmupTask): boolean {
    return canStartWarmRead(task)
  }

  function canWarmWindow(requestKey: string): boolean {
    return itemStates.value.get(requestKey) === undefined
  }

  function isWarmParentCurrent(task: LocalBrowseBranchWarmupTask): boolean {
    const parentState = itemStates.value.get(localBrowseWindowKey(task.parentTarget))

    return (
      parentState?.kind === 'loaded' &&
      parentState.window.addSourceView === task.parentTarget.addSourceView &&
      parentState.window.identity.entryPointKind === task.parentTarget.entryPointKind &&
      parentState.window.identity.resolvedRootPath === task.parentTarget.resolvedRootPath &&
      parentState.window.identity.resolvedParentPath === task.parentTarget.resolvedParentPath &&
      parentState.window.items.some(
        (item) =>
          isWarmableLocalBrowseItem(item) &&
          item.identity.resolvedItemPath === task.target.resolvedParentPath
      )
    )
  }

  function noteStaleWarmRead(task: LocalBrowseBranchWarmupTask): void {
    staleIgnoredWarmReads += 1
    traceBranchWarmup({
      parentNodeId: task.parentNodeId,
      parentWindowIdentity: localBrowseWindowKey(task.parentTarget),
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

  function traceBranchWarmup(trace: LocalBrowseBranchWarmupTrace): void {
    options.warmup?.trace?.(trace)
  }

  return {
    entryPointsState,
    itemStates,
    clearItemWindows,
    refreshEntryPoints,
    refreshBrowserWindows,
    requestNodeChildren,
    requestNodeMore,
    start,
    stop
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

function acceptedEntryPointsResult(
  state: LocalBrowseEntryPointsState
): ReadLocalBrowseEntryPointsResult | undefined {
  if (state.kind === 'ready' || state.kind === 'refreshing') {
    return state.result
  }

  return undefined
}

function readItemsRequest(
  target: LocalBrowseDirectoryTarget,
  offset: number
): ReadLocalBrowseItemsRequest {
  return {
    entryPointKind: target.entryPointKind,
    resolvedRootPath: target.resolvedRootPath,
    resolvedParentPath: target.resolvedParentPath,
    itemFilter: localBrowseItemFilterForTarget(target),
    offset,
    limit: readLimit
  }
}

function loadedWindowFromResult(
  result: Extract<
    Awaited<ReturnType<RendererApi['library']['localBrowse']['readItems']>>,
    {
      readonly state: 'read'
    }
  >,
  target: LocalBrowseDirectoryTarget
): LoadedLocalBrowseItems {
  const nextOffset =
    result.offset + result.items.length < result.totalItems
      ? result.offset + result.items.length
      : undefined

  return {
    addSourceView: target.addSourceView,
    identity: result.windowIdentity,
    label: target.label,
    items: result.items,
    totalItems: result.totalItems,
    status: result.status,
    failure: result.failure,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: Math.min(result.limit, readLimit)
  }
}

function appendLoadedWindow(
  current: LoadedLocalBrowseItems,
  next: LoadedLocalBrowseItems
): LoadedLocalBrowseItems {
  const nextOffset =
    current.items.length + next.items.length < next.totalItems
      ? current.items.length + next.items.length
      : undefined

  return {
    addSourceView: current.addSourceView,
    identity: current.identity,
    label: current.label,
    items: [...current.items, ...next.items],
    totalItems: next.totalItems,
    status: next.status,
    failure: next.failure,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: current.limit
  }
}

function isExpectedWindow(
  result: Extract<
    Awaited<ReturnType<RendererApi['library']['localBrowse']['readItems']>>,
    {
      readonly state: 'read'
    }
  >,
  target: LocalBrowseDirectoryTarget,
  expectedOffset: number
): boolean {
  return (
    result.offset === expectedOffset &&
    result.windowIdentity.entryPointKind === target.entryPointKind &&
    result.windowIdentity.resolvedRootPath === target.resolvedRootPath &&
    result.windowIdentity.resolvedParentPath === target.resolvedParentPath
  )
}

function windowTargetFromBinding(
  binding: RowBinding,
  addSourceView: AddSourceView = defaultAddSourceView
): LocalBrowseDirectoryTarget | undefined {
  if (binding.kind === 'localBrowseEntryPoint') {
    return localBrowseRootTarget(binding.target, addSourceView)
  }

  if (binding.kind === 'localBrowseItem') {
    return binding.target
  }

  return undefined
}

export function localBrowseStateForBinding(
  itemStates: ReadonlyMap<string, LocalBrowseItemState>,
  binding: RowBinding,
  addSourceView: AddSourceView = defaultAddSourceView
): LocalBrowseItemState | undefined {
  const target = windowTargetFromBinding(binding, addSourceView)

  return target === undefined ? undefined : itemStates.get(localBrowseWindowKey(target))
}

export function localBrowseWindowKeyForIdentity(
  identity: LoadedLocalBrowseItems['identity'],
  addSourceView: AddSourceView = defaultAddSourceView
): string {
  return localBrowseWindowKeyFromIdentity(identity, addSourceView)
}

type LocalBrowseBranchWarmupOptions = {
  readonly enabled?: boolean
  readonly shouldContinue?: (anchorNodeId: BrowserTreeNodeId) => boolean
  readonly trace?: (trace: LocalBrowseBranchWarmupTrace) => void
}

type WarmableLocalBrowseTarget = {
  readonly nodeId: BrowserTreeNodeId
  readonly target: LocalBrowseDirectoryTarget
}

type LocalBrowseBranchWarmupTask = {
  readonly generation: number
  readonly anchorNodeId: BrowserTreeNodeId
  readonly parentNodeId: BrowserTreeNodeId
  readonly parentTarget: LocalBrowseDirectoryTarget
  readonly nodeId: BrowserTreeNodeId
  readonly target: LocalBrowseDirectoryTarget
  readonly remainingDepth: number
  readonly requestKey: string
}

function isWarmableLocalBrowseItem(item: LocalBrowseItem): boolean {
  return (
    (item.itemKind === 'directory' || item.itemKind === 'rejectedRoot') &&
    item.availableOperations.some((operation) => operation.kind === 'browseChildren')
  )
}

function localBrowseItemNodeId(item: LocalBrowseItem): BrowserTreeNodeId {
  return `local-browse-item:${item.identity.entryPointKind}:${encodeURIComponent(
    item.identity.resolvedRootPath
  )}:${encodeURIComponent(item.identity.resolvedItemPath)}`
}
