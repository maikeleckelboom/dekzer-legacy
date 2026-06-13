import { onMounted, onUnmounted, ref, shallowRef } from 'vue'
import type { Ref } from 'vue'

import type { ReadLocalBrowseItemsRequest } from '../../../shared/library/localBrowse/items'
import type { ReadLocalBrowseEntryPointsResult } from '../../../shared/library/localBrowse/entryPoints'
import type { RendererApi } from '../../../shared/rendererApi'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { RowBinding } from '../state'
import {
  localBrowseRootTarget,
  localBrowseProfileForTarget,
  localBrowseWindowKey,
  localBrowseWindowKeyFromIdentity,
  type LoadedLocalBrowseItems,
  type LocalBrowseDirectoryTarget,
  type LocalBrowseEntryPointsState,
  type LocalBrowseItemState,
  type LocalBrowseMoreTarget
} from './types'
import { defaultProfile, type ProfileKey } from '../browseProfile/types'

const readLimit = 50
const safeEntryPointsReadFailure = 'Unable to read local browse entry points.'
const safeItemsReadFailure = 'Unable to read local browse items.'
const safeUnexpectedWindowFailure = 'The local browse read returned an unexpected item window.'

export type LocalBrowseReadApi = Pick<RendererApi['library'], 'localBrowse'>

export type LocalBrowseController = {
  readonly entryPointsState: Ref<LocalBrowseEntryPointsState>
  readonly itemStates: Ref<ReadonlyMap<string, LocalBrowseItemState>>
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
  options: { readonly profile?: Ref<ProfileKey> } = {}
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
  options: { readonly profile?: Ref<ProfileKey> } = {}
): LocalBrowseController {
  const entryPointsState = ref<LocalBrowseEntryPointsState>({ kind: 'unread' })
  const itemStates = shallowRef<ReadonlyMap<string, LocalBrowseItemState>>(new Map())
  const profile = options.profile ?? ref<ProfileKey>(defaultProfile)
  let started = false
  let entryPointReadSequence = 0
  let itemReadSequence = 0

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
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
        return readItems(localBrowseRootTarget(binding.target, profile.value))
      case 'localBrowseItem':
        return binding.target === undefined ? false : readItems(binding.target)
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
      binding === undefined ? undefined : windowTargetFromBinding(binding, profile.value)

    if (target === undefined) {
      return false
    }

    const state = itemStates.value.get(localBrowseWindowKey(target))

    if (state?.kind !== 'loaded' || state.window.nextOffset === undefined) {
      return false
    }

    return readMore({
      ...target,
      ownerNodeId: nodeId,
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

    for (const target of targets.values()) {
      refreshedAny = true
      allSucceeded = (await readItems(target)) && allSucceeded
    }

    return refreshedAny ? allSucceeded : true
  }

  function browserWindowRefreshTargets(
    expandedNodeIds: ReadonlySet<BrowserTreeNodeId>,
    projection: BrowserProjection | undefined
  ): ReadonlyMap<string, LocalBrowseDirectoryTarget> | undefined {
    if (projection?.kind !== 'tree') {
      return undefined
    }

    const targets = new Map<string, LocalBrowseDirectoryTarget>()

    function addTarget(target: LocalBrowseDirectoryTarget): void {
      targets.set(localBrowseWindowKey(target), target)
    }

    for (const binding of projection.bindingsById.values()) {
      if (binding.kind === 'localBrowseEntryPoint') {
        const target = localBrowseRootTarget(binding.target, profile.value)
        const state = itemStates.value.get(localBrowseWindowKey(target))

        if (state?.kind === 'loaded') {
          addTarget(target)
        }
      } else if (binding.kind === 'localBrowseItem' && binding.target !== undefined) {
        const state = itemStates.value.get(localBrowseWindowKey(binding.target))

        if (state?.kind === 'loaded') {
          addTarget(binding.target)
        }
      }
    }

    for (const nodeId of expandedNodeIds) {
      const binding = projection.bindingsById.get(nodeId)

      if (binding?.kind === 'localBrowseEntryPoint') {
        addTarget(localBrowseRootTarget(binding.target, profile.value))
      } else if (binding?.kind === 'localBrowseItem' && binding.target !== undefined) {
        addTarget(binding.target)
      }
    }

    return targets
  }

  async function readItems(target: LocalBrowseDirectoryTarget): Promise<boolean> {
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

      setItemState(requestKey, {
        kind: 'loaded',
        window: loadedWindowFromResult(result, target)
      })
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

  return {
    entryPointsState,
    itemStates,
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
    profile: localBrowseProfileForTarget(target),
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
    profile: target.profile,
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
    profile: current.profile,
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
  profile: ProfileKey = defaultProfile
): LocalBrowseDirectoryTarget | undefined {
  if (binding.kind === 'localBrowseEntryPoint') {
    return localBrowseRootTarget(binding.target, profile)
  }

  if (binding.kind === 'localBrowseItem') {
    return binding.target
  }

  return undefined
}

export function localBrowseStateForBinding(
  itemStates: ReadonlyMap<string, LocalBrowseItemState>,
  binding: RowBinding,
  profile: ProfileKey = defaultProfile
): LocalBrowseItemState | undefined {
  const target = windowTargetFromBinding(binding, profile)

  return target === undefined ? undefined : itemStates.get(localBrowseWindowKey(target))
}

export function localBrowseWindowKeyForIdentity(
  identity: LoadedLocalBrowseItems['identity'],
  profile: ProfileKey = defaultProfile
): string {
  return localBrowseWindowKeyFromIdentity(identity, profile)
}
