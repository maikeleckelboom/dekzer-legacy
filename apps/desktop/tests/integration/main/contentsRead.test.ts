import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  ContentsCoverage,
  ContentsReadReply,
  ContentsReadRequest
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  readContentsThroughHost,
  registerContentsReadIpc
} from '../../../src/main/libraryContents/read'
import {
  contentsReadChannels,
  type ContentsReadResult
} from '../../../src/shared/libraryContents/read'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/libraryBoundary'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('contents reads through the host', () => {
  it('maps host state, sourceLocation scope, policy, success, and policy conflicts', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(readContentsThroughHost(idleHost, sourceLocationRequest())).resolves.toMatchObject(
      {
        state: 'hostUnavailable',
        error: { code: 'hostNotStarted' }
      }
    )

    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readContents: async (request) => {
          expect(request).toEqual({
            scope: {
              type: 'sourceLocation',
              payload: { sourceLocationId: '33' }
            },
            policy: {
              mediaClasses: ['audio', 'image', 'unsupported'],
              rowProfile: { kind: 'sourceFile' }
            },
            recursion: 'recursive',
            limit: 50
          } satisfies ContentsReadRequest)

          return readyContentsReply(request)
        }
      })
    )

    await expect(
      readContentsThroughHost(successHost, sourceLocationRequest())
    ).resolves.toMatchObject({
      state: 'ready',
      result: {
        scope: { kind: 'sourceLocation', sourceLocationId: '33' },
        policy: {
          mediaClasses: ['audio', 'image', 'unsupported'],
          rowProfile: { kind: 'sourceFile' }
        },
        recursion: 'recursive',
        rows: [
          {
            id: 'source-file:12',
            sourceId: '7',
            sourceFileId: '12',
            label: 'Cover.jpg',
            mediaClass: 'image'
          }
        ]
      }
    })

    const conflictHost = await startedHostWithClient(
      config,
      createFakeClient({
        readContents: async (request) => ({
          result: {
            state: 'policyConflict',
            scope: request.scope,
            policy: request.policy,
            recursion: request.recursion,
            rows: [],
            coverage: completeCoverage(),
            detail: 'primaryMedia rows do not support image media classes.'
          }
        })
      })
    )

    await expect(
      readContentsThroughHost(conflictHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          mediaClasses: ['audio', 'image'],
          rowProfile: { kind: 'primaryMedia' }
        },
        recursion: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'policyConflict',
      error: {
        code: 'policyConflict',
        message: 'primaryMedia rows do not support image media classes.'
      }
    })

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          mediaClasses: [],
          rowProfile: { kind: 'sourceFile' }
        },
        recursion: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('registers the contents read IPC channel', () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())
    const registration: {
      channel?: string
      handler?: (request: unknown) => Promise<ContentsReadResult>
    } = {}

    registerContentsReadIpc(
      {
        handle(channel, listener): void {
          registration.channel = channel
          registration.handler = (request) => listener({}, request)
        }
      },
      idleHost
    )

    expect(registration.channel).toBe(contentsReadChannels.read)
    expect(typeof registration.handler).toBe('function')
  })
})

function sourceLocationRequest(): Parameters<typeof readContentsThroughHost>[1] {
  return {
    scope: {
      kind: 'sourceLocation',
      sourceLocationId: '33'
    },
    policy: {
      mediaClasses: ['unsupported', 'image', 'audio', 'image'],
      rowProfile: { kind: 'sourceFile' }
    },
    recursion: 'recursive',
    limit: 50
  }
}

function readyContentsReply(request: ContentsReadRequest): ContentsReadReply {
  return {
    result: {
      state: 'ready',
      scope: request.scope,
      policy: request.policy,
      recursion: request.recursion,
      rows: [
        {
          id: 'source-file:12',
          sourceId: '7',
          sourceFileId: '12',
          parentDirectoryId: null,
          label: 'Cover.jpg',
          relativePath: 'Covers/Cover.jpg',
          fileName: 'Cover.jpg',
          mediaClass: 'image',
          fileKind: 'image',
          presence: 'present',
          updatedAtMs: 100
        }
      ],
      coverage: completeCoverage()
    }
  }
}

function completeCoverage(): ContentsCoverage {
  return {
    state: 'complete',
    recursiveScopeComplete: true,
    emptyResultAuthoritative: false,
    detail: 'Complete.'
  }
}

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-contents-read-'))
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
