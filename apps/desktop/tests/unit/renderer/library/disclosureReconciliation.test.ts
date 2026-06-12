import { describe, expect, it, vi } from 'vitest'

import { createDisclosureReconciler } from '../../../../src/renderer/library/runtime/disclosureReconciliation'
import type {
  BrowserState,
  LoadedChildren,
  RowBinding
} from '../../../../src/renderer/library/state'
import { flattenVisibleTree } from '../../../../src/renderer/library/tree/listProjection'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNodeId } from '../../../../src/renderer/library/tree/types'
import type {
  ChildRow,
  EntryPoint,
  HierarchyCoverage
} from '../../../../src/shared/library/hierarchy/read'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/library/navigation/read'

describe('createDisclosureReconciler', () => {
  it('requests children for expanded source and directory bindings without loaded children', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const sourceReadStates = new Map([['navigation-row:7', { kind: 'unloaded' }]] as const)
    const directoryReadStates = new Map([['12', { kind: 'unloaded' }]] as const)

    const requested = reconciler.reconcile({
      projection: projectionWithBindings(
        sourceBinding('navigation-row:7'),
        directoryBinding('source-directory:12', '12')
      ),
      expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12']),
      sourceReadStates,
      directoryReadStates
    })

    expect(requested).toEqual(['navigation-row:7', 'source-directory:12'])
    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({ status: 'pending' })
    expect(reconciler.getLedgerEntry('source-directory:12')).toEqual({ status: 'pending' })
    expect(requestNodeChildren).toHaveBeenCalledWith('navigation-row:7')
    expect(requestNodeChildren).toHaveBeenCalledWith('source-directory:12')
  })

  it('clears pending ledger entries when expanded source and directory children load', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const projection = projectionWithBindings(
      sourceBinding('navigation-row:7'),
      directoryBinding('source-directory:12', '12')
    )
    const expandedNodeIds = new Set(['navigation-row:7', 'source-directory:12'])

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map([['12', { kind: 'unloaded' }]])
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({ status: 'pending' })
    expect(reconciler.getLedgerEntry('source-directory:12')).toEqual({ status: 'pending' })

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'loaded',
            children: loadedChildren([])
          }
        ]
      ]),
      directoryReadStates: new Map([
        [
          '12',
          {
            kind: 'loaded',
            children: loadedChildren([], { parentDirectoryId: '12' })
          }
        ]
      ])
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toBeUndefined()
    expect(reconciler.getLedgerEntry('source-directory:12')).toBeUndefined()
  })

  it('clears ledger entries for collapsed nodes', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const projection = projectionWithBindings(sourceBinding('navigation-row:7'))

    reconciler.reconcile({
      projection,
      expandedNodeIds: new Set(['navigation-row:7']),
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({ status: 'pending' })

    reconciler.reconcile({
      projection,
      expandedNodeIds: new Set(),
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toBeUndefined()
  })

  it('clears ledger entries when current bindings disappear', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const expandedNodeIds = new Set(['navigation-row:7'])

    reconciler.reconcile({
      projection: projectionWithBindings(sourceBinding('navigation-row:7')),
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({ status: 'pending' })

    reconciler.reconcile({
      projection: projectionWithBindings(),
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toBeUndefined()
  })

  it('transitions pending source and directory ledger entries to failed from read state', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren, now: () => 1234 })
    const projection = projectionWithBindings(
      sourceBinding('navigation-row:7'),
      directoryBinding('source-directory:12', '12')
    )
    const expandedNodeIds = new Set(['navigation-row:7', 'source-directory:12'])

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map([['12', { kind: 'unloaded' }]])
    })

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Source read failed.',
            errorCode: 'readFailed'
          }
        ]
      ]),
      directoryReadStates: new Map([
        [
          '12',
          {
            kind: 'failed',
            detail: 'Directory read failed.',
            errorCode: 'readFailed'
          }
        ]
      ])
    })

    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({
      status: 'failed',
      reason: 'Source read failed.',
      failedAt: 1234
    })
    expect(reconciler.getLedgerEntry('source-directory:12')).toEqual({
      status: 'failed',
      reason: 'Directory read failed.',
      failedAt: 1234
    })
  })

  it('suppresses automatic retries while a failed ledger entry exists', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren, now: () => 1234 })
    const projection = projectionWithBindings(sourceBinding('navigation-row:7'))
    const expandedNodeIds = new Set(['navigation-row:7'])

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })
    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Source read failed.',
            errorCode: 'readFailed'
          }
        ]
      ]),
      directoryReadStates: new Map()
    })

    requestNodeChildren.mockClear()

    const requested = reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Source read failed.',
            errorCode: 'readFailed'
          }
        ]
      ]),
      directoryReadStates: new Map()
    })

    expect(requested).toEqual([])
    expect(requestNodeChildren).not.toHaveBeenCalled()
  })

  it('clears only failed entries on scan-completion recovery', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren, now: () => 1234 })
    const projection = projectionWithBindings(
      sourceBinding('navigation-row:7'),
      sourceBinding('navigation-row:9')
    )
    const expandedNodeIds = new Set(['navigation-row:7', 'navigation-row:9'])

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        ['navigation-row:7', { kind: 'unloaded' }],
        ['navigation-row:9', { kind: 'unloaded' }]
      ]),
      directoryReadStates: new Map()
    })
    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Source read failed.',
            errorCode: 'readFailed'
          }
        ],
        ['navigation-row:9', { kind: 'unloaded' }]
      ]),
      directoryReadStates: new Map()
    })

    reconciler.clearFailed()

    expect(reconciler.getLedgerEntry('navigation-row:7')).toBeUndefined()
    expect(reconciler.getLedgerEntry('navigation-row:9')).toEqual({ status: 'pending' })
  })

  it('requests an expanded failed node again after failed entries are cleared', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren, now: () => 1234 })
    const projection = projectionWithBindings(sourceBinding('navigation-row:7'))
    const expandedNodeIds = new Set(['navigation-row:7'])
    const failedSourceStates = new Map([
      [
        'navigation-row:7',
        {
          kind: 'failed',
          detail: 'Source read failed.',
          errorCode: 'readFailed'
        }
      ]
    ] as const)

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([['navigation-row:7', { kind: 'unloaded' }]]),
      directoryReadStates: new Map()
    })
    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: failedSourceStates,
      directoryReadStates: new Map()
    })
    reconciler.clearFailed()
    requestNodeChildren.mockClear()

    const requested = reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: failedSourceStates,
      directoryReadStates: new Map()
    })

    expect(requested).toEqual(['navigation-row:7'])
    expect(requestNodeChildren).toHaveBeenCalledWith('navigation-row:7')
    expect(reconciler.getLedgerEntry('navigation-row:7')).toEqual({ status: 'pending' })
  })

  it('clears failed entries for explicit user retry without clearing pending entries', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren, now: () => 1234 })
    const projection = projectionWithBindings(
      sourceBinding('navigation-row:7'),
      sourceBinding('navigation-row:9')
    )
    const expandedNodeIds = new Set(['navigation-row:7', 'navigation-row:9'])

    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        ['navigation-row:7', { kind: 'unloaded' }],
        ['navigation-row:9', { kind: 'unloaded' }]
      ]),
      directoryReadStates: new Map()
    })
    reconciler.reconcile({
      projection,
      expandedNodeIds,
      sourceReadStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Source read failed.',
            errorCode: 'readFailed'
          }
        ],
        ['navigation-row:9', { kind: 'unloaded' }]
      ]),
      directoryReadStates: new Map()
    })

    reconciler.clearFailedForNode('navigation-row:7')
    reconciler.clearFailedForNode('navigation-row:9')

    expect(reconciler.getLedgerEntry('navigation-row:7')).toBeUndefined()
    expect(reconciler.getLedgerEntry('navigation-row:9')).toEqual({ status: 'pending' })
  })

  it('does nothing for expanded node IDs whose bindings are absent', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const state = browserState({
      sourceChildren: loadedChildren([])
    })

    const expandedNodeIds = new Set<BrowserTreeNodeId>(['source-directory:12'])

    reconciler.reconcile({
      projection: projectTree(state),
      expandedNodeIds,
      sourceReadStates: state.sourceReadStates,
      directoryReadStates: state.directoryReadStates
    })

    expect(expandedNodeIds).toEqual(new Set(['source-directory:12']))
    expect(requestNodeChildren).not.toHaveBeenCalled()
  })

  it('retries on a later projection pass when an expanded binding appears', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const expandedNodeIds = new Set<BrowserTreeNodeId>(['source-directory:12'])
    const absentState = browserState({
      sourceChildren: loadedChildren([])
    })

    reconciler.reconcile({
      projection: projectTree(absentState),
      expandedNodeIds,
      sourceReadStates: absentState.sourceReadStates,
      directoryReadStates: absentState.directoryReadStates
    })

    const presentState = browserState({
      sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
      directoryStates: new Map([['12', { kind: 'unloaded' }]])
    })

    reconciler.reconcile({
      projection: projectTree(presentState),
      expandedNodeIds,
      sourceReadStates: presentState.sourceReadStates,
      directoryReadStates: presentState.directoryReadStates
    })

    expect(requestNodeChildren).toHaveBeenCalledTimes(1)
    expect(requestNodeChildren).toHaveBeenCalledWith('source-directory:12')
  })

  it('does not request a collapsed descendant', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const state = browserState({
      sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
      directoryStates: new Map([['12', { kind: 'unloaded' }]])
    })

    reconciler.reconcile({
      projection: projectTree(state),
      expandedNodeIds: new Set(['navigation-row:7']),
      sourceReadStates: state.sourceReadStates,
      directoryReadStates: state.directoryReadStates
    })

    expect(requestNodeChildren).not.toHaveBeenCalled()
  })

  it('does not re-request loaded, loading, or refreshing children', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const state = browserState({
      sourceChildren: loadedChildren([
        directoryNode('12', 'Loading Album'),
        directoryNode('13', 'Refreshing Album'),
        directoryNode('14', 'Loaded Album')
      ]),
      directoryStates: new Map([
        [
          '12',
          {
            kind: 'loading',
            requestKey: 'source:7/directory:12',
            sequence: 1
          }
        ],
        [
          '13',
          {
            kind: 'refreshing',
            children: loadedChildren([], { parentDirectoryId: '13' }),
            requestKey: 'source:7/directory:13',
            sequence: 2
          }
        ],
        [
          '14',
          {
            kind: 'loaded',
            children: loadedChildren([], { parentDirectoryId: '14' })
          }
        ]
      ])
    })

    reconciler.reconcile({
      projection: projectTree(state),
      expandedNodeIds: new Set([
        'navigation-row:7',
        'source-directory:12',
        'source-directory:13',
        'source-directory:14'
      ]),
      sourceReadStates: state.sourceReadStates,
      directoryReadStates: state.directoryReadStates
    })

    expect(requestNodeChildren).not.toHaveBeenCalled()
  })

  it('keeps expanded descendants visually expanded after their refreshed binding still exists', () => {
    const requestNodeChildren = vi.fn()
    const reconciler = createDisclosureReconciler({ requestNodeChildren })
    const expandedNodeIds = new Set<BrowserTreeNodeId>(['navigation-row:7', 'source-directory:12'])
    const initialState = browserState({
      sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
      directoryStates: new Map([['12', { kind: 'unloaded' }]])
    })

    reconciler.reconcile({
      projection: projectTree(initialState),
      expandedNodeIds,
      sourceReadStates: initialState.sourceReadStates,
      directoryReadStates: initialState.directoryReadStates
    })

    const refreshedState = browserState({
      sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
      directoryStates: new Map([
        [
          '12',
          {
            kind: 'loaded',
            children: loadedChildren([directoryNode('99', 'Nested Album', '12')], {
              parentDirectoryId: '12'
            })
          }
        ]
      ])
    })
    const visibleItems = flattenVisibleTree({
      nodes: projectTree(refreshedState).nodes,
      expandedNodeIds
    })

    expect(requestNodeChildren).toHaveBeenCalledWith('source-directory:12')
    expect(visibleItems.find((item) => item.id === 'source-directory:12')).toMatchObject({
      isExpanded: true
    })
    expect(visibleItems.find((item) => item.id === 'source-directory:99')).toMatchObject({
      parentId: 'source-directory:12'
    })
  })
})

