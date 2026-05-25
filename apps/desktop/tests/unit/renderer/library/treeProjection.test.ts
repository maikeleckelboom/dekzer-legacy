import { describe, expect, it } from 'vitest'

import type { LibraryBoundaryHostStatus } from '../../../../src/shared/libraryBoundary/status'
import type { ChildRow, EntryPoint } from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow,
  NavigationRowSelectorKind
} from '../../../../src/shared/libraryNavigation/readRows'
import type { BrowserState, LoadedChildren } from '../../../../src/renderer/library/state'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId
} from '../../../../src/renderer/library/tree/types'

describe('projectState', () => {
  it('projects source hierarchy rows and load-more actions', () => {
    const projection = projectTree(
      browserState({
        rows: [sourceNavigationRow('\\\\?\\C:\\Users\\Maikel\\Music')],
        sourceChildren: loadedChildren(
          [directoryNode('12', 'Album'), fileNode('11', 'track.wav')],
          { totalRows: 3 }
        )
      })
    )

    const sourceNode = projection.nodes[0]
    expect(sourceNode).toMatchObject({
      id: 'navigation-row:7',
      label: 'Music'
    })
    expect(loadedChildIds(sourceNode)).toEqual([
      'source-directory:12',
      'source-file:11',
      'more:navigation-row:7:2'
    ])
    expect(projection.bindingsById.get('source-directory:12')).toMatchObject({
      kind: 'directory',
      sourceId: '7',
      directoryId: '12'
    })

    const moreBinding = projection.bindingsById.get('more:navigation-row:7:2')
    expect(moreBinding).toMatchObject({
      kind: 'more',
      state: 'available'
    })
    if (moreBinding?.kind !== 'more') {
      throw new Error('Expected load-more binding.')
    }
    expect(moreBinding.target).toMatchObject({
      ownerNodeId: 'navigation-row:7',
      entryPoint: sourceEntryPoint(),
      offset: 2,
      limit: 50
    })
    expect(findNode(projection.nodes, 'more:navigation-row:7:2')?.action).toMatchObject({
      kind: 'loadMore',
      state: { kind: 'idle' }
    })
  })

  it('filters unwired navigation rows and hidden non-media child rows', () => {
    const projection = projectTree(
      browserState({
        rows: [
          sourceNavigationRow(),
          navigationRowWithSelectorKind('allAudio', '100'),
          navigationRowWithNullSelector()
        ],
        sourceChildren: loadedChildren([fileNode('11', 'track.wav'), fileNode('99', 'desktop.ini')])
      })
    )

    expect(projection.nodes.map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(loadedChildIds(projection.nodes[0])).toEqual(['source-file:11'])
    expect(projection.bindingsById.has('navigation-row:100')).toBe(false)
    expect(projection.bindingsById.has('navigation-row:99')).toBe(false)

    const unsupportedOnly = projectTree(
      browserState({
        rows: [navigationRowWithSelectorKind('allAudio', '100')]
      })
    )
    expect(unsupportedOnly.bindingsById.has('navigation-row:100')).toBe(false)
    expect(unsupportedOnly.nodes.some((node) => node.id === 'navigation-row:100')).toBe(false)
  })

  it('projects host status instead of stale navigation rows', () => {
    const staleState = browserState({
      sourceChildren: loadedChildren([fileNode('11', 'track.wav')])
    })
    const failedProjection = projectTree({
      ...staleState,
      hostStatus: hostWithState('failed', 'Host failed')
    })

    expect(failedProjection.nodes[0]?.label).toBe('Library engine failed to start')
    expect(failedProjection.bindingsById.has('navigation-row:7')).toBe(false)

    const stoppedProjection = projectTree({
      ...staleState,
      hostStatus: hostWithState('stopped')
    })
    expect(stoppedProjection.nodes[0]?.label).toBe('Library engine unavailable')
    expect(stoppedProjection.bindingsById.has('navigation-row:7')).toBe(false)
  })

  it('uses directory coverage facts for disclosure state', () => {
    const hasMediaProjection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryMediaState: { kind: 'hasMediaDescendants' }
          })
        ])
      })
    )
    expect(
      findNode(hasMediaProjection.nodes, 'source-directory:12')?.hasDirectoryDisclosureHint
    ).toBe(true)

    const confirmedEmptyProjection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryMediaState: { kind: 'noMediaDescendants' },
            directoryScanState: 'complete'
          })
        ])
      })
    )
    expect(findNode(confirmedEmptyProjection.nodes, 'source-directory:12')?.children.kind).toBe(
      'none'
    )

    const unknownProjection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryMediaState: { kind: 'unknown' },
            directoryScanState: 'pending'
          })
        ])
      })
    )
    const unknownNode = findNode(unknownProjection.nodes, 'source-directory:12')
    expect(unknownNode?.children.kind).toBe('deferred')
    expect(unknownNode?.hasDirectoryDisclosureHint).toBe(true)
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

