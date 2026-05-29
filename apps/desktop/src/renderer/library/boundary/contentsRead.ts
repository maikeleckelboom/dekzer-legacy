import { onMounted, onUnmounted, ref } from 'vue'
import type { Ref } from 'vue'

import type {
  ContentsReadPolicy,
  ContentsReadResult,
  ContentsRecursion,
  ContentsFileRow
} from '../../../shared/libraryContents/read'
import type { SourceFileVisibility } from '../../../shared/libraryHierarchy/readChildren'
import type { RendererApi } from '../../../shared/rendererApi'
import type { RowBinding } from '../state'

export type ContentsBoundaryState =
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
      readonly result: ContentsReadResult
      readonly nextCursor?: string
      readonly accumulatedRows?: readonly ContentsFileRow[]
    }
  | {
      readonly kind: 'failed'
      readonly requestKey: string
      readonly detail: string
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
const safeContentsRequestFailure = 'Unable to request library contents.'
const defaultContentsPolicy: ContentsReadPolicy = {
  mediaClasses: ['audio', 'video'],
  rowProfile: { kind: 'primaryMedia' }
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
  let accumulatedRows: readonly ContentsFileRow[] | undefined = undefined

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
  }

  function clear(): void {
    state.value = {
      kind: 'idle',
      detail: 'No contents scope is active.'
    }
    accumulatedRows = undefined
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

    if (
      !options.force &&
      !cursor &&
      (currentState.kind === 'loading' || currentState.kind === 'ready') &&
      currentState.requestKey === requestKey
    ) {
      return false
    }

    const isSameRequest =
      currentState.kind === 'ready' &&
      currentState.requestKey === requestKey &&
      cursor !== undefined

    const sequence = ++readSequence
    state.value = {
      kind: 'loading',
      requestKey,
      sequence,
      detail: options.cursor !== undefined ? 'Loading more contents.' : 'Loading contents.'
    }

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

      let rows: readonly ContentsFileRow[]
      const readyResult = result.state === 'ready' ? result.result : undefined
      if (isSameRequest && accumulatedRows !== undefined && readyResult !== undefined) {
        rows = [...accumulatedRows, ...(readyResult.rows ?? [])]
      } else if (readyResult !== undefined) {
        rows = readyResult.rows ?? []
      } else {
        rows = []
      }

      const nextCursor = readyResult?.nextCursor
      if (result.state === 'ready') {
        accumulatedRows = nextCursor !== undefined ? rows : undefined
      } else {
        accumulatedRows = undefined
      }

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

      state.value = {
        kind: 'failed',
        requestKey,
        detail: safeContentsRequestFailure
      }
      accumulatedRows = undefined
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

export function contentsPolicyForVisibility(
  sourceFileVisibility: SourceFileVisibility
): ContentsReadPolicy {
  switch (sourceFileVisibility) {
    case 'performance':
      return {
        mediaClasses: ['audio', 'video'],
        rowProfile: { kind: 'primaryMedia' }
      }
    case 'performanceAndImages':
      return {
        mediaClasses: ['audio', 'video', 'image'],
        rowProfile: { kind: 'sourceFile' }
      }
  }
}
