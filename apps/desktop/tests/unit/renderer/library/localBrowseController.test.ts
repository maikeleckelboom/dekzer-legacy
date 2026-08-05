import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

import {
  createLocalBrowseController,
  localBrowseWindowKeyForIdentity,
  type LocalBrowseReadApi
} from '../../../../src/renderer/library/localBrowse/controller'
import {
  localBrowseWindowKey,
  type LoadedLocalBrowseItems
} from '../../../../src/renderer/library/localBrowse/types'
import type { AddSourceView } from '../../../../src/renderer/library/addSource/view'
import type { BrowserState } from '../../../../src/renderer/library/state'
import { projectAddSourceState } from '../../../../src/renderer/library/localBrowse/projection'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { LocalBrowseEntryPoint } from '../../../../src/shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  ReadLocalBrowseItemsRequest
} from '../../../../src/shared/library/localBrowse/items'

describe('createLocalBrowseController', () => {
  it('keys local browse windows by Add Source view', () => {
    const target = {
      addSourceView: 'inventory' as const,
      entryPointKind: 'music' as const,
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      label: 'Music'
    }

    expect(localBrowseWindowKey(target)).toBe(
      'inventory:music:C%3A%5CUsers%5CMaikel%5CMusic:C%3A%5CUsers%5CMaikel%5CMusic'
    )
    expect(localBrowseWindowKeyForIdentity(target, 'inventory')).toBe(localBrowseWindowKey(target))
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
      itemFilter: 'audio',
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
      itemFilter: 'audio',
      offset: 0,
      limit: 50
    })
  })

  it('uses selected Add Source view for reads and preserves it for read-more', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const addSourceView = ref<AddSourceView>('inventory')
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
      addSourceView
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
      addSourceView: 'inventory',
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    await controller.requestNodeMore(musicNodeId, projection)

    expect(itemRequests.map((request) => request.itemFilter)).toEqual(['allFiles', 'allFiles'])
    expect(itemRequests[1]).toMatchObject({
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      offset: 1
    })

    addSourceView.value = 'preview'
    await controller.requestNodeChildren(musicNodeId, projection)
    expect(itemRequests[2]?.itemFilter).toBe('audio')
  })

  it('clears loaded item windows so removed sources do not remain active duplicates', async () => {
    const controller = createLocalBrowseController(testLocalBrowseApi())
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    expect(controller.itemStates.value.size).toBe(1)

    controller.clearItemWindows()

    expect(controller.itemStates.value.size).toBe(0)
  })

  it('ignores stale in-flight item reads after item windows are cleared', async () => {
    const pendingRead =
      deferred<Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>>>()
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: vi.fn(() => pendingRead.promise)
      })
    )
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')
    const read = controller.requestNodeChildren(musicNodeId, projection)

    expect(controller.itemStates.value.size).toBe(1)
    controller.clearItemWindows()
    pendingRead.resolve({
      state: 'read',
      status: 'complete',
      windowIdentity: {
        entryPointKind: 'music',
        resolvedRootPath: 'C:\\Users\\Maikel\\Music',
        resolvedParentPath: 'C:\\Users\\Maikel\\Music'
      },
      offset: 0,
      limit: 50,
      totalItems: 1,
      items: [
        {
          ...directoryItem(),
          status: 'duplicateOfAdmittedSource',
          matchedSourceId: '7'
        }
      ],
      failure: null
    })

    await expect(read).resolves.toBe(false)
    expect(controller.itemStates.value.size).toBe(0)
  })

  it('warms child folder windows with Add Source view-specific identities', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const addSourceView = ref<AddSourceView>('preview')
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          if (request.resolvedParentPath.endsWith('Albums')) {
            return localBrowseItemsResult(request, [])
          }

          return localBrowseItemsResult(request, [
            directoryItem(),
            fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\track.flac')
          ])
        }
      }),
      {
        addSourceView,
        warmup: { enabled: true }
      }
    )
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests.map((request) => request.itemFilter)).toEqual(['audio', 'audio'])
    expect(
      controller.itemStates.value.get(
        localBrowseWindowKey({
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
          label: 'Albums'
        })
      )
    ).toMatchObject({
      kind: 'loaded',
      window: {
        items: []
      }
    })

    addSourceView.value = 'inventory'
    projection = projectTree({
      addSourceView: 'inventory',
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests.map((request) => request.itemFilter)).toEqual([
      'audio',
      'audio',
      'allFiles',
      'allFiles'
    ])
    expect(
      controller.itemStates.value.get(
        localBrowseWindowKey({
          addSourceView: 'inventory',
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
          label: 'Albums'
        })
      )
    ).toMatchObject({
      kind: 'loaded',
      window: {
        addSourceView: 'inventory',
        items: []
      }
    })
  })

  it('coalesces explicit child folder opens with active warm reads', async () => {
    const pendingWarmRead =
      deferred<Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>>>()
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    let warmRequest: ReadLocalBrowseItemsRequest | undefined
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          if (request.resolvedParentPath.endsWith('Albums')) {
            warmRequest = structuredClone(request)
            return pendingWarmRead.promise
          }

          return localBrowseItemsResult(request, [directoryItem()])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const albumsNodeId = requiredNodeIdByLabel(projection, 'Albums')
    const albumsKey = localBrowseWindowKey({
      addSourceView: 'preview',
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
      label: 'Albums'
    })

    const explicitRead = controller.requestNodeChildren(albumsNodeId, projection)
    await waitForMicrotasks()

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Albums'
    ])
    expect(controller.itemStates.value.get(albumsKey)).toBeUndefined()
    const duringProjection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    expect(requiredNodeByLabel(duringProjection, 'Albums').children.kind).toBe('deferred')

    if (warmRequest === undefined) {
      throw new Error('Expected active warm read request.')
    }
    pendingWarmRead.resolve(
      localBrowseItemsResult(warmRequest, [
        fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\Albums\\track.flac')
      ])
    )
    await expect(explicitRead).resolves.toBe(true)

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Albums'
    ])
    expect(controller.itemStates.value.get(albumsKey)).toMatchObject({
      kind: 'loaded',
      window: {
        addSourceView: 'preview',
        items: [{ displayName: 'track.flac' }]
      }
    })
  })

  it('falls back to explicit local browse read when an adopted warm read fails', async () => {
    const pendingWarmRead =
      deferred<Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>>>()
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    let albumsReadCount = 0
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          if (!request.resolvedParentPath.endsWith('Albums')) {
            return localBrowseItemsResult(request, [directoryItem()])
          }

          albumsReadCount += 1
          return albumsReadCount === 1
            ? pendingWarmRead.promise
            : localBrowseItemsResult(request, [
                fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\Albums\\track.flac')
              ])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const albumsNodeId = requiredNodeIdByLabel(projection, 'Albums')
    const explicitRead = controller.requestNodeChildren(albumsNodeId, projection)
    await waitForMicrotasks()

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Albums'
    ])

    pendingWarmRead.resolve({
      state: 'readFailed',
      error: { code: 'readFailed', message: 'Warm read failed.' }
    })
    await expect(explicitRead).resolves.toBe(true)

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Albums',
      'C:\\Users\\Maikel\\Music\\Albums'
    ])
  })

  it('keeps accepted local browse children visible while a same-branch read is pending', async () => {
    const pendingAlbumsRead =
      deferred<Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>>>()
    let albumsReadCount = 0
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          if (!request.resolvedParentPath.endsWith('Albums')) {
            return localBrowseItemsResult(request, [directoryItem()])
          }

          albumsReadCount += 1
          return albumsReadCount === 1
            ? localBrowseItemsResult(request, [
                fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\Albums\\track.flac')
              ])
            : pendingAlbumsRead.promise
        }
      })
    )
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const albumsNodeId = requiredNodeIdByLabel(projection, 'Albums')

    await expect(controller.requestNodeChildren(albumsNodeId, projection)).resolves.toBe(true)
    expect(firstLoadedChildLabels(treeNodes(controller), 'Albums')).toEqual(['track.flac'])

    const refresh = controller.requestNodeChildren(albumsNodeId, projection)
    await waitForMicrotasks()

    expect(controller.itemStates.value.get(albumsWindowKey())).toMatchObject({
      kind: 'refreshing',
      window: {
        items: [{ displayName: 'track.flac' }]
      }
    })
    expect(firstLoadedChildLabels(treeNodes(controller), 'Albums')).toEqual(['track.flac'])
    expect(requiredNodeByLabel(projectFromController(controller), 'Albums').children.kind).toBe(
      'loaded'
    )

    pendingAlbumsRead.resolve(
      localBrowseItemsResult(
        {
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
          itemFilter: 'audio',
          offset: 0,
          limit: 50
        },
        [fileItem('fresh.flac', 'C:\\Users\\Maikel\\Music\\Albums\\fresh.flac')]
      )
    )
    await expect(refresh).resolves.toBe(true)

    expect(firstLoadedChildLabels(treeNodes(controller), 'Albums')).toEqual(['fresh.flac'])
  })

  it('does not warm terminal file rows', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          return localBrowseItemsResult(request, [
            fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\track.flac', {
              availableOperations: [{ kind: 'browseChildren' }]
            })
          ])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests).toHaveLength(1)
  })

  it('respects the local browse warmup breadth bound', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const childDirectories = Array.from({ length: 25 }, (_, index) =>
      directoryNamed(`Folder ${index + 1}`, `C:\\Users\\Maikel\\Music\\Folder ${index + 1}`)
    )
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          return localBrowseItemsResult(
            request,
            request.resolvedParentPath === 'C:\\Users\\Maikel\\Music' ? childDirectories : []
          )
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests).toHaveLength(25)
    expect(itemRequests.slice(1).map((request) => request.resolvedParentPath)).toEqual(
      childDirectories.slice(0, 24).map((item) => item.identity.resolvedItemPath)
    )
    expect(itemRequests.map((request) => request.resolvedParentPath)).not.toContain(
      'C:\\Users\\Maikel\\Music\\Folder 25'
    )
  })

  it('does not warm rejected, protected, blocked, or already-admitted rows', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          return localBrowseItemsResult(request, [
            {
              ...directoryNamed('Rejected', 'C:\\Users\\Maikel\\Music\\Rejected'),
              status: 'rejected'
            },
            {
              ...directoryNamed('Blocked', 'C:\\Users\\Maikel\\Music\\Blocked'),
              status: 'permissionBlocked'
            },
            {
              ...directoryNamed('Admitted', 'C:\\Users\\Maikel\\Music\\Admitted'),
              status: 'duplicateOfAdmittedSource',
              matchedSourceId: '7'
            },
            {
              ...directoryNamed('Available', 'C:\\Users\\Maikel\\Music\\Available')
            }
          ])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Available'
    ])
  })

  it('does not duplicate warm reads for loaded, loading, or refreshing child folders', async () => {
    const itemRequests: ReadLocalBrowseItemsRequest[] = []
    const loadedKey = localBrowseWindowKey({
      addSourceView: 'preview',
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Loaded',
      label: 'Loaded'
    })
    const loadingKey = localBrowseWindowKey({
      addSourceView: 'preview',
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Loading',
      label: 'Loading'
    })
    const refreshingKey = localBrowseWindowKey({
      addSourceView: 'preview',
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Refreshing',
      label: 'Refreshing'
    })
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          itemRequests.push(structuredClone(request))

          return localBrowseItemsResult(request, [
            directoryNamed('Loaded', 'C:\\Users\\Maikel\\Music\\Loaded'),
            directoryNamed('Loading', 'C:\\Users\\Maikel\\Music\\Loading'),
            directoryNamed('Refreshing', 'C:\\Users\\Maikel\\Music\\Refreshing'),
            directoryNamed('Fresh', 'C:\\Users\\Maikel\\Music\\Fresh')
          ])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.itemStates.value = new Map([
      [
        loadedKey,
        {
          kind: 'loaded',
          window: loadedWindowForKey('Loaded', 'C:\\Users\\Maikel\\Music\\Loaded', [])
        }
      ],
      [
        loadingKey,
        {
          kind: 'loading',
          requestKey: loadingKey,
          sequence: 1,
          detail: 'Loading local browse items.'
        }
      ],
      [
        refreshingKey,
        {
          kind: 'refreshing',
          window: loadedWindowForKey('Refreshing', 'C:\\Users\\Maikel\\Music\\Refreshing', []),
          requestKey: refreshingKey,
          sequence: 2,
          detail: 'Refreshing local browse items.'
        }
      ]
    ])
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    expect(itemRequests.map((request) => request.resolvedParentPath)).toEqual([
      'C:\\Users\\Maikel\\Music',
      'C:\\Users\\Maikel\\Music\\Fresh'
    ])
  })

  it('projects warmed child rows and empty warm results without cold loading', async () => {
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          if (request.resolvedParentPath.endsWith('Albums')) {
            return localBrowseItemsResult(request, [
              fileItem('track.flac', 'C:\\Users\\Maikel\\Music\\Albums\\track.flac')
            ])
          }

          if (request.resolvedParentPath.endsWith('Empty')) {
            return localBrowseItemsResult(request, [])
          }

          return localBrowseItemsResult(request, [
            directoryNamed('Albums', 'C:\\Users\\Maikel\\Music\\Albums'),
            directoryNamed('Empty', 'C:\\Users\\Maikel\\Music\\Empty')
          ])
        }
      }),
      { warmup: { enabled: true } }
    )
    controller.start()

    await controller.refreshEntryPoints()
    let projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    await waitForWarmupQueue()

    projection = projectFromController(controller)
    expect(firstLoadedChildLabels([requiredNodeByLabel(projection, 'Albums')], 'Albums')).toEqual([
      'track.flac'
    ])
    expect(requiredNodeByLabel(projection, 'Empty').children).toMatchObject({
      kind: 'loaded',
      nodes: [
        {
          role: 'state',
          label: 'No local items',
          icon: 'state'
        }
      ]
    })
  })

  it('ignores late warm reads after the warmup guard stops allowing work', async () => {
    const pendingWarmRead =
      deferred<Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>>>()
    let allowWarmup = true
    const controller = createLocalBrowseController(
      testLocalBrowseApi({
        readItems: async (request) => {
          if (request.resolvedParentPath.endsWith('Albums')) {
            return pendingWarmRead.promise
          }

          return localBrowseItemsResult(request, [directoryItem()])
        }
      }),
      {
        warmup: {
          enabled: true,
          shouldContinue: () => allowWarmup
        }
      }
    )
    controller.start()

    await controller.refreshEntryPoints()
    const projection = projectTree({
      sourceReadStates: new Map(),
      directoryReadStates: new Map(),
      localBrowseEntryPointsState: controller.entryPointsState.value,
      localBrowseItemStates: controller.itemStates.value
    })
    const musicNodeId = requiredNodeIdByLabel(projection, 'Music')

    await expect(controller.requestNodeChildren(musicNodeId, projection)).resolves.toBe(true)
    allowWarmup = false
    pendingWarmRead.resolve(
      localBrowseItemsResult(
        {
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
          itemFilter: 'audio',
          offset: 0,
          limit: 50
        },
        []
      )
    )
    await waitForWarmupQueue()

    expect(
      controller.itemStates.value.has(
        localBrowseWindowKey({
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:\\Users\\Maikel\\Music',
          resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
          label: 'Albums'
        })
      )
    ).toBe(false)
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
  return directoryNamed('Albums', 'C:\\Users\\Maikel\\Music\\Albums')
}

function directoryNamed(displayName: string, resolvedItemPath: string): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedItemPath
    },
    itemKind: 'directory',
    displayName,
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
        resolvedPath: resolvedItemPath
      }
    ],
    failure: null
  }
}

