import { describe, expect, it } from 'vitest'
import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

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

  it('counts visible source locations as Library Home roots', () => {
    const projection = projectLibraryHome({
      state: browserState({
        sourceReadinessByNodeId: new Map([
          [
            'navigation-row:9',
            {
              kind: 'ready',
              sourceNodeId: 'navigation-row:9',
              detail: 'The source location hierarchy is ready.'
            }
          ]
        ])
      }),
      bindingsById: sourceLocationProjection(['9']).bindingsById
    })

    expect(projection.productState).toBe('ready')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Ready'
      })
    ])
    expect(projection.rows.some((row) => row.action?.kind === 'chooseMusicFolder')).toBe(false)
  })

  it('uses profile-aware empty current view copy when all roots are empty', () => {
    const projection = projectLibraryHome({
      state: browserState({
        libraryBrowseProfile: 'playable',
        sourceReadinessByNodeId: new Map([
          [
            'navigation-row:7',
            {
              kind: 'empty',
              sourceNodeId: 'navigation-row:7',
              detail: 'No visible playable media.'
            }
          ],
          [
            'navigation-row:8',
            {
              kind: 'empty',
              sourceNodeId: 'navigation-row:8',
              detail: 'No visible playable media.'
            }
          ]
        ])
      }),
      bindingsById: sourceProjection(['7', '8']).bindingsById
    })

    expect(projection.productState).toBe('emptyCurrentView')
    expect(projection.title).toBe('No playable media in this view.')
    expect(projection.detail).toBe('No playable media in this view.')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'No playable media in this view',
        detail: '2 sources. No playable media in this view.'
      })
    ])
  })

  it('keeps hard availability states ahead of ready and empty roots', () => {
    const hardStates = [
      { readiness: 'missing', expected: 'missing' },
      { readiness: 'blocked', expected: 'blocked' },
      { readiness: 'unavailable', expected: 'unavailable' }
    ] as const

    for (const { readiness, expected } of hardStates) {
      const projection = projectLibraryHome({
        state: browserState({
          sourceReadinessByNodeId: new Map([
            [
              'navigation-row:7',
              {
                kind: readiness,
                sourceNodeId: 'navigation-row:7',
                detail: 'Hard source state.'
              }
            ],
            [
              'navigation-row:8',
              {
                kind: 'empty',
                sourceNodeId: 'navigation-row:8',
                detail: 'No visible rows.'
              }
            ],
            [
              'navigation-row:9',
              {
                kind: 'ready',
                sourceNodeId: 'navigation-row:9',
                detail: 'Ready.'
              }
            ]
          ])
        }),
        bindingsById: sourceProjection(['7', '8', '9']).bindingsById
      })

      expect(projection.productState).toBe(expected)
    }
  })

  it('makes maintenanceNeeded reachable from real maintenance input', () => {
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
      bindingsById: sourceProjection(['7']).bindingsById,
      sourceMaintenanceBySourceId: new Map([
        [
          '7',
          maintenance({
            remainingHashCandidates: 2
          })
        ]
      ])
    })

    expect(projection.productState).toBe('maintenanceNeeded')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Maintenance needed',
        detail: 'Run maintenance to finish preparing music.'
      })
    ])
  })

  it('does not let maintenance backlog beat harder source failures', () => {
    const projection = projectLibraryHome({
      state: browserState({
        sourceReadinessByNodeId: new Map([
          [
            'navigation-row:7',
            {
              kind: 'missing',
              sourceNodeId: 'navigation-row:7',
              detail: 'Missing.'
            }
          ]
        ])
      }),
      bindingsById: sourceProjection(['7']).bindingsById,
      sourceIntegrityBySourceId: new Map([
        [
          '7',
          integrity({
            remainingProbeCandidates: 1
          })
        ]
      ])
    })

    expect(projection.productState).toBe('missing')
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
    readonly libraryBrowseProfile?: BrowserState['libraryBrowseProfile']
  } = {}
): BrowserState {
  return {
    navigationReadResult: {
      state: 'ready',
      rows: []
    },
    sourceReadStates: new Map(),
    directoryReadStates: new Map(),
    ...(options.libraryBrowseProfile === undefined
      ? {}
      : { libraryBrowseProfile: options.libraryBrowseProfile }),
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

function sourceLocationProjection(sourceLocationIds: readonly string[]): BrowserProjection {
  return {
    kind: 'tree',
    nodes: sourceLocationIds.map(sourceNode),
    bindingsById: new Map(
      sourceLocationIds.map((sourceLocationId) => [
        `navigation-row:${sourceLocationId}`,
        sourceLocationBinding(sourceLocationId)
      ])
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

function sourceLocationBinding(sourceLocationId: string): RowBinding {
  return {
    kind: 'source',
    navigationRow: {
      navigationRowId: sourceLocationId,
      stableKey: `source-location:${sourceLocationId}`,
      parentNavigationRowId: null,
      family: 'sources',
      rowKind: 'location',
      displayName: `Location ${sourceLocationId}`,
      siblingPosition: 0,
      selectable: true,
      selectorKind: 'sourceLocation',
      selectorPayload: sourceLocationId,
      updatedAtMs: 100,
      rowVersion: '1'
    },
    target: {
      navigationRowId: sourceLocationId,
      entryPoint: {
        kind: 'sourceLocation',
        sourceLocationId
      },
      label: `Location ${sourceLocationId}`
    }
  }
}

function maintenance(
  overrides: Partial<ReadSourceMaintenanceReply> = {}
): ReadSourceMaintenanceReply {
  return {
    sourceId: '7',
    status: 'idle',
    remainingHashCandidates: 0,
    remainingProbeCandidates: 0,
    remainingPlayableMediaPromotionCandidates: 0,
    remainingTrackIdentityCandidateProductionCandidates: 0,
    remainingTrackIdentityDecisionProductionCandidates: 0,
    ...overrides
  }
}

function integrity(
  overrides: Partial<ReadSourceIntegrityReply['evidenceAndMaintenance']> = {}
): ReadSourceIntegrityReply {
  return {
    sourceId: '7',
    sourceAvailability: {
      state: 'mounted'
    },
    coverageIntegrity: {
      state: 'complete',
      subtreeCoverageComplete: true,
      emptyResultAuthoritative: true,
      totalDirectoriesCount: 1,
      missingDirectoriesCount: 0,
      pendingDirectoriesCount: 0,
      scanningDirectoriesCount: 0,
      blockedDirectoriesCount: 0,
      failedDirectoriesCount: 0
    },
    evidenceAndMaintenance: {
      remainingHashCandidates: 0,
      remainingProbeCandidates: 0,
      remainingPlayableMediaPromotionCandidates: 0,
      remainingTrackIdentityCandidateProductionCandidates: 0,
      remainingTrackIdentityDecisionProductionCandidates: 0,
      ...overrides
    },
    runtimeMaintenance: {
      state: 'idle'
    }
  }
}
