import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  LibraryBoundaryHost,
  type LibraryBoundaryHostClient,
  type LibraryBoundaryHostTransport
} from '../src/main/libraryBoundary/host'
import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import { readThroughHost, registerReadChildrenIpc } from '../src/main/libraryHierarchy/readChildren'
import {
  readNavigationRowsThroughHost,
  registerReadNavigationRowsIpc
} from '../src/main/libraryNavigation/readRows'
import {
  createLibraryHierarchyReadController,
  type LibraryBrowserApi
} from '../src/renderer/libraryBrowser/boundary/hierarchyRead'
import type { BrowserTreeNode } from '../src/renderer/libraryBrowser/tree/types'
import {
  hierarchyReadChannels,
  type ReadErrorCode,
  type ReadRequest,
  type ReadResult
} from '../src/shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type NavigationReadRowsResult
} from '../src/shared/libraryNavigation/readRows'
import { firstAvailableSourceReadRequest } from './support/libraryHierarchy'
import { createFakeClient, deferred, silentLogger, testApp } from './support/libraryBoundary'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-hierarchy-read-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  const config = resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })

  await validatesHierarchyReadHandler(config)
  validatesHierarchyReadIpcRegistration(config)
  await validatesNavigationReadHandler(config)
  validatesNavigationReadIpcRegistration(config)
  await validatesRefreshReturnsTrueWithZeroSources()
  await validatesRendererHierarchyReadController()
  await validatesRendererWindowedMore()
  await validatesEntryPointRejection()
}

async function validatesHierarchyReadHandler(config: LibraryBoundaryHostConfig): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await readThroughHost(idleHost, firstAvailableSourceReadRequest())

  assert.equal(hostUnavailable.state, 'hostUnavailable')
  assertReadError(hostUnavailable, 'hostNotStarted')

  const noTargetHost = await startedHostWithClient(
    config,
    createFakeClient({
      readNavigationRows: async () => ({
        rows: []
      })
    })
  )
  const noTarget = await readThroughHost(noTargetHost, firstAvailableSourceReadRequest())

  assert.equal(noTarget.state, 'noTarget')
  assertReadError(noTarget, 'noTarget')

  const successHost = await startedHostWithClient(
    config,
    createFakeClient({
      readNavigationRows: async () => ({
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
      }),
      readLiteralHierarchyChildren: async (request) => {
        assert.deepEqual(request.entryPoint, {
          type: 'source',
          payload: {
            sourceId: '7'
          }
        })
        assert.equal(request.parentSourceDirectoryId, null)
        assert.equal(request.offset, 0)
        assert.equal(request.limit, 50)

        return {
          window: {
            entryPoint: request.entryPoint,
            parentSourceDirectoryId: null,
            offset: request.offset,
            limit: request.limit,
            totalRows: 2,
            rows: [
              {
                nodeKind: 'directory',
                sourceId: '7',
                sourceDirectoryId: '12',
                sourceFileId: null,
                parentSourceDirectoryId: null,
                relativePath: 'Album',
                displayName: 'Album',
                presenceState: 'present',
                sizeBytes: null,
                modifiedAtNs: null,
                updatedAtMs: 100,
                hasChildDirectories: true,
                directoryMediaState: { kind: 'hasMediaDescendants' },
                directoryScanState: 'scanning'
              },
              {
                nodeKind: 'file',
                sourceId: '7',
                sourceDirectoryId: null,
                sourceFileId: '11',
                parentSourceDirectoryId: null,
                relativePath: 'track.wav',
                displayName: 'track.wav',
                presenceState: 'present',
                sizeBytes: null,
                modifiedAtNs: null,
                updatedAtMs: 101
              }
            ]
          }
        }
      }
    })
  )
  const success = await readThroughHost(successHost, firstAvailableSourceReadRequest())

  assert.equal(success.state, 'ready')
  if (success.state !== 'ready') {
    assert.fail('expected ready hierarchy read result')
  }
  assert.equal(success.window.root.id, 'source:7')
  assert.equal(success.window.root.label, 'Source Fixture')
  assert.deepEqual(success.window.nodes, [
    {
      id: 'source-directory:12',
      kind: 'directory',
      label: 'Album',
      directoryId: '12',
      presence: 'present',
      hasChildDirectories: true,
      directoryMediaState: { kind: 'hasMediaDescendants' },
      directoryScanState: 'scanning',
      updatedAtMs: 100
    },
    {
      id: 'source-file:11',
      kind: 'file',
      label: 'track.wav',
      fileId: '11',
      presence: 'present',
      updatedAtMs: 101
    }
  ])

  const invalidTarget = await readThroughHost(successHost, {
    target: {
      kind: 'entryPoint',
      entryPoint: {
        kind: 'source',
        sourceId: '0'
      }
    }
  })

  assert.equal(invalidTarget.state, 'invalidRequest')
  assertReadError(invalidTarget, 'invalidRequest')

  const malformedRow = await readThroughHost(
    await startedHostWithClient(
      config,
      createFakeClient({
        readNavigationRows: async () => ({
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
        }),
        readLiteralHierarchyChildren: async (request) => ({
          window: {
            entryPoint: request.entryPoint,
            parentSourceDirectoryId: null,
            offset: request.offset,
            limit: request.limit,
            totalRows: 1,
            rows: [
              {
                nodeKind: 'file',
                sourceId: '7',
                sourceDirectoryId: null,
                sourceFileId: null,
                parentSourceDirectoryId: null,
                relativePath: 'track.wav',
                displayName: 'track.wav',
                presenceState: 'present',
                sizeBytes: null,
                modifiedAtNs: null,
                updatedAtMs: 101
              }
            ]
          }
        })
      })
    ),
    firstAvailableSourceReadRequest()
  )

  assert.equal(malformedRow.state, 'readFailed')
  assertReadError(malformedRow, 'readFailed')
}

