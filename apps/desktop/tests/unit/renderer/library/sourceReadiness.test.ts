import { describe, expect, it } from 'vitest'

import type { ScanProgressState } from '../../../../src/renderer/library/boundary/boundaryEvents'
import type { LocalRootsReadState } from '../../../../src/renderer/library/boundary/localRootActions'
import {
  deriveSourceReadiness,
  projectSourceReadinessByNodeId,
  type SourceReadiness
} from '../../../../src/renderer/library/runtime/sourceReadiness'
import type {
  BrowserState,
  LoadedChildren,
  SourceState
} from '../../../../src/renderer/library/state'
import { flattenVisibleTree } from '../../../../src/renderer/library/tree/listProjection'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type {
  ChildRow,
  EntryPoint,
  HierarchyCoverage
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/libraryNavigation/readRows'
import type { SourceLifecycleRecord } from '../../../../src/shared/librarySourceLifecycle/readSourceLifecycle'

describe('source readiness', () => {
  it('scan progress drives scanning without synthesizing children', () => {
    const state = browserState()
    const progress: ScanProgressState = {
      kind: 'scanning',
      rootId: '7',
      scanRunId: 'scan-1',
      directoriesVisited: 3,
      filesVisited: 4,
      filesDiscovered: 2,
      mediaCandidates: 2,
      queuedWorkItems: 0
    }

    const readiness = readinessFor(state, { progress })
    expect(readiness?.kind).toBe('scanning')
    expect(readiness?.detail).toContain('2 files discovered')

    const projection = projectTree(withReadiness(state, readyRoots('available'), progress))
    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode.children.kind).toBe('deferred')
  })

  it('completed scan alone is not ready', () => {
    const progress: ScanProgressState = {
      kind: 'completed',
      rootId: '7',
      scanRunId: 'scan-1',
      filesDiscovered: 1,
      queuedWorkItems: 0
    }

    const state = browserState()
    expect(readinessFor(state, { progress })?.kind).toBe('registered')

    const readyState = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Album')])
      }
    })
    expect(readinessFor(readyState, { progress })?.kind).toBe('ready')
  })

  it('unavailable source keeps prior directory rows visible', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Albums'), directoryNode('13', 'Singles')])
      }
    })
    const projection = projectTree(withReadiness(state, readyRoots('unavailable')))
    const visible = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set(['navigation-row:7'])
    })

    expect(projection.bindingsById.get('navigation-row:7')).toMatchObject({ kind: 'source' })
    expect(visible.map((item) => item.id)).toEqual([
      'navigation-row:7',
      'source-directory:12',
      'source-directory:13'
    ])
  })

  it('terminal scan progress states', () => {
    expect(
      readinessFor(browserState(), {
        progress: {
          kind: 'blocked',
          rootId: '7',
          detail: 'Permission denied.'
        }
      })?.kind
    ).toBe('blocked')

    expect(
      readinessFor(browserState(), {
        progress: {
          kind: 'failed',
          rootId: '7',
          detail: 'Scanner failed.'
        }
      })?.kind
    ).toBe('failed')
  })

  it('branch refresh keeps source ready', () => {
    const state = browserState({
      sourceState: {
        kind: 'refreshing',
        children: loadedChildren([directoryNode('12', 'Album')]),
        requestKey: 'source:7',
        sequence: 2,
        detail: 'Refreshing hierarchy children.'
      }
    })

    expect(readinessFor(state)?.kind).toBe('ready')

    const projection = projectTree(withReadiness(state))
    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode.children.kind).toBe('loaded')
  })

  it('operation state does not override readiness', () => {
    const readiness = deriveSourceReadiness({
      sourceNodeId: 'navigation-row:7',
      sourceLabel: 'Source Fixture',
      rootId: '7',
      sourceReadState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Album')])
      },
      currentScanStatus: 'failed'
    })

    expect(readiness.kind).toBe('ready')
  })

  it('backend barrier beats active scan', () => {
    const blocked = readinessFor(browserState(), {
      lifecycle: sourceLifecycle({
        accessState: 'blocked',
        accessIssueKind: 'permissionDenied',
        scanPhase: 'complete'
      })
    })
    expect(blocked?.kind).toBe('blocked')

    const scanning = scanningReadiness({
      lifecycle: sourceLifecycle({
        accessState: 'blocked',
        accessIssueKind: 'permissionDenied',
        scanPhase: 'complete'
      })
    })
    expect(scanning?.kind).toBe('blocked')

    const unmounted = scanningReadiness({
      lifecycle: sourceLifecycle({
        mountStatus: 'unmounted',
        accessState: 'blocked',
        accessIssueKind: 'unavailableMount',
        scanPhase: 'complete'
      })
    })
    expect(unmounted?.kind).toBe('unavailable')
  })

  it('active scan beats backend scan phase when usable', () => {
    const readiness = scanningReadiness({
      lifecycle: sourceLifecycle({
        mountStatus: 'mounted',
        accessState: 'accessible',
        scanPhase: 'blocked',
        scanIssueKind: 'permissionDenied'
      })
    })
    expect(readiness?.kind).toBe('scanning')
  })

  it('scans when lifecycle is unknown', () => {
    const readiness = scanningReadiness({
      lifecycle: sourceLifecycle({
        mountStatus: 'unknown',
        accessState: 'unknown',
        scanPhase: 'idle'
      }),
      progress: {
        kind: 'scanning',
        rootId: '7',
        scanRunId: 'scan-2',
        directoriesVisited: 0,
        filesVisited: 0,
        filesDiscovered: 0,
        mediaCandidates: 0,
        queuedWorkItems: 0
      }
    })
    expect(readiness?.kind).toBe('scanning')
  })

  it('backend scan facts beat stale terminal events', () => {
    const readiness = readinessFor(browserState(), {
      lifecycle: sourceLifecycle({
        accessState: 'accessible',
        scanPhase: 'complete'
      }),
      progress: {
        kind: 'failed',
        rootId: '7',
        detail: 'Stale scan event.'
      }
    })

    expect(readiness?.kind).toBe('registered')
  })

  it('hierarchy coverage decides ready vs empty', () => {
    const readyState = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Album')])
      }
    })
    const emptyState = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([])
      }
    })
    const progress: ScanProgressState = {
      kind: 'completed',
      rootId: '7',
      scanRunId: 'scan-1',
      filesDiscovered: 0,
      queuedWorkItems: 0
    }

    expect(readinessFor(readyState, { progress })?.kind).toBe('ready')
    expect(readinessFor(emptyState, { progress })?.kind).toBe('empty')
    expect(readinessFor(browserState(), { progress })?.kind).toBe('registered')
  })

  it('keeps file rows out of tree visibility', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([fileNode('99', 'hidden.wav')])
      }
    })
    const projection = projectTree(withReadiness(state))

    expect(projection.bindingsById.has('source-file:99')).toBe(false)
    expect(findNode(projection.nodes, 'source-file:99')).toBeUndefined()
  })
})

