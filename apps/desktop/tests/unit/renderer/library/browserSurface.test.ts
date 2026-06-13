import { readFileSync } from 'node:fs'

import { describe, expect, it } from 'vitest'

describe('library browser surface containment', () => {
  it('owns panel and contents scrolling inside the library surface', () => {
    const panel = readRendererSource('panel.vue')
    const table = readRendererSource('contents/table.vue')

    expect(panel).toContain(
      'h-[80svh] min-h-0 flex-col overflow-hidden border border-(--color-border)'
    )
    expect(panel).toContain(
      'grid min-h-0 flex-1 grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)] overflow-hidden'
    )
    expect(table).toContain('min-h-0 flex-1 overflow-auto scrollbar-gutter-stable')
    expect(table).toContain(
      'sticky top-0 z-10 border-b border-(--color-border) bg-(--color-background)'
    )
  })

  it('keeps browse profile control icon-only and accessible in the header', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain('aria-label="Browse view"')
    expect(panel).toContain('Browse view: ${selectedBrowseProfileLabel}')
    expect(panel).toContain('<Icon role="action.browseView" size="md" />')
    expect(panel).not.toContain('Browse profile:')
    expect(panel).toContain('role="listbox"')
    expect(panel).toContain(':aria-selected="browseProfile.profile.value === option.key"')
  })

  it('keeps library search control compact, icon-first, and accessible', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain('aria-label="Search library"')
    expect(panel).toContain('title="Search library"')
    expect(panel).toContain('<Icon role="action.search" size="md" />')
    expect(panel).toContain('placeholder="Search library"')
    expect(panel).toContain('@keydown.escape.stop.prevent="handleSearchEscape"')
    expect(panel).toContain('aria-label="Clear search"')
    expect(panel).toContain("action.kind === 'loadSearchPage'")
    expect(panel).toContain('searchFilterRead.loadNext()')
    expect(panel).not.toContain('Search:')
  })
})

function readRendererSource(relativePath: string): string {
  return readFileSync(
    new URL(`../../../../src/renderer/library/${relativePath}`, import.meta.url),
    {
      encoding: 'utf8'
    }
  )
}