function validatesHierarchyReadIpcRegistration(config: LibraryBoundaryHostConfig): void {
  const host = new LibraryBoundaryHost(config, silentLogger())

  const registration: {
    channel?: string
    handler?: (request: unknown) => Promise<ReadResult>
  } = {}

  registerReadChildrenIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (request) => listener({}, request)
      }
    },
    host
  )

  assert.equal(registration.channel, hierarchyReadChannels.readChildren)
  assert.equal(typeof registration.handler, 'function')
}

async function validatesNavigationReadHandler(config: LibraryBoundaryHostConfig): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await readNavigationRowsThroughHost(idleHost, {
    parentNavigationRowId: null
  })

  assert.equal(hostUnavailable.state, 'hostUnavailable')
  assertNavigationReadError(hostUnavailable, 'hostNotStarted')

  const successHost = await startedHostWithClient(
    config,
    createFakeClient({
      readNavigationRows: async (request) => {
        assert.deepEqual(request, {
          parentNavigationRowId: null
        })
        return {
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
    })
  )
  const success = await readNavigationRowsThroughHost(successHost, {
    parentNavigationRowId: null
  })

  assert.equal(success.state, 'ready')
  if (success.state !== 'ready') {
    assert.fail('expected ready navigation read result')
  }
  assert.deepEqual(
    success.rows.map((row) => row.displayName),
    ['Source Fixture']
  )

  const invalid = await readNavigationRowsThroughHost(successHost, {
    parentNavigationRowId: 'not-a-navigation-row-id'
  })
  assert.equal(invalid.state, 'invalidRequest')
  assertNavigationReadError(invalid, 'invalidRequest')
}

function validatesNavigationReadIpcRegistration(config: LibraryBoundaryHostConfig): void {
  const host = new LibraryBoundaryHost(config, silentLogger())

  const registration: {
    channel?: string
    handler?: (request: unknown) => Promise<NavigationReadRowsResult>
  } = {}

  registerReadNavigationRowsIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (request) => listener({}, request)
      }
    },
    host
  )

  assert.equal(registration.channel, navigationReadChannels.readRows)
  assert.equal(typeof registration.handler, 'function')
}

