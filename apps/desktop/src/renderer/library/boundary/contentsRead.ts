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
  readonly cursor?: string
}

export type ContentsReadController = {
  readonly state: Ref<ContentsBoundaryState>
  readonly readForBinding: (
    binding: RowBinding | undefined,
    options?: ReadOptions
  ) => Promise<boolean>
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
const safeContentsRequestFailure = 'Unable to request library contents.'
const defaultContentsPolicy: ContentsReadPolicy = {
  mediaClasses: ['audio'],
  rowProfile: { kind: 'sourceFile' }
}
const contentsRecursion: ContentsRecursion = 'recursive'

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
  let loadingTimer: ReturnType<typeof setTimeout> | undefined = undefined

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
    clearLoadingTimer()
  }

  function clear(): void {
    clearLoadingTimer()
    state.value = {
      kind: 'idle',
      detail: 'No contents scope is active.'
    }
  }

  async function readForBinding(
    binding: RowBinding | undefined,
    options: ReadOptions = {}
  ): Promise<boolean> {
    const scope = contentsScopeForBinding(binding)

    if (scope === undefined) {
      clear()
      return false
    }

    const policy = defaultContentsPolicy
    const requestKey = contentsRequestKey(scope, policy, contentsRecursion)
    const currentState = state.value
    const cursor = options.cursor

    if (!options.force && !cursor && currentRequestKey(currentState) === requestKey) {
      return false
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
      ...(cursor === undefined ? {} : { cursor })
    }
    startPendingRead(pending)

    try {
      const result = await contentsApi.read({
        scope,
        policy,
        recursion: contentsRecursion,
        limit: readLimit,
        ...(cursor === undefined ? {} : { cursor })
      })

      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      clearLoadingTimer()
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
      return true
    } catch {
      if (!started || !isCurrentLoading(requestKey, sequence)) {
        return false
      }

      clearLoadingTimer()
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

  function startPendingRead(pending: ContentsPendingRead): void {
    clearLoadingTimer()

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
      return
    }

    state.value = {
      kind: 'idle',
      pending,
      detail: 'Contents request is pending.'
    }

    loadingTimer = setTimeout(() => {
      if (!started || !isCurrentLoading(pending.requestKey, pending.sequence)) {
        return
      }

      state.value = {
        kind: 'loading',
        requestKey: pending.requestKey,
        sequence: pending.sequence,
        detail: pending.detail
      }
      loadingTimer = undefined
    }, loadingThresholdMs)
  }

  function clearLoadingTimer(): void {
    if (loadingTimer === undefined) {
      return
    }

    clearTimeout(loadingTimer)
    loadingTimer = undefined
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
        detail: currentState.detail ?? 'Loading contents.'
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

  return {
    state,
    readForBinding,
    clear,
    start,
    stop
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
