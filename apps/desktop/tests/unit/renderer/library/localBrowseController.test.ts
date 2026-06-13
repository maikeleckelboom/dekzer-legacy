import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import {
  createLocalBrowseController,
  localBrowseWindowKeyForIdentity,
  type LocalBrowseReadApi
} from '../../../../src/renderer/library/localBrowse/controller'
import { localBrowseWindowKey } from '../../../../src/renderer/library/localBrowse/types'
import type { LocalPreviewMode } from '../../../../src/renderer/library/localBrowse/previewMode'
import type { BrowserState } from '../../../../src/renderer/library/state'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import type { LocalBrowseEntryPoint } from '../../../../src/shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  ReadLocalBrowseItemsRequest
} from '../../../../src/shared/library/localBrowse/items'

describe('createLocalBrowseController', () => {
  it('keys local browse windows by local preview mode', () => {
    const target = {
      previewMode: 'advancedInventory' as const,
      entryPointKind: 'music' as const,
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }

    expect(localBrowseWindowKey(target)).toBe(
      'advancedInventory:music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic'
    )
    expect(localBrowseWindowKeyForIdentity(target, 'advancedInventory')).toBe(
      localBrowseWindowKey(target)
    )
  })

  it('reads entry points and item windows with boundary identities', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const readItems = vi.fn(async (request: ReadLocalBrowseItemsRequest) => {
      itemRequests.push(structuredClone(request))

      if (request.resolvedParentPath.endsWith('Albums')) {
        return {
          state: 'read' as const,
          status: 'complete' as const,
          windowIdentity: {
            entryPointKind: request.entryPointKind,
            resolvedRootPath: request.resolvedRootPath,
            resolvedParentPath: request.resolvedParentPath
          },
          offset: request.offset,
          limit: request.limit,
          totalItems: 0,
          items: [],
          failure: null
        }
      }

      return {
        state: 'read' as const,
        status: 'complete' as const,
        windowIdentity: {
          entryPointKind: request.entryPointKind,
          resolvedRootPath: request.resolvedRootPath,
          resolvedParentPath: request.resolvedParentPath
        },
        offset: request.offset,
        limit: request.limit,
        totalItems: 1,
        items: [directoryItem()],
        failure: null
      }
    })
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems
      })
    )
    controller.start()

    await expect(controller.refreshEntryPoints()).resolves.toBe(true)

    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    expect(itemRequests[0]).toEqual({
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      profile: 'audio',
      offset: 0,
      limit: 50
    })

    projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const albumsNodeId = requiredNodeIdByLabel(projection, 'Albums')

    await expect(controller.requestNodeChildren(albumsNodeId, projection)).resolves.toBe(true)
    expect(itemRequests[1]).toEqual({
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
      profile: 'audio',
      offset: 0,
      limit: 50
    })
  })

  it('uses selected local preview mode for reads and preserves it for read-more', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const localPreviewMode = ref<LocalPreviewMode>('advancedInventory')
    const readItems = vi.fn(async (request: ReadLocalBrowseItemsRequest) => {
      itemRequests.push(structuredClone(request))

      return {
        state: 'read' as const,
        status: 'complete' as const,
        windowIdentity: {
          entryPointKind: request.entryPointKind,
          resolvedRootPath: request.resolvedRootPath,
          resolvedParentPath: request.resolvedParentPath
        },
        offset: request.offset,
        limit: request.limit,
        totalItems: request.offset === 0 ? 2 : 2,
        items:
          request.offset === 0
            ? [directoryItem()]
            : [
                {
                  ...directoryItem(),
                  displayName: 'More',
                  identity: {
                    ...directoryItem().identity,
                    resolvedItemPath: 'C:\\Users\\Maikel\\Music\\More'
                  }
                }
              ],
        failure: null
      }
    })
    const controller = createLocalBrowseController(testLocalBrowseApi({ readItems }), {
      localPreviewMode
    })
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')
    await controller.requestNodeChildren(musicNodeId, projection)

    projection = projectTree({
      localPreviewMode: 'advancedInventory',
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    await controller.requestNodeMore(musicNodeId, projection)

    expect(itemRequests.map((request) => request.profile)).toEqual(['allFiles', 'allFiles'])
    expect(itemRequests[1]).toMatchObject({
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      offset: 1
    })

    localPreviewMode.value = 'musicEvidence'
    await controller.requestNodeChildren(musicNodeId, projection)
    expect(itemRequests[2]?.profile).toBe('audio')
  })
})

function testLocalBrowseApi(
  overrides: Partial<LocalBrowseReadApi['localBrowse']> = {}
): LocalBrowseReadApi {
  return {
    localBrowse: {
      readEntryPoints: async () => ({
        state: 'read',
        status: 'complete',
        entries: [musicEntryPoint()],
        failure: null
      }),
      readItems: async () => ({
        state: 'read',
        status: 'complete',
        windowIdentity: {
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music'
        },
        offset: 0,
        limit: 50,
        totalItems: 0,
        items: [],
        failure: null
      }),
      ...overrides
    }
  }
}

function musicEntryPoint(): LocalBrowseEntryPoint {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedPath: 'C:\\Users\\Maikel\\Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    availableOperations: [
      { kind: 'browseChildren' },
      { kind: 'chooseDescendant' },
      {
        kind: 'requestSourceAdmission',
        requestKind: 'defaultMusicFolder',
        resolvedPath: 'C:\\Users\\Maikel\\Music'
      }
    ],
    failure: null
  }
}

function directoryItem(): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedItemPath: 'C:\\Users\\Maikel\\Music\\Albums'
    },
    itemKind: 'directory',
    displayName: 'Albums',
    status: 'available',
    platform: 'windows',
    fileKind: null,
    mediaRelevance: null,
    availableOperations: [
      { kind: 'browseChildren' },
      { kind: 'chooseDescendant' },
      {
        kind: 'requestSourceAdmission',
        requestKind: 'selectedDirectory',
        resolvedPath: 'C:\\Users\\Maikel\\Music\\Albums'
      }
    ],
    failure: null
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

function requiredNodeIdByLabel(projection: BrowserProjection, label: string): string {
  const nodes = [...projection.nodes]

  while (nodes.length > 0) {
    const node = nodes.shift()
    if (node?.label === label) {
      return node.id
    }
    if (node?.children.kind === 'loaded') {
      nodes.push(...node.children.nodes)
    }
  }

  throw new Error(`Expected node labelled ${label}.`)
}
