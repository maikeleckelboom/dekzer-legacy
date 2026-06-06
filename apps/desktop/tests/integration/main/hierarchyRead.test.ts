import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  readThroughHost,
  registerReadChildrenIpc
} from '../../../src/main/libraryHierarchy/readChildren'
import {
  readNavigationRowsThroughHost,
  registerReadNavigationRowsIpc
} from '../../../src/main/libraryNavigation/readRows'
import {
  hierarchyReadChannels,
  type ReadResult
} from '../../../src/shared/libraryHierarchy/readChildren'
import type { LibraryTreeCoverage } from '@dekzer/library-boundary-contract'
import {
  navigationReadChannels,
  type NavigationReadRowsResult
} from '../../../src/shared/libraryNavigation/readRows'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/libraryBoundary'
import { firstAvailableSourceReadRequest } from '../../support/libraryHierarchy'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('hierarchy and navigation reads through the host', () => {
  it('maps hierarchy host, target, success, and malformed row outcomes', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readThroughHost(idleHost, firstAvailableSourceReadRequest())
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    await expect(
      readThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            readNavigationRows: async () => ({ rows: [] })
          })
        ),
        firstAvailableSourceReadRequest()
      )
    ).resolves.toMatchObject({
      state: 'noTarget',
      error: { code: 'noTarget' }
    })

    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readNavigationRows: async () => ({ rows: [sourceNavigationRow()] }),
        readLibraryTreeChildren: async (request) => {
          expect(request).toMatchObject({
            entryPoint: {
              type: 'source',
              payload: { sourceId: '7' }
            },
            parentSourceDirectoryId: null,
            offset: 0,
            limit: 50
          })

          return {
            window: {
              entryPoint: request.entryPoint,
              parentSourceDirectoryId: null,
              offset: request.offset,
              limit: request.limit,
              totalRows: 2,
              coverage: completeCoverage(),
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
                  directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
                  directoryImageMediaState: { kind: 'noImageMediaDescendants' },
                  directoryScanState: 'scanning',
                  navigableChildScopeState: 'hasNavigableChildScopes'
                },
                {
                  nodeKind: 'file',
                  sourceId: '7',
                  sourceDirectoryId: null,
                  sourceFileId: '11',
                  parentSourceDirectoryId: null,
                  relativePath: 'track.wav',
                  displayName: 'track.wav',
                  fileClass: 'audio',
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

    expect(success).toMatchObject({
      state: 'ready',
      window: {
        root: { id: 'source:7', label: 'Source Fixture' },
        nodes: [
          { id: 'source-directory:12', kind: 'directory', label: 'Album' },
          { id: 'source-file:11', kind: 'file', label: 'track.wav' }
        ]
      }
    })

    await expect(
      readThroughHost(successHost, {
        target: {
          kind: 'entryPoint',
          entryPoint: { kind: 'source', sourceId: '0' }
        }
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    await expect(
      readThroughHost(
        await startedHostWithClient(
          config,
          createFakeClient({
            readNavigationRows: async () => ({ rows: [sourceNavigationRow()] }),
            readLibraryTreeChildren: async (request) => ({
              window: {
                entryPoint: request.entryPoint,
                parentSourceDirectoryId: null,
                offset: request.offset,
                limit: request.limit,
                totalRows: 1,
                coverage: completeCoverage(),
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
    ).resolves.toMatchObject({
      state: 'readFailed',
      error: { code: 'readFailed' }
    })

    const dirWithoutNavigableChildScopeState = await readThroughHost(
      await startedHostWithClient(
        config,
        createFakeClient({
          readNavigationRows: async () => ({ rows: [sourceNavigationRow()] }),
          readLibraryTreeChildren: async (request) => ({
            window: {
              entryPoint: request.entryPoint,
              parentSourceDirectoryId: null,
              offset: request.offset,
              limit: request.limit,
              totalRows: 1,
              coverage: completeCoverage(),
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
                  directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
                  directoryImageMediaState: { kind: 'noImageMediaDescendants' },
                  directoryScanState: 'scanning'
                }
              ]
            }
          })
        })
      ),
      firstAvailableSourceReadRequest()
    )

    expect(dirWithoutNavigableChildScopeState).toMatchObject({
      state: 'readFailed',
      error: { code: 'readFailed' }
    })
  })

  it('maps navigation reads and registers read IPC channels', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readNavigationRowsThroughHost(idleHost, { parentNavigationRowId: null })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readNavigationRows: async (request) => {
          expect(request).toEqual({ parentNavigationRowId: null })
          return { rows: [sourceNavigationRow()] }
        }
      })
    )

    await expect(
      readNavigationRowsThroughHost(successHost, { parentNavigationRowId: null })
    ).resolves.toMatchObject({
      state: 'ready',
      rows: [{ displayName: 'Source Fixture' }]
    })

    await expect(
      readNavigationRowsThroughHost(successHost, {
        parentNavigationRowId: 'not-a-navigation-row-id'
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    const hierarchyRegistration: {
      channel?: string
      handler?: (request: unknown) => Promise<ReadResult>
    } = {}
    registerReadChildrenIpc(
      {
        handle(channel, listener): void {
          hierarchyRegistration.channel = channel
          hierarchyRegistration.handler = (request) => listener({}, request)
        }
      },
      idleHost
    )
    expect(hierarchyRegistration.channel).toBe(hierarchyReadChannels.readChildren)
    expect(typeof hierarchyRegistration.handler).toBe('function')

    const navigationRegistration: {
      channel?: string
      handler?: (request: unknown) => Promise<NavigationReadRowsResult>
    } = {}
    registerReadNavigationRowsIpc(
      {
        handle(channel, listener): void {
          navigationRegistration.channel = channel
          navigationRegistration.handler = (request) => listener({}, request)
        }
      },
      idleHost
    )
    expect(navigationRegistration.channel).toBe(navigationReadChannels.readRows)
    expect(typeof navigationRegistration.handler).toBe('function')
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-hierarchy-read-'))
  tempRoots.push(tempRoot)
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  return resolveHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [boundaryStdioBinaryPathEnvVar]: fakeBinaryPath
    },
    platform: 'linux'
  })
}

function sourceNavigationRow(): Awaited<
  ReturnType<ReturnType<typeof createFakeClient>['readNavigationRows']>
>['rows'][number] {
  return {
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
}

function completeCoverage(): LibraryTreeCoverage {
  return {
    state: 'complete' as const,
    subtreeCoverageComplete: true,
    emptyResultAuthoritative: false,
    detail: 'Complete.'
  }
}