async function validatesRefreshReturnsTrueWithZeroSources(): Promise<void> {
  let readRowsCalled = false
  const controller = createLibraryHierarchyReadController(
    testLibraryApi({
      readRows: async () => {
        readRowsCalled = true
        return {
          state: 'ready',
          rows: []
        }
      },
      readChildren: async () => {
        throw new Error('readChildren should not be called with zero sources')
      }
    })
  )

  const result = await controller.refresh()

  assert.equal(readRowsCalled, true)
  assert.equal(result, true)
  assert.equal(controller.navigationReadResult.value?.state, 'ready')
  assert.equal(controller.navigationReadResult.value?.rows.length, 0)
}

async function validatesRendererHierarchyReadController(): Promise<void> {
  const requests: ReadRequest[] = []
  const navigationRequests: unknown[] = []
  const directory12Read = deferred<ReadResult>()
  const directory14Read = deferred<ReadResult>()
  let directory13Attempts = 0
  const controller = createLibraryHierarchyReadController(
    testLibraryApi({
      readRows: async (request) => {
        navigationRequests.push(structuredClone(request))
        return navigationSourceReadRowsResult()
      },
      readChildren: async (request) => {
        const clonedRequest = structuredClone(request)
        requests.push(clonedRequest)

        if (clonedRequest.parentDirectoryId === '12') {
          return directory12Read.promise
        }

        if (clonedRequest.parentDirectoryId === '13') {
          directory13Attempts += 1
          return directory13Attempts === 1
            ? hierarchyReadError(
                'readFailed',
                'readFailed',
                'Unable to read library hierarchy children.'
              )
            : emptyDirectoryReadResult('13')
        }

        if (clonedRequest.parentDirectoryId === '14') {
          return directory14Read.promise
        }

        return directoryRootHierarchyReadResult()
      }
    })
  )

  await controller.refresh()

  assert.deepEqual(navigationRequests, [{ parentNavigationRowId: null }])
  assert.deepEqual(requests[0], {
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
  assert.equal(controller.currentRoot.value?.id, 'source:7')
  assert.equal(controller.directoryReadStates.value.get('12')?.kind, 'unloaded')
  assert.equal(controller.directoryReadStates.value.get('13')?.kind, 'unloaded')
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'unloaded')

  let projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected live hierarchy projection')
  }
  assert.equal(projection.bindingsById.get('source-directory:12')?.kind, 'directory')
  assert.deepEqual(
    (projection.bindingsById.get('source-directory:12') as { entryPoint: unknown } | undefined)
      ?.entryPoint,
    {
      kind: 'source',
      sourceId: '7'
    }
  )
  assert.equal(
    firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:12'),
    'deferred'
  )
  assert.equal(await controller.requestDirectoryChildren('source-directory:99'), false)
  assert.equal(requests.length, 1)

  const firstDirectoryRequest = controller.requestDirectoryChildren('source-directory:12')
  assert.equal(await controller.requestDirectoryChildren('source-directory:12'), false)
  assert.equal(requests.length, 2)
  assert.deepEqual(requests[1], {
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
  assert.equal(controller.directoryReadStates.value.get('12')?.kind, 'loading')

  directory12Read.resolve(loadedDirectoryReadResult('12'))
  assert.equal(await firstDirectoryRequest, true)
  assert.equal(controller.directoryReadStates.value.get('12')?.kind, 'loaded')
  assert.equal(controller.directoryReadStates.value.get('99')?.kind, 'unloaded')

  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected live hierarchy projection after child read')
  }
  assert.equal(firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:12'), 'loaded')
  assert.equal(
    firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:99'),
    'deferred'
  )

  assert.equal(await controller.requestDirectoryChildren('source-directory:13'), true)
  assert.equal(controller.directoryReadStates.value.get('13')?.kind, 'failed')
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected failed directory projection')
  }
  assert.equal(firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:13'), 'loaded')

  assert.equal(await controller.requestDirectoryChildren('source-directory:13'), true)
  const retriedState = controller.directoryReadStates.value.get('13')
  assert.equal(retriedState?.kind, 'loaded')
  if (retriedState?.kind !== 'loaded') {
    assert.fail('expected retried directory read to load')
  }
  assert.deepEqual(retriedState.children.rows, [])
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected empty directory projection')
  }
  assert.equal(firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:13'), 'none')
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'source-directory:13'), [])

  assert.equal(await controller.loadFirstSource(), false)

  const staleDirectoryRequest = controller.requestDirectoryChildren('source-directory:14')
  directory14Read.resolve(loadedDirectoryReadResult('14'))
  assert.equal(await staleDirectoryRequest, true)
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'loaded')
}

