import { strict as assert } from 'node:assert'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
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
import {
  readLiteralHierarchyChildrenThroughHost,
  registerLibraryHierarchyReadIpc
} from '../src/main/libraryHierarchy/read'
import {
  createLibraryHierarchyReadController,
  type LibraryHierarchyBoundaryApi
} from '../src/renderer/libraryBrowser/hierarchyRead'
import type { BrowserTreeNode } from '../src/renderer/libraryBrowser/tree/types'
import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadErrorCode,
  type LibraryHierarchyReadRequest,
  type LibraryHierarchyReadResult
} from '../src/shared/libraryHierarchy/read'
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
  await validatesRendererHierarchyReadController()
  validatesRendererDirectoryTargetsAreDomainOwned()
  validatesFixtureModeDoesNotRequestBackendChildren()
}

async function validatesHierarchyReadHandler(config: LibraryBoundaryHostConfig): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const hostUnavailable = await readLiteralHierarchyChildrenThroughHost(
    idleHost,
    firstAvailableSourceReadRequest()
  )

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
  const noTarget = await readLiteralHierarchyChildrenThroughHost(
    noTargetHost,
    firstAvailableSourceReadRequest()
  )

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
            totalRows: 1,
            rows: [
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
  const success = await readLiteralHierarchyChildrenThroughHost(
    successHost,
    firstAvailableSourceReadRequest()
  )

  assert.equal(success.state, 'ready')
  if (success.state !== 'ready') {
    assert.fail('expected ready hierarchy read result')
  }
  assert.equal(success.window.root.id, 'source:7')
  assert.equal(success.window.root.label, 'Source Fixture')
  assert.deepEqual(success.window.nodes, [
    {
      id: 'source-file:11',
      kind: 'file',
      label: 'track.wav',
      sourceFileId: '11',
      presenceState: 'present',
      updatedAtMs: 101
    }
  ])

  const invalidTarget = await readLiteralHierarchyChildrenThroughHost(successHost, {
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

  const malformedRow = await readLiteralHierarchyChildrenThroughHost(
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
    handler?: (request: unknown) => Promise<LibraryHierarchyReadResult>
  } = {}

  registerLibraryHierarchyReadIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (request) => listener({}, request)
      }
    },
    host
  )

  assert.equal(registration.channel, libraryHierarchyReadIpcChannels.readLiteralHierarchyChildren)
  assert.equal(typeof registration.handler, 'function')
}

async function validatesRendererHierarchyReadController(): Promise<void> {
  const requests: LibraryHierarchyReadRequest[] = []
  const directory12Read = deferred<LibraryHierarchyReadResult>()
  const directory14Read = deferred<LibraryHierarchyReadResult>()
  let directory13Attempts = 0
  const controller = createLibraryHierarchyReadController(
    testLibraryBoundary(async (request) => {
      requests.push(request)

      if (request.parentSourceDirectoryId === '12') {
        return directory12Read.promise
      }

      if (request.parentSourceDirectoryId === '13') {
        directory13Attempts += 1
        return directory13Attempts === 1
          ? hierarchyReadError(
              'readFailed',
              'readFailed',
              'Unable to read library hierarchy children.'
            )
          : emptyDirectoryHierarchyReadResult('13')
      }

      if (request.parentSourceDirectoryId === '14') {
        return directory14Read.promise
      }

      return directoryRootHierarchyReadResult()
    })
  )

  await controller.readFirstAvailableSourceHierarchy()

  assert.deepEqual(requests[0], firstAvailableSourceReadRequest())
  assert.equal(controller.currentRoot.value?.id, 'source:7')
  assert.equal(controller.directoryReadStates.value.get('12')?.kind, 'unloaded')
  assert.equal(controller.directoryReadStates.value.get('13')?.kind, 'unloaded')
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'unloaded')

  let projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected live hierarchy projection')
  }
  assert.deepEqual(
    [...projection.directoryReadTargetsByNodeId.entries()],
    [
      ['source-directory:12', { sourceDirectoryId: '12' }],
      ['source-directory:13', { sourceDirectoryId: '13' }],
      ['source-directory:14', { sourceDirectoryId: '14' }]
    ]
  )
  assert.equal(
    firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:12'),
    'unloaded'
  )

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
    parentSourceDirectoryId: '12',
    offset: 0,
    limit: 50
  })
  assert.equal(controller.directoryReadStates.value.get('12')?.kind, 'loading')

  directory12Read.resolve(loadedDirectoryHierarchyReadResult('12'))
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
    'unloaded'
  )

  assert.equal(await controller.requestDirectoryChildren('source-directory:13'), true)
  assert.equal(controller.directoryReadStates.value.get('13')?.kind, 'failed')
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected failed directory projection')
  }
  assert.equal(firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:13'), 'failed')

  assert.equal(await controller.requestDirectoryChildren('source-directory:13'), true)
  const retriedState = controller.directoryReadStates.value.get('13')
  assert.equal(retriedState?.kind, 'loaded')
  if (retriedState?.kind !== 'loaded') {
    assert.fail('expected retried directory read to load')
  }
  assert.deepEqual(retriedState.window.nodes, [])
  projection = controller.browserProjection.value
  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected empty directory projection')
  }
  assert.equal(firstProjectedDirectoryStateKind(projection.nodes, 'source-directory:13'), 'loaded')

  const staleDirectoryRequest = controller.requestDirectoryChildren('source-directory:14')
  await controller.readFirstAvailableSourceHierarchy()
  directory14Read.resolve(loadedDirectoryHierarchyReadResult('14'))
  assert.equal(await staleDirectoryRequest, false)
  assert.equal(controller.directoryReadStates.value.get('14')?.kind, 'unloaded')
}

