import { describe, expect, it } from 'vitest'

import {
  createViewModeController,
  defaultViewMode,
  parseViewMode,
  viewModeLabel
} from '../../../../src/renderer/library/viewMode/model'

describe('library view mode model', () => {
  it('defaults to List view', () => {
    const view = createViewModeController()

    expect(defaultViewMode).toBe('list')
    expect(view.view.value).toBe('list')
    expect(viewModeLabel(view.view.value)).toBe('List')
  })

  it('falls back to List for unsupported restored values', () => {
    const view = createViewModeController('columns')

    expect(parseViewMode('grid')).toBe('list')
    expect(view.restoreView('grid')).toBe(true)
    expect(view.view.value).toBe('list')
  })

  it('switches renderer form without owning contents membership', () => {
    const view = createViewModeController()
    const membership = ['row-1', 'row-2']

    expect(view.setView('columns')).toBe(true)

    expect(view.view.value).toBe('columns')
    expect(membership).toEqual(['row-1', 'row-2'])
  })
})