function projectTree(state: BrowserState): BrowserProjection {
  const projection = projectState(state)

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected tree projection.')
  }

  return projection
}

function projectionWithBindings(
  ...bindings: readonly (readonly [BrowserTreeNodeId, RowBinding])[]
): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [],
    bindingsById: new Map(bindings)
  }
}

function sourceBinding(nodeId: BrowserTreeNodeId): readonly [BrowserTreeNodeId, RowBinding] {
  const navigationRow = sourceNavigationRow()

  return [
    nodeId,
    {
      kind: 'source',
      navigationRow,
      target: {
        navigationRowId: navigationRow.navigationRowId,
        entryPoint: sourceEntryPoint(),
        label: navigationRow.displayName
      }
    }
  ]
}

function directoryBinding(
  nodeId: BrowserTreeNodeId,
  directoryId: string
): readonly [BrowserTreeNodeId, RowBinding] {
  return [
    nodeId,
    {
      kind: 'directory',
      sourceId: '7',
      directoryId,
      entryPoint: sourceEntryPoint(),
      label: 'Source Fixture'
    }
  ]
}

function browserState(
  options: {
    readonly sourceChildren?: LoadedChildren
    readonly sourceStates?: BrowserState['sourceReadStates']
    readonly directoryStates?: BrowserState['directoryReadStates']
  } = {}
): BrowserState {
  return {
    navigationReadResult: readyNavigation([sourceNavigationRow()]),
    sourceReadStates:
      options.sourceStates ??
      (options.sourceChildren === undefined
        ? new Map()
        : new Map([
            [
              'navigation-row:7',
              {
                kind: 'loaded',
                children: options.sourceChildren
              }
            ]
          ])),
    directoryReadStates: options.directoryStates ?? new Map()
  }
}

function readyNavigation(rows: readonly NavigationRow[]): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows
  }
}

function sourceNavigationRow(): NavigationRow {
  return {
    navigationRowId: '7',
    stableKey: 'source:7',
    parentNavigationRowId: null,
    family: 'sources',
    rowKind: 'source',
    displayName: 'Source Fixture',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '7',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function loadedChildren(
  rows: readonly ChildRow[],
  options: {
    readonly parentDirectoryId?: string
  } = {}
): LoadedChildren {
  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    ...(options.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: options.parentDirectoryId }),
    rows,
    totalRows: rows.length,
    coverage: completeCoverage(),
    limit: 50
  }
}

function sourceEntryPoint(): EntryPoint {
  return {
    kind: 'source',
    sourceId: '7'
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string
): Extract<ChildRow, { readonly kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'scanning',
    navigableChildScopeState: 'hasNavigableChildScopes',
    updatedAtMs: 100
  }
}

function completeCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    subtreeCoverageComplete: true,
    emptyResultAuthoritative: false
  }
}
