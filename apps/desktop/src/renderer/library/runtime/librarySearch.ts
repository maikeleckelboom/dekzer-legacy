import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'

import {
  defaultLibraryBrowseProfile,
  type LibraryBrowseProfile
} from '../libraryBrowseProfile/types'
import type { RowBinding } from '../state'
import type { SearchFilterReadController } from './searchFilterState'
import type {
  SearchFilterReadRequest,
  SearchFilterScope
} from '../../../shared/library/searchFilter/read'

export type LibrarySearchController = {
  readonly query: Ref<string>
  readonly searchText: Ref<string>
  readonly activeQuery: Ref<string>
  readonly searchOpen: Ref<boolean>
  readonly searchActive: ComputedRef<boolean>
  readonly openSearch: () => void
  readonly submitSearch: (query?: string) => Promise<boolean>
  readonly clearSearch: (options?: { readonly close?: boolean }) => void
  readonly handleEscape: () => void
  readonly dispose: () => void
}

export type LibrarySearchReadPort = Pick<SearchFilterReadController, 'submit' | 'clear'>

export type LibrarySearchControllerOptions = {
  readonly profile?: Ref<LibraryBrowseProfile>
  readonly scope?: Ref<SearchFilterScope>
  readonly searchFilterRead: LibrarySearchReadPort
  readonly debounceMs?: number
}

const defaultSearchDebounceMs = 250
const searchLimit = 100

export function createLibrarySearchController(
  options: LibrarySearchControllerOptions
): LibrarySearchController {
  const profile = options.profile ?? ref<LibraryBrowseProfile>(defaultLibraryBrowseProfile)
  const scope = options.scope ?? ref<SearchFilterScope>(librarySearchScope())
  const debounceMs = options.debounceMs ?? defaultSearchDebounceMs
  const searchText = ref('')
  const activeQuery = ref('')
  const searchOpen = ref(false)
  const searchActive = computed(() => activeQuery.value.length > 0)
  let debounceTimer: ReturnType<typeof setTimeout> | undefined
  let suppressSearchWatch = false

  function openSearch(): void {
    searchOpen.value = true
  }

  async function submitSearch(query: string = activeQuery.value): Promise<boolean> {
    const trimmedQuery = normalizeQuery(query)

    if (trimmedQuery.length === 0) {
      options.searchFilterRead.clear()
      return false
    }

    activeQuery.value = trimmedQuery
    return options.searchFilterRead.submit(
      createLibrarySearchRequest(trimmedQuery, profile.value, scope.value)
    )
  }

  function clearSearch(clearOptions: { readonly close?: boolean } = {}): void {
    clearDebounce()
    suppressSearchWatch = true
    searchText.value = ''
    suppressSearchWatch = false
    activeQuery.value = ''
    options.searchFilterRead.clear()

    if (clearOptions.close ?? true) {
      searchOpen.value = false
    }
  }

  function handleEscape(): void {
    if (normalizeQuery(searchText.value).length === 0) {
      clearSearch({ close: true })
      return
    }

    clearSearch({ close: false })
  }

  function scheduleSearch(): void {
    if (suppressSearchWatch) {
      return
    }

    clearDebounce()

    const trimmedQuery = normalizeQuery(searchText.value)
    activeQuery.value = trimmedQuery

    if (trimmedQuery.length === 0) {
      options.searchFilterRead.clear()
      return
    }

    debounceTimer = setTimeout(() => {
      debounceTimer = undefined
      void submitSearch(trimmedQuery)
    }, debounceMs)
  }

  function refreshActiveSearch(): void {
    if (activeQuery.value.length === 0) {
      return
    }

    clearDebounce()
    void submitSearch(activeQuery.value)
  }

  function clearDebounce(): void {
    if (debounceTimer === undefined) {
      return
    }

    clearTimeout(debounceTimer)
    debounceTimer = undefined
  }

  const stopSearchWatch = watch(searchText, scheduleSearch, { flush: 'sync' })
  const stopProfileWatch = watch(profile, refreshActiveSearch, { flush: 'sync' })
  const stopScopeWatch = watch(() => librarySearchScopeKey(scope.value), refreshActiveSearch, {
    flush: 'sync'
  })

  function dispose(): void {
    clearDebounce()
    stopSearchWatch()
    stopProfileWatch()
    stopScopeWatch()
  }

  return {
    query: searchText,
    searchText,
    activeQuery,
    searchOpen,
    searchActive,
    openSearch,
    submitSearch,
    clearSearch,
    handleEscape,
    dispose
  }
}

export function createLibrarySearchRequest(
  query: string,
  profile: LibraryBrowseProfile,
  scope: SearchFilterScope = librarySearchScope()
): SearchFilterReadRequest {
  return {
    scope,
    recursion: 'recursive',
    textQuery: query,
    targetKinds: ['sourceFile'],
    filters: searchFiltersForProfile(profile),
    sort: 'relevance',
    limit: searchLimit
  }
}

export function librarySearchScopeForBinding(binding: RowBinding | undefined): SearchFilterScope {
  if (binding === undefined) {
    return librarySearchScope()
  }

  switch (binding.kind) {
    case 'source':
      if (binding.target.entryPoint.kind === 'source') {
        return {
          type: 'source',
          payload: {
            sourceId: binding.target.entryPoint.sourceId
          }
        }
      }

      return {
        type: 'sourceLocation',
        payload: {
          sourceLocationId: binding.target.entryPoint.sourceLocationId
        }
      }
    case 'directory':
      return {
        type: 'directory',
        payload: {
          sourceId: binding.sourceId,
          sourceDirectoryId: binding.directoryId
        }
      }
    case 'file':
    case 'navigation':
    case 'readState':
    case 'more':
    case 'addSourceSection':
    case 'localBrowseEntryPoint':
    case 'localBrowseItem':
    case 'localBrowseMore':
      return librarySearchScope()
  }
}

function librarySearchScope(): Extract<SearchFilterScope, { readonly type: 'library' }> {
  return { type: 'library' }
}

function librarySearchScopeKey(scope: SearchFilterScope): string {
  switch (scope.type) {
    case 'library':
      return 'library'
    case 'source':
      return `source:${scope.payload.sourceId}`
    case 'sourceLocation':
      return `source-location:${scope.payload.sourceLocationId}`
    case 'directory':
      return `directory:${scope.payload.sourceId}:${scope.payload.sourceDirectoryId}`
  }
}

function searchFiltersForProfile(
  profile: LibraryBrowseProfile
): SearchFilterReadRequest['filters'] {
  switch (profile) {
    case 'audio':
      return { fileClasses: ['audio'] }
    case 'playable':
      return { fileClasses: ['audio', 'video'] }
    case 'allFiles':
      return { fileClasses: ['audio', 'video', 'image', 'unsupported'] }
  }
}

function normalizeQuery(query: string): string {
  return query.trim()
}
