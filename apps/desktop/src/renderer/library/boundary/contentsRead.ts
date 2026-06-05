import { onMounted, onUnmounted, ref } from 'vue'
import type { Ref } from 'vue'

import type {
  ContentsReadPolicy,
  ContentsReadResult,
  ContentsRecursion,
  ContentsFileRow
} from '../../../shared/libraryContents/read'
import type { RendererApi } from '../../../shared/rendererApi'
import type { RowBinding } from '../state'

export type ContentsBoundaryState =
  | {
      readonly kind: 'idle'
      readonly pending?: ContentsPendingRead
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
      readonly result: ContentsReadResult
      readonly nextCursor?: string
      readonly accumulatedRows?: readonly ContentsFileRow[]
      readonly pending?: ContentsPendingRead
      readonly refreshError?: string
    }
  | {
      readonly kind: 'failed'
      readonly requestKey: string
      readonly detail: string
    }

export type ContentsPendingRead = {
  readonly requestKey: string
  readonly sequence: number
  readonly detail: string
  readonly presentation: 'deferred' | 'visible'
  readonly cursor?: string
}

export type ContentsReadController = {
  readonly state: Ref<ContentsBoundaryState>
  readonly readForBinding: (
    binding: RowBinding | undefined,
    options?: ReadOptions
  ) => Promise<boolean>
  readonly preloadForBinding: (binding: RowBinding | undefined) => void
  readonly cancelPreloadForBinding: (binding: RowBinding | undefined) => void
  readonly clearWarmSnapshots: () => void
  readonly clear: () => void
  readonly start: () => void
  readonly stop: () => void
}

type ReadOptions = {
  readonly force?: boolean
  readonly cursor?: string
}

type LibraryContentsApi = RendererApi['library']['contents']

const readLimit = 100
const loadingThresholdMs = 125
const preloadThresholdMs = 125
const warmSnapshotTtlMs = 10_000
const maxWarmSnapshots = 16
const maxSpeculativeReads = 1
const safeContentsRequestFailure = 'Unable to request library contents.'
const defaultContentsPolicy: ContentsReadPolicy = {
  mediaClasses: ['audio'],
  rowProfile: { kind: 'sourceFile' }
}
const contentsRecursion: ContentsRecursion = 'recursive'

type ContentsReadTarget = {
  readonly scope: NonNullable<Parameters<LibraryContentsApi['read']>[0]['scope']>
  readonly policy: ContentsReadPolicy
  readonly recursion: ContentsRecursion
  readonly requestKey: string
}

type WarmSnapshot = {
  readonly requestKey: string
  readonly result: ContentsReadResult
  readonly nextCursor?: string
  readonly accumulatedRows?: readonly ContentsFileRow[]
  readonly expiresAtMs: number
}

type SpeculativeRead = {
  readonly requestKey: string
  readonly promise: Promise<ContentsReadResult>
}

