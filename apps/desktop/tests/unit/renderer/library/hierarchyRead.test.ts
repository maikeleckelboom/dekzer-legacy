import { describe, expect, it } from 'vitest'

import {
  createLibraryHierarchyReadController,
  type LibraryHierarchyReadApi
} from '../../../../src/renderer/library/boundary/hierarchyRead'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type {
  ChildRow,
  HierarchyCoverage,
  ReadErrorCode,
  ReadRequest,
  ReadResult
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type { NavigationReadRowsResult } from '../../../../src/shared/libraryNavigation/readRows'

describe('createLibraryHierarchyReadController', () => {
  it('refreshes navigation, reads the first source, and loads directory children', async () => {
    const navigationRequests: unknown[] = []
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async (request) => {
          navigationRequests.push(structuredClone(request))
          return navigationSourceReadRowsResult()
        },
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResult('12')
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    expect(navigationRequests).toEqual([{ parentNavigationRowId: null }])
    expect(readRequests[0]).toEqual({
      target: {
        kind: 'entryPoint',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        },
        label: 'Source Fixture'
      },
      offset: 0,
      limit: 50
    })
    expect(controller.currentRoot.value?.id).toBe('source:7')
    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('unloaded')

    expect(firstProjectedDirectoryStateKind(treeNodes(controller), 'source-directory:12')).toBe(
      'deferred'
    )

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(readRequests[1]).toEqual({
      target: {
        kind: 'entryPoint',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        },
        label: 'Source Fixture'
      },
      parentDirectoryId: '12',
      offset: 0,
      limit: 50
    })
    expect(controller.directoryReadStates.value.get('12')).toMatchObject({
      kind: 'loaded'
    })
  })

  it('loads source and directory continuation windows and retries failed directory continuations', async () => {
    const readRequests: ReadRequest[] = []
    let directoryMoreAttempts = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === undefined) {
            return request.offset === 1
              ? sourceMoreReadResult()
              : partialSourceHierarchyReadResult()
          }

          if (request.parentDirectoryId === '12') {
            if (request.offset === 1) {
              directoryMoreAttempts += 1
              return directoryMoreAttempts === 1
                ? hierarchyReadError(
                    'readFailed',
                    'readFailed',
                    'Unable to read more directory children.'
                  )
                : directoryMoreReadResult('12')
            }

            return partialDirectoryHierarchyReadResult('12')
          }

          return emptyDirectoryReadResult(request.parentDirectoryId)
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-directory:12',
      'more:navigation-row:7:1'
    ])

    await expect(controller.requestNodeChildren('more:navigation-row:7:1')).resolves.toBe(true)
    expect(readRequests[1]).toMatchObject({ offset: 1 })
    expect(readRequests[1]).not.toHaveProperty('parentDirectoryId')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-directory:12'
    ])

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'more:source-directory:12:1'
    ])

    await expect(controller.requestNodeChildren('more:source-directory:12:1')).resolves.toBe(true)
    expect(
      findProjectedNode(treeNodes(controller), 'more:source-directory:12:1')?.action
    ).toMatchObject({
      kind: 'loadMore',
      state: { kind: 'failed' }
    })

    await expect(controller.requestNodeChildren('more:source-directory:12:1')).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([])
  })

  it('rejects unexpected continuation windows without replacing loaded rows', async () => {
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          readCount += 1

          if (readCount === 1) {
            return partialSourceHierarchyReadResult()
          }

          return wrongSourceContinuationResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    const beforeState = controller.sourceReadStates.value.get('navigation-row:7')
    expect(beforeState).toMatchObject({ kind: 'loaded' })
    if (beforeState?.kind === 'loaded') {
      expect(beforeState.children.rows).toHaveLength(1)
    }

    await expect(controller.requestNodeChildren('more:navigation-row:7:1')).resolves.toBe(true)

    const afterState = controller.sourceReadStates.value.get('navigation-row:7')
    expect(afterState).toMatchObject({ kind: 'loaded' })
    if (afterState?.kind === 'loaded') {
      expect(afterState.children.rows).toHaveLength(1)
      expect(afterState.children.more).toMatchObject({
        kind: 'failed',
        detail: 'The hierarchy read returned an unexpected child window.'
      })
    }
  })

  it('refresh re-reads navigation and first source after initial load', async () => {
    const readRequests: ReadRequest[] = []
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount++
          return navigationSourceReadRowsResult()
        },
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))
          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(navigationReadCount).toBe(1)
    expect(readRequests).toHaveLength(1)

    await expect(controller.refresh()).resolves.toBe(true)
    expect(navigationReadCount).toBe(2)
    expect(readRequests).toHaveLength(2)

    expect(readRequests[0]).toMatchObject({ offset: 0 })
    expect(readRequests[1]).toMatchObject({ offset: 0 })
  })

  it('refreshNavigationRows rereads navigation without forcing a source child read', async () => {
    const readRequests: ReadRequest[] = []
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          return navigationSourceReadRowsResult()
        },
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))
          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refreshNavigationRows()).resolves.toBe(true)

    expect(navigationReadCount).toBe(1)
    expect(readRequests).toEqual([])
    expect(treeNodes(controller).map((node) => node.id)).toEqual(['navigation-row:7'])
  })

  it('keeps accepted navigation rows and current root visible while navigation refresh is pending', async () => {
    const navigationRefresh = deferred<NavigationReadRowsResult>()
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          return navigationReadCount === 1
            ? navigationSourceReadRowsResult()
            : navigationRefresh.promise
        },
        readChildren: async () => directoryRootHierarchyReadResult()
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(controller.currentRoot.value?.id).toBe('source:7')
    expect(treeNodes(controller).map((node) => node.id)).toEqual(['navigation-row:7'])

    const refresh = controller.refreshNavigationRows()
    await waitForMicrotasks()

    expect(controller.navigationReadIsLoading.value).toBe(true)
    expect(controller.navigationReadResult.value?.state).toBe('ready')
    expect(controller.currentRoot.value?.id).toBe('source:7')
    expect(treeNodes(controller).map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(controller.browserProjection.value?.bindingsById.has('navigation-row:7')).toBe(true)

    navigationRefresh.resolve(navigationTwoSourceReadRowsResult())
    await expect(refresh).resolves.toBe(true)

    expect(treeNodes(controller).map((node) => node.id)).toEqual([
      'navigation-row:7',
      'navigation-row:9'
    ])
  })

  it('retains accepted navigation rows when a navigation refresh returns an error result', async () => {
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          return navigationReadCount === 1
            ? navigationSourceReadRowsResult()
            : {
                state: 'readFailed',
                error: {
                  code: 'readFailed',
                  message: 'Navigation refresh failed.'
                }
              }
        },
        readChildren: async () => directoryRootHierarchyReadResult()
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.refreshNavigationRows()).resolves.toBe(true)

    expect(controller.navigationReadRequestError.value).toBe('Navigation refresh failed.')
    expect(controller.navigationReadResult.value).toMatchObject({
      state: 'ready',
      rows: [{ navigationRowId: '7' }]
    })
    expect(treeNodes(controller).map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(controller.currentRoot.value?.id).toBe('source:7')
  })

  it('retains accepted navigation rows when a navigation refresh request fails', async () => {
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          if (navigationReadCount === 1) {
            return navigationSourceReadRowsResult()
          }
          throw new Error('read failed')
        },
        readChildren: async () => directoryRootHierarchyReadResult()
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.refreshNavigationRows()).resolves.toBe(true)

    expect(controller.navigationReadRequestError.value).toBe(
      'Unable to request library navigation rows.'
    )
    expect(treeNodes(controller).map((node) => node.id)).toEqual(['navigation-row:7'])
    expect(controller.browserProjection.value?.bindingsById.has('navigation-row:7')).toBe(true)
  })

  it('does not let an older navigation refresh overwrite newer accepted rows', async () => {
    const staleNavigationRefresh = deferred<NavigationReadRowsResult>()
    const currentNavigationRefresh = deferred<NavigationReadRowsResult>()
    let navigationReadCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          if (navigationReadCount === 1) {
            return navigationSourceReadRowsResult()
          }
          return navigationReadCount === 2
            ? staleNavigationRefresh.promise
            : currentNavigationRefresh.promise
        },
        readChildren: async () => directoryRootHierarchyReadResult()
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    const staleRefresh = controller.refreshNavigationRows()
    const currentRefresh = controller.refreshNavigationRows()

    currentNavigationRefresh.resolve(navigationTwoSourceReadRowsResult())
    await expect(currentRefresh).resolves.toBe(true)
    expect(treeNodes(controller).map((node) => node.id)).toEqual([
      'navigation-row:7',
      'navigation-row:9'
    ])

    staleNavigationRefresh.resolve(navigationSourceReadRowsResult('Stale Source'))
    await expect(staleRefresh).resolves.toBe(false)
    expect(treeNodes(controller).map((node) => node.id)).toEqual([
      'navigation-row:7',
      'navigation-row:9'
    ])
    expect(treeNodes(controller)[0]?.label).toBe('First Source')
  })

  it('refreshBrowserWindows unions loaded and expanded windows without duplicate reads', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResult('12')
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    readRequests.length = 0

    await expect(
      controller.refreshBrowserWindows(new Set(['navigation-row:7', 'source-directory:12']))
    ).resolves.toBe(true)

    expect(readRequests.map((request) => request.parentDirectoryId ?? 'root')).toEqual([
      'root',
      '12'
    ])
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-directory:99'
    ])
  })

  it('refreshBrowserWindows loads expanded windows where the projection supports them', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))
          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refreshNavigationRows()).resolves.toBe(true)
    expect(controller.sourceReadStates.value.get('navigation-row:7')).toBeUndefined()

    await expect(controller.refreshBrowserWindows(new Set(['navigation-row:7']))).resolves.toBe(
      true
    )

    expect(readRequests).toHaveLength(1)
    expect(readRequests[0]).toMatchObject({
      target: {
        kind: 'entryPoint',
        entryPoint: { kind: 'source', sourceId: '7' }
      },
      offset: 0,
      limit: 50
    })
    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')

    readRequests.length = 0
    await expect(controller.refreshBrowserWindows(new Set(['source-directory:12']))).resolves.toBe(
      true
    )

    expect(readRequests.map((request) => request.parentDirectoryId ?? 'root')).toEqual([
      'root',
      '12'
    ])
  })

  it('load-more requests preserve parentDirectoryId, offset, and limit', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            if (request.offset === 1) {
              return directoryMoreReadResult('12')
            }
            return partialDirectoryHierarchyReadResult('12')
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)

    await expect(controller.requestNodeChildren('more:source-directory:12:1')).resolves.toBe(true)

    const moreRequest = readRequests[2]
    if (moreRequest === undefined) {
      throw new Error('Expected a request for the next directory page.')
    }
    expect(moreRequest).toMatchObject({
      parentDirectoryId: '12',
      offset: 1,
      limit: 50
    })
    expect(moreRequest.target).toMatchObject({
      kind: 'entryPoint',
      entryPoint: { kind: 'source', sourceId: '7' }
    })
  })

  it('keeps loaded directory children visible while a source refresh is pending', async () => {
    const refreshSourceRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          readCount += 1

          if (readCount === 1) {
            return directoryRootHierarchyReadResult()
          }

          return refreshSourceRead.promise
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(readCount).toBe(1)
    const firstChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(firstChildren).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])

    const refreshPromise = controller.refresh()
    await waitForMicrotasks()

    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('refreshing')
    const duringChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(duringChildren).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])

    refreshSourceRead.resolve(directoryRootHierarchyReadResult())
    await expect(refreshPromise).resolves.toBe(true)

    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(afterChildren).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])
  })

  it('keeps loaded directory children visible while a directory refresh is pending', async () => {
    const refreshDirRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readCount += 1

          if (request.parentDirectoryId === '12') {
            if (readCount <= 2) {
              return loadedDirectoryReadResult('12')
            }

            return refreshDirRead.promise
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    const beforeChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(beforeChildren).toEqual(['source-directory:99'])

    const refreshPromise = controller.requestDirectoryChildren('source-directory:12')
    await waitForMicrotasks()

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('refreshing')
    const duringChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(duringChildren).toEqual(['source-directory:99'])

    refreshDirRead.resolve(loadedDirectoryReadResult('12'))
    await expect(refreshPromise).resolves.toBe(true)

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(afterChildren).toEqual(['source-directory:99'])
  })

  it('retains loaded source children when a source refresh fails', async () => {
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          if (request.offset === 0 && request.parentDirectoryId === undefined) {
            return directoryRootHierarchyReadResult()
          }

          return {
            state: 'readFailed',
            error: { code: 'readFailed', message: 'Source refresh failed.' }
          } as ReadResult
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    const firstChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(firstChildren).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])

    await expect(controller.refresh()).resolves.toBe(true)

    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(afterChildren).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])
  })

  it('retains loaded directory children when a directory refresh fails', async () => {
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readCount += 1

          if (request.parentDirectoryId === '12') {
            if (readCount <= 2) {
              return loadedDirectoryReadResult('12')
            }

            return {
              state: 'readFailed',
              error: { code: 'readFailed', message: 'Directory refresh failed.' }
            } as ReadResult
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    const beforeChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(beforeChildren).toEqual(['source-directory:99'])

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(afterChildren).toEqual(['source-directory:99'])
  })

  it('uses loading when no prior children exist and refreshing when they do', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResult('12')
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(readRequests).toHaveLength(1)

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(readRequests).toHaveLength(2)

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(readRequests).toHaveLength(3)
    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('loaded')
  })

  it('source refresh does not clear loaded directory children under a different source', async () => {
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationTwoSourceReadRowsResult(),
        readChildren: async () => {
          if (controller.sourceReadStates.value.has('navigation-row:9')) {
            return directoryRootHierarchyReadResultForSource9()
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestNodeChildren('navigation-row:9')).resolves.toBe(true)

    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])

    await expect(controller.refresh()).resolves.toBe(true)

    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-directory:12',
      'source-directory:13',
      'source-directory:14'
    ])
    expect(controller.sourceReadStates.value.get('navigation-row:9')?.kind).toBe('loaded')
  })

  it('state placeholder node ID remains stable across state transitions', async () => {
    const deferredDirRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readCount += 1

          if (readCount === 1) {
            return directoryRootHierarchyReadResult()
          }

          if (request.parentDirectoryId === '12') {
            return deferredDirRead.promise
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    const deferredNode = findProjectedNode(treeNodes(controller), 'source-directory:12')
    expect(deferredNode?.children.kind).toBe('deferred')
    if (deferredNode?.children.kind === 'deferred') {
      expect(deferredNode.children.stateNode.id).toBe('read-state:source-directory:12')
    }

    const readPromise = controller.requestDirectoryChildren('source-directory:12')
    await waitForMicrotasks()

    const loadingNode = findProjectedNode(treeNodes(controller), 'source-directory:12')
    expect(loadingNode?.children.kind).toBe('loading')
    if (loadingNode?.children.kind === 'loading') {
      expect(loadingNode.children.stateNode.id).toBe('read-state:source-directory:12')
    }

    deferredDirRead.resolve(
      loadedDirectoryReadResult('12') as Extract<ReadResult, { state: 'ready' }>
    )
    await readPromise

    const loadedNode = findProjectedNode(treeNodes(controller), 'source-directory:12')
    expect(loadedNode?.children.kind).toBe('loaded')
  })
})

