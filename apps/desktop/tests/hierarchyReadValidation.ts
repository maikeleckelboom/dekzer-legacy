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
import {
  readLiteralHierarchyChildrenThroughHost,
  registerLibraryHierarchyReadIpc
} from '../src/main/libraryHierarchy/read'
import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadErrorCode,
  type LibraryHierarchyReadRequest,
  type LibraryHierarchyReadResult
} from '../src/shared/libraryHierarchy/read'
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

function firstAvailableSourceReadRequest(): LibraryHierarchyReadRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
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
