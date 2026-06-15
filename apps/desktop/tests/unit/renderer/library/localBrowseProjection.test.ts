import { describe, expect, it } from 'vitest'

import type { BrowserState } from '../../../../src/renderer/library/state'
import {
  projectAddSourceState,
  addSourceSectionNodeId
} from '../../../../src/renderer/library/localBrowse/projection'
import type {
  LoadedLocalBrowseItems,
  LocalBrowseItemState
} from '../../../../src/renderer/library/localBrowse/types'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import {
  canRevealBrowserTreeChildren,
  flattenVisibleTree
} from '../../../../src/renderer/library/tree/listProjection'
import type {
  LocalBrowseEntryPoint,
  LocalBrowseEntryPointKind
} from '../../../../src/shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  LocalBrowseItemKind
} from '../../../../src/shared/library/localBrowse/items'
import type { NavigationReadRowsResult } from '../../../../src/shared/library/navigation/read'

describe('local browse tree projection', () => {
  it('projects local browse entry points for an empty durable source navigation', () => {
    const projection = projectTree(
      browserState({
        entries: [musicEntryPoint()]
      })
    )

    expect(projection.nodes.map((node) => node.id)).toEqual([addSourceSectionNodeId])
    expect(projection.nodes[0]?.label).toBe('Add Source')
    expect(firstLoadedChildLabels(projection.nodes[0])).toEqual(['Music'])
    expect(projection.bindingsById.get('add-source:section')).toEqual({
      kind: 'addSourceSection'
    })
  })

  it('orders music and user folders before broad drive roots', () => {
    const projection = projectTree(
      browserState({
        entries: [
          entryPoint('systemDriveRoot', 'System Drive', 'C:\\'),
          entryPoint('localDataVolumeRoot', 'Data', 'D:\\'),
          entryPoint('removableVolumeRoot', 'USB', 'E:\\'),
          entryPoint('downloads', 'Downloads', 'C:\\Users\\Maikel\\Downloads'),
          entryPoint('music', 'Music', 'C:\\Users\\Maikel\\Music'),
          entryPoint('desktop', 'Desktop', 'C:\\Users\\Maikel\\Desktop'),
          entryPoint('userHome', 'Home', 'C:\\Users\\Maikel')
        ]
      })
    )

    expect(firstLoadedChildLabels(projection.nodes[0])).toEqual([
      'Music',
      'Downloads',
      'Desktop',
      'Home',
      'USB',
      'Data',
      'System Drive'
    ])
    expect(findNodeDetail(projection, 'System Drive')).toBe(
      'Choose a specific folder inside this drive.'
    )
  })

  it('keeps local browse entries out of source bindings', () => {
    const projection = projectTree(
      browserState({
        entries: [musicEntryPoint()]
      })
    )

    expect([...projection.bindingsById.values()].some((binding) => binding.kind === 'source')).toBe(
      false
    )
    expect(
      [...projection.bindingsById.values()].some((binding) => binding.kind === 'directory')
    ).toBe(false)
    expect(
      [...projection.bindingsById.values()].some(
        (binding) => binding.kind === 'localBrowseEntryPoint'
      )
    ).toBe(true)
  })

  it('preserves the default music admission operation from the boundary', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            availableOperations: defaultMusicEntryOperations()
          })
        ]
      })
    )
    const binding = [...projection.bindingsById.values()].find(
      (candidate) => candidate.kind === 'localBrowseEntryPoint'
    )

    expect(binding).toMatchObject({
      kind: 'localBrowseEntryPoint',
      entry: {
        identity: { entryPointKind: 'music', resolvedPath: 'C:\\Users\\Maikel\\Music' },
        availableOperations: defaultMusicEntryOperations()
      }
    })
  })

  it('does not project duplicate entry points as fresh admission candidates', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            status: 'duplicateOfAdmittedSource',
            availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
          })
        ]
      })
    )
    const binding = [...projection.bindingsById.values()].find(
      (candidate) => candidate.kind === 'localBrowseEntryPoint'
    )

    expect(binding).toMatchObject({
      kind: 'localBrowseEntryPoint',
      entry: {
        status: 'duplicateOfAdmittedSource',
        availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
      }
    })
    expect(findNodeDetail(projection, 'Music')).toBe('Already added as a music source.')
  })

  it('projects removed entry points as restorable instead of active duplicates', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            status: 'restorableSource',
            matchedSourceId: '7',
            availableOperations: defaultMusicEntryOperations()
          })
        ]
      })
    )
    const binding = [...projection.bindingsById.values()].find(
      (candidate) => candidate.kind === 'localBrowseEntryPoint'
    )

    expect(binding).toMatchObject({
      kind: 'localBrowseEntryPoint',
      entry: {
        status: 'restorableSource',
        matchedSourceId: '7',
        availableOperations: defaultMusicEntryOperations()
      }
    })
    expect(findNodeDetail(projection, 'Music')).toBe(
      'Source was removed. Restore it to use this folder again.'
    )
  })

  it('projects local browse child row kinds without source bindings', () => {
    const rootTarget = {
      addSourceView: 'preview' as const,
      entryPointKind: 'music' as const,
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }
    const projection = projectTree({
      ...browserState({ entries: [musicEntryPoint()] }),
      localBrowseItemStates: new Map([
        [
          'preview:music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic',
          {
            kind: 'loaded',
            window: {
              identity: {
                entryPointKind: rootTarget.entryPointKind,
                resolvedRootPath: rootTarget.resolvedRootPath,
                resolvedParentPath: rootTarget.resolvedParentPath
              },
              addSourceView: rootTarget.addSourceView,
              label: rootTarget.label,
              items: [
                item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums'),
                item('mediaFile', 'track.flac', 'C:\\Users\\Maikel\\Music\\track.flac', {
                  fileKind: 'audio',
                  mediaRelevance: 'mediaRelevant',
                  availableOperations: [
                    {
                      kind: 'requestSourceAdmission',
                      requestKind: 'parentDirectory',
                      resolvedPath: 'C:\\Users\\Maikel\\Music'
                    }
                  ]
                }),
                item('unsupportedFile', 'notes.txt', 'C:\\Users\\Maikel\\Music\\notes.txt', {
                  fileKind: 'textDoc',
                  mediaRelevance: 'unsupported'
                }),
                item('inaccessible', 'Blocked', 'C:\\Users\\Maikel\\Music\\Blocked', {
                  status: 'permissionBlocked'
                }),
                item('rejectedRoot', 'Windows', 'C:\\Users\\Maikel\\Music\\Windows', {
                  status: 'rejected'
                })
              ],
              totalItems: 5,
              status: 'complete',
              failure: null,
              limit: 50
            }
          }
        ]
      ])
    })
    const section = projection.nodes[0]
    const music = section?.children.kind === 'loaded' ? section.children.nodes[0] : undefined
    const childLabels = firstLoadedChildLabels(music)

    expect(childLabels).toEqual(['Albums', 'track.flac', 'notes.txt', 'Blocked', 'Windows'])
    expect([...projection.bindingsById.values()].some((binding) => binding.kind === 'source')).toBe(
      false
    )
    expect(
      [...projection.bindingsById.values()].filter((binding) => binding.kind === 'localBrowseItem')
    ).toHaveLength(5)
  })

  it('keeps local browse file rows terminal even if an operation is present', () => {
    const rootTarget = {
      addSourceView: 'preview' as const,
      entryPointKind: 'music' as const,
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }
    const projection = projectTree({
      ...browserState({ entries: [musicEntryPoint()] }),
      localBrowseItemStates: new Map([
        [
          'preview:music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic',
          {
            kind: 'loaded',
            window: {
              identity: {
                entryPointKind: rootTarget.entryPointKind,
                resolvedRootPath: rootTarget.resolvedRootPath,
                resolvedParentPath: rootTarget.resolvedParentPath
              },
              addSourceView: rootTarget.addSourceView,
              label: rootTarget.label,
              items: [
                item('mediaFile', 'track.flac', 'C:\\Users\\Maikel\\Music\\track.flac', {
                  fileKind: 'audio',
                  mediaRelevance: 'mediaRelevant',
                  availableOperations: [{ kind: 'browseChildren' }]
                })
              ],
              totalItems: 1,
              status: 'complete',
              failure: null,
              limit: 50
            }
          }
        ]
      ])
    })

    const trackNode = findNode(projection, 'track.flac')
    const trackBinding =
      trackNode === undefined ? undefined : projection.bindingsById.get(trackNode.id)

    expect(trackNode).toMatchObject({
      role: 'localBrowseFile',
      children: { kind: 'none' }
    })
    expect(trackNode === undefined ? undefined : canRevealBrowserTreeChildren(trackNode)).toBe(
      false
    )
    expect(trackBinding).toMatchObject({
      kind: 'localBrowseItem'
    })
    expect(trackBinding).not.toHaveProperty('target')
  })

  it('renders accepted child windows immediately when a local browse folder is expanded', () => {
    const projection = projectTree(
      stateWithMusicWindow([item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')], {
        childStates: new Map([
          [
            localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
            {
              kind: 'loaded',
              window: loadedWindow({
                label: 'Albums',
                resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
                items: [item('directory', 'Nested', 'C:\\Users\\Maikel\\Music\\Albums\\Nested')]
              })
            }
          ]
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set([
        addSourceSectionNodeId,
        localBrowseEntryNodeId('C:\\Users\\Maikel\\Music'),
        localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums')
      ])
    })

    expect(
      childLabelsFor(visibleItems, localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums'))
    ).toEqual(['Nested'])
    expect(
      childItemsFor(visibleItems, localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums')).some(
        (child) => child.node.icon === 'loading'
      )
    ).toBe(false)
  })

  it('keeps retained child rows visible while the same local browse folder is refreshing', () => {
    const albumsNodeId = localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums')
    const projection = projectTree(
      stateWithMusicWindow([item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')], {
        childStates: new Map([
          [
            localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
            {
              kind: 'refreshing',
              requestKey: localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
              sequence: 2,
              detail: 'Refreshing local browse items.',
              window: loadedWindow({
                label: 'Albums',
                resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
                items: [item('directory', 'Retained', 'C:\\Users\\Maikel\\Music\\Albums\\Retained')]
              })
            }
          ]
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set([
        addSourceSectionNodeId,
        localBrowseEntryNodeId('C:\\Users\\Maikel\\Music'),
        albumsNodeId
      ])
    })

    expect(childLabelsFor(visibleItems, albumsNodeId)).toEqual(['Retained'])
    expect(childItemsFor(visibleItems, albumsNodeId).map((child) => child.node.icon)).not.toContain(
      'loading'
    )
  })

  it('projects honest loading for a cold expanded local browse folder', () => {
    const albumsNodeId = localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums')
    const projection = projectTree(
      stateWithMusicWindow([item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')], {
        childStates: new Map([
          [
            localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
            {
              kind: 'loading',
              requestKey: localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
              sequence: 1,
              detail: 'Loading local browse items.'
            }
          ]
        ])
      })
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set([
        addSourceSectionNodeId,
        localBrowseEntryNodeId('C:\\Users\\Maikel\\Music'),
        albumsNodeId
      ])
    })
    const childRows = childItemsFor(visibleItems, albumsNodeId)

    expect(childRows).toHaveLength(1)
    expect(childRows[0]?.node).toMatchObject({
      role: 'state',
      icon: 'loading'
    })
  })

  it('keeps available local browse directory disclosure stable across read states', () => {
    const albums = item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')
    const albumsNodeId = localBrowseItemNodeId(albums.identity.resolvedItemPath)
    const parentExpandedIds = new Set([
      addSourceSectionNodeId,
      localBrowseEntryNodeId('C:\\Users\\Maikel\\Music')
    ])
    const beforeProjection = projectTree(stateWithMusicWindow([albums]))
    const loadingProjection = projectTree(
      stateWithMusicWindow([albums], {
        childStates: new Map([
          [
            localBrowseStateKey(albums.identity.resolvedItemPath),
            {
              kind: 'loading',
              requestKey: localBrowseStateKey(albums.identity.resolvedItemPath),
              sequence: 1,
              detail: 'Loading local browse items.'
            }
          ]
        ])
      })
    )
    const loadedProjection = projectTree(
      stateWithMusicWindow([albums], {
        childStates: new Map([
          [
            localBrowseStateKey(albums.identity.resolvedItemPath),
            {
              kind: 'loaded',
              window: loadedWindow({
                label: 'Albums',
                resolvedParentPath: albums.identity.resolvedItemPath,
                items: [item('directory', 'Nested', 'C:\\Users\\Maikel\\Music\\Albums\\Nested')]
              })
            }
          ]
        ])
      })
    )

    expect(visibleItemFor(beforeProjection, albumsNodeId, parentExpandedIds)).toMatchObject({
      canRevealChildren: true,
      isBranch: true
    })
    expect(findNode(beforeProjection, 'Albums')).toMatchObject({
      children: { kind: 'deferred' },
      action: { kind: 'loadChildren' }
    })

    expect(visibleItemFor(loadingProjection, albumsNodeId, parentExpandedIds)).toMatchObject({
      canRevealChildren: true,
      isBranch: true
    })
    expect(findNode(loadingProjection, 'Albums')?.children.kind).toBe('loading')

    expect(visibleItemFor(loadedProjection, albumsNodeId, parentExpandedIds)).toMatchObject({
      canRevealChildren: true,
      isBranch: true
    })
    expect(firstLoadedChildLabels(findNode(loadedProjection, 'Albums'))).toEqual(['Nested'])
  })

  it('projects empty only after an accepted local browse read proves empty', () => {
    const albumsNodeId = localBrowseItemNodeId('C:\\Users\\Maikel\\Music\\Albums')
    const projection = projectTree(
      stateWithMusicWindow([item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')], {
        childStates: new Map([
          [
            localBrowseStateKey('C:\\Users\\Maikel\\Music\\Albums'),
            {
              kind: 'loaded',
              window: loadedWindow({
                label: 'Albums',
                resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
                items: []
              })
            }
          ]
        ])
      })
    )
    const unknownProjection = projectTree(
      stateWithMusicWindow([item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')])
    )
    const visibleItems = flattenVisibleTree({
      nodes: projection.nodes,
      expandedNodeIds: new Set([
        addSourceSectionNodeId,
        localBrowseEntryNodeId('C:\\Users\\Maikel\\Music'),
        albumsNodeId
      ])
    })

    expect(childItemsFor(visibleItems, albumsNodeId)[0]?.node).toMatchObject({
      role: 'state',
      label: 'No local items',
      icon: 'state'
    })
    expect(findNode(unknownProjection, 'Albums')?.children.kind).toBe('deferred')
  })

  it('keeps warmed empty local browse folders as stable collapsed branches', () => {
    const albums = item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')
    const albumsNodeId = localBrowseItemNodeId(albums.identity.resolvedItemPath)
    const projection = projectTree(
      stateWithMusicWindow([albums], {
        childStates: new Map([
          [
            localBrowseStateKey(albums.identity.resolvedItemPath),
            {
              kind: 'loaded',
              window: loadedWindow({
                label: 'Albums',
                resolvedParentPath: albums.identity.resolvedItemPath,
                items: []
              })
            }
          ]
        ])
      })
    )
    const parentExpandedIds = new Set([
      addSourceSectionNodeId,
      localBrowseEntryNodeId('C:\\Users\\Maikel\\Music')
    ])
    const expandedIds = new Set([...parentExpandedIds, albumsNodeId])

    expect(visibleItemFor(projection, albumsNodeId, parentExpandedIds)).toMatchObject({
      canRevealChildren: true,
      isBranch: true,
      isExpanded: false
    })
    expect(
      childItemsFor(visibleItemsFor(projection, expandedIds), albumsNodeId)[0]?.node
    ).toMatchObject({
      role: 'state',
      label: 'No local items'
    })
  })

  it('keeps retryable failed local browse reads as failed disclosure branches', () => {
    const albums = item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums')
    const albumsNodeId = localBrowseItemNodeId(albums.identity.resolvedItemPath)
    const projection = projectTree(
      stateWithMusicWindow([albums], {
        childStates: new Map([
          [
            localBrowseStateKey(albums.identity.resolvedItemPath),
            {
              kind: 'failed',
              detail: 'Unable to read local browse items.',
              errorCode: 'readFailed'
            }
          ]
        ])
      })
    )
    const visible = visibleItemFor(
      projection,
      albumsNodeId,
      new Set([addSourceSectionNodeId, localBrowseEntryNodeId('C:\\Users\\Maikel\\Music')])
    )

    expect(visible).toMatchObject({
      canRevealChildren: true,
      canActivateAction: true,
      isBranch: true
    })
    expect(findNode(projection, 'Albums')).toMatchObject({
      children: { kind: 'failed' },
      action: { kind: 'loadChildren', state: { kind: 'failed' } }
    })
  })

  it('does not expose disclosure for unavailable local browse item rows', () => {
    const rows = [
      item('directory', 'Rejected', 'C:\\Users\\Maikel\\Music\\Rejected', { status: 'rejected' }),
      item('directory', 'Blocked', 'C:\\Users\\Maikel\\Music\\Blocked', {
        status: 'permissionBlocked'
      }),
      item('directory', 'Admitted', 'C:\\Users\\Maikel\\Music\\Admitted', {
        status: 'duplicateOfAdmittedSource',
        matchedSourceId: '7'
      }),
      item('directory', 'Removed', 'C:\\Users\\Maikel\\Music\\Removed', {
        status: 'restorableSource',
        matchedSourceId: '7'
      }),
      item('rejectedRoot', 'Protected Root', 'C:\\Users\\Maikel\\Music\\Protected Root', {
        status: 'rejected',
        availableOperations: [{ kind: 'browseChildren' }]
      })
    ]
    const projection = projectTree(stateWithMusicWindow(rows))
    const visibleItems = visibleItemsFor(
      projection,
      new Set([addSourceSectionNodeId, localBrowseEntryNodeId('C:\\Users\\Maikel\\Music')])
    )

    for (const row of rows) {
      const nodeId = localBrowseItemNodeId(row.identity.resolvedItemPath)
      const binding = projection.bindingsById.get(nodeId)

      expect(visibleItems.find((visible) => visible.id === nodeId)).toMatchObject({
        canRevealChildren: false,
        isBranch: false
      })
      expect(findNode(projection, row.displayName)).toMatchObject({
        children: { kind: 'none' }
      })
      expect(binding).toMatchObject({ kind: 'localBrowseItem' })
      expect(binding).not.toHaveProperty('target')
    }
  })

  it('does not expose disclosure for unavailable local browse entry points', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            status: 'duplicateOfAdmittedSource',
            availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
          })
        ]
      })
    )
    const musicNodeId = localBrowseEntryNodeId('C:\\Users\\Maikel\\Music')

    expect(
      visibleItemFor(projection, musicNodeId, new Set([addSourceSectionNodeId]))
    ).toMatchObject({
      canRevealChildren: false,
      isBranch: false
    })
    expect(findNode(projection, 'Music')).toMatchObject({
      children: { kind: 'none' }
    })
  })

  it('does not reuse a loaded window from another Add Source view', () => {
    const rootTarget = {
      addSourceView: 'preview' as const,
      entryPointKind: 'music' as const,
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }
    const projection = projectTree({
      ...browserState({ entries: [musicEntryPoint()] }),
      addSourceView: 'inventory',
      localBrowseItemStates: new Map([
        [
          'preview:music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic',
          {
            kind: 'loaded',
            window: {
              addSourceView: rootTarget.addSourceView,
              identity: {
                entryPointKind: rootTarget.entryPointKind,
                resolvedRootPath: rootTarget.resolvedRootPath,
                resolvedParentPath: rootTarget.resolvedParentPath
              },
              label: rootTarget.label,
              items: [item('mediaFile', 'track.flac', 'C:\\Users\\Maikel\\Music\\track.flac')],
              totalItems: 1,
              status: 'complete',
              failure: null,
              limit: 50
            }
          }
        ]
      ])
    })
    const music =
      projection.nodes[0]?.children.kind === 'loaded'
        ? projection.nodes[0].children.nodes[0]
        : undefined

    expect(firstLoadedChildLabels(music)).toEqual([])
    expect(music?.children.kind).toBe('deferred')
  })

  it('keeps Add Source rows out of the admitted library projection', () => {
    const projection = projectState(
      browserState({
        entries: [musicEntryPoint()]
      })
    )

    expect(projection?.kind).toBe('tree')
    expect(projection?.nodes.map((node) => node.id)).toEqual(['read-state:navigation'])
    expect(projection?.bindingsById.has(addSourceSectionNodeId)).toBe(false)
    expect(
      [...(projection?.bindingsById.values() ?? [])].some((binding) =>
        binding.kind.startsWith('localBrowse')
      )
    ).toBe(false)
  })
})

