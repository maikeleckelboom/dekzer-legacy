import { describe, expect, it } from 'vitest'

import type { LibraryBoundaryHostStatus } from '../../../../src/shared/libraryBoundary/status'
import type {
  ChildRow,
  EntryPoint,
  HierarchyCoverage,
  SourceFileVisibility
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow,
  NavigationRowSelectorKind
} from '../../../../src/shared/libraryNavigation/readRows'
import type { BrowserState, LoadedChildren } from '../../../../src/renderer/library/state'
import {
  canRevealBrowserTreeChildren,
  flattenVisibleTree,
  isBrowserTreeBranch
} from '../../../../src/renderer/library/tree/listProjection'
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
      limit: 50,
      sourceFileVisibility: 'performance'
    })
    expect(findNode(projection.nodes, 'more:navigation-row:7:2')?.action).toMatchObject({
      kind: 'loadMore',
      state: { kind: 'idle' }
    })
  })

  it('filters unwired navigation rows', () => {
    const projection = projectTree(
      browserState({
        rows: [
          sourceNavigationRow(),
          navigationRowWithSelectorKind('allAudio', '100'),
          navigationRowWithNullSelector()
        ],
        sourceChildren: loadedChildren([
          fileNode('11', 'track.wav'),
          fileNode('99', 'clip.mp4', { mediaClass: 'video' })
        ])
      })
    )

    expect(projection.nodes.map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(loadedChildIds(projection.nodes[0])).toEqual(['source-file:11', 'source-file:99'])
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

  it('projects literal file presentation from backend media class', () => {
    const projection = projectTree(
      browserState({
        sourceFileVisibility: 'performanceAndImages',
        sourceChildren: loadedChildren(
          [
            fileNode('11', 'cover.mp3', { mediaClass: 'image' }),
            fileNode('13', 'clip.wav', { mediaClass: 'video' }),
            fileNode('14', 'track.raw', { mediaClass: 'audio' })
          ],
          { sourceFileVisibility: 'performanceAndImages' }
        )
      })
    )

    const sourceNode = projection.nodes[0]
    expect(loadedChildIds(sourceNode)).toEqual([
      'source-file:11',
      'source-file:13',
      'source-file:14'
    ])
    expect(requiredNode(projection.nodes, 'source-file:11')).toMatchObject({
      icon: 'image',
      detail: 'Image file'
    })
    expect(requiredNode(projection.nodes, 'source-file:13')).toMatchObject({
      icon: 'video',
      detail: 'Video file'
    })
    expect(requiredNode(projection.nodes, 'source-file:14')).toMatchObject({
      icon: 'music',
      detail: 'Audio file'
    })
  })

  it('projects unexpected backend-hidden media classes defensively', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          fileNode('12', 'desktop.ini', { mediaClass: 'unsupported' }),
          fileNode('15', 'mystery', { mediaClass: 'none' })
        ])
      })
    )

    expect(loadedChildIds(projection.nodes[0])).toEqual(['source-file:12', 'source-file:15'])
    expect(requiredNode(projection.nodes, 'source-file:12')).toMatchObject({
      icon: 'file',
      detail: 'File'
    })
    expect(requiredNode(projection.nodes, 'source-file:15')).toMatchObject({
      icon: 'file',
      detail: 'File'
    })
    expect(projection.bindingsById.get('source-file:12')).toMatchObject({ kind: 'file' })
    expect(projection.bindingsById.get('source-file:15')).toMatchObject({ kind: 'file' })
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

  it('projecting an unloaded loadable directory creates a branch with deferred child state', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('deferred')
    expect(node.children.kind === 'deferred' ? node.children.stateNode.role : undefined).toBe(
      'state'
    )
    expect(node.action).toMatchObject({ kind: 'loadChildren', state: { kind: 'idle' } })
    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(canRevealBrowserTreeChildren(node)).toBe(true)
  })

  it('expanding a deferred directory cannot flatten to zero visible child rows', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending'
          })
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })

    expect(
      childItemsFor(visibleItems, 'source-directory:12').map((item) => item.node.role)
    ).toEqual(['state'])
  })

  it('loading directory remains a branch and keeps disclosure', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loading',
              requestKey: 'source:7/directory:12',
              sequence: 1,
              detail: 'Loading children.'
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('loading')
    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(canRevealBrowserTreeChildren(node)).toBe(true)
  })

  it('loading directory materializes a stable loading child representation when expanded', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loading',
              requestKey: 'source:7/directory:12',
              sequence: 1,
              detail: 'Loading children.'
            }
          ]
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })
    const childRows = childItemsFor(visibleItems, 'source-directory:12')

    expect(childRows).toHaveLength(1)
    expect(childRows[0]?.node).toMatchObject({
      role: 'state',
      icon: 'loading'
    })
  })

  it('failed directory remains a branch and materializes explicit failure state when expanded', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'failed',
              detail: 'Unable to read children.'
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })

    expect(node.children.kind).toBe('failed')
    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(childItemsFor(visibleItems, 'source-directory:12')[0]?.node).toMatchObject({
      role: 'state',
      icon: 'warning'
    })
  })

  it('complete no-media/no-child directory can still project as leaf with no children', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('keeps image-only directories expandable in performance-and-images visibility', () => {
    const projection = projectTree(
      browserState({
        sourceFileVisibility: 'performanceAndImages',
        sourceChildren: loadedChildren(
          [
            directoryNode('50', 'Covers', {
              hasChildDirectories: false,
              directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
              directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
              directoryScanState: 'complete'
            })
          ],
          { sourceFileVisibility: 'performanceAndImages' }
        )
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:50')

    expect(node.children.kind).toBe('deferred')
    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(canRevealBrowserTreeChildren(node)).toBe(true)
  })

  it('state-only child rows do not accidentally decide branch identity', () => {
    const node: BrowserTreeNode = {
      id: 'state-only-owner',
      label: 'State-only owner',
      role: 'literalDirectory',
      children: {
        kind: 'loaded',
        nodes: [
          {
            id: 'state-only-child',
            label: 'Status',
            role: 'state',
            children: { kind: 'none' }
          }
        ]
      }
    }
    const visibleItems = flattenVisibleTree({
      nodes: [node],
      expandedNodeIds: new Set(['state-only-owner'])
    })

    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(visibleItems).toHaveLength(1)
  })

  it('expanded state alone does not create disclosure for a confirmed leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete'
          })
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: [requiredNode(projection.nodes, 'source-directory:12')],
      expandedNodeIds: new Set(['source-directory:12'])
    })

    expect(visibleItems).toHaveLength(1)
    expect(visibleItems[0]).toMatchObject({
      isBranch: false,
      canRevealChildren: false,
      isExpanded: false
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

function requiredNode(
  nodes: readonly BrowserTreeNode[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNode {
  const node = findNode(nodes, nodeId)

  expect(node).toBeDefined()
  if (node === undefined) {
    throw new Error(`Expected projected node ${nodeId}.`)
  }

  return node
}

function childItemsFor(
  visibleItems: ReturnType<typeof flattenVisibleTree>,
  parentId: BrowserTreeNodeId
): ReturnType<typeof flattenVisibleTree> {
  return visibleItems.filter((item) => item.parentId === parentId)
}

function browserState(
  options: {
    readonly rows?: readonly NavigationRow[]
    readonly sourceChildren?: LoadedChildren
    readonly directoryStates?: BrowserState['directoryReadStates']
    readonly sourceFileVisibility?: SourceFileVisibility
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
    directoryReadStates: options.directoryStates ?? new Map(),
    sourceFileVisibility: options.sourceFileVisibility ?? 'performance'
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
    readonly sourceFileVisibility?: SourceFileVisibility
  } = {}
): LoadedChildren {
  const totalRows = options.totalRows ?? rows.length
  const nextOffset = rows.length < totalRows ? rows.length : undefined

  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    sourceFileVisibility: options.sourceFileVisibility ?? 'performance',
    rows,
    totalRows,
    coverage: {
      state: 'complete',
      recursiveScopeComplete: true,
      emptyResultAuthoritative: rows.length === 0
    } satisfies HierarchyCoverage,
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
    readonly directoryPrimaryMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryPrimaryMediaState']
    readonly directoryImageMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryImageMediaState']
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
    directoryPrimaryMediaState: options.directoryPrimaryMediaState ?? {
      kind: 'hasPrimaryMediaDescendants'
    },
    directoryImageMediaState: options.directoryImageMediaState ?? {
      kind: 'noImageMediaDescendants'
    },
    directoryScanState: options.directoryScanState ?? 'scanning',
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  options: {
    readonly mediaClass?: Extract<ChildRow, { readonly kind: 'file' }>['mediaClass']
  } = {}
): Extract<ChildRow, { readonly kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    mediaClass: options.mediaClass ?? 'audio',
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
