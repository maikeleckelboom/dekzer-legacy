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

    expect(panel).toContain(':aria-label="toolbarModel.libraryBrowseProfile.label"')
    expect(panel).toContain(':title="toolbarModel.libraryBrowseProfile.title"')
    expect(panel).toContain('<Icon role="action.browseView" size="md" />')
    expect(panel).not.toContain('Browse profile:')
    expect(panel).toContain('role="listbox"')
    expect(panel).toContain(':aria-selected="libraryBrowseProfile.profile.value === option.key"')
    expect(panel).toContain(':aria-label="toolbarModel.addSourceView.label"')
  })

  it('keeps library search control compact, icon-first, and accessible', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain('border border-(--color-text-muted) bg-(--color-background)')
    expect(panel).toContain('focus-visible:border-(--color-accent)')
    expect(panel).toContain(':aria-label="toolbarModel.search.label"')
    expect(panel).toContain(':title="toolbarModel.search.title"')
    expect(panel).toContain('<Icon role="action.search" size="md" />')
    expect(panel).toContain(':placeholder="toolbarModel.search.placeholder"')
    expect(panel).toContain('@keydown.escape.stop.prevent="handleSearchEscape"')
    expect(panel).toContain('aria-label="Clear search"')
    expect(panel).toContain('@click="clearSearchAndReturnToScope"')
    expect(panel).toContain("action.kind === 'loadSearchPage'")
    expect(panel).toContain('searchFilterRead.loadNext()')
    expect(panel).not.toContain('Search:')
  })

  it('keeps Add Source product states out of panel conditionals', () => {
    const panel = readRendererSource('panel.vue')
    const contents = readRendererSource('contents/projection.ts')

    expect(panel).not.toContain('Choose where your music lives')
    expect(panel).not.toContain('Choose a specific folder inside this drive')
    expect(contents).toContain('projectAddSourceProjection')
  })

  it('renders explicit workstation surfaces and source-added handoff wiring', () => {
    const panel = readRendererSource('panel.vue')
    const table = readRendererSource('contents/table.vue')

    expect(panel).toContain('const activeSurfaceTitle = computed')
    expect(panel).toContain("activeSurface.value === 'addSource' ? 'Add Source' : 'Library Browse'")
    expect(panel).toContain('sourceAdmissionHandoffFromRoot(root)')
    expect(panel).toContain('projectSourceAdmissionHandoff({')
    expect(panel).toContain(':source-admission-handoff="sourceAdmissionHandoffView"')
    expect(panel).toContain("action.kind === 'viewSource'")
    expect(panel).toContain('showAdmittedSource(action.sourceId)')
    expect(panel).toContain("action.kind === 'scanSource'")
    expect(panel).toContain('rootLifecycle.scanRoot(action.sourceId)')
    expect(panel).toContain("action.kind === 'keepBrowsing'")
    expect(panel).toContain('openAddSourceIntake()')
    expect(table).toContain('{{ sourceAdmissionHandoff.title }}')
    expect(table).toContain('sourceAdmissionHandoff.actions')
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
