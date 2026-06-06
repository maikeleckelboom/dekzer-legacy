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
})

function readRendererSource(relativePath: string): string {
  return readFileSync(
    new URL(`../../../../src/renderer/library/${relativePath}`, import.meta.url),
    {
      encoding: 'utf8'
    }
  )
}