async function validatesRendererWindowedMore(): Promise<void> {
  const requests: ReadRequest[] = []
  const directory14More = deferred<ReadResult>()
  let rootReads = 0
  let directory12MoreAttempts = 0
  const controller = createLibraryHierarchyReadController(
    testLibraryApi({
      readRows: async () => navigationSourceReadRowsResult(),
      readChildren: async (request) => {
        const clonedRequest = structuredClone(request)
        requests.push(clonedRequest)

        if (clonedRequest.parentDirectoryId === undefined) {
          if (clonedRequest.offset === 2) {
            return sourceMoreReadResult()
          }

          rootReads += 1
          return rootReads === 1
            ? partialSourceHierarchyReadResult()
            : refreshedSourceHierarchyReadResult()
        }

        if (clonedRequest.parentDirectoryId === '12') {
          if (clonedRequest.offset === 1) {
            directory12MoreAttempts += 1
            return directory12MoreAttempts === 1
              ? hierarchyReadError(
                  'readFailed',
                  'readFailed',
                  'Unable to read more directory children.'
                )
              : directoryMoreReadResult('12')
          }

          return partialDirectoryHierarchyReadResult('12')
        }

        if (clonedRequest.parentDirectoryId === '14') {
          if (clonedRequest.offset === 1) {
            return directory14More.promise
          }

          return partialDirectoryHierarchyReadResult('14')
        }

        return emptyDirectoryReadResult(clonedRequest.parentDirectoryId)
      }
    })
  )

  await controller.refresh()

  let projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected partial source hierarchy projection')
  }

  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'navigation-row:7'), [
    'source-directory:12',
    'source-directory:14',
    'more:navigation-row:7:2'
  ])
  assert.equal(projection.bindingsById.get('more:navigation-row:7:2')?.kind, 'more')
  assert.equal(
    findProjectedNode(projection.nodes, 'more:navigation-row:7:2')?.children.kind,
    'none'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:navigation-row:7:2')?.action?.kind,
    'loadMore'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:navigation-row:7:2')?.action?.state.kind,
    'idle'
  )
  assert.deepEqual(
    (projection.bindingsById.get('more:navigation-row:7:2') as { target: unknown } | undefined)
      ?.target,
    {
      ownerNodeId: 'navigation-row:7',
      entryPoint: {
        kind: 'source',
        sourceId: '7'
      },
      label: 'Source Fixture',
      offset: 2,
      limit: 50
    }
  )
  assert.equal(requests.length, 1)

  assert.equal(await controller.requestNodeChildren('more:navigation-row:7:2'), true)
  assert.deepEqual(requests[1], {
    target: {
      kind: 'entryPoint',
      entryPoint: {
        kind: 'source',
        sourceId: '7'
      },
      label: 'Source Fixture'
    },
    offset: 2,
    limit: 50
  })

  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected completed source hierarchy projection')
  }
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'navigation-row:7'), [
    'source-directory:12',
    'source-directory:14',
    'source-file:99'
  ])

  assert.equal(await controller.requestDirectoryChildren('source-directory:12'), true)
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected partial directory hierarchy projection')
  }
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'source-directory:12'), [
    'source-file:12-a',
    'more:source-directory:12:1'
  ])
  assert.deepEqual(
    (projection.bindingsById.get('more:source-directory:12:1') as { target: unknown } | undefined)
      ?.target,
    {
      ownerNodeId: 'source-directory:12',
      entryPoint: {
        kind: 'source',
        sourceId: '7'
      },
      label: 'Source Fixture',
      parentDirectoryId: '12',
      offset: 1,
      limit: 50
    }
  )

  assert.equal(await controller.requestNodeChildren('more:source-directory:12:1'), true)
  assert.deepEqual(requests[3], {
    target: {
      kind: 'entryPoint',
      entryPoint: {
        kind: 'source',
        sourceId: '7'
      },
      label: 'Source Fixture'
    },
    parentDirectoryId: '12',
    offset: 1,
    limit: 50
  })
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected failed more projection')
  }
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'source-directory:12'), [
    'source-file:12-a',
    'more:source-directory:12:1'
  ])
  assert.equal(projection.bindingsById.get('more:source-directory:12:1')?.kind, 'more')
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:12:1')?.children.kind,
    'none'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:12:1')?.action?.kind,
    'loadMore'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:12:1')?.action?.state.kind,
    'failed'
  )

  assert.equal(await controller.requestNodeChildren('more:source-directory:12:1'), true)
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected retried more projection')
  }
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'source-directory:12'), [
    'source-file:12-a',
    'source-file:12-b'
  ])
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'navigation-row:7'), [
    'source-directory:12',
    'source-directory:14',
    'source-file:99'
  ])

  assert.equal(await controller.requestDirectoryChildren('source-directory:14'), true)
  const staleMore = controller.requestNodeChildren('more:source-directory:14:1')
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'loaded')
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected loading more projection')
  }
  assert.deepEqual(firstLoadedChildIds(projection.nodes, 'source-directory:14'), [
    'source-file:14-a',
    'more:source-directory:14:1'
  ])
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:14:1')?.children.kind,
    'none'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:14:1')?.action?.kind,
    'loadMore'
  )
  assert.equal(
    findProjectedNode(projection.nodes, 'more:source-directory:14:1')?.action?.state.kind,
    'loading'
  )
  assert.equal(await controller.loadFirstSource(), false)
  directory14More.resolve(directoryMoreReadResult('14'))
  assert.equal(await staleMore, true)
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'loaded')
}

