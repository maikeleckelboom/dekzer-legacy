import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import { readSearchFilter } from '../boundary/searchFilterRead'
import {
  projectSearchFilterResultRows,
  type SearchResultViewModel
} from '../searchFilter/projection'
import type {
  SearchFilterQueryIdentity,
  SearchFilterReadRequest,
  SearchFilterReadResult,
  SearchFilterResult,
  SearchFilterResultRow,
  SearchFilterState
} from '../../../shared/library/searchFilter/read'

export type QueryId = string & { readonly __brand: 'SearchQueryId' }
export type RequestToken = number
export type SearchCursor = string
export type IndexGeneration = string
export type ResultState = SearchFilterState
export type ResultDetail = string

export type RetainedSnapshot = {
  readonly queryId: QueryId
  readonly rows: readonly SearchFilterResultRow[]
  readonly nextCursor: SearchCursor | null
  readonly indexGeneration: IndexGeneration
  readonly resultState: ResultState
  readonly resultDetail?: ResultDetail
}

export type SearchQueryState =
  | { readonly kind: 'Idle' }
  | {
      readonly kind: 'Pending'
      readonly queryId: QueryId
      readonly requestToken: RequestToken
      readonly priorRetained?: RetainedSnapshot
    }
  | {
      readonly kind: 'Retained'
      readonly queryId: QueryId
      readonly rows: readonly SearchFilterResultRow[]
      readonly nextCursor: SearchCursor | null
      readonly indexGeneration: IndexGeneration
      readonly resultState: ResultState
      readonly resultDetail?: ResultDetail
    }
  | {
      readonly kind: 'Accumulating'
      readonly queryId: QueryId
      readonly accumulated: readonly SearchFilterResultRow[]
      readonly nextCursor: SearchCursor
      readonly indexGeneration: IndexGeneration
      readonly requestToken: RequestToken
    }

export type SearchFilterReadApi = {
  readonly read: (request: SearchFilterReadRequest) => Promise<SearchFilterReadResult>
}

export type SearchFilterReadController = {
  readonly state: Ref<SearchQueryState>
  readonly viewModels: ComputedRef<readonly SearchResultViewModel[]>
  readonly submit: (request: SearchFilterReadRequest) => Promise<boolean>
  readonly loadNext: () => Promise<boolean>
  readonly invalidationSignal: () => Promise<boolean>
  readonly clear: () => void
  readonly start: () => void
  readonly stop: () => void
}

const defaultSearchFilterLimit = 100
const safeSearchFilterReadFailure = 'Unable to read library search/filter results.'

