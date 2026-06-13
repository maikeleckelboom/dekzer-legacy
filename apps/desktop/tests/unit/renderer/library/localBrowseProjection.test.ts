import { describe, expect, it } from 'vitest'

import type { BrowserState } from '../../../../src/renderer/library/state'
import {
  projectAddSourceState,
  addSourceSectionNodeId
} from '../../../../src/renderer/library/localBrowse/projection'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
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