function readinessFor(
  state: BrowserState,
  options: {
    readonly roots?: LocalRootsReadState
    readonly progress?: ScanProgressState
    readonly lifecycle?: SourceLifecycleRecord
  } = {}
): SourceReadiness | undefined {
  const projection = projectTree(state)
  const lifecycleBySourceId = sourceLifecycleBySourceId(options.lifecycle)
  const readiness = projectSourceReadinessByNodeId({
    projection,
    localRootsReadState: options.roots ?? readyRoots('available'),
    sourceReadStates: state.sourceReadStates,
    ...(lifecycleBySourceId === undefined
      ? {}
      : { sourceLifecycleBySourceId: lifecycleBySourceId }),
    scanProgressByRootId: scanProgressByRootId(options.progress)
  })

  return readiness.get('navigation-row:7')
}

function scanningReadiness(options: {
  readonly lifecycle: SourceLifecycleRecord
  readonly progress?: ScanProgressState
}): SourceReadiness | undefined {
  return readinessFor(browserState(), {
    lifecycle: options.lifecycle,
    progress:
      options.progress ??
      ({
        kind: 'scanning',
        rootId: '7',
        scanRunId: 'scan-2',
        directoriesVisited: 1,
        filesVisited: 0,
        filesDiscovered: 0,
        mediaCandidates: 0,
        queuedWorkItems: 0
      } satisfies ScanProgressState)
  })
}

