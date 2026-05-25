import { describe, expect, it } from 'vitest'

import {
  createLibraryHierarchyReadController,
  type LibraryBrowserApi
} from '../../../../src/renderer/library/boundary/hierarchyRead'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type {
  ChildRow,
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
})

function testLibraryApi(options: {
  readonly readRows?: LibraryBrowserApi['navigation']['readRows']
  readonly readChildren: (request: ReadRequest) => Promise<ReadResult>
}): LibraryBrowserApi {
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
    },
    selectedContents: {
      read: async () => ({
        state: 'readFailed',
        error: {
          code: 'readFailed',
          message: 'Selected contents should not be called by hierarchy read tests.'
        }
      })
    },
    roots: {
      chooseAndRegisterLocal: async () => ({ state: 'canceled' }),
      runScan: async () => ({
        state: 'scanFailed',
        error: {
          code: 'scanFailed',
          message: 'Local root scan should not be called by hierarchy read tests.'
        }
      }),
      readLocalRoots: async () => ({
        state: 'hostFailed',
        error: {
          code: 'hostFailed',
          message: 'Local root read should not be called by hierarchy read tests.'
        }
      }),
      unregisterLocalRoot: async () => ({
        state: 'invalidRequest',
        error: {
          code: 'invalidRequest',
          message: 'Local root unregister should not be called by hierarchy read tests.'
        }
      })
    },
    browser: {
      viewState: {
        readViewState: async () => ({ state: 'empty' }),
        writeViewState: async () => ({ state: 'written' })
      }
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

function directoryRootHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: sourceRoot(),
      offset: 0,
      limit: 50,
      totalRows: 3,
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
      nodes: [
        fileNode(`${parentDirectoryId}-track`, 'track.wav', parentDirectoryId),
        directoryNode('99', 'Nested Album', parentDirectoryId)
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

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string
): Extract<ChildRow, { kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryMediaState: { kind: 'hasMediaDescendants' },
    directoryScanState: 'scanning',
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  parentDirectoryId?: string
): Extract<ChildRow, { kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    mediaClass: 'audio',
    presence: 'present',
    updatedAtMs: 101
  }
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
