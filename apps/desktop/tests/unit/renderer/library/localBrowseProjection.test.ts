import { describe, expect, it } from 'vitest'

import type { BrowserState } from '../../../../src/renderer/library/state'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type { LocalBrowseEntryPoint } from '../../../../src/shared/library/localBrowse/entryPoints'
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

    expect(projection.nodes.map((node) => node.id)).toEqual(['local-browse:section'])
    expect(firstLoadedChildLabels(projection.nodes[0])).toEqual(['Music'])
    expect(projection.bindingsById.get('local-browse:section')).toEqual({
      kind: 'localBrowseSection'
    })
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

  it('preserves the default music admission action from the boundary', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            admissionAction: 'requestDefaultMusicFolderAdmission'
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
        identity: { entryPointKind: 'music', canonicalPath: 'C:\\Users\\Maikel\\Music' },
        admissionAction: 'requestDefaultMusicFolderAdmission'
      }
    })
  })

  it('does not project duplicate entry points as fresh admission candidates', () => {
    const projection = projectTree(
      browserState({
        entries: [
          musicEntryPoint({
            status: 'duplicateOfAdmittedSource',
            admissionAction: null,
            availableActions: {
              canBrowse: true,
              canRequestAdmission: false,
              canChooseDescendant: true,
              canRequestParentAdmission: false
            }
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
        admissionAction: null,
        availableActions: {
          canRequestAdmission: false
        }
      }
    })
    expect(findNodeDetail(projection, 'Music')).toBe('Already added as a library source.')
  })

  it('projects local browse child row kinds without source bindings', () => {
    const rootTarget = {
      entryPointKind: 'music' as const,
      rootCanonicalPath: 'C:\\Users\\Maikel\\Music',
      parentCanonicalPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }
    const projection = projectTree({
      ...browserState({ entries: [musicEntryPoint()] }),
      localBrowseItemStates: new Map([
        [
          'music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic',
          {
            kind: 'loaded',
            window: {
              identity: {
                entryPointKind: rootTarget.entryPointKind,
                rootCanonicalPath: rootTarget.rootCanonicalPath,
                parentCanonicalPath: rootTarget.parentCanonicalPath
              },
              label: rootTarget.label,
              items: [
                item('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums'),
                item('mediaFile', 'track.flac', 'C:\\Users\\Maikel\\Music\\track.flac', {
                  fileKind: 'audio',
                  mediaRelevance: 'mediaRelevant',
                  admissionAction: 'requestParentAdmission',
                  availableActions: {
                    canBrowse: false,
                    canRequestAdmission: false,
                    canChooseDescendant: false,
                    canRequestParentAdmission: true
                  }
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
})

function projectTree(state: BrowserState): BrowserProjection {
  const projection = projectState(state)

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
      canonicalPath: 'C:\\Users\\Maikel\\Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    admissionAction: 'requestDefaultMusicFolderAdmission',
    availableActions: {
      canBrowse: true,
      canRequestAdmission: true,
      canChooseDescendant: true,
      canRequestParentAdmission: false
    },
    failure: null,
    ...overrides
  }
}

function item(
  itemKind: LocalBrowseItemKind,
  displayName: string,
  itemCanonicalPath: string,
  overrides: Partial<LocalBrowseItem> = {}
): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      rootCanonicalPath: 'C:\\Users\\Maikel\\Music',
      itemCanonicalPath
    },
    itemKind,
    displayName,
    status: 'available',
    platform: 'windows',
    fileKind: null,
    mediaRelevance: null,
    admissionAction: itemKind === 'directory' ? 'requestAdmission' : null,
    availableActions: {
      canBrowse: itemKind === 'directory',
      canRequestAdmission: itemKind === 'directory',
      canChooseDescendant: itemKind === 'directory',
      canRequestParentAdmission: false
    },
    failure: null,
    ...overrides
  }
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
