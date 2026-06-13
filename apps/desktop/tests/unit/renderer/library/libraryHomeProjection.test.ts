import { describe, expect, it } from 'vitest'

import {
  libraryBrowseEmptyStateLabel,
  projectLibraryHome,
  projectLibrarySourceReadiness
} from '../../../../src/renderer/library/libraryHome/projection'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { BrowserState, RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'

describe('Library home projection', () => {
  it('shows a clear Add Source CTA when Library Browse has no admitted sources', () => {
    const projection = projectLibraryHome({
      state: browserState(),
      bindingsById: new Map()
    })

    expect(projection.productState).toBe('noSources')
    expect(projection.title).toBe('Library')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Add Source',
        action: {
          kind: 'chooseMusicFolder',
          label: 'Add Source'
        }
      })
    ])
  })

  it('summarizes admitted sources when nothing is selected', () => {
    const projection = projectLibraryHome({
      state: browserState({
        sourceReadinessByNodeId: new Map([
          [
            'navigation-row:7',
            {
              kind: 'ready',
              sourceNodeId: 'navigation-row:7',
              detail: 'The source hierarchy is ready.'
            }
          ]
        ])
      }),
      bindingsById: sourceProjection(['7']).bindingsById
    })

    expect(projection.productState).toBe('ready')
    expect(projection.title).toBe('Library ready')
    expect(projection.detail).toBe(
      'Your library is ready. Select a source, folder, or search your music.'
    )
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Ready',
        detail: 'Ready to browse.'
      })
    ])
  })

  it('maps source readiness to product-facing Library states', () => {
    expect(projectLibrarySourceReadiness({ sourceReadiness: undefined })).toMatchObject({
      productState: 'indexing',
      badge: 'Indexing'
    })
    expect(
      projectLibrarySourceReadiness({
        sourceReadiness: {
          kind: 'registered',
          sourceNodeId: 'navigation-row:7',
          detail: 'Registered.'
        }
      })
    ).toMatchObject({ productState: 'needsScan', badge: 'Needs scan' })
    expect(
      projectLibrarySourceReadiness({
        sourceReadiness: {
          kind: 'missing',
          sourceNodeId: 'navigation-row:7',
          detail: 'Missing.'
        }
      })
    ).toMatchObject({ productState: 'missing', badge: 'Missing' })
    expect(
      projectLibrarySourceReadiness({
        sourceReadiness: {
          kind: 'unavailable',
          sourceNodeId: 'navigation-row:7',
          detail: 'Unavailable.'
        }
      })
    ).toMatchObject({ productState: 'unavailable', badge: 'Offline/unavailable' })
    expect(
      projectLibrarySourceReadiness({
        sourceReadiness: {
          kind: 'empty',
          sourceNodeId: 'navigation-row:7',
          detail: 'Empty.'
        }
      })
    ).toMatchObject({
      productState: 'emptyCurrentView',
      badge: 'No audio tracks in this view'
    })
  })

  it('owns Library Browse profile empty copy outside Add Source', () => {
    expect(libraryBrowseEmptyStateLabel('audio')).toBe('No audio tracks in this view.')
    expect(libraryBrowseEmptyStateLabel('playable')).toBe('No playable media in this view.')
    expect(libraryBrowseEmptyStateLabel('allFiles')).toBe('No files in this source inventory view.')
  })
})

function browserState(
  options: {
    readonly sourceReadinessByNodeId?: BrowserState['sourceReadinessByNodeId']
  } = {}
): BrowserState {
  return {
    navigationReadResult: {
      state: 'ready',
      rows: []
    },
    sourceReadStates: new Map(),
    directoryReadStates: new Map(),
    ...(options.sourceReadinessByNodeId === undefined
      ? {}
      : { sourceReadinessByNodeId: options.sourceReadinessByNodeId })
  }
}

function sourceProjection(sourceIds: readonly string[]): BrowserProjection {
  return {
    kind: 'tree',
    nodes: sourceIds.map(sourceNode),
    bindingsById: new Map(
      sourceIds.map((sourceId) => [`navigation-row:${sourceId}`, sourceBinding(sourceId)])
    )
  }
}

function sourceNode(sourceId: string): BrowserTreeNode {
  return {
    id: `navigation-row:${sourceId}`,
    role: 'source',
    label: `Source ${sourceId}`,
    children: { kind: 'none' }
  }
}

function sourceBinding(sourceId: string): RowBinding {
  return {
    kind: 'source',
    navigationRow: {
      navigationRowId: sourceId,
      stableKey: `source:${sourceId}`,
      parentNavigationRowId: null,
      family: 'sources',
      rowKind: 'source',
      displayName: `Source ${sourceId}`,
      siblingPosition: 0,
      selectable: true,
      selectorKind: 'source',
      selectorPayload: sourceId,
      updatedAtMs: 100,
      rowVersion: '1'
    },
    target: {
      navigationRowId: sourceId,
      entryPoint: {
        kind: 'source',
        sourceId
      },
      label: `Source ${sourceId}`
    }
  }
}