async function validatesEntryPointRejection(): Promise<void> {
  {
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => ({
          state: 'ready',
          window: {
            root: {
              id: 'source:999',
              label: 'Wrong Source',
              entryPoint: { kind: 'source', sourceId: '999' }
            },
            offset: 0,
            limit: 50,
            totalRows: 1,
            nodes: [
              {
                id: 'source-file:999',
                kind: 'file' as const,
                label: 'intruder.wav',
                fileId: '999',
                presence: 'present' as const,
                updatedAtMs: 100
              }
            ]
          }
        })
      })
    )

    await controller.refresh()
    const sourceState = controller.sourceReadStates.value.get('navigation-row:7')
    assert.equal(sourceState?.kind, 'failed')
    assert.equal(
      (sourceState as { detail: string })?.detail,
      'The hierarchy read returned an unexpected child window.'
    )
    assert.equal(controller.directoryReadStates.value.size, 0)
  }

  {
    let directoryReadAttempt = false
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          if (!directoryReadAttempt) {
            directoryReadAttempt = true
            return directoryRootHierarchyReadResult()
          }

          return {
            state: 'ready' as const,
            window: {
              root: {
                id: 'source:999',
                label: 'Wrong Source',
                entryPoint: { kind: 'source', sourceId: '999' }
              },
              parentDirectoryId: '12',
              offset: 0,
              limit: 50,
              totalRows: 1,
              nodes: [
                {
                  id: 'source-file:999',
                  kind: 'file' as const,
                  label: 'intruder.wav',
                  fileId: '999',
                  parentDirectoryId: '12',
                  presence: 'present' as const,
                  updatedAtMs: 100
                }
              ]
            }
          }
        }
      })
    )

    await controller.refresh()
    await controller.requestDirectoryChildren('source-directory:12')
    const state = controller.directoryReadStates.value.get('12')
    assert.equal(state?.kind, 'failed')
    assert.equal(
      (state as { detail: string })?.detail,
      'The hierarchy read returned an unexpected child window.'
    )
  }

  {
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          readCount += 1

          if (readCount === 1) {
            return partialSourceHierarchyReadResult()
          }

          return {
            state: 'ready' as const,
            window: {
              root: {
                id: 'source:999',
                label: 'Wrong Source',
                entryPoint: { kind: 'source', sourceId: '999' }
              },
              offset: 2,
              limit: 50,
              totalRows: 3,
              nodes: [
                {
                  id: 'source-file:999',
                  kind: 'file' as const,
                  label: 'intruder.wav',
                  fileId: '999',
                  presence: 'present' as const,
                  updatedAtMs: 100
                }
              ]
            }
          }
        }
      })
    )

    await controller.refresh()
    const beforeState = controller.sourceReadStates.value.get('navigation-row:7')
    assert.equal(beforeState?.kind, 'loaded')
    if (beforeState?.kind === 'loaded') {
      assert.equal(beforeState.children.rows.length, 2)
    }

    await controller.requestNodeChildren('more:navigation-row:7:2')
    const afterState = controller.sourceReadStates.value.get('navigation-row:7')
    assert.equal(afterState?.kind, 'loaded')
    if (afterState?.kind === 'loaded') {
      assert.equal(afterState.children.rows.length, 2)
      assert.equal(afterState.children.more?.kind, 'failed')
    }
  }

  {
    let readCount = 0
    const controller = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async () => {
          readCount += 1

          if (readCount === 1) {
            return {
              state: 'ready' as const,
              window: {
                root: {
                  id: 'source:7',
                  label: 'Source Fixture',
                  entryPoint: { kind: 'source' as const, sourceId: '7' }
                },
                offset: 0,
                limit: 50,
                totalRows: 1,
                nodes: [directoryNode('12', 'Album')]
              }
            }
          }

          if (readCount === 2) {
            return partialDirectoryHierarchyReadResult('12')
          }

          return {
            state: 'ready' as const,
            window: {
              root: {
                id: 'source:999',
                label: 'Wrong Source',
                entryPoint: { kind: 'source', sourceId: '999' }
              },
              parentDirectoryId: '12',
              offset: 1,
              limit: 50,
              totalRows: 2,
              nodes: [
                {
                  id: 'source-file:12-b',
                  kind: 'file' as const,
                  label: 'b.wav',
                  fileId: '12-b',
                  parentDirectoryId: '12',
                  presence: 'present' as const,
                  updatedAtMs: 101
                }
              ]
            }
          }
        }
      })
    )

    await controller.refresh()
    await controller.requestDirectoryChildren('source-directory:12')

    const beforeState = controller.directoryReadStates.value.get('12')
    assert.equal(beforeState?.kind, 'loaded')
    if (beforeState?.kind === 'loaded') {
      assert.equal(beforeState.children.rows.length, 1)
    }

    await controller.requestNodeChildren('more:source-directory:12:1')
    const afterState = controller.directoryReadStates.value.get('12')
    assert.equal(afterState?.kind, 'loaded')
    if (afterState?.kind === 'loaded') {
      assert.equal(afterState.children.rows.length, 1)
      assert.equal(afterState.children.more?.kind, 'failed')
    }
  }
}

