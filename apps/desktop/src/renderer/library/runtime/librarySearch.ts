import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'

import { defaultProfile, type ProfileKey } from '../browseProfile/types'
import type { SearchFilterReadController } from './searchFilterState'
import type { SearchFilterReadRequest } from '../../../shared/library/searchFilter/read'

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
  readonly profile?: Ref<ProfileKey>
  readonly searchFilterRead: LibrarySearchReadPort
  readonly debounceMs?: number
}

const defaultSearchDebounceMs = 250
const searchLimit = 100

export function createLibrarySearchController(
  options: LibrarySearchControllerOptions
): LibrarySearchController {
  const profile = options.profile ?? ref<ProfileKey>(defaultProfile)
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
    return options.searchFilterRead.submit(createLibrarySearchRequest(trimmedQuery, profile.value))
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

  function dispose(): void {
    clearDebounce()
    stopSearchWatch()
    stopProfileWatch()
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
  profile: ProfileKey
): SearchFilterReadRequest {
  return {
    scope: { type: 'library' },
    recursion: 'recursive',
    textQuery: query,
    targetKinds: ['sourceFile'],
    filters: searchFiltersForProfile(profile),
    sort: 'relevance',
    limit: searchLimit
  }
}

function searchFiltersForProfile(profile: ProfileKey): SearchFilterReadRequest['filters'] {
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
