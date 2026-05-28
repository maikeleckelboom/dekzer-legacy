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
      limit: 50,
      sourceFileVisibility: 'performance'
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
      limit: 50,
      sourceFileVisibility: 'performance'
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
      'source-directory:12',
      'source-file:99'
    ])

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:12-a',
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
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:12-a',
      'source-file:12-b'
    ])
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

  it('keeps source-file visibility in hierarchy read identity', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          return request.sourceFileVisibility === 'performanceAndImages'
            ? imageHierarchyReadResult(request.sourceFileVisibility)
            : audioHierarchyReadResult(request.sourceFileVisibility ?? 'performance')
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(readRequests[0]?.sourceFileVisibility).toBe('performance')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11'
    ])

    controller.setSourceFileVisibility('performanceAndImages')
    await waitForReadRequestCount(readRequests, 2)
    expect(readRequests[1]?.sourceFileVisibility).toBe('performanceAndImages')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11',
      'source-file:12'
    ])

    controller.setSourceFileVisibility('performance')
    await waitForReadRequestCount(readRequests, 3)
    expect(readRequests[2]?.sourceFileVisibility).toBe('performance')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11'
    ])
  })

  it('replays expanded source and directory reads when source-file visibility changes', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResultForVisibility(
              '12',
              request.sourceFileVisibility ?? 'performance'
            )
          }

          return directoryRootHierarchyReadResultForVisibility(
            request.sourceFileVisibility ?? 'performance'
          )
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:12-track',
      'source-directory:99'
    ])

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })
    expect(controller.sourceReadStates.value.get('navigation-row:7')).toMatchObject({
      kind: 'refreshing'
    })
    expect(controller.directoryReadStates.value.get('12')).toMatchObject({
      kind: 'loaded'
    })

    await waitForReadRequestCount(readRequests, 4)

    expect(readRequests[2]).not.toHaveProperty('parentDirectoryId')
    expect(readRequests[2]?.sourceFileVisibility).toBe('performanceAndImages')
    expect(readRequests[3]).toMatchObject({
      parentDirectoryId: '12',
      sourceFileVisibility: 'performanceAndImages'
    })
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:12-track',
      'source-directory:99'
    ])
  })

  it('ignores stale in-flight directory reads from the previous source-file visibility', async () => {
    const readRequests: ReadRequest[] = []
    const staleDirectoryRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            if (request.sourceFileVisibility === 'performance') {
              return staleDirectoryRead.promise
            }

            return imageDirectoryReadResult('12')
          }

          return directoryRootHierarchyReadResultForVisibility(
            request.sourceFileVisibility ?? 'performance'
          )
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    const staleRead = controller.requestDirectoryChildren('source-directory:12')
    await waitForReadRequestCount(readRequests, 2)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })
    await waitForReadRequestCount(readRequests, 4)

    staleDirectoryRead.resolve(loadedDirectoryReadResult('12'))
    await expect(staleRead).resolves.toBe(false)
    await waitForMicrotasks()

    expect(controller.sourceFileVisibility.value).toBe('performanceAndImages')
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:cover'
    ])
  })

  it('stops visibility replay when an expanded directory is unavailable under the new projection', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.sourceFileVisibility === 'performanceAndImages') {
            return imageHierarchyReadResult('performanceAndImages')
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
    })
    await waitForReadRequestCount(readRequests, 3)
    await waitForMicrotasks()

    expect(readRequests).toHaveLength(3)
    expect(controller.directoryReadStates.value.get('12')).toMatchObject({
      kind: 'unloaded'
    })
    expect(findProjectedNode(treeNodes(controller), 'source-directory:12')).toBeUndefined()
  })

  it('ignores stale source-file visibility responses', async () => {
    const readRequests: ReadRequest[] = []
    const imageRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.sourceFileVisibility === 'performanceAndImages') {
            return imageRead.promise
          }

          return audioHierarchyReadResult(request.sourceFileVisibility ?? 'performance')
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    controller.setSourceFileVisibility('performanceAndImages')
    await waitForReadRequestCount(readRequests, 2)
    controller.setSourceFileVisibility('performance')
    await waitForReadRequestCount(readRequests, 3)

    imageRead.resolve(imageHierarchyReadResult('performanceAndImages'))
    await waitForMicrotasks()

    expect(controller.sourceFileVisibility.value).toBe('performance')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11'
    ])
  })

  it('replays selected source when the selected node is not expanded', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationTwoSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.target?.kind === 'entryPoint') {
            const ep = request.target.entryPoint

            if (ep.kind === 'source' && ep.sourceId === '9') {
              return directoryRootHierarchyReadResultForSource9(
                request.sourceFileVisibility ?? 'performance'
              )
            }
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(readRequests[0]?.target).toMatchObject({
      kind: 'entryPoint',
      entryPoint: { kind: 'source', sourceId: '7' }
    })

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:9'])
    })
    await waitForReadRequestCount(readRequests, 2)

    expect(readRequests[1]?.target).toMatchObject({
      kind: 'entryPoint',
      entryPoint: { kind: 'source', sourceId: '9' }
    })
    expect(readRequests[1]?.sourceFileVisibility).toBe('performanceAndImages')
    expect(controller.sourceReadStates.value.get('navigation-row:9')?.kind).toBe('loaded')
    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
  })

  it('replays selected directory and its owning source when the source is not in the replay set', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResultForVisibility(
              '12',
              request.sourceFileVisibility ?? 'performance'
            )
          }

          return directoryRootHierarchyReadResultForVisibility(
            request.sourceFileVisibility ?? 'performance'
          )
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['source-directory:12'])
    })
    await waitForReadRequestCount(readRequests, 3)

    expect(readRequests[2]).toMatchObject({
      parentDirectoryId: '12',
      sourceFileVisibility: 'performanceAndImages'
    })
    expect(firstLoadedChildIds(treeNodes(controller), 'source-directory:12')).toEqual([
      'source-file:12-track',
      'source-directory:99'
    ])
  })

  it('replays selected directory under a non-first source when the owning source is not expanded', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationTwoSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '92') {
            return loadedDirectoryReadResultForSource9(
              '92',
              request.sourceFileVisibility ?? 'performance'
            )
          }

          if (request.target?.kind === 'entryPoint') {
            const ep = request.target.entryPoint

            if (ep.kind === 'source' && ep.sourceId === '9') {
              return directoryRootHierarchyReadResultForSource9(
                request.sourceFileVisibility ?? 'performance'
              )
            }
          }

          return directoryRootHierarchyReadResult()
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestNodeChildren('navigation-row:9')).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:92')).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['source-directory:92'])
    })
    await waitForReadRequestCount(readRequests, 4)

    const replayDirRead = readRequests[3]

    expect(replayDirRead).toMatchObject({
      parentDirectoryId: '92',
      sourceFileVisibility: 'performanceAndImages'
    })
    expect(controller.directoryReadStates.value.get('92')?.kind).toBe('loaded')
  })

  it('falls back to first source when replay set contains only non-loadable bindings', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          return audioHierarchyReadResult(request.sourceFileVisibility ?? 'performance')
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['source-file:11'])
    })
    await waitForReadRequestCount(readRequests, 2)

    expect(readRequests[1]?.sourceFileVisibility).toBe('performanceAndImages')
    expect(readRequests[1]).not.toHaveProperty('parentDirectoryId')
    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
  })

  it('does not send duplicate reads when the selected node is also in the expanded replay set', async () => {
    const readRequests: ReadRequest[] = []
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))

          if (request.parentDirectoryId === '12') {
            return loadedDirectoryReadResultForVisibility(
              '12',
              request.sourceFileVisibility ?? 'performance'
            )
          }

          return directoryRootHierarchyReadResultForVisibility(
            request.sourceFileVisibility ?? 'performance'
          )
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:7', 'navigation-row:7', 'source-directory:12'])
    })
    await waitForReadRequestCount(readRequests, 4)

    expect(readRequests).toHaveLength(4)
    const replaySourceReads = readRequests.filter(
      (rq, i) => i >= 2 && rq.parentDirectoryId === undefined
    )
    expect(replaySourceReads).toHaveLength(1)
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

    refreshSourceRead.resolve(directoryRootHierarchyReadResultForVisibility('performance'))
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
    expect(beforeChildren).toEqual(['source-file:12-track', 'source-directory:99'])

    const refreshPromise = controller.requestDirectoryChildren('source-directory:12')
    await waitForMicrotasks()

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('refreshing')
    const duringChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(duringChildren).toEqual(['source-file:12-track', 'source-directory:99'])

    refreshDirRead.resolve(loadedDirectoryReadResult('12'))
    await expect(refreshPromise).resolves.toBe(true)

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(afterChildren).toEqual(['source-file:12-track', 'source-directory:99'])
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
    expect(beforeChildren).toEqual(['source-file:12-track', 'source-directory:99'])

    await expect(controller.requestDirectoryChildren('source-directory:12')).resolves.toBe(true)

    expect(controller.directoryReadStates.value.get('12')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'source-directory:12')
    expect(afterChildren).toEqual(['source-file:12-track', 'source-directory:99'])
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

  it('visibility change keeps old source children visible until replacement arrives', async () => {
    const imageRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          readCount += 1

          if (readCount === 1) {
            return audioHierarchyReadResult('performance')
          }

          return imageRead.promise
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11'
    ])

    controller.setSourceFileVisibility('performanceAndImages', {
      replayNodeIds: new Set(['navigation-row:7'])
    })
    await waitForMicrotasks()

    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('refreshing')
    const duringChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(duringChildren).toEqual(['source-file:11'])

    imageRead.resolve(imageHierarchyReadResult('performanceAndImages'))
    await waitForMicrotasks()

    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
    const afterChildren = firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')
    expect(afterChildren).toEqual(['source-file:11', 'source-file:12'])
  })

  it('stale response from old visibility does not overwrite newer loaded state', async () => {
    const staleSourceRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readCount += 1

          if (readCount === 1) {
            return audioHierarchyReadResult('performance')
          }

          if (readCount === 2) {
            return staleSourceRead.promise
          }

          return audioHierarchyReadResult(request.sourceFileVisibility ?? 'performance')
        }
      })
    )

    await expect(controller.refresh()).resolves.toBe(true)

    controller.setSourceFileVisibility('performanceAndImages')
    await waitForMicrotasks()
    controller.setSourceFileVisibility('performance')
    await waitForMicrotasks()

    expect(readCount).toBeGreaterThanOrEqual(3)

    staleSourceRead.resolve(imageHierarchyReadResult('performanceAndImages'))
    await waitForMicrotasks()

    expect(controller.sourceFileVisibility.value).toBe('performance')
    expect(controller.sourceReadStates.value.get('navigation-row:7')?.kind).toBe('loaded')
    expect(firstLoadedChildIds(treeNodes(controller), 'navigation-row:7')).toEqual([
      'source-file:11'
    ])
  })

  it('source refresh does not clear loaded directory children under a different source', async () => {
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationTwoSourceReadRowsResult(),
        readChildren: async (request) => {
          if (request.target?.kind === 'entryPoint') {
            const ep = request.target.entryPoint

            if (ep.kind === 'source' && ep.sourceId === '9') {
              return directoryRootHierarchyReadResultForSource9(
                request.sourceFileVisibility ?? 'performance'
              )
            }
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

function navigationSourceReadRowsResult(): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: [
      {
        navigationRowId: '7',
        stableKey: 'source:7',
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: 'Source Fixture',
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
  return directoryRootHierarchyReadResultForVisibility('performance')
}

function directoryRootHierarchyReadResultForVisibility(
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      sourceFileVisibility,
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
      sourceFileVisibility: 'performance',
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
      sourceFileVisibility: 'performance',
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode('99', 'root-track.wav')]
    }
  }
}

function loadedDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return loadedDirectoryReadResultForVisibility(parentDirectoryId, 'performance')
}

function loadedDirectoryReadResultForVisibility(
  parentDirectoryId: string,
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      sourceFileVisibility,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [
        fileNode(`${parentDirectoryId}-track`, 'track.wav', parentDirectoryId),
        directoryNode('99', 'Nested Album', parentDirectoryId)
      ]
    }
  }
}

