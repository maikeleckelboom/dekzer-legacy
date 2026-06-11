import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import type {
  ReadAttachmentSourceFilesRequest,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceFileAttachmentRequest
} from '@dekzer/library-boundary-contract'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import {
  readAttachmentSourceFilesThroughHost,
  readSourceAttachmentSummaryThroughHost,
  readSourceFileAttachmentThroughHost
} from '../../../src/main/library/attachmentIdentity/read'

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

describe('attachment identity reads through the host', () => {
  it('validates ids, forwards read requests, and returns typed replies', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(
      readSourceFileAttachmentThroughHost(idleHost, { sourceFileId: '11' })
    ).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: { code: 'hostNotStarted' }
    })

    let receivedSourceFileRequest: ReadSourceFileAttachmentRequest | undefined
    let receivedAttachmentRequest: ReadAttachmentSourceFilesRequest | undefined
    let receivedSummaryRequest: ReadSourceAttachmentSummaryRequest | undefined
    const successHost = await startedHostWithClient(
      config,
      createFakeClient({
        readSourceFileAttachment: async (request) => {
          receivedSourceFileRequest = request
          return {
            status: 'ok',
            attachmentLink: {
              attachmentId: '7',
              sourceFileId: '11',
              sourceId: '3',
              contentHashAlgorithm: 'blake3',
              contentHashValue: 'abc',
              fileKind: 'audio',
              linkStatus: 'current',
              createdAtMs: 100,
              updatedAtMs: 200
            }
          }
        },
        readAttachmentSourceFiles: async (request) => {
          receivedAttachmentRequest = request
          return {
            status: 'notFound',
            sourceFileLinks: [],
            effectiveLimit: 25,
            remainingSourceFileLinks: 0
          }
        },
        readSourceAttachmentSummary: async (request) => {
          receivedSummaryRequest = request
          return {
            status: 'ok',
            summary: {
              sourceId: '3',
              currentLinksCount: 1,
              staleLinksCount: 0,
              sourceFilesWithCurrentBlake3FactsCount: 1,
              sourceFilesWithAttachmentLinksCount: 1,
              sourceFilesMissingAttachmentLinksCount: 0,
              unmaterializedBlake3FactsCount: 0
            }
          }
        }
      })
    )

    await expect(
      readSourceFileAttachmentThroughHost(successHost, {
        sourceFileId: '11',
        materialize: true
      })
    ).resolves.toMatchObject({
      state: 'ok',
      reply: {
        status: 'ok',
        attachmentLink: {
          attachmentId: '7',
          sourceFileId: '11',
          linkStatus: 'current'
        }
      }
    })
    expect(receivedSourceFileRequest).toEqual({
      sourceFileId: '11'
    })

    await expect(
      readAttachmentSourceFilesThroughHost(successHost, {
        attachmentId: '7',
        limit: 25,
        relocate: true
      })
    ).resolves.toMatchObject({
      state: 'notFound',
      reply: {
        status: 'notFound',
        sourceFileLinks: []
      }
    })
    expect(receivedAttachmentRequest).toEqual({
      attachmentId: '7',
      limit: 25
    })

    await expect(
      readSourceAttachmentSummaryThroughHost(successHost, {
        sourceId: '3',
        runMaintenance: true
      })
    ).resolves.toMatchObject({
      state: 'ok',
      reply: {
        summary: {
          currentLinksCount: 1
        }
      }
    })
    expect(receivedSummaryRequest).toEqual({
      sourceId: '3'
    })

    await expect(
      readSourceFileAttachmentThroughHost(successHost, { sourceFileId: '0' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readAttachmentSourceFilesThroughHost(successHost, { attachmentId: '7', limit: 0 })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      readSourceAttachmentSummaryThroughHost(successHost, { sourceId: '-1' })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-attachment-identity-'))
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
