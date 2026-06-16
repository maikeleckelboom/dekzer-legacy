import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  isValidViewState,
  readViewStateFromHost,
  writeViewStateThroughHost,
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
        version: 3,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 'valid-node',
        expandedLibraryNodeIds: ['valid-node'],
        expandedAddSourceNodeIds: []
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
        version: 3,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 'navigation-row:7',
        selectedAddSourceNodeId: 'add-source:section',
        expandedLibraryNodeIds: ['navigation-row:7', 'source-directory:12', 'navigation-row:7'],
        expandedAddSourceNodeIds: ['add-source:section', 'add-source:section'],
        libraryBrowseProfile: 'playable',
        addSourceView: 'inventory'
      })
    ).resolves.toEqual({ state: 'written' })

    await expect(readViewStateFromHost(fakeHost(dir))).resolves.toEqual({
      state: 'ready',
      viewState: {
        version: 3,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 'navigation-row:7',
        selectedAddSourceNodeId: 'add-source:section',
        expandedLibraryNodeIds: ['navigation-row:7', 'source-directory:12'],
        expandedAddSourceNodeIds: ['add-source:section'],
        libraryBrowseProfile: 'playable',
        addSourceView: 'inventory'
      }
    })
  })

  it('validates write payloads before touching persistence', async () => {
    const dir = tempRootFor('dekzer-desktop-view-state-ipc-')

    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: []
      })
    ).toBe(true)
    expect(
      isValidViewState({
        version: 2,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: []
      })
    ).toBe(false)
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 42,
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: []
      })
    ).toBe(false)
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: ['valid', 42],
        expandedAddSourceNodeIds: []
      })
    ).toBe(false)
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        libraryBrowseProfile: 'allFiles'
      })
    ).toBe(true)
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        libraryBrowseProfile: 'invalidProfile'
      })
    ).toBe(false)

    expect(writeViewStateThroughHost(fakeHost(dir), { version: 3 })).toEqual({
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
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: []
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
