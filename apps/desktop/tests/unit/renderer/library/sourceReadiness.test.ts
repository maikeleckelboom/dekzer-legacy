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

describe('source readiness projection', () => {
  it('uses scan progress for source-level scanning without synthesizing children', () => {
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

  it('completed scan alone keeps the source registered until an authoritative read is ready', () => {
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
        children: loadedChildren([fileNode('11', 'track.wav')])
      }
    })
    expect(readinessFor(readyState, { progress })?.kind).toBe('ready')
  })

  it('keeps an unavailable known source and its prior child rows visible', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Albums'), fileNode('11', 'track.wav')])
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
      'source-file:11'
    ])
  })

  it('represents blocked and failed terminal scan progress honestly', () => {
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

  it('keeps branch refresh separate from source-level readiness', () => {
    const state = browserState({
      sourceState: {
        kind: 'refreshing',
        children: loadedChildren([fileNode('11', 'track.wav')]),
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

  it('does not let operation-only terminal state become durable source readiness', () => {
    const readiness = deriveSourceReadiness({
      sourceNodeId: 'navigation-row:7',
      sourceLabel: 'Source Fixture',
      rootId: '7',
      sourceReadState: {
        kind: 'loaded',
        children: loadedChildren([fileNode('11', 'track.wav')])
      },
      currentScanStatus: 'failed'
    })

    expect(readiness.kind).toBe('ready')
  })

  it('prefers backend source lifecycle facts while preserving active scan immediacy', () => {
    const blockedLifecycle = sourceLifecycle({
      accessState: 'blocked',
      accessIssueKind: 'permissionDenied',
      scanPhase: 'complete'
    })

    expect(readinessFor(browserState(), { lifecycle: blockedLifecycle })?.kind).toBe('blocked')

    expect(
      readinessFor(browserState(), {
        lifecycle: blockedLifecycle,
        progress: {
          kind: 'scanning',
          rootId: '7',
          scanRunId: 'scan-2',
          directoriesVisited: 1,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })?.kind
    ).toBe('scanning')
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

function withReadiness(
  state: BrowserState,
  roots: LocalRootsReadState = readyRoots('available'),
  progress?: ScanProgressState
): BrowserState {
  const readiness = projectSourceReadinessByNodeId({
    projection: projectTree(state),
    localRootsReadState: roots,
    sourceReadStates: state.sourceReadStates,
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
