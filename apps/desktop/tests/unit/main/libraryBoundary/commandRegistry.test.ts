import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import { registerLibraryIpcCommands } from '../../../../src/main/library/boundary/commandRegistry'
import {
  boundaryStdioBinaryPathEnvVar,
  resolveHostConfig
} from '../../../../src/main/library/boundary/config'
import { BoundaryEventPump } from '../../../../src/main/library/boundary/eventPump'
import { LibraryBoundaryHost } from '../../../../src/main/library/boundary/host'
import { HostStatusController } from '../../../../src/main/library/boundary/status'
import { libraryControlChannels } from '../../../../src/shared/library/boundary/controlPlane'
import { silentLogger, silentStatusLogger, testApp } from '../../../support/library/boundary'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('registerLibraryIpcCommands', () => {
  it('registers all library control-plane channels in the command registry', () => {
    const { registrations } = registerCommands()

    expect([...registrations.keys()].sort()).toEqual([...expectedLibraryControlChannels()].sort())
  })

  it('delegates registered handlers to existing operation behavior', async () => {
    const { registrations, boundaryEventPump } = registerCommands()
    const webContents = testWebContents(17)

    await expect(
      registrations.get(libraryControlChannels.contents.read)?.({}, null)
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })
    await expect(
      registrations.get(libraryControlChannels.musicalAnalysis.analyzePlayableMedia)?.({}, null)
    ).resolves.toMatchObject({
      result: {
        status: 'blocked',
        warnings: [{ code: 'invalid_request' }]
      }
    })

    expect(
      registrations.get(libraryControlChannels.boundary.events.subscribe)?.({
        sender: webContents
      })
    ).toEqual({ kind: 'subscribed' })
    expect(boundaryEventPump.subscriberCount).toBe(1)

    expect(
      registrations.get(libraryControlChannels.boundary.events.unsubscribe)?.({
        sender: webContents
      })
    ).toEqual({ kind: 'unsubscribed' })
    expect(boundaryEventPump.subscriberCount).toBe(0)
  })
})

function registerCommands(): {
  readonly registrations: Map<string, (event: unknown, ...args: readonly unknown[]) => unknown>
  readonly boundaryEventPump: BoundaryEventPump
} {
  const host = new LibraryBoundaryHost(hostConfig(), silentLogger())
  const hostStatusController = new HostStatusController(host, silentStatusLogger())
  const boundaryEventPump = new BoundaryEventPump(host)
  const registrations = new Map<string, (event: unknown, ...args: readonly unknown[]) => unknown>()

  registerLibraryIpcCommands({
    ipcMain: {
      handle(channel, listener): void {
        registrations.set(channel, listener)
      }
    },
    host,
    hostStatusController,
    boundaryEventPump,
    localRootChoiceDependencies: {
      dialog: {
        async showOpenDialog() {
          return { canceled: true, filePaths: [] }
        }
      }
    }
  })

  return { registrations, boundaryEventPump }
}

function expectedLibraryControlChannels(): readonly string[] {
  return [
    libraryControlChannels.attachmentIdentity.readSourceFileAttachment,
    libraryControlChannels.attachmentIdentity.readAttachmentSourceFiles,
    libraryControlChannels.attachmentIdentity.readSourceAttachmentSummary,
    libraryControlChannels.boundary.getStatus,
    libraryControlChannels.boundary.events.subscribe,
    libraryControlChannels.boundary.events.unsubscribe,
    libraryControlChannels.contents.read,
    libraryControlChannels.searchFilter.read,
    libraryControlChannels.hierarchy.read,
    libraryControlChannels.localBrowse.items.read,
    libraryControlChannels.localBrowse.entryPoints.read,
    libraryControlChannels.musicalAnalysis.analyzePlayableMedia,
    libraryControlChannels.navigation.read,
    libraryControlChannels.roots.cancel,
    libraryControlChannels.roots.chooseLocal,
    libraryControlChannels.roots.read,
    libraryControlChannels.roots.registerLocalPath,
    libraryControlChannels.roots.scan,
    libraryControlChannels.roots.unregister,
    libraryControlChannels.source.activity,
    libraryControlChannels.source.fileHashing,
    libraryControlChannels.source.integrity,
    libraryControlChannels.source.lifecycle,
    libraryControlChannels.source.maintenance.run,
    libraryControlChannels.source.maintenance.read,
    libraryControlChannels.trackIdentity.candidates.read,
    libraryControlChannels.trackIdentity.decisions.accept,
    libraryControlChannels.trackIdentity.decisions.reject,
    libraryControlChannels.trackIdentity.decisions.defer,
    libraryControlChannels.viewState.read,
    libraryControlChannels.viewState.write
  ]
}

function hostConfig(): ConstructorParameters<typeof LibraryBoundaryHost>[0] {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-command-registry-'))
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

function testWebContents(id: number): Parameters<BoundaryEventPump['subscribe']>[0] {
  const sentMessages: unknown[] = []

  return {
    id,
    send: (...args: unknown[]) => {
      sentMessages.push(args)
    }
  }
}