async function startedHostWithClient(
  config: LibraryBoundaryHostConfig,
  client: LibraryBoundaryHostClient
): Promise<LibraryBoundaryHost> {
  const ready = deferred<void>()
  ready.resolve()

  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createTransport: () =>
      ({
        ready: ready.promise,
        close: async () => undefined,
        execute: async () => {
          throw new Error('execute should not be called by hierarchy read validation')
        }
      }) satisfies LibraryBoundaryHostTransport,
    createClient: () => client
  })

  await host.start()
  return host
}

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
    roots: {
      chooseAndRegisterLocal: async () => ({
        state: 'dialogFailed',
        error: {
          code: 'dialogFailed',
          message: 'Local root choice should not be called by hierarchy read validation.'
        }
      }),
      runScan: async () => ({
        state: 'scanFailed',
        error: {
          code: 'scanFailed',
          message: 'Local root scan should not be called by hierarchy read validation.'
        }
      }),
      readLocalRoots: async () => ({
        state: 'hostFailed',
        error: {
          code: 'hostFailed',
          message: 'Local root read should not be called by hierarchy read validation.'
        }
      }),
      unregisterLocalRoot: async () => ({
        state: 'invalidRequest',
        error: {
          code: 'invalidRequest',
          message: 'Local root unregister should not be called by hierarchy read validation.'
        }
      })
    },
    browser: {
      viewState: {
        readViewState: async () => ({
          state: 'empty'
        }),
        writeViewState: async () => ({
          state: 'written'
        })
      }
    }
  }
}

