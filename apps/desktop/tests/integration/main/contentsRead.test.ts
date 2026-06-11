import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  ContentsScopeCoverage,
  ContentsReadReply,
  ContentsReadRequest
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import { readContentsThroughHost } from '../../../src/main/library/contents/read'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/library/boundary'

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
              kind: 'sourceFileInventory',
              fileClasses: ['audio', 'image', 'unsupported']
            },
            scopeDepth: 'recursive',
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
          kind: 'sourceFileInventory',
          fileClasses: ['audio', 'image', 'unsupported']
        },
        scopeDepth: 'recursive',
        rows: [
          {
            id: 'source-file:12',
            sourceId: '7',
            sourceFileId: '12',
            label: 'Cover.jpg',
            fileClass: 'image'
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
            scopeDepth: request.scopeDepth,
            rows: [],
            scopeCoverage: completeCoverage(),
            hasPolicyOmittedRows: false,
            detail: 'primaryMedia rows do not support image media kinds.'
          }
        })
      })
    )

    await expect(
      readContentsThroughHost(conflictHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          kind: 'primaryMedia',
          mediaKinds: ['audio']
        },
        scopeDepth: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'policyConflict',
      error: {
        code: 'policyConflict',
        message: 'primaryMedia rows do not support image media kinds.'
      }
    })

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          kind: 'sourceFileInventory',
          fileClasses: []
        },
        scopeDepth: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          kind: 'sourceFileInventory',
          fileClasses: 'audio'
        },
        scopeDepth: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          kind: 'sourceFileInventory',
          fileClasses: ['none']
        },
        scopeDepth: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('passes audioBrowse policy through with no caller filter', async () => {
    const config = hostConfig()
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readContents: async (request) => {
          expect(request.policy).toEqual({ kind: 'audioBrowse' })
          return {
            result: {
              state: 'empty',
              scope: request.scope,
              policy: request.policy,
              scopeDepth: request.scopeDepth,
              rows: [],
              scopeCoverage: {
                state: 'complete',
                subtreeCoverageComplete: true,
                emptyResultAuthoritative: true
              },
              hasPolicyOmittedRows: false
            }
          }
        }
      })
    )

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: {
          kind: 'audioBrowse'
        },
        scopeDepth: 'recursive',
        limit: 25
      })
    ).resolves.toMatchObject({
      state: 'ready',
      result: {
        policy: {
          kind: 'audioBrowse'
        }
      }
    })
  })

  it('passes playableMediaBrowse and required omission metadata without fileClasses', async () => {
    const config = hostConfig()
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readContents: async (request) => {
          expect(request.policy).toEqual({ kind: 'playableMediaBrowse' })
          expect(request.policy).not.toHaveProperty('fileClasses')
          return {
            result: {
              state: 'empty',
              scope: request.scope,
              policy: request.policy,
              scopeDepth: request.scopeDepth,
              rows: [],
              scopeCoverage: {
                state: 'complete',
                subtreeCoverageComplete: true,
                emptyResultAuthoritative: false
              },
              hasPolicyOmittedRows: true
            }
          }
        }
      })
    )

    await expect(
      readContentsThroughHost(successHost, {
        scope: { kind: 'source', sourceId: '7' },
        policy: { kind: 'playableMediaBrowse' },
        scopeDepth: 'recursive'
      })
    ).resolves.toMatchObject({
      state: 'ready',
      result: {
        policy: { kind: 'playableMediaBrowse' },
        hasPolicyOmittedRows: true
      }
    })
  })
})

function sourceLocationRequest(): Parameters<typeof readContentsThroughHost>[1] {
  return {
    scope: {
      kind: 'sourceLocation',
      sourceLocationId: '33'
    },
    policy: {
      kind: 'sourceFileInventory',
      fileClasses: ['unsupported', 'image', 'audio', 'image']
    },
    scopeDepth: 'recursive',
    limit: 50
  }
}

function readyContentsReply(request: ContentsReadRequest): ContentsReadReply {
  return {
    result: {
      state: 'ready',
      scope: request.scope,
      policy: request.policy,
      scopeDepth: request.scopeDepth,
      rows: [
        {
          id: 'source-file:12',
          sourceId: '7',
          sourceFileId: '12',
          parentDirectoryId: null,
          label: 'Cover.jpg',
          relativePath: 'Covers/Cover.jpg',
          fileName: 'Cover.jpg',
          fileClass: 'image',
          fileKind: 'image',
          presence: 'present',
          updatedAtMs: 100
        }
      ],
      scopeCoverage: completeCoverage(),
      hasPolicyOmittedRows: false
    }
  }
}

function completeCoverage(): ContentsScopeCoverage {
  return {
    state: 'complete',
    subtreeCoverageComplete: true,
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
