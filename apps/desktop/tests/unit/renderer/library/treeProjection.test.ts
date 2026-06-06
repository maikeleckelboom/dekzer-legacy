import { describe, expect, it } from 'vitest'

import type { LibraryBoundaryHostStatus } from '../../../../src/shared/libraryBoundary/status'
import type {
  ChildRow,
  EntryPoint,
  HierarchyCoverage,
  NavigableChildScopeState
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
  hasDisclosureAffordance,
  isConfirmedDirectoryLeaf,
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId
} from '../../../../src/renderer/library/tree/types'

describe('projectState', () => {
  it('projects source hierarchy directory rows and load-more actions', () => {
    const projection = projectTree(
      browserState({
        rows: [sourceNavigationRow('\\\\?\\C:\\Users\\Maikel\\Music')],
        sourceChildren: loadedChildren(
          [directoryNode('12', 'Album'), fileNode('11', 'track.wav')],
          { totalRows: 3 }
        )
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode).toMatchObject({
      id: 'navigation-row:7',
      label: 'Music'
    })
    expect(loadedChildIds(sourceNode)).toEqual(['source-directory:12', 'more:navigation-row:7:2'])
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
          fileNode('99', 'clip.mp4', { fileClass: 'video' })
        ])
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')

    expect(projection.nodes.map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(loadedChildIds(sourceNode)).toEqual([])
    expect(projection.bindingsById.has('navigation-row:100')).toBe(false)
    expect(projection.bindingsById.has('navigation-row:99')).toBe(false)
    expect(projection.bindingsById.has('source-file:11')).toBe(false)
    expect(projection.bindingsById.has('source-file:99')).toBe(false)

    const unsupportedOnly = projectTree(
      browserState({
        rows: [navigationRowWithSelectorKind('allAudio', '100')]
      })
    )
    expect(unsupportedOnly.bindingsById.has('navigation-row:100')).toBe(false)
    expect(unsupportedOnly.nodes.some((node) => node.id === 'navigation-row:100')).toBe(false)
  })

  it('excludes literal file rows from the tree projection', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          fileNode('11', 'track.wav', { fileClass: 'audio' }),
          fileNode('12', 'clip.mp4', { fileClass: 'video' }),
          fileNode('13', 'cover.jpg', { fileClass: 'image' }),
          fileNode('14', 'album.cue', { fileClass: 'unsupported' }),
          fileNode('15', 'mystery', { fileClass: 'none' })
        ])
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')

    expect(loadedChildIds(sourceNode)).toEqual([])
    for (const fileId of ['11', '12', '13', '14', '15']) {
      expect(findNode(projection.nodes, `source-file:${fileId}`)).toBeUndefined()
      expect(projection.bindingsById.has(`source-file:${fileId}`)).toBe(false)
    }
  })

  it('projects host status instead of stale navigation rows', () => {
    const staleState = browserState({
      sourceChildren: loadedChildren([directoryNode('12', 'Album')])
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

  it('unloaded directory with known child scopes is a deferred branch', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: true,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'scanning',
            navigableChildScopeState: 'hasNavigableChildScopes'
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

  it('deferred directory expansion preserves state node', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: true,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'scanning',
            navigableChildScopeState: 'hasNavigableChildScopes'
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

  it('loading directory with known child scopes remains a branch', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: true,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'scanning',
            navigableChildScopeState: 'hasNavigableChildScopes'
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

  it('loading directory exposes state node when expanded', () => {
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

  it('failed directory exposes error state node when expanded', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'failed',
              errorCode: 'readFailed',
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
    expect(node.action).toMatchObject({
      kind: 'loadChildren',
      state: { kind: 'failed' }
    })
    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(childItemsFor(visibleItems, 'source-directory:12')[0]?.node).toMatchObject({
      role: 'state',
      icon: 'warning'
    })
  })

  it('completed childless directory is a leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('directory with only audio files is a leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Singles', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loaded',
              children: loadedChildren([fileNode('15', 'track.wav')], {
                parentDirectoryId: '12'
              })
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(projection.bindingsById.get('source-directory:12')).toMatchObject({
      kind: 'directory',
      sourceId: '7',
      directoryId: '12'
    })
    expect(projection.bindingsById.has('source-file:15')).toBe(false)
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

  it('leaf expansion does not create disclosure', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Album', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
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

  it('refreshing directory ignores prior file children for branch identity', () => {
    const priorChildren = loadedChildren([fileNode('15', 'prior-track.wav')])
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([directoryNode('12', 'Album')]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'refreshing',
              children: priorChildren,
              requestKey: 'source:7/directory:12/v:images',
              sequence: 2,
              detail: 'Refreshing children.'
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('refreshing source ignores prior file children', () => {
    const projection = projectTree(
      browserState({
        sourceStates: new Map([
          [
            'navigation-row:7',
            {
              kind: 'refreshing',
              children: loadedChildren([fileNode('11', 'cover.mp3', { fileClass: 'audio' })]),
              requestKey: 'source:7',
              sequence: 1,
              detail: 'Refreshing hierarchy children.'
            }
          ]
        ])
      })
    )
    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')

    expect(sourceNode.children.kind).toBe('none')
  })

  it('read failure does not project as source unavailable label', () => {
    const projection = projectTree(
      browserState({
        sourceStates: new Map([
          [
            'navigation-row:7',
            {
              kind: 'failed',
              detail: 'Unable to read library hierarchy children.',
              errorCode: 'readFailed'
            }
          ]
        ])
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')

    const children = sourceNode.children
    expect(children.kind).toBe('failed')
    if (children.kind === 'failed') {
      expect(children.stateNode.label).toBe('Hierarchy read failed')
    }
    expect(sourceNode.action).toMatchObject({
      kind: 'loadChildren',
      state: { kind: 'failed' }
    })
    expect(projection.bindingsById.get('navigation-row:7')).toMatchObject({
      kind: 'source'
    })
  })

  it('notFound error projects as source unavailable label', () => {
    const projection = projectTree(
      browserState({
        sourceStates: new Map([
          [
            'navigation-row:7',
            {
              kind: 'failed',
              detail: 'The requested library hierarchy target is not available.',
              errorCode: 'notFound'
            }
          ]
        ])
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')

    const children = sourceNode.children
    expect(children.kind).toBe('failed')
    if (children.kind === 'failed') {
      expect(children.stateNode.label).toBe('Source unavailable')
    }
    expect(sourceNode.action).toBeUndefined()
  })

  it('loaded source with sourceUnavailable coverage projects unavailable state', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: makeCoverageChildren({
          coverageState: 'sourceUnavailable',
          emptyResultAuthoritative: false,
          detail: 'The selected source is unavailable.'
        })
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode.children.kind).toBe('loaded')

    const children = sourceNode.children
    expect(children.kind).toBe('loaded')
    if (children.kind === 'loaded') {
      expect(children.nodes).toHaveLength(1)
      expect(children.nodes[0]?.label).toBe('Source unavailable')
    }
  })

  it('loaded source with locationMissing coverage keeps source row bound', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: makeCoverageChildren({
          coverageState: 'locationMissing',
          emptyResultAuthoritative: false,
          detail: 'The selected source location is missing.'
        })
      })
    )

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode.children.kind).toBe('loaded')
    expect(projection.bindingsById.get('navigation-row:7')).toMatchObject({
      kind: 'source'
    })
  })

  it('complete empty library tree window keeps source row bound', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: makeCoverageChildren({
          coverageState: 'complete',
          emptyResultAuthoritative: true,
          detail: 'No visible items in this scope.'
        })
      })
    )

    expect(projection.bindingsById.get('navigation-row:7')).toMatchObject({
      kind: 'source'
    })
  })

  it('video-only folder with no child folders has no disclosure', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Videos', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loaded',
              children: loadedChildren([fileNode('15', 'clip.mp4', { fileClass: 'video' })], {
                parentDirectoryId: '12'
              })
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('mixed audio/video folder with no child folders has no disclosure', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Mixed Media', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loaded',
              children: loadedChildren(
                [
                  fileNode('15', 'track.flac', { fileClass: 'audio' }),
                  fileNode('16', 'clip.mp4', { fileClass: 'video' })
                ],
                { parentDirectoryId: '12' }
              )
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('unknown navigable child scope state is not projected as expandable', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Pending Scan', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending',
            navigableChildScopeState: 'unknown'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('folder with unknown scope state does not reveal disclosure after child read', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Music Folder', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending',
            navigableChildScopeState: 'unknown'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loaded',
              children: loadedChildren([fileNode('15', 'track.flac', { fileClass: 'audio' })], {
                parentDirectoryId: '12'
              })
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(projection.bindingsById.has('source-file:15')).toBe(false)
  })

  it('noNavigableChildScopes folder is a leaf regardless of loaded state', () => {
    const priorChildren = loadedChildren([fileNode('15', 'track.wav')], {
      parentDirectoryId: '12'
    })
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Scanned Leaf', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'refreshing',
              children: priorChildren,
              requestKey: 'source:7/directory:12/v:audio',
              sequence: 2,
              detail: 'Refreshing children.'
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(isBrowserTreeBranch(node)).toBe(false)
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
  })

  it('folder with child folders is both selectable and expandable', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Parent Album', {
            hasChildDirectories: true,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'hasNavigableChildScopes'
          })
        ]),
        directoryStates: new Map([
          [
            '12',
            {
              kind: 'loaded',
              children: loadedChildren(
                [
                  directoryNode('20', 'Sub-Album', {
                    hasChildDirectories: false,
                    directoryPrimaryMediaState: {
                      kind: 'noPrimaryMediaDescendants'
                    },
                    directoryImageMediaState: {
                      kind: 'noImageMediaDescendants'
                    },
                    directoryScanState: 'complete',
                    navigableChildScopeState: 'noNavigableChildScopes'
                  }),
                  fileNode('15', 'track.flac', { fileClass: 'audio' })
                ],
                { parentDirectoryId: '12' }
              )
            }
          ]
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(isBrowserTreeBranch(node)).toBe(true)
    expect(canRevealBrowserTreeChildren(node)).toBe(true)
    expect(projection.bindingsById.get('source-directory:12')).toMatchObject({
      kind: 'directory',
      sourceId: '7',
      directoryId: '12'
    })
  })

  it('contents loaded state is independent from tree branch cache', () => {
    const failState = browserState({
      sourceStates: new Map([
        [
          'navigation-row:7',
          {
            kind: 'failed',
            detail: 'Unable to read library hierarchy children.',
            errorCode: 'readFailed'
          }
        ]
      ])
    })

    const projection = projectTree(failState)
    const sourceBinding = projection.bindingsById.get('navigation-row:7')
    expect(sourceBinding).toMatchObject({ kind: 'source' })

    const sourceNode = requiredNode(projection.nodes, 'navigation-row:7')
    expect(sourceNode.children.kind).toBe('failed')
  })

  it('isConfirmedDirectoryLeaf returns true only for known no navigable child scopes', () => {
    expect(
      isConfirmedDirectoryLeaf(
        directoryNode('1', 'Known Leaf', { navigableChildScopeState: 'noNavigableChildScopes' })
      )
    ).toBe(true)

    expect(
      isConfirmedDirectoryLeaf(
        directoryNode('2', 'Unknown', { navigableChildScopeState: 'unknown' })
      )
    ).toBe(false)

    expect(
      isConfirmedDirectoryLeaf(
        directoryNode('3', 'Branch', { navigableChildScopeState: 'hasNavigableChildScopes' })
      )
    ).toBe(false)
  })

  it('haveDisclosureAffordance returns true only for known navigable child scopes', () => {
    expect(
      hasDisclosureAffordance(
        directoryNode('1', 'Branch', { navigableChildScopeState: 'hasNavigableChildScopes' })
      )
    ).toBe(true)

    expect(
      hasDisclosureAffordance(
        directoryNode('2', 'Unknown', { navigableChildScopeState: 'unknown' })
      )
    ).toBe(false)

    expect(
      hasDisclosureAffordance(
        directoryNode('3', 'Known Leaf', { navigableChildScopeState: 'noNavigableChildScopes' })
      )
    ).toBe(false)
  })

  it('unknown child scope and confirmed leaf are semantically distinct', () => {
    const unknownDir = directoryNode('12', 'Unknown', { navigableChildScopeState: 'unknown' })
    const leafDir = directoryNode('12', 'Leaf', {
      navigableChildScopeState: 'noNavigableChildScopes'
    })

    expect(isConfirmedDirectoryLeaf(unknownDir)).toBe(false)
    expect(isConfirmedDirectoryLeaf(leafDir)).toBe(true)
    expect(hasDisclosureAffordance(unknownDir)).toBe(false)
    expect(hasDisclosureAffordance(leafDir)).toBe(false)
  })

  it('media-only folder is a confirmed leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Media Only', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(isBrowserTreeBranch(node)).toBe(false)
  })

  it('video-only folder is a confirmed leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Videos', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'noImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(isBrowserTreeBranch(node)).toBe(false)
  })

  it('mixed media-only folder is a confirmed leaf', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Mixed', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('none')
    expect(canRevealBrowserTreeChildren(node)).toBe(false)
    expect(isBrowserTreeBranch(node)).toBe(false)
  })

  it('unknown-to-branch transition shows disclosure without changing projection identity', () => {
    const unknownProjection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Transition Folder', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'pending',
            navigableChildScopeState: 'unknown'
          })
        ])
      })
    )
    const unknownNode = requiredNode(unknownProjection.nodes, 'source-directory:12')
    expect(unknownNode.children.kind).toBe('none')
    expect(canRevealBrowserTreeChildren(unknownNode)).toBe(false)
    expect(isBrowserTreeBranch(unknownNode)).toBe(false)

    const branchProjection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Transition Folder', {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'unknown' },
            directoryImageMediaState: { kind: 'unknown' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'hasNavigableChildScopes'
          })
        ])
      })
    )
    const branchNode = requiredNode(branchProjection.nodes, 'source-directory:12')
    expect(branchNode.children.kind).toBe('deferred')
    expect(canRevealBrowserTreeChildren(branchNode)).toBe(true)
    expect(isBrowserTreeBranch(branchNode)).toBe(true)

    expect(unknownProjection.bindingsById.get('source-directory:12')).toMatchObject({
      kind: 'directory'
    })
    expect(branchProjection.bindingsById.get('source-directory:12')).toMatchObject({
      kind: 'directory'
    })
  })

  it('folder with known child scopes shows disclosure', () => {
    const projection = projectTree(
      browserState({
        sourceChildren: loadedChildren([
          directoryNode('12', 'Known Branch', {
            navigableChildScopeState: 'hasNavigableChildScopes'
          })
        ])
      })
    )
    const node = requiredNode(projection.nodes, 'source-directory:12')

    expect(node.children.kind).toBe('deferred')
    expect(canRevealBrowserTreeChildren(node)).toBe(true)
    expect(isBrowserTreeBranch(node)).toBe(true)
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
    readonly sourceStates?: BrowserState['sourceReadStates']
    readonly directoryStates?: BrowserState['directoryReadStates']
  } = {}
): BrowserState {
  return {
    navigationReadResult: readyNavigation(options.rows ?? [sourceNavigationRow()]),
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

function loadedChildren(
  rows: readonly ChildRow[],
  options: {
    readonly parentDirectoryId?: string
    readonly totalRows?: number
  } = {}
): LoadedChildren {
  const totalRows = options.totalRows ?? rows.length
  const nextOffset = rows.length < totalRows ? rows.length : undefined

  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    ...(options.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: options.parentDirectoryId }),
    rows,
    totalRows,
    coverage: {
      state: 'complete',
      subtreeCoverageComplete: true,
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
    readonly navigableChildScopeState?: NavigableChildScopeState
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
    navigableChildScopeState: options.navigableChildScopeState ?? 'hasNavigableChildScopes',
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  options: {
    readonly fileClass?: Extract<ChildRow, { readonly kind: 'file' }>['fileClass']
  } = {}
): Extract<ChildRow, { readonly kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    fileClass: options.fileClass ?? 'audio',
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

function makeCoverageChildren(options: {
  readonly coverageState: HierarchyCoverage['state']
  readonly emptyResultAuthoritative: boolean
  readonly detail: string
  readonly rows?: readonly ChildRow[]
}): LoadedChildren {
  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    rows: options.rows ?? [],
    totalRows: options.rows?.length ?? 0,
    coverage: {
      state: options.coverageState,
      subtreeCoverageComplete: options.coverageState === 'complete',
      emptyResultAuthoritative: options.emptyResultAuthoritative,
      ...(options.detail === undefined ? {} : { detail: options.detail })
    },
    limit: 50
  }
}