function navigationSourceReadRowsResult(): Awaited<
  ReturnType<LibraryBrowserApi['navigation']['readRows']>
> {
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
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
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
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      offset: 0,
      limit: 50,
      totalRows: 3,
      nodes: [directoryNode('12', 'Album'), directoryNode('14', 'Stale Album')]
    }
  }
}

function sourceMoreReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      offset: 2,
      limit: 50,
      totalRows: 3,
      nodes: [
        {
          id: 'source-file:99',
          kind: 'file',
          label: 'root-track.wav',
          fileId: '99',
          presence: 'present',
          updatedAtMs: 101
        }
      ]
    }
  }
}

function refreshedSourceHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      offset: 0,
      limit: 50,
      totalRows: 2,
      nodes: [directoryNode('12', 'Album'), directoryNode('14', 'Stale Album')]
    }
  }
}

function loadedDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 2,
      nodes: [
        {
          id: `source-file:${parentDirectoryId}-track`,
          kind: 'file',
          label: 'track.wav',
          fileId: `${parentDirectoryId}11`,
          parentDirectoryId,
          presence: 'present',
          updatedAtMs: 101
        },
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
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 2,
      nodes: [
        {
          id: `source-file:${parentDirectoryId}-a`,
          kind: 'file',
          label: 'a.wav',
          fileId: `${parentDirectoryId}-a`,
          parentDirectoryId,
          presence: 'present',
          updatedAtMs: 101
        }
      ]
    }
  }
}

function directoryMoreReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      parentDirectoryId,
      offset: 1,
      limit: 50,
      totalRows: 2,
      nodes: [
        {
          id: `source-file:${parentDirectoryId}-b`,
          kind: 'file',
          label: 'b.wav',
          fileId: `${parentDirectoryId}-b`,
          parentDirectoryId,
          presence: 'present',
          updatedAtMs: 102
        }
      ]
    }
  }
}

function emptyDirectoryReadResult(
  parentDirectoryId: string
): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      parentDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 0,
      nodes: []
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
    error: {
      code,
      message
    }
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string
): Extract<
  Extract<ReadResult, { state: 'ready' }>['window']['nodes'][number],
  { kind: 'directory' }
> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryMediaState: { kind: 'hasMediaDescendants' },
    directoryScanState: 'scanning',
    updatedAtMs: 100
  }
}

function firstProjectedDirectoryStateKind(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): string | undefined {
  return findProjectedNode(nodes, nodeId)?.children.kind
}

function firstLoadedChildIds(nodes: readonly BrowserTreeNode[], nodeId: string): readonly string[] {
  const node = findProjectedNode(nodes, nodeId)

  if (node?.children.kind !== 'loaded') {
    return []
  }

  return node.children.nodes.map((child) => child.id)
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

function assertReadError(result: ReadResult, code: ReadErrorCode): void {
  if (result.state === 'ready') {
    assert.fail(`expected hierarchy read error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}

function assertNavigationReadError(
  result: NavigationReadRowsResult,
  code: Exclude<NavigationReadRowsResult, { state: 'ready' }>['error']['code']
): void {
  if (result.state === 'ready') {
    assert.fail(`expected navigation read error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}