function withReadiness(
  state: BrowserState,
  roots: LocalRootsReadState = readyRoots('available'),
  progress?: ScanProgressState,
  lifecycle?: SourceLifecycleRecord
): BrowserState {
  const lifecycleBySourceId = sourceLifecycleBySourceId(lifecycle)
  const readiness = projectSourceReadinessByNodeId({
    projection: projectTree(state),
    localRootsReadState: roots,
    sourceReadStates: state.sourceReadStates,
    ...(lifecycleBySourceId === undefined
      ? {}
      : { sourceLifecycleBySourceId: lifecycleBySourceId }),
    scanProgressByRootId: scanProgressByRootId(progress)
  })

  return {
    ...state,
    sourceReadinessByNodeId: readiness
  }
}

function sourceLifecycleBySourceId(
  lifecycle: SourceLifecycleRecord | undefined
): ReadonlyMap<string, SourceLifecycleRecord> | undefined {
  if (lifecycle === undefined) {
    return undefined
  }

  return new Map([[lifecycle.sourceId, lifecycle]])
}

function sourceLifecycle(overrides: Partial<SourceLifecycleRecord> = {}): SourceLifecycleRecord {
  return {
    sourceId: '7',
    sourceClass: 'externalMounted',
    isUserVisible: true,
    mountStatus: 'mounted',
    accessState: 'accessible',
    scanPhase: 'idle',
    updatedAtMs: 100,
    ...overrides
  }
}

function scanProgressByRootId(
  progress: ScanProgressState | undefined
): ReadonlyMap<string, ScanProgressState> {
  if (progress === undefined || progress.kind === 'idle') {
    return new Map()
  }

  return new Map([[progress.rootId, progress]])
}

function browserState(
  options: {
    readonly sourceState?: SourceState
  } = {}
): BrowserState {
  return {
    navigationReadResult: readyNavigation([sourceNavigationRow()]),
    sourceReadStates:
      options.sourceState === undefined
        ? new Map()
        : new Map([['navigation-row:7', options.sourceState]]),
    directoryReadStates: new Map()
  }
}

function readyNavigation(rows: readonly NavigationRow[]): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows
  }
}

function readyRoots(availability: 'available' | 'unavailable'): LocalRootsReadState {
  return {
    kind: 'ready',
    roots: [{ rootId: '7', canonicalPath: 'C:/Music', availability }]
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

function loadedChildren(rows: readonly ChildRow[]): LoadedChildren {
  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    rows,
    totalRows: rows.length,
    coverage: {
      state: 'complete',
      recursiveScopeComplete: true,
      emptyResultAuthoritative: rows.length === 0
    } satisfies HierarchyCoverage,
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
  label: string
): Extract<ChildRow, { readonly kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    presence: 'present',
    hasChildDirectories: true,
    directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'complete',
    childRowState: 'hasChildRows',
    updatedAtMs: 100
  }
}

function fileNode(fileId: string, label: string): Extract<ChildRow, { readonly kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    mediaClass: 'audio',
    presence: 'present',
    updatedAtMs: 100
  }
}

function projectTree(state: BrowserState): BrowserProjection {
  const projection = projectState(state)

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected tree projection.')
  }

  return projection
}

function requiredNode(nodes: readonly BrowserTreeNode[], nodeId: string): BrowserTreeNode {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.children.kind === 'loaded') {
      const child = requiredNodeOrUndefined(node.children.nodes, nodeId)
      if (child !== undefined) {
        return child
      }
    }
  }

  throw new Error(`Expected projected node ${nodeId}.`)
}

function requiredNodeOrUndefined(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): BrowserTreeNode | undefined {
  return nodes.find((node) => node.id === nodeId)
}

function findNode(nodes: readonly BrowserTreeNode[], nodeId: string): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.children.kind === 'loaded') {
      const child = findNode(node.children.nodes, nodeId)

      if (child !== undefined) {
        return child
      }
    }
  }

  return undefined
}