function projectTree(state: BrowserState): BrowserProjection {
  const projection = projectAddSourceState({
    addSourceView: state.addSourceView ?? 'preview',
    entryPointsState: state.localBrowseEntryPointsState,
    itemStates: state.localBrowseItemStates
  })

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected tree projection.')
  }

  return projection
}

function browserState(options: {
  readonly entries: readonly LocalBrowseEntryPoint[]
}): BrowserState {
  return {
    navigationReadResult: emptyNavigation(),
    sourceReadStates: new Map(),
    directoryReadStates: new Map(),
    localBrowseEntryPointsState: {
      kind: 'ready',
      result: {
        state: 'read',
        status: 'complete',
        entries: options.entries,
        failure: null
      }
    }
  }
}

function stateWithMusicWindow(
  items: readonly LocalBrowseItem[],
  options: {
    readonly childStates?: ReadonlyMap<string, LocalBrowseItemState>
  } = {}
): BrowserState {
  return {
    ...browserState({ entries: [musicEntryPoint()] }),
    localBrowseItemStates: new Map([
      [
        localBrowseStateKey('C:\\Users\\Maikel\\Music'),
        {
          kind: 'loaded',
          window: loadedWindow({
            label: 'Music',
            resolvedParentPath: 'C:\\Users\\Maikel\\Music',
            items
          })
        }
      ],
      ...(options.childStates ?? new Map())
    ])
  }
}