function fileItem(
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
    itemKind: 'mediaFile',
    displayName,
    status: 'available',
    platform: 'windows',
    fileKind: 'audio',
    mediaRelevance: 'mediaRelevant',
    availableOperations: [],
    failure: null,
    ...overrides
  }
}

function localBrowseItemsResult(
  request: ReadLocalBrowseItemsRequest,
  items: readonly LocalBrowseItem[]
): Awaited<ReturnType<LocalBrowseReadApi['localBrowse']['readItems']>> {
  return {
    state: 'read',
    status: 'complete',
    windowIdentity: {
      entryPointKind: request.entryPointKind,
      resolvedRootPath: request.resolvedRootPath,
      resolvedParentPath: request.resolvedParentPath
    },
    offset: request.offset,
    limit: request.limit,
    totalItems: items.length,
    items,
    failure: null
  }
}

function loadedWindowForKey(
  label: string,
  resolvedParentPath: string,
  items: readonly LocalBrowseItem[]
): LoadedLocalBrowseItems {
  return {
    addSourceView: 'preview',
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedParentPath
    },
    label,
    items,
    totalItems: items.length,
    status: 'complete',
    failure: null,
    limit: 50
  }
}

function projectTree(state: BrowserState): BrowserProjection {
  const projection = projectAddSourceState({
    addSourceView: state.addSourceView ?? 'preview',
    ...(state.localBrowseEntryPointsState === undefined
      ? {}
      : { entryPointsState: state.localBrowseEntryPointsState }),
    ...(state.localBrowseItemStates === undefined
      ? {}
      : { itemStates: state.localBrowseItemStates })
  })

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected tree projection.')
  }

  return projection
}

