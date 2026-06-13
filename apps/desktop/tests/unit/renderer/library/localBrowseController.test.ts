import { describe, expect, it, vi } from 'vitest'

import {
  createLocalBrowseController,
  type LocalBrowseReadApi
} from '../../../../src/renderer/library/localBrowse/controller'
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
      profile: 'audioBrowse',
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
      profile: 'audioBrowse',
      offset: 0,
      limit: 50
    })
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