function loadedWindow(options: {
  readonly label: string
  readonly resolvedParentPath: string
  readonly items: readonly LocalBrowseItem[]
}): LoadedLocalBrowseItems {
  return {
    addSourceView: 'preview',
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: options.resolvedParentPath
    },
    label: options.label,
    items: options.items,
    totalItems: options.items.length,
    status: 'complete',
    failure: null,
    limit: 50
  }
}

function emptyNavigation(): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: []
  }
}

function musicEntryPoint(overrides: Partial<LocalBrowseEntryPoint> = {}): LocalBrowseEntryPoint {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedPath: 'C:\\Users\\Maikel\\Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    availableOperations: defaultMusicEntryOperations(),
    failure: null,
    ...overrides
  }
}

function entryPoint(
  entryPointKind: LocalBrowseEntryPointKind,
  displayName: string,
  resolvedPath: string
): LocalBrowseEntryPoint {
  return musicEntryPoint({
    identity: {
      entryPointKind,
      resolvedPath
    },
    displayName,
    availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
  })
}

function item(
  itemKind: LocalBrowseItemKind,
  displayName: string,
  resolvedItemPath: string,
  overrides: Partial<LocalBrowseItem> = {}
): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedItemPath
    },
    itemKind,
    displayName,
    status: 'available',
    platform: 'windows',
    fileKind: null,
    mediaRelevance: null,
    availableOperations:
      itemKind === 'directory'
        ? [
            { kind: 'browseChildren' },
            { kind: 'chooseDescendant' },
            {
              kind: 'requestSourceAdmission',
              requestKind: 'selectedDirectory',
              resolvedPath: resolvedItemPath
            }
          ]
        : [],
    failure: null,
    ...overrides
  }
}

