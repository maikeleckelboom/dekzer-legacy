import { describe, expect, it } from 'vitest'

import { mapLibraryTreeNode } from '../../../../src/main/libraryHierarchy/mapping'
import type { LibraryTreeNode } from '@dekzer/library-boundary-contract'

function makeDirectoryNode(overrides: Partial<LibraryTreeNode> = {}): LibraryTreeNode {
  return {
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
    childRowState: 'hasChildRows',
    ...overrides
  }
}

function makeFileNode(overrides: Partial<LibraryTreeNode> = {}): LibraryTreeNode {
  return {
    nodeKind: 'file',
    sourceId: '7',
    sourceDirectoryId: null,
    sourceFileId: '11',
    parentSourceDirectoryId: null,
    relativePath: 'track.wav',
    displayName: 'track.wav',
    mediaClass: 'audio',
    presenceState: 'present',
    sizeBytes: null,
    modifiedAtNs: null,
    updatedAtMs: 101,
    ...overrides
  }
}

describe('mapLibraryTreeNode', () => {
  it('maps a well-formed directory node', () => {
    const result = mapLibraryTreeNode(makeDirectoryNode())

    expect(result).not.toBeUndefined()
    expect(result).toMatchObject({
      id: 'source-directory:12',
      kind: 'directory',
      label: 'Album',
      sourceId: '7',
      directoryId: '12',
      presence: 'present',
      hasChildDirectories: true,
      directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
      directoryImageMediaState: { kind: 'noImageMediaDescendants' },
      directoryScanState: 'scanning',
      childRowState: 'hasChildRows',
      updatedAtMs: 100
    })
  })

  it('rejects a directory node without sourceDirectoryId', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ sourceDirectoryId: null })
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without childRowState', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ childRowState: undefined })
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryScanState', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ directoryScanState: undefined })
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryPrimaryMediaState', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ directoryPrimaryMediaState: undefined })
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryImageMediaState', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ directoryImageMediaState: undefined })
    )

    expect(result).toBeUndefined()
  })

  it('defaults missing hasChildDirectories to false for directory nodes', () => {
    const result = mapLibraryTreeNode(
      makeDirectoryNode({ hasChildDirectories: undefined })
    )

    expect(result).not.toBeUndefined()
    if (result?.kind === 'directory') {
      expect(result.hasChildDirectories).toBe(false)
    }
  })

  it('maps a well-formed file node', () => {
    const result = mapLibraryTreeNode(makeFileNode())

    expect(result).not.toBeUndefined()
    expect(result).toMatchObject({
      id: 'source-file:11',
      kind: 'file',
      label: 'track.wav',
      sourceId: '7',
      fileId: '11',
      mediaClass: 'audio',
      presence: 'present',
      updatedAtMs: 101
    })
  })

  it('rejects a file node without sourceFileId', () => {
    const result = mapLibraryTreeNode(
      makeFileNode({ sourceFileId: null })
    )

    expect(result).toBeUndefined()
  })

  it('defaults missing mediaClass to none for file nodes', () => {
    const result = mapLibraryTreeNode(
      makeFileNode({ mediaClass: undefined })
    )

    expect(result).not.toBeUndefined()
    if (result?.kind === 'file') {
      expect(result.mediaClass).toBe('none')
    }
  })
})