export function useSearchFilterRead(
  searchFilterApi: SearchFilterReadApi = { read: readSearchFilter }
): SearchFilterReadController {
  const controller = createSearchFilterReadController(searchFilterApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createSearchFilterReadController(
  searchFilterApi: SearchFilterReadApi
): SearchFilterReadController {
  const state = ref<SearchQueryState>({ kind: 'Idle' })
  const viewModels = computed(() => projectSearchFilterResultRows(currentRows(state.value)))
  let requestToken: RequestToken = 0
  let activeRequest: SearchFilterReadRequest | undefined
  let started = false

  function start(): void {
    started = true
  }

  function stop(): void {
    started = false
  }

  function clear(): void {
    activeRequest = undefined
    state.value = { kind: 'Idle' }
  }

  async function submit(request: SearchFilterReadRequest): Promise<boolean> {
    activeRequest = firstPageRequest(request)
    return issueFirstPageRead(activeRequest, retainedSnapshot(state.value))
  }

  async function loadNext(): Promise<boolean> {
    const currentState = state.value

    if (currentState.kind === 'Accumulating') {
      return false
    }

    if (currentState.kind !== 'Retained' || currentState.nextCursor === null) {
      return false
    }

    if (activeRequest === undefined) {
      return false
    }

    const token = nextRequestToken()
    const request = {
      ...firstPageRequest(activeRequest),
      cursor: currentState.nextCursor
    }
    state.value = {
      kind: 'Accumulating',
      queryId: currentState.queryId,
      accumulated: currentState.rows,
      nextCursor: currentState.nextCursor,
      indexGeneration: currentState.indexGeneration,
      requestToken: token
    }

    return issueRead(request, token)
  }

  async function invalidationSignal(): Promise<boolean> {
    if (activeRequest === undefined) {
      return true
    }

    return issueFirstPageRead(firstPageRequest(activeRequest), retainedSnapshot(state.value))
  }

  async function issueFirstPageRead(
    request: SearchFilterReadRequest,
    priorRetained: RetainedSnapshot | undefined
  ): Promise<boolean> {
    const token = nextRequestToken()
    state.value = {
      kind: 'Pending',
      queryId: createSearchQueryId(request),
      requestToken: token,
      ...(priorRetained === undefined ? {} : { priorRetained })
    }
    return issueRead(request, token)
  }

  async function issueRead(
    request: SearchFilterReadRequest,
    token: RequestToken
  ): Promise<boolean> {
    try {
      const result = await searchFilterApi.read(request)

      if (!started) {
        return false
      }

      if (result.state !== 'ready') {
        return handleReadFailure(token, result.error.message)
      }

      return receiveResponse(token, result.reply.result)
    } catch {
      if (!started) {
        return false
      }

      return handleReadFailure(token, safeSearchFilterReadFailure)
    }
  }

  function receiveResponse(token: RequestToken, result: SearchFilterResult): boolean {
    const currentState = state.value

    if (currentState.kind === 'Pending') {
      if (currentState.requestToken !== token) {
        return false
      }

      if (isCursorInvalidResult(result)) {
        void reissueAfterCursorInvalidation(currentState)
        return true
      }

      if (!queryIdentityMatchesRequest(result.queryIdentity, activeRequest)) {
        return false
      }

      state.value = retainedStateFromResult(
        createSearchQueryIdFromIdentity(result.queryIdentity),
        result
      )
      return true
    }

    if (currentState.kind === 'Accumulating') {
      if (currentState.requestToken !== token) {
        return false
      }

      if (isCursorInvalidResult(result)) {
        void reissueAfterCursorInvalidation(currentState)
        return true
      }

      if (!queryIdentityMatchesRequest(result.queryIdentity, activeRequest)) {
        return false
      }

      if (result.queryIdentity.indexGeneration !== currentState.indexGeneration) {
        void reissueAfterCursorInvalidation(currentState)
        return true
      }

      state.value = retainedStateFromResult(currentState.queryId, result, currentState.accumulated)
      return true
    }

    return false
  }

  function handleReadFailure(token: RequestToken, detail: string): boolean {
    const currentState = state.value

    if (currentState.kind === 'Pending' && currentState.requestToken === token) {
      if (currentState.priorRetained !== undefined) {
        state.value = retainedStateFromSnapshot(currentState.priorRetained, detail)
        return true
      }

      state.value = { kind: 'Idle' }
      return true
    }

    if (currentState.kind === 'Accumulating' && currentState.requestToken === token) {
      state.value = {
        kind: 'Retained',
        queryId: currentState.queryId,
        rows: currentState.accumulated,
        nextCursor: currentState.nextCursor,
        indexGeneration: currentState.indexGeneration,
        resultState: 'partial',
        resultDetail: detail
      }
      return true
    }

    return false
  }

  async function reissueAfterCursorInvalidation(
    invalidatedState: Extract<SearchQueryState, { readonly kind: 'Pending' | 'Accumulating' }>
  ): Promise<boolean> {
    if (activeRequest === undefined) {
      state.value = { kind: 'Idle' }
      return false
    }

    const priorRetained =
      invalidatedState.kind === 'Pending' ? invalidatedState.priorRetained : undefined
    return issueFirstPageRead(firstPageRequest(activeRequest), priorRetained)
  }

  function nextRequestToken(): RequestToken {
    requestToken += 1
    return requestToken
  }

  return {
    state,
    viewModels,
    submit,
    loadNext,
    invalidationSignal,
    clear,
    start,
    stop
  }
}

export function createSearchQueryId(request: SearchFilterReadRequest): QueryId {
  return stableStringify(searchQueryIdentityInput(request)) as QueryId
}

export function createSearchQueryIdFromIdentity(identity: SearchFilterQueryIdentity): QueryId {
  return stableStringify(searchQueryIdentityInputFromEcho(identity)) as QueryId
}

export function queryIdentityMatchesRequest(
  identity: SearchFilterQueryIdentity,
  request: SearchFilterReadRequest | undefined
): boolean {
  if (request === undefined) {
    return false
  }

  return (
    stableStringify(searchQueryIdentityInputFromEcho(identity)) ===
    stableStringify(searchQueryIdentityInput(request))
  )
}

function retainedStateFromResult(
  queryId: QueryId,
  result: SearchFilterResult,
  priorRows: readonly SearchFilterResultRow[] = []
): SearchQueryState {
  return {
    kind: 'Retained',
    queryId,
    rows: [...priorRows, ...result.rows],
    nextCursor: result.nextCursor ?? null,
    indexGeneration: result.indexGeneration,
    resultState: result.state,
    ...(result.detail === undefined ? {} : { resultDetail: result.detail })
  }
}

function retainedStateFromSnapshot(
  snapshot: RetainedSnapshot,
  resultDetail: string
): SearchQueryState {
  return {
    kind: 'Retained',
    queryId: snapshot.queryId,
    rows: snapshot.rows,
    nextCursor: snapshot.nextCursor,
    indexGeneration: snapshot.indexGeneration,
    resultState: snapshot.resultState,
    resultDetail
  }
}

function retainedSnapshot(state: SearchQueryState): RetainedSnapshot | undefined {
  if (state.kind !== 'Retained') {
    return undefined
  }

  return {
    queryId: state.queryId,
    rows: state.rows,
    nextCursor: state.nextCursor,
    indexGeneration: state.indexGeneration,
    resultState: state.resultState,
    ...(state.resultDetail === undefined ? {} : { resultDetail: state.resultDetail })
  }
}

function currentRows(state: SearchQueryState): readonly SearchFilterResultRow[] {
  if (state.kind === 'Retained') {
    return state.rows
  }

  return state.kind === 'Pending' ? (state.priorRetained?.rows ?? []) : []
}

function firstPageRequest(request: SearchFilterReadRequest): SearchFilterReadRequest {
  const firstPage: SearchFilterReadRequest = { ...request }
  delete firstPage.cursor
  return firstPage
}

function isCursorInvalidResult(result: SearchFilterResult): boolean {
  return result.state === 'cursorInvalid'
}

type SearchQueryIdentityInput = {
  readonly scope: SearchFilterReadRequest['scope']
  readonly recursion: SearchFilterReadRequest['recursion']
  readonly textQuery?: string
  readonly targetKinds: readonly string[]
  readonly filters: SearchFilterReadRequest['filters']
  readonly sort: SearchFilterReadRequest['sort']
  readonly pageSize: number
}

const defaultSearchFilterTargetKinds = [
  'source',
  'sourceLocation',
  'directory',
  'sourceFile'
] satisfies NonNullable<SearchFilterReadRequest['targetKinds']>

const searchFilterTargetKindStorageValues = {
  source: 'source',
  sourceLocation: 'source_location',
  directory: 'directory',
  sourceFile: 'source_file'
} satisfies Record<NonNullable<SearchFilterReadRequest['targetKinds']>[number], string>

const searchFilterFileClassStorageValues = {
  audio: 'audio',
  video: 'video',
  image: 'image',
  unsupported: 'unsupported',
  none: 'none'
} satisfies Record<NonNullable<SearchFilterReadRequest['filters']['fileClasses']>[number], string>

const searchFilterFileKindStorageValues = {
  audio: 'audio',
  video: 'video',
  image: 'image',
  cueSheet: 'cue_sheet',
  logDoc: 'log_doc',
  textDoc: 'text_doc',
  archive: 'archive',
  other: 'other',
  unknown: 'unknown'
} satisfies Record<NonNullable<SearchFilterReadRequest['filters']['fileKinds']>[number], string>

const searchFilterMediaRelevanceStorageValues = {
  audioWorkflow: 'audio_workflow',
  playableMedia: 'playable_media',
  explicitInventory: 'explicit_inventory',
  companionFile: 'companion_file',
  notMediaRelevant: 'not_media_relevant'
} satisfies Record<
  NonNullable<SearchFilterReadRequest['filters']['mediaRelevance']>[number],
  string
>

const searchFilterPresenceStateStorageValues = {
  present: 'present',
  missing: 'missing',
  removed: 'removed'
} satisfies Record<
  NonNullable<SearchFilterReadRequest['filters']['presenceStates']>[number],
  string
>

const searchFilterSourceAccessStateStorageValues = {
  accessible: 'accessible',
  missing: 'missing',
  blocked: 'blocked',
  unknown: 'unknown'
} satisfies Record<
  NonNullable<SearchFilterReadRequest['filters']['sourceAccessStates']>[number],
  string
>

const searchFilterAttachmentLinkStateStorageValues = {
  current: 'current',
  stale: 'stale',
  missing: 'missing',
  notApplicable: 'not_applicable'
} satisfies Record<
  NonNullable<SearchFilterReadRequest['filters']['attachmentLinkStates']>[number],
  string
>

function searchQueryIdentityInput(request: SearchFilterReadRequest): SearchQueryIdentityInput {
  const textQuery = normalizeTextQuery(request.textQuery)

  return {
    scope: request.scope,
    recursion: request.recursion,
    ...(textQuery === undefined ? {} : { textQuery }),
    targetKinds: canonicalTargetKinds(request.targetKinds),
    filters: canonicalFilters(request.filters),
    sort: request.sort,
    pageSize: request.limit ?? defaultSearchFilterLimit
  }
}

function searchQueryIdentityInputFromEcho(
  identity: SearchFilterQueryIdentity
): SearchQueryIdentityInput {
  const textQuery = normalizeTextQuery(identity.textQuery)

  return {
    scope: identity.scope,
    recursion: identity.recursion,
    ...(textQuery === undefined ? {} : { textQuery }),
    targetKinds: canonicalTargetKinds(identity.targetKinds),
    filters: canonicalFilters(identity.filters),
    sort: identity.sort,
    pageSize: identity.pageSize
  }
}

function normalizeTextQuery(query: string | undefined): string | undefined {
  const normalized = query?.trim().toLowerCase()
  return normalized === undefined || normalized === '' ? undefined : normalized
}

function canonicalTargetKinds(
  targetKinds: SearchFilterReadRequest['targetKinds']
): NonNullable<SearchFilterReadRequest['targetKinds']> {
  const kinds =
    targetKinds === undefined || targetKinds.length === 0
      ? defaultSearchFilterTargetKinds
      : targetKinds

  return sortAndDedupeByStorage(kinds, searchFilterTargetKindStorageValues)
}

function canonicalFilters(
  filters: SearchFilterReadRequest['filters']
): SearchFilterReadRequest['filters'] {
  const canonical: SearchFilterReadRequest['filters'] = {}
  const fileClasses = sortAndDedupeByStorage(
    filters.fileClasses ?? [],
    searchFilterFileClassStorageValues
  )
  const fileKinds = sortAndDedupeByStorage(
    filters.fileKinds ?? [],
    searchFilterFileKindStorageValues
  )
  const mediaRelevance = sortAndDedupeByStorage(
    filters.mediaRelevance ?? [],
    searchFilterMediaRelevanceStorageValues
  )
  const presenceStates = sortAndDedupeByStorage(
    filters.presenceStates ?? [],
    searchFilterPresenceStateStorageValues
  )
  const sourceAccessStates = sortAndDedupeByStorage(
    filters.sourceAccessStates ?? [],
    searchFilterSourceAccessStateStorageValues
  )
  const attachmentLinkStates = sortAndDedupeByStorage(
    filters.attachmentLinkStates ?? [],
    searchFilterAttachmentLinkStateStorageValues
  )

  if (fileClasses.length > 0) {
    canonical.fileClasses = fileClasses
  }
  if (fileKinds.length > 0) {
    canonical.fileKinds = fileKinds
  }
  if (mediaRelevance.length > 0) {
    canonical.mediaRelevance = mediaRelevance
  }
  if (presenceStates.length > 0) {
    canonical.presenceStates = presenceStates
  }
  if (sourceAccessStates.length > 0) {
    canonical.sourceAccessStates = sourceAccessStates
  }
  if (filters.blake3 !== undefined) {
    canonical.blake3 = filters.blake3
  }
  if (filters.probe !== undefined) {
    canonical.probe = filters.probe
  }
  if (attachmentLinkStates.length > 0) {
    canonical.attachmentLinkStates = attachmentLinkStates
  }

  return canonical
}

function sortAndDedupeByStorage<Value extends string>(
  values: readonly Value[],
  storageValues: Record<Value, string>
): Value[] {
  return [...new Set(values)].sort((left, right) =>
    storageValues[left].localeCompare(storageValues[right])
  )
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) {
    const items = value.map(stableStringify)
    const primitiveItemsOnly = value.every(
      (item) => item === null || ['boolean', 'number', 'string'].includes(typeof item)
    )
    return `[${(primitiveItemsOnly ? items.sort() : items).join(',')}]`
  }

  if (value !== null && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>)
      .filter(([, entryValue]) => entryValue !== undefined)
      .sort(([left], [right]) => left.localeCompare(right))

    return `{${entries
      .map(([key, entryValue]) => `${JSON.stringify(key)}:${stableStringify(entryValue)}`)
      .join(',')}}`
  }

  return JSON.stringify(value)
}