function defaultMusicEntryOperations(): LocalBrowseEntryPoint['availableOperations'] {
  return [
    { kind: 'browseChildren' },
    { kind: 'chooseDescendant' },
    {
      kind: 'requestSourceAdmission',
      requestKind: 'defaultMusicFolder',
      resolvedPath: 'C:\\Users\\Maikel\\Music'
    }
  ]
}

function firstLoadedChildLabels(
  node: BrowserProjection['nodes'][number] | undefined
): readonly string[] {
  return node?.children.kind === 'loaded' ? node.children.nodes.map((child) => child.label) : []
}

function findNodeDetail(projection: BrowserProjection, label: string): string | undefined {
  const nodes = [...projection.nodes]

  while (nodes.length > 0) {
    const node = nodes.shift()
    if (node?.label === label) {
      return node.detail
    }
    if (node?.children.kind === 'loaded') {
      nodes.push(...node.children.nodes)
    }
  }

  return undefined
}

function findNode(
  projection: BrowserProjection,
  label: string
): BrowserProjection['nodes'][number] | undefined {
  const nodes = [...projection.nodes]

  while (nodes.length > 0) {
    const node = nodes.shift()
    if (node?.label === label) {
      return node
    }
    if (node?.children.kind === 'loaded') {
      nodes.push(...node.children.nodes)
    }
  }

  return undefined
}