function browserState(
  options: {
    readonly rows?: readonly NavigationRow[]
    readonly sourceChildren?: LoadedChildren
  } = {}
): BrowserState {
  return {
    navigationReadResult: readyNavigation(options.rows ?? [sourceNavigationRow()]),
    sourceReadStates:
      options.sourceChildren === undefined
        ? new Map()
        : new Map([
            [
              'navigation-row:7',
              {
                kind: 'loaded',
                children: options.sourceChildren
              }
            ]
          ]),
    directoryReadStates: new Map()
  }
}

function readyNavigation(rows: readonly NavigationRow[]): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows
  }
}

function loadedChildren(
  rows: readonly ChildRow[],
  options: {
    readonly totalRows?: number
  } = {}
): LoadedChildren {
  const totalRows = options.totalRows ?? rows.length
  const nextOffset = rows.length < totalRows ? rows.length : undefined

  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    rows,
    totalRows,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: 50
  }
}

function sourceEntryPoint(): EntryPoint {
  return {
    kind: 'source',
    sourceId: '7'
  }
}

function sourceNavigationRow(displayName = 'Source Fixture'): NavigationRow {
  return {
    navigationRowId: '7',
    stableKey: 'source:7',
    parentNavigationRowId: null,
    family: 'sources',
    rowKind: 'source',
    displayName,
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '7',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function navigationRowWithSelectorKind(
  selectorKind: NavigationRowSelectorKind,
  navigationRowId: string
): NavigationRow {
  return {
    navigationRowId,
    stableKey: `view:${selectorKind}`,
    parentNavigationRowId: null,
    family: 'views',
    rowKind: 'view',
    displayName: selectorKind,
    siblingPosition: 0,
    selectable: true,
    selectorKind,
    selectorPayload:
      selectorKind === 'source' || selectorKind === 'sourceLocation' ? navigationRowId : null,
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function navigationRowWithNullSelector(): NavigationRow {
  return {
    navigationRowId: '99',
    stableKey: 'group:structural',
    parentNavigationRowId: null,
    family: null,
    rowKind: 'collectionGroup',
    displayName: 'Structural Group',
    siblingPosition: 0,
    selectable: true,
    selectorKind: null,
    selectorPayload: null,
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  options: {
    readonly hasChildDirectories?: boolean
    readonly directoryMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryMediaState']
    readonly directoryScanState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryScanState']
  } = {}
): Extract<ChildRow, { readonly kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    presence: 'present',
    hasChildDirectories: options.hasChildDirectories ?? true,
    directoryMediaState: options.directoryMediaState ?? { kind: 'hasMediaDescendants' },
    directoryScanState: options.directoryScanState ?? 'scanning',
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
    presence: 'present',
    updatedAtMs: 100
  }
}

function hostWithState(
  state: LibraryBoundaryHostStatus['state'],
  message?: string
): LibraryBoundaryHostStatus {
  return {
    state,
    environment: 'development',
    binaryPolicy: { kind: 'developmentBinary', source: 'repoDebugTarget' },
    lastError:
      message === undefined
        ? null
        : {
            code: 'unknown',
            message
          }
  }
}

function loadedChildIds(node: BrowserTreeNode | undefined): readonly BrowserTreeNodeId[] {
  return node?.children.kind === 'loaded' ? node.children.nodes.map((child) => child.id) : []
}

function findNode(
  nodes: readonly BrowserTreeNode[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNode | undefined {
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
