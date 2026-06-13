import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import {
  createLibrarySearchController,
  createLibrarySearchRequest
} from '../../../../src/renderer/library/runtime/librarySearch'
import type { ProfileKey } from '../../../../src/renderer/library/browseProfile/types'
import type { SearchFilterReadRequest } from '../../../../src/shared/library/searchFilter/read'

describe('library search controller', () => {
  it('defaults closed and opens the search input state', () => {
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read })

    expect(search.searchOpen.value).toBe(false)
    expect(search.searchText.value).toBe('')
    expect(search.searchActive.value).toBe(false)

    search.openSearch()

    expect(search.searchOpen.value).toBe(true)
    search.dispose()
  })

  it('does not submit backend search for an empty query', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read, debounceMs: 250 })

    search.searchText.value = '   '
    vi.advanceTimersByTime(300)

    expect(read.submitted).toEqual([])
    expect(read.clearCount).toBe(1)
    expect(search.searchActive.value).toBe(false)

    search.dispose()
    vi.useRealTimers()
  })

  it('submits a non-empty query after debounce', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read, debounceMs: 250 })

    search.searchText.value = 'am'
    vi.advanceTimersByTime(249)
    expect(read.submitted).toEqual([])

    vi.advanceTimersByTime(1)
    expect(read.submitted).toEqual([createLibrarySearchRequest('am', 'audio')])

    search.dispose()
    vi.useRealTimers()
  })

  it('debounces typing instead of submitting each keystroke', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read, debounceMs: 250 })

    search.searchText.value = 'a'
    vi.advanceTimersByTime(100)
    search.searchText.value = 'am'
    vi.advanceTimersByTime(100)
    search.searchText.value = 'ame'
    vi.advanceTimersByTime(249)

    expect(read.submitted).toEqual([])

    vi.advanceTimersByTime(1)
    expect(read.submitted).toEqual([createLibrarySearchRequest('ame', 'audio')])

    search.dispose()
    vi.useRealTimers()
  })

  it('clear restores selected contents mode by clearing active search state', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read, debounceMs: 250 })

    search.searchText.value = 'amen'
    vi.advanceTimersByTime(250)
    expect(search.searchActive.value).toBe(true)

    search.clearSearch()

    expect(search.searchActive.value).toBe(false)
    expect(search.searchOpen.value).toBe(false)
    expect(read.clearCount).toBe(1)

    search.dispose()
    vi.useRealTimers()
  })

  it('escape clears text first, then closes an empty search', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const search = createLibrarySearchController({ searchFilterRead: read, debounceMs: 250 })

    search.openSearch()
    search.searchText.value = 'amen'
    search.handleEscape()

    expect(search.searchText.value).toBe('')
    expect(search.searchOpen.value).toBe(true)
    expect(search.searchActive.value).toBe(false)

    search.handleEscape()

    expect(search.searchOpen.value).toBe(false)

    search.dispose()
    vi.useRealTimers()
  })

  it('refreshes active search when browse profile changes', () => {
    vi.useFakeTimers()
    const read = searchReadSpy()
    const profile = ref<ProfileKey>('audio')
    const search = createLibrarySearchController({
      searchFilterRead: read,
      profile,
      debounceMs: 250
    })

    search.searchText.value = 'amen'
    vi.advanceTimersByTime(250)
    profile.value = 'playable'

    expect(read.submitted).toEqual([
      createLibrarySearchRequest('amen', 'audio'),
      createLibrarySearchRequest('amen', 'playable')
    ])

    search.dispose()
    vi.useRealTimers()
  })

  it('uses library-wide indexed source-file search and excludes local browse paths', () => {
    expect(createLibrarySearchRequest('amen', 'allFiles')).toEqual({
      scope: { type: 'library' },
      recursion: 'recursive',
      textQuery: 'amen',
      targetKinds: ['sourceFile'],
      filters: { fileClasses: ['audio', 'video', 'image', 'unsupported'] },
      sort: 'relevance',
      limit: 100
    })
  })
})

function searchReadSpy(): {
  readonly submitted: SearchFilterReadRequest[]
  readonly clearCount: number
  readonly submit: (request: SearchFilterReadRequest) => Promise<boolean>
  readonly clear: () => void
} {
  const spy = {
    submitted: [] as SearchFilterReadRequest[],
    clearCount: 0,
    async submit(request: SearchFilterReadRequest): Promise<boolean> {
      spy.submitted.push(request)
      return true
    },
    clear(): void {
      spy.clearCount += 1
    }
  }

  return spy
}