function requiredNodeIdByLabel(projection: BrowserProjection, label: string): string {
  return requiredNodeByLabel(projection, label).id
}

function requiredNodeByLabel(
  projection: BrowserProjection,
  label: string
): BrowserProjection['nodes'][number] {
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

  throw new Error(`Expected node labelled ${label}.`)
}

function projectFromController(
  controller: ReturnType<typeof createLocalBrowseController>
): BrowserProjection {
  return projectTree({
    sourceReadStates: new Map(),
    directoryReadStates: new Map(),
    localBrowseEntryPointsState: controller.entryPointsState.value,
    localBrowseItemStates: controller.itemStates.value
  })
}

function treeNodes(
  controller: ReturnType<typeof createLocalBrowseController>
): readonly BrowserProjection['nodes'][number][] {
  return projectFromController(controller).nodes
}

function firstLoadedChildLabels(
  nodes: readonly BrowserProjection['nodes'][number][],
  label: string
): readonly string[] {
  const node = findNodeByLabel(nodes, label)
  return node?.children.kind === 'loaded' ? node.children.nodes.map((child) => child.label) : []
}

function findNodeByLabel(
  nodes: readonly BrowserProjection['nodes'][number][],
  label: string
): BrowserProjection['nodes'][number] | undefined {
  const remaining = [...nodes]

  while (remaining.length > 0) {
    const node = remaining.shift()

    if (node?.label === label) {
      return node
    }

    if (node?.children.kind === 'loaded') {
      remaining.push(...node.children.nodes)
    }
  }

  return undefined
}

function albumsWindowKey(): string {
  return localBrowseWindowKey({
    addSourceView: 'preview',
    entryPointKind: 'music',
    resolvedRootPath: 'C:\\Users\\Maikel\\Music',
    resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
    label: 'Albums'
  })
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolveDeferred: (value: T) => void = () => undefined
  const promise = new Promise<T>((resolve) => {
    resolveDeferred = resolve
  })

  return {
    promise,
    resolve: resolveDeferred
  }
}

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

async function waitForWarmupQueue(): Promise<void> {
  for (let index = 0; index < 40; index += 1) {
    await waitForMicrotasks()
  }
}
