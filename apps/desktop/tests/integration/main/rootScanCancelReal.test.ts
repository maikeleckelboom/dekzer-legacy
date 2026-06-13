import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import process from 'node:process'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

import type { LibraryBoundaryHostConfig } from '../../../src/main/library/boundary/config'
import { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import { registerLocalRoot } from '../../../src/main/library/roots/register'
import { runLocalRootScanThroughHost } from '../../../src/main/library/roots/scan'
import { cancelRootScanThroughHost } from '../../../src/main/library/roots/cancel'
import type { LocalRootScanResult } from '../../../src/shared/library/roots/scan'
import type {
  CancelRootScanResult,
  CancelRootScanAcceptedResult
} from '../../../src/shared/library/roots/cancel'
import type { LocalRootRegistrationResult } from '../../../src/shared/library/roots/register'
import { silentLogger } from '../../support/library/boundary'

const boundaryStdioBinaryPathEnvVar = 'DEKZER_LIBRARY_BOUNDARY_STDIO_BINARY'

function resolveBinaryPath(): string {
  const envPath = process.env[boundaryStdioBinaryPathEnvVar]?.trim()
  if (envPath && envPath.length > 0) {
    return resolve(envPath)
  }
  const executableName =
    process.platform === 'win32' ? 'library-boundary-stdio.exe' : 'library-boundary-stdio'
  return resolve(process.cwd(), '..', '..', 'target', 'debug', executableName)
}

const binaryPath = resolveBinaryPath()

const describeOrSkip = existsSync(binaryPath) ? describe : describe.skip

const noLog = {
  error() {
    /* silent */
  }
}

describeOrSkip('real root scan cancellation through desktop boundary', () => {
  let host: LibraryBoundaryHost
  let tempRootDir: string

  beforeAll(async () => {
    tempRootDir = await mkdtemp(join(tmpdir(), 'dekzer-desktop-real-cancel-'))
    const userDataPath = join(tempRootDir, 'user-data')
    await mkdir(userDataPath, { recursive: true })

    const sourceRoot = join(tempRootDir, 'large-root')
    await mkdir(sourceRoot, { recursive: true })
    for (let d = 0; d < 40; d++) {
      const dir = join(sourceRoot, `dir-${String(d).padStart(3, '0')}`)
      await mkdir(dir)
      const writes: Promise<void>[] = []
      for (let f = 0; f < 60; f++) {
        writes.push(
          writeFile(
            join(dir, `track-${String(f).padStart(3, '0')}.wav`),
            Buffer.from('not-real-audio-data')
          )
        )
      }
      await Promise.all(writes)
    }

    const config: LibraryBoundaryHostConfig = {
      storageEnvironment: {
        kind: 'userDataRoot',
        userDataPath,
        source: 'environmentOverride'
      },
      environment: 'development',
      binaryPolicy: {
        kind: 'developmentBinary',
        binaryPath,
        source: 'environmentOverride'
      }
    }

    host = new LibraryBoundaryHost(config, silentLogger())
    await host.start()
    expect(host.state).toBe('started')
  })

  afterAll(async () => {
    await host?.stop().catch(() => undefined)
    if (tempRootDir) {
      await rm(tempRootDir, { recursive: true, force: true }).catch(() => undefined)
    }
  })

  it(
    'returns accepted from CancelRootScan, emits SourceScanCancelled, and does not emit SourceScanCompleted',
    { timeout: 120_000 },
    async () => {
      const sourceRoot = join(tempRootDir, 'large-root')

      const registration = (await registerLocalRoot(host, {
        requestedPath: sourceRoot
      })) as LocalRootRegistrationResult
      expect(registration.state).toBe('registered')
      if (registration.state !== 'registered') {
        throw new Error('Expected local root registration to succeed.')
      }
      const rootId = registration.root.rootId

      const scanResult = (await runLocalRootScanThroughHost(
        host,
        { rootId },
        noLog
      )) as LocalRootScanResult
      expect(scanResult.state).toBe('started')
      if (scanResult.state !== 'started') {
        throw new Error('Expected local root scan to start.')
      }
      const scanRunId = scanResult.scanRunId

      const cancelResult = (await cancelRootScanThroughHost(
        host,
        { scanRunId },
        noLog
      )) as CancelRootScanResult
      expect(cancelResult.state).toBe('accepted')
      expect((cancelResult as CancelRootScanAcceptedResult).status).toBe('accepted')

      let cursor: number | null = null
      let started = false
      let cancelled = false
      let completed = false
      let cancelledPhase: string | null = null

      for (let i = 0; i < 120 && !cancelled; i++) {
        const eventsReply = await host.client.readAfterBoundaryEvents({
          lastSeenEventSequence: cursor,
          maxEvents: 64
        })
        cursor = eventsReply.latestEventSequence

        for (const event of eventsReply.events) {
          if (event.type !== 'sourceScanEvent') continue
          const se = event.payload
          if (se.rootId !== rootId || se.scanRunId !== scanRunId) continue

          switch (se.kind) {
            case 'sourceScanStarted':
              started = true
              break
            case 'sourceScanCancelled':
              expect(started).toBe(true)
              expect(completed).toBe(false)
              cancelled = true
              cancelledPhase = se.phase
              break
            case 'sourceScanCompleted':
              completed = true
              break
          }
        }

        if (!cancelled) {
          await new Promise((r) => setTimeout(r, 50))
        }
      }

      expect(started).toBe(true)
      expect(cancelled).toBe(true)
      expect(cancelledPhase).toBe('interrupted')
      expect(completed).toBe(false)
    }
  )
})