function childItemsFor(
  visibleItems: ReturnType<typeof flattenVisibleTree>,
  parentId: string
): ReturnType<typeof flattenVisibleTree> {
  return visibleItems.filter((item) => item.parentId === parentId)
}

function childLabelsFor(
  visibleItems: ReturnType<typeof flattenVisibleTree>,
  parentId: string
): readonly string[] {
  return childItemsFor(visibleItems, parentId).map((item) => item.node.label)
}

function visibleItemsFor(
  projection: BrowserProjection,
  expandedNodeIds: ReadonlySet<string>
): ReturnType<typeof flattenVisibleTree> {
  return flattenVisibleTree({
    nodes: projection.nodes,
    expandedNodeIds
  })
}

function visibleItemFor(
  projection: BrowserProjection,
  nodeId: string,
  expandedNodeIds: ReadonlySet<string>
): ReturnType<typeof flattenVisibleTree>[number] {
  const visibleItem = visibleItemsFor(projection, expandedNodeIds).find(
    (item) => item.id === nodeId
  )

  expect(visibleItem).toBeDefined()
  if (visibleItem === undefined) {
    throw new Error(`Expected visible item ${nodeId}.`)
  }

  return visibleItem
}

function localBrowseStateKey(resolvedParentPath: string): string {
  return `preview:music:C%3A%5CUsers%5CMaikel%5CMusic:${encodeURIComponent(resolvedParentPath)}`
}

function localBrowseEntryNodeId(resolvedPath: string): string {
  return `local-browse-entry:music:${encodeURIComponent(resolvedPath)}`
}

function localBrowseItemNodeId(resolvedItemPath: string): string {
  return `local-browse-item:music:C%3A%5CUsers%5CMaikel%5CMusic:${encodeURIComponent(
    resolvedItemPath
  )}`
}
