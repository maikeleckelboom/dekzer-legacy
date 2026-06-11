import { libraryControlChannels } from '../../../src/shared/library/boundary/controlPlane'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  isValidViewState,
  readViewStateFromHost,
  registerLibraryViewStateIpc,
  writeViewStateToHost
} from '../../../src/main/library/viewState/persistence'
import type { LibraryBoundaryHost } from '../../../src/main/library/boundary/host'
import type { LibraryBoundaryHostConfig } from '../../../src/main/library/boundary/config'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('persisted library view state', () => {
  it('returns empty for missing or malformed payloads', async () => {
    const missingDir = tempRootFor('dekzer-desktop-view-state-missing-')
    await expect(readViewStateFromHost(fakeHost(missingDir))).resolves.toEqual({ state: 'empty' })

    const malformedDir = tempRootFor('dekzer-desktop-view-state-malformed-')
    await expect(
      writeViewStateToHost(fakeHost(malformedDir), {
        version: 1,
        selectedNodeId: 'valid-node',
        expandedNodeIds: ['valid-node']
      })
    ).resolves.toEqual({ state: 'written' })

    writeFileSync(join(malformedDir, 'library-view-state.json'), '{ malformed }', 'utf-8')

    await expect(readViewStateFromHost(fakeHost(malformedDir))).resolves.toEqual({
      state: 'empty'
    })
  })

  it('writes only persisted identifiers and deduplicates expanded ids', async () => {
    const dir = tempRootFor('dekzer-desktop-view-state-write-')

    await expect(
      writeViewStateToHost(fakeHost(dir), {
        version: 1,
        selectedNodeId: 'navigation-row:7',
        expandedNodeIds: ['navigation-row:7', 'source-directory:12', 'navigation-row:7']
      })
    ).resolves.toEqual({ state: 'written' })

    await expect(readViewStateFromHost(fakeHost(dir))).resolves.toEqual({
      state: 'ready',
      viewState: {
        version: 1,
        selectedNodeId: 'navigation-row:7',
        expandedNodeIds: ['navigation-row:7', 'source-directory:12']
      }
    })
  })

  it('validates IPC write payloads before touching persistence', async () => {
    const dir = tempRootFor('dekzer-desktop-view-state-ipc-')
    const registration: {
      read?: () => unknown
      write?: (viewState: unknown) => unknown
    } = {}

    registerLibraryViewStateIpc(
      {
        handle(channel, listener): void {
          if (channel === libraryControlChannels.viewState.read) {
            registration.read = () => listener({})
          }

          if (channel === libraryControlChannels.viewState.write) {
            registration.write = (viewState) => listener({}, viewState)
          }
        }
      },
      fakeHost(dir)
    )

    expect(isValidViewState({ version: 1, expandedNodeIds: [] })).toBe(true)
    expect(isValidViewState({ version: 2, expandedNodeIds: [] })).toBe(false)
    expect(isValidViewState({ version: 1, selectedNodeId: 42, expandedNodeIds: [] })).toBe(false)
    expect(isValidViewState({ version: 1, expandedNodeIds: ['valid', 42] })).toBe(false)

    expect(registration.write?.({ version: 1 })).toEqual({
      state: 'failed',
      detail: 'Invalid view state payload.'
    })
  })

  it('returns failed when persistence cannot write', async () => {
    const path = tempRootFor('dekzer-desktop-view-state-write-failure-')
    rmSync(path, { recursive: true, force: true })
    writeFileSync(path, 'not-a-directory')

    await expect(
      writeViewStateToHost(fakeHost(path), {
        version: 1,
        expandedNodeIds: []
      })
    ).resolves.toMatchObject({
      state: 'failed'
    })
  })
})

function fakeHost(userDataPath: string): LibraryBoundaryHost {
  return {
    config: {
      storageEnvironment: {
        userDataPath
      }
    } as LibraryBoundaryHostConfig
  } as LibraryBoundaryHost
}

function tempRootFor(prefix: string): string {
  const tempRoot = mkdtempSync(join(tmpdir(), prefix))
  tempRoots.push(tempRoot)
  return tempRoot
}
