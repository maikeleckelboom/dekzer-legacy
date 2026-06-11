import { libraryControlChannels } from '../../../src/shared/library/boundary/controlPlane'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type { HashSourceFilesBlake3Request } from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import {
  hashSourceFilesBlake3ThroughHost,
  registerSourceFileHashingIpc
} from '../../../src/main/library/source/fileHashing'
import { type HashSourceFilesBlake3Result } from '../../../src/shared/library/source/fileHashing'
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

describe('source file BLAKE3 hashing through the host', () => {
  it('validates source scope, forwards only source id and limit, and returns typed outcomes', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      hashSourceFilesBlake3ThroughHost(idleHost, { sourceId: '7', limit: 4 })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    let receivedRequest: HashSourceFilesBlake3Request | undefined
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        hashSourceFilesBlake3: async (request) => {
          receivedRequest = request
          return {
            effectiveLimit: 4,
            outcomes: [
              {
                sourceFileId: '11',
                sourceId: '7',
                relativePath: 'a.flac',
                status: {
                  type: 'hashed',
                  payload: {
                    contentHashAlgorithm: 'blake3',
                    contentHashValue: 'abc',
                    acceptedArtifactId: '90',
                    workItemId: '91'
                  }
                }
              }
            ],
            hashedCount: 1,
            skippedCount: 0,
            failedCount: 0,
            remainingCandidates: 0
          }
        }
      })
    )

    await expect(
      hashSourceFilesBlake3ThroughHost(successHost, {
        sourceId: '7',
        limit: 4,
        absolutePath: 'C:/RendererMustNotControlThis'
      })
    ).resolves.toMatchObject({
      state: 'completed',
      result: {
        effectiveLimit: 4,
        outcomes: [{ sourceFileId: '11', relativePath: 'a.flac' }],
        hashedCount: 1
      }
    })
    expect(receivedRequest).toEqual({
      sourceId: '7',
      limit: 4
    })

    await expect(
      hashSourceFilesBlake3ThroughHost(successHost, { sourceId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      hashSourceFilesBlake3ThroughHost(successHost, { sourceId: '7', limit: 0 })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })

  it('registers the source file hashing IPC channel', () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())
    const registration: {
      channel?: string
      handler?: (request: unknown) => Promise<HashSourceFilesBlake3Result>
    } = {}

    registerSourceFileHashingIpc(
      {
        handle(channel, listener): void {
          registration.channel = channel
          registration.handler = (request) => listener({}, request)
        }
      },
      idleHost
    )

    expect(registration.channel).toBe(libraryControlChannels.source.fileHashing)
    expect(typeof registration.handler).toBe('function')
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-source-file-hashing-'))
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