function testLibraryApi(options: {
  readonly readRows?: LibraryHierarchyReadApi['navigation']['readRows']
  readonly readChildren: (request: ReadRequest) => Promise<ReadResult>
}): LibraryHierarchyReadApi {
  return {
    host: {
      getStatus: async () => ({
        state: 'started',
        environment: 'development',
        binaryPolicy: {
          kind: 'developmentBinary',
          source: 'environmentOverride'
        },
        lastError: null
      }),
      onStatusChanged: () => () => undefined
    },
    navigation: {
      readRows:
        options.readRows ??
        (async () => ({
          state: 'ready',
          rows: []
        }))
    },
    hierarchy: {
      readChildren: options.readChildren
    }
  }
}

function navigationSourceReadRowsResult(displayName = 'Source Fixture'): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: [
      {
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
    ]
  }
}

function navigationTwoSourceReadRowsResult(): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: [
      {
        navigationRowId: '7',
        stableKey: 'source:7',
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: 'First Source',
        siblingPosition: 0,
        selectable: true,
        selectorKind: 'source',
        selectorPayload: '7',
        updatedAtMs: 100,
        rowVersion: '1'
      },
      {
        navigationRowId: '9',
        stableKey: 'source:9',
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: 'Second Source',
        siblingPosition: 1,
        selectable: true,
        selectorKind: 'source',
        selectorPayload: '9',
        updatedAtMs: 200,
        rowVersion: '1'
      }
    ]
  }
}

function directoryRootHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      totalRows: 3,
      coverage: completeCoverage(),
      nodes: [
        directoryNode('12', 'Album'),
        directoryNode('13', 'Empty Album'),
        directoryNode('14', 'Stale Album')
      ]
    }
  }
}

function partialSourceHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [directoryNode('12', 'Album')]
    }
  }
}

function sourceMoreReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 1,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode('99', 'root-track.wav')]
    }
  }
}

function loadedDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [
        fileNode(`${parentDirectoryId}-track`, 'track.wav', parentDirectoryId),
        directoryNode('99', 'Nested Album', parentDirectoryId)
      ]
    }
  }
}

function directoryRootHierarchyReadResultForSource9(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot9(),
      offset: 0,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [
        directoryNode('92', 'Albums', undefined, '9'),
        fileNode('91', 'track9.wav', undefined, 'audio', '9')
      ]
    }
  }
}

function partialDirectoryHierarchyReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode(`${parentDirectoryId}-a`, 'a.wav', parentDirectoryId)]
    }
  }
}

function directoryMoreReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 1,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode(`${parentDirectoryId}-b`, 'b.wav', parentDirectoryId)]
    }
  }
}

function emptyDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 0,
      coverage: completeEmptyCoverage(),
      nodes: []
    }
  }
}

function wrongSourceContinuationResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:999',
        label: 'Wrong Source',
        entryPoint: { kind: 'source', sourceId: '999' }
      },
      offset: 1,
      limit: 50,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode('999', 'intruder.wav')]
    }
  }
}

function hierarchyReadError(
  state: Exclude<ReadResult['state'], 'ready'>,
  code: ReadErrorCode,
  message: string
): ReadResult {
  return {
    state,
    error: { code, message }
  }
}

function sourceRoot(): Extract<ReadResult, { state: 'ready' }>['window']['root'] {
  return {
    id: 'source:7',
    label: 'Source Fixture',
    entryPoint: {
      kind: 'source',
      sourceId: '7'
    }
  }
}

function sourceRoot9(): Extract<ReadResult, { state: 'ready' }>['window']['root'] {
  return {
    id: 'source:9',
    label: 'Second Source',
    entryPoint: {
      kind: 'source',
      sourceId: '9'
    }
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string,
  sourceId = '7'
): Extract<ChildRow, { kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId,
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'scanning',
    navigableChildScopeState: 'hasNavigableChildScopes',
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  parentDirectoryId?: string,
  fileClass: Extract<ChildRow, { kind: 'file' }>['fileClass'] = 'audio',
  sourceId = '7'
): Extract<ChildRow, { kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId,
    fileId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    fileClass,
    presence: 'present',
    updatedAtMs: 101
  }
}

function completeCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    subtreeCoverageComplete: true,
    emptyResultAuthoritative: false
  }
}

function completeEmptyCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    subtreeCoverageComplete: true,
    emptyResultAuthoritative: true
  }
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

function treeNodes(
  controller: ReturnType<typeof createLibraryHierarchyReadController>
): readonly BrowserTreeNode[] {
  const projection = controller.browserProjection.value

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected live hierarchy projection.')
  }

  return projection.nodes
}

function firstProjectedDirectoryStateKind(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): string | undefined {
  return findProjectedNode(nodes, nodeId)?.children.kind
}

function firstLoadedChildIds(nodes: readonly BrowserTreeNode[], nodeId: string): readonly string[] {
  const node = findProjectedNode(nodes, nodeId)

  return node?.children.kind === 'loaded' ? node.children.nodes.map((child) => child.id) : []
}

function findProjectedNode(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.children.kind === 'loaded') {
      const child = findProjectedNode(node.children.nodes, nodeId)

      if (child !== undefined) {
        return child
      }
    }
  }

  return undefined
}