export function useContentsRead(
  contentsApi: LibraryContentsApi = getRendererApi().library.contents
): ContentsReadController {
  const controller = createContentsReadController(contentsApi)

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

export function createContentsReadController(
  contentsApi: LibraryContentsApi
): ContentsReadController {
  const state = ref<ContentsBoundaryState>({
    kind: 'idle',
    detail: 'No contents scope has been requested.'
  })
  let readSequence = 0
  let started = false
  let thresholdTimer: ReturnType<typeof setTimeout> | undefined = undefined
  let preloadTimer: ReturnType<typeof setTimeout> | undefined = undefined
  let scheduledPreloadKey: string | undefined = undefined
  let warmGeneration = 0
  const warmSnapshots = new Map<string, WarmSnapshot>()
  const speculativeReads = new Map<string, SpeculativeRead>()

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
    clearThresholdTimer()
    clearPreloadTimer()
  }

  function clear(): void {
    clearThresholdTimer()
    clearPreloadTimer()
    clearWarmSnapshots()
    state.value = {
      kind: 'idle',
      detail: 'No contents scope is active.'
    }
  }

  async function readForBinding(
    binding: RowBinding | undefined,
    options: ReadOptions = {}
  ): Promise<boolean> {
    const target = contentsReadTargetForBinding(binding)

    if (target === undefined) {
      clear()
      return false
    }

    const requestKey = target.requestKey
    const currentState = state.value
    const cursor = options.cursor

    if (options.force) {
      deleteWarmSnapshot(requestKey)
    }

    if (!options.force && !cursor && currentRequestKey(currentState) === requestKey) {
      return false
    }

    if (!options.force && cursor === undefined) {
      const warmSnapshot = freshWarmSnapshot(requestKey)
      if (warmSnapshot !== undefined) {
        clearThresholdTimer()
        state.value = stateFromWarmSnapshot(warmSnapshot)
        return true
      }
    }

    const isSameRequest =
      currentState.kind === 'ready' &&
      currentState.requestKey === requestKey &&
      cursor !== undefined

    const sequence = ++readSequence
    const pending: ContentsPendingRead = {
      requestKey,
      sequence,
      detail: options.cursor !== undefined ? 'Loading more contents.' : 'Loading contents.',
      presentation: pendingPresentation(currentState, requestKey),
      ...(cursor === undefined ? {} : { cursor })
    }
    startPendingRead(pending)

    try {
      const result =
        cursor === undefined && !options.force
          ? await (speculativeReads.get(requestKey)?.promise ?? readContents(target))
          : await readContents(target, cursor)

      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      clearThresholdTimer()
      let rows: readonly ContentsFileRow[]
      const readyResult = result.state === 'ready' ? result.result : undefined
      const previousRows = currentAcceptedRows(requestKey)
      if (isSameRequest && previousRows !== undefined && readyResult !== undefined) {
        rows = [...previousRows, ...(readyResult.rows ?? [])]
      } else if (readyResult !== undefined) {
        rows = readyResult.rows ?? []
      } else {
        rows = []
      }

      if (result.state !== 'ready' && hasAcceptedSnapshot()) {
        retainAcceptedSnapshot(safeContentsResultFailure(result))
        return true
      }

      const nextCursor = readyResult?.nextCursor
      const stateUpdate: ContentsBoundaryState = {
        kind: 'ready',
        requestKey,
        result
      }
      if (nextCursor !== undefined) {
        ;(stateUpdate as { nextCursor?: string }).nextCursor = nextCursor
      }
      if (rows.length > 0) {
        ;(stateUpdate as { accumulatedRows?: readonly ContentsFileRow[] }).accumulatedRows = rows
      }
      state.value = stateUpdate
      storeWarmSnapshotFromState(stateUpdate)
      return true
    } catch {
      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      clearThresholdTimer()
      if (hasAcceptedSnapshot()) {
        retainAcceptedSnapshot(safeContentsRequestFailure)
        return true
      }

      state.value = {
        kind: 'failed',
        requestKey,
        detail: safeContentsRequestFailure
      }
      return true
    }
  }

  function preloadForBinding(binding: RowBinding | undefined): void {
    const target = contentsReadTargetForBinding(binding)

    clearPreloadTimer()

    if (target === undefined || freshWarmSnapshot(target.requestKey) !== undefined) {
      return
    }

    scheduledPreloadKey = target.requestKey
    preloadTimer = setTimeout(() => {
      preloadTimer = undefined

      if (!started || scheduledPreloadKey !== target.requestKey) {
        return
      }

      scheduledPreloadKey = undefined
      void startSpeculativeRead(target)
    }, preloadThresholdMs)
  }

  function cancelPreloadForBinding(binding: RowBinding | undefined): void {
    const target = contentsReadTargetForBinding(binding)

    if (target === undefined || scheduledPreloadKey !== target.requestKey) {
      return
    }

    clearPreloadTimer()
  }

  function startPendingRead(pending: ContentsPendingRead): void {
    clearThresholdTimer()

    const currentState = state.value
    if (currentState.kind === 'ready') {
      state.value = {
        kind: 'ready',
        requestKey: currentState.requestKey,
        result: currentState.result,
        ...(currentState.nextCursor === undefined ? {} : { nextCursor: currentState.nextCursor }),
        ...(currentState.accumulatedRows === undefined
          ? {}
          : { accumulatedRows: currentState.accumulatedRows }),
        pending
      }
      if (pending.presentation === 'deferred') {
        scheduleDeferredPendingPromotion(pending)
      }
      return
    }

    state.value = {
      kind: 'idle',
      pending,
      detail: 'Contents request is pending.'
    }

    thresholdTimer = setTimeout(() => {
      thresholdTimer = undefined

      if (!started || !isCurrentLoading(pending.requestKey, pending.sequence)) {
        return
      }

      state.value = {
        kind: 'loading',
        requestKey: pending.requestKey,
        sequence: pending.sequence,
        detail: pending.detail
      }
    }, loadingThresholdMs)
  }

  function scheduleDeferredPendingPromotion(pending: ContentsPendingRead): void {
    thresholdTimer = setTimeout(() => {
      thresholdTimer = undefined

      if (!started || !isCurrentLoading(pending.requestKey, pending.sequence)) {
        return
      }

      const currentState = state.value
      if (
        currentState.kind !== 'ready' ||
        currentState.pending === undefined ||
        currentState.pending.requestKey !== pending.requestKey ||
        currentState.pending.sequence !== pending.sequence ||
        currentState.pending.presentation !== 'deferred'
      ) {
        return
      }

      state.value = {
        kind: 'ready',
        requestKey: currentState.requestKey,
        result: currentState.result,
        ...(currentState.nextCursor === undefined ? {} : { nextCursor: currentState.nextCursor }),
        ...(currentState.accumulatedRows === undefined
          ? {}
          : { accumulatedRows: currentState.accumulatedRows }),
        pending: {
          ...currentState.pending,
          presentation: 'visible'
        }
      }
    }, loadingThresholdMs)
  }

  function clearThresholdTimer(): void {
    if (thresholdTimer === undefined) {
      return
    }

    clearTimeout(thresholdTimer)
    thresholdTimer = undefined
  }

  function clearPreloadTimer(): void {
    if (preloadTimer !== undefined) {
      clearTimeout(preloadTimer)
      preloadTimer = undefined
    }

    scheduledPreloadKey = undefined
  }

  function clearWarmSnapshots(): void {
    warmGeneration++
    warmSnapshots.clear()
  }

  function pendingPresentation(
    currentState: ContentsBoundaryState,
    requestKey: string
  ): ContentsPendingRead['presentation'] {
    return currentState.kind === 'ready' && currentState.requestKey !== requestKey
      ? 'deferred'
      : 'visible'
  }

  function isCurrentLoading(requestKey: string, sequence: number): boolean {
    const currentState = state.value
    const pending = pendingRead(currentState)
    return pending?.requestKey === requestKey && pending.sequence === sequence
  }

  function pendingRead(currentState: ContentsBoundaryState): ContentsPendingRead | undefined {
    if (currentState.kind === 'idle' || currentState.kind === 'ready') {
      return currentState.pending
    }

    if (currentState.kind === 'loading') {
      return {
        requestKey: currentState.requestKey,
        sequence: currentState.sequence,
        detail: currentState.detail ?? 'Loading contents.',
        presentation: 'visible'
      }
    }

    return undefined
  }

  function currentRequestKey(currentState: ContentsBoundaryState): string | undefined {
    return pendingRead(currentState)?.requestKey ?? acceptedRequestKey(currentState)
  }

  function acceptedRequestKey(currentState: ContentsBoundaryState): string | undefined {
    if (currentState.kind === 'ready') {
      return currentState.requestKey
    }

    if (currentState.kind === 'loading') {
      return currentState.requestKey
    }

    return undefined
  }

  function hasAcceptedSnapshot(): boolean {
    return state.value.kind === 'ready'
  }

  function currentAcceptedRows(requestKey: string): readonly ContentsFileRow[] | undefined {
    const currentState = state.value

    if (currentState.kind !== 'ready' || currentState.requestKey !== requestKey) {
      return undefined
    }

    if (currentState.accumulatedRows !== undefined) {
      return currentState.accumulatedRows
    }

    return currentState.result.state === 'ready' ? currentState.result.result.rows : undefined
  }

  function retainAcceptedSnapshot(detail: string): void {
    const currentState = state.value

    if (currentState.kind !== 'ready') {
      return
    }

    state.value = {
      kind: 'ready',
      requestKey: currentState.requestKey,
      result: currentState.result,
      ...(currentState.nextCursor === undefined ? {} : { nextCursor: currentState.nextCursor }),
      ...(currentState.accumulatedRows === undefined
        ? {}
        : { accumulatedRows: currentState.accumulatedRows }),
      refreshError: detail
    }
  }

  function safeContentsResultFailure(result: ContentsReadResult): string {
    return result.state === 'ready' ? safeContentsRequestFailure : result.error.message
  }

  function readContents(target: ContentsReadTarget, cursor?: string): Promise<ContentsReadResult> {
    return contentsApi.read({
      scope: target.scope,
      policy: target.policy,
      recursion: target.recursion,
      limit: readLimit,
      ...(cursor === undefined ? {} : { cursor })
    })
  }

  function startSpeculativeRead(
    target: ContentsReadTarget
  ): Promise<ContentsReadResult> | undefined {
    const existing = speculativeReads.get(target.requestKey)
    if (existing !== undefined) {
      return existing.promise
    }

    if (speculativeReads.size >= maxSpeculativeReads) {
      return undefined
    }

    const promise = readContents(target)
    const generation = warmGeneration
    speculativeReads.set(target.requestKey, {
      requestKey: target.requestKey,
      promise
    })

    void promise
      .then((result) => {
        if (result.state === 'ready' && generation === warmGeneration) {
          storeWarmSnapshot({
            requestKey: target.requestKey,
            result,
            ...(result.result.nextCursor === undefined
              ? {}
              : { nextCursor: result.result.nextCursor }),
            ...((result.result.rows ?? []).length === 0
              ? {}
              : { accumulatedRows: result.result.rows ?? [] })
          })
        }
      })
      .catch(() => {
        // Failed speculative reads are intentionally invisible to the accepted contents state.
      })
      .finally(() => {
        speculativeReads.delete(target.requestKey)
      })

    return promise
  }

  function freshWarmSnapshot(requestKey: string): WarmSnapshot | undefined {
    const snapshot = warmSnapshots.get(requestKey)

    if (snapshot === undefined) {
      return undefined
    }

    if (Date.now() > snapshot.expiresAtMs) {
      warmSnapshots.delete(requestKey)
      return undefined
    }

    warmSnapshots.delete(requestKey)
    warmSnapshots.set(requestKey, snapshot)
    return snapshot
  }

  function deleteWarmSnapshot(requestKey: string): void {
    warmGeneration++
    warmSnapshots.delete(requestKey)
  }

  function storeWarmSnapshotFromState(stateUpdate: ContentsBoundaryState): void {
    if (stateUpdate.kind !== 'ready' || stateUpdate.result.state !== 'ready') {
      return
    }

    storeWarmSnapshot({
      requestKey: stateUpdate.requestKey,
      result: stateUpdate.result,
      ...(stateUpdate.nextCursor === undefined ? {} : { nextCursor: stateUpdate.nextCursor }),
      ...(stateUpdate.accumulatedRows === undefined
        ? {}
        : { accumulatedRows: stateUpdate.accumulatedRows })
    })
  }

  function storeWarmSnapshot(snapshot: Omit<WarmSnapshot, 'expiresAtMs'>): void {
    warmSnapshots.delete(snapshot.requestKey)
    warmSnapshots.set(snapshot.requestKey, {
      ...snapshot,
      expiresAtMs: Date.now() + warmSnapshotTtlMs
    })

    while (warmSnapshots.size > maxWarmSnapshots) {
      const oldestKey = warmSnapshots.keys().next().value
      if (oldestKey === undefined) {
        return
      }

      warmSnapshots.delete(oldestKey)
    }
  }

  function stateFromWarmSnapshot(snapshot: WarmSnapshot): ContentsBoundaryState {
    return {
      kind: 'ready',
      requestKey: snapshot.requestKey,
      result: snapshot.result,
      ...(snapshot.nextCursor === undefined ? {} : { nextCursor: snapshot.nextCursor }),
      ...(snapshot.accumulatedRows === undefined
        ? {}
        : { accumulatedRows: snapshot.accumulatedRows })
    }
  }

  return {
    state,
    readForBinding,
    preloadForBinding,
    cancelPreloadForBinding,
    clearWarmSnapshots,
    clear,
    start,
    stop
  }
}

function contentsReadTargetForBinding(
  binding: RowBinding | undefined
): ContentsReadTarget | undefined {
  const scope = contentsScopeForBinding(binding)

  if (scope === undefined) {
    return undefined
  }

  const policy = defaultContentsPolicy
  return {
    scope,
    policy,
    recursion: contentsRecursion,
    requestKey: contentsRequestKey(scope, policy, contentsRecursion)
  }
}

function contentsScopeForBinding(
  binding: RowBinding | undefined
): Parameters<LibraryContentsApi['read']>[0]['scope'] | undefined {
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

function contentsRequestKey(
  scope: NonNullable<Parameters<LibraryContentsApi['read']>[0]['scope']>,
  policy: ContentsReadPolicy,
  recursion: ContentsRecursion
): string {
  const policyKey = `${policy.rowProfile.kind}:${policy.mediaClasses.join(',')}:${recursion}`

  switch (scope.kind) {
    case 'source':
      return `source:${scope.sourceId}:${policyKey}`
    case 'sourceLocation':
      return `source-location:${scope.sourceLocationId}:${policyKey}`
    case 'directory':
      return `directory:${scope.sourceId}:${scope.sourceDirectoryId}:${policyKey}`
  }
}