function validatesRendererDirectoryTargetsAreDomainOwned(): void {
  const hierarchyReadSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/hierarchyRead.ts', import.meta.url),
    'utf8'
  )
  const hierarchyProjectionSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/hierarchyProjection.ts', import.meta.url),
    'utf8'
  )

  assert.match(hierarchyProjectionSource, /sourceDirectoryId: node\.sourceDirectoryId/)
  assert.doesNotMatch(hierarchyReadSource, /source-directory:/)
  assert.doesNotMatch(hierarchyProjectionSource, /source-directory:/)
  assert.doesNotMatch(hierarchyReadSource, /\.split\(/)
  assert.doesNotMatch(hierarchyProjectionSource, /\.split\(/)
}

function validatesFixtureModeDoesNotRequestBackendChildren(): void {
  const panelSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/panel.vue', import.meta.url),
    'utf8'
  )
  const fixtureSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/fixture.ts', import.meta.url),
    'utf8'
  )

  assert.match(panelSource, /if \(!isLiveTree\.value\) \{/)
  assert.match(panelSource, /void requestDirectoryChildren\(nodeId\)/)
  assert.doesNotMatch(fixtureSource, /readLiteralHierarchyChildren/)
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

function testLibraryBoundary(
  readLiteralHierarchyChildren: (
    request: LibraryHierarchyReadRequest
  ) => Promise<LibraryHierarchyReadResult>
): LibraryHierarchyBoundaryApi {
  return {
    getStatus: async () => ({
      state: 'started',
      environment: 'development',
      binaryPolicy: {
        kind: 'developmentBinary',
        source: 'environmentOverride'
      },
      lastError: null
    }),
    onStatusChanged: () => () => undefined,
    readLiteralHierarchyChildren
  }
}

function directoryRootHierarchyReadResult(): Extract<
  LibraryHierarchyReadResult,
  { state: 'ready' }
> {
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

function loadedDirectoryHierarchyReadResult(
  parentSourceDirectoryId: string
): Extract<LibraryHierarchyReadResult, { state: 'ready' }> {
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
      parentSourceDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 2,
      nodes: [
        {
          id: `source-file:${parentSourceDirectoryId}-track`,
          kind: 'file',
          label: 'track.wav',
          sourceFileId: `${parentSourceDirectoryId}11`,
          parentSourceDirectoryId,
          presenceState: 'present',
          updatedAtMs: 101
        },
        directoryNode('99', 'Nested Album', parentSourceDirectoryId)
      ]
    }
  }
}

function emptyDirectoryHierarchyReadResult(
  parentSourceDirectoryId: string
): Extract<LibraryHierarchyReadResult, { state: 'ready' }> {
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
      parentSourceDirectoryId,
      offset: 0,
      limit: 50,
      totalRows: 0,
      nodes: []
    }
  }
}

function hierarchyReadError(
  state: Exclude<LibraryHierarchyReadResult['state'], 'ready'>,
  code: LibraryHierarchyReadErrorCode,
  message: string
): LibraryHierarchyReadResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function directoryNode(
  sourceDirectoryId: string,
  label: string,
  parentSourceDirectoryId?: string
): Extract<
  Extract<LibraryHierarchyReadResult, { state: 'ready' }>['window']['nodes'][number],
  { kind: 'directory' }
> {
  return {
    id: `source-directory:${sourceDirectoryId}`,
    kind: 'directory',
    label,
    sourceDirectoryId,
    ...(parentSourceDirectoryId === undefined ? {} : { parentSourceDirectoryId }),
    presenceState: 'present',
    updatedAtMs: 100
  }
}

function firstProjectedDirectoryStateKind(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): string | undefined {
  return findProjectedNode(nodes, nodeId)?.childrenState.kind
}

function findProjectedNode(
  nodes: readonly BrowserTreeNode[],
  nodeId: string
): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.childrenState.kind === 'loaded') {
      const child = findProjectedNode(node.childrenState.children, nodeId)

      if (child !== undefined) {
        return child
      }
    }
  }

  return undefined
}

function assertReadError(
  result: LibraryHierarchyReadResult,
  code: LibraryHierarchyReadErrorCode
): void {
  if (result.state === 'ready') {
    assert.fail(`expected hierarchy read error ${String(code)}`)
  }

  assert.equal(result.error.code, code)
}