function directoryRootHierarchyReadResultForSource9(
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot9(),
      offset: 0,
      limit: 50,
      sourceFileVisibility,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [
        directoryNode('92', 'Albums', undefined, '9'),
        fileNode('91', 'track9.wav', undefined, 'audio', '9')
      ]
    }
  }
}

function loadedDirectoryReadResultForSource9(
  parentDirectoryId: string,
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot9(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      sourceFileVisibility,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [
        fileNode(`${parentDirectoryId}-track`, 'track9.wav', parentDirectoryId, 'audio', '9'),
        directoryNode('99', 'Nested', parentDirectoryId, '9')
      ]
    }
  }
}

function imageDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      parentDirectoryId,
      offset: 0,
      limit: 50,
      sourceFileVisibility: 'performanceAndImages',
      totalRows: 1,
      coverage: completeCoverage(),
      nodes: [fileNode('cover', 'cover.jpg', parentDirectoryId, 'image')]
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
      sourceFileVisibility: 'performance',
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
      sourceFileVisibility: 'performance',
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
      sourceFileVisibility: 'performance',
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
      sourceFileVisibility: 'performance',
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
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  parentDirectoryId?: string,
  mediaClass: Extract<ChildRow, { kind: 'file' }>['mediaClass'] = 'audio',
  sourceId = '7'
): Extract<ChildRow, { kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId,
    fileId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    mediaClass,
    presence: 'present',
    updatedAtMs: 101
  }
}

function audioHierarchyReadResult(
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      sourceFileVisibility,
      totalRows: 1,
      coverage: completeCoverage(),
      nodes: [fileNode('11', 'track.wav')]
    }
  }
}

function imageHierarchyReadResult(
  sourceFileVisibility: NonNullable<ReadRequest['sourceFileVisibility']>
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      sourceFileVisibility,
      totalRows: 2,
      coverage: completeCoverage(),
      nodes: [fileNode('11', 'track.wav'), fileNode('12', 'cover.jpg', undefined, 'image')]
    }
  }
}

function completeCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    recursiveScopeComplete: true,
    emptyResultAuthoritative: false
  }
}

function completeEmptyCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    recursiveScopeComplete: true,
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

async function waitForReadRequestCount(
  readRequests: readonly ReadRequest[],
  expectedCount: number
): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt++) {
    if (readRequests.length >= expectedCount) {
      await waitForMicrotasks()
      return
    }

    await waitForMicrotasks()
  }

  throw new Error(`Expected ${expectedCount} hierarchy read requests.`)
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
