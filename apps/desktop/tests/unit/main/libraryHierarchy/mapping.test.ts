import { describe, expect, it } from 'vitest'

import { mapLibraryTreeNode } from '../../../../src/main/library/hierarchy/mapping'
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
    directoryPlayableMediaState: { kind: 'hasPlayableMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'scanning',
    navigableChildScopeState: 'hasNavigableChildScopes',
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
    fileClass: 'audio',
    presenceState: 'present',
    sizeBytes: null,
    modifiedAtNs: null,
    updatedAtMs: 101,
    ...overrides
  }
}

function withoutProperty(node: LibraryTreeNode, property: keyof LibraryTreeNode): LibraryTreeNode {
  const copy: Partial<LibraryTreeNode> = { ...node }
  delete copy[property]
  return copy as LibraryTreeNode
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
      directoryPlayableMediaState: { kind: 'hasPlayableMediaDescendants' },
      directoryImageMediaState: { kind: 'noImageMediaDescendants' },
      directoryScanState: 'scanning',
      navigableChildScopeState: 'hasNavigableChildScopes',
      updatedAtMs: 100
    })
  })

  it('rejects a directory node without sourceDirectoryId', () => {
    const result = mapLibraryTreeNode(makeDirectoryNode({ sourceDirectoryId: null }))

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without navigableChildScopeState', () => {
    const result = mapLibraryTreeNode(
      withoutProperty(makeDirectoryNode(), 'navigableChildScopeState')
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryScanState', () => {
    const result = mapLibraryTreeNode(withoutProperty(makeDirectoryNode(), 'directoryScanState'))

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryPlayableMediaState', () => {
    const result = mapLibraryTreeNode(
      withoutProperty(makeDirectoryNode(), 'directoryPlayableMediaState')
    )

    expect(result).toBeUndefined()
  })

  it('rejects a directory node without directoryImageMediaState', () => {
    const result = mapLibraryTreeNode(
      withoutProperty(makeDirectoryNode(), 'directoryImageMediaState')
    )

    expect(result).toBeUndefined()
  })

  it('defaults missing hasChildDirectories to false for directory nodes', () => {
    const result = mapLibraryTreeNode(withoutProperty(makeDirectoryNode(), 'hasChildDirectories'))

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
      fileClass: 'audio',
      presence: 'present',
      updatedAtMs: 101
    })
  })

  it('rejects a file node without sourceFileId', () => {
    const result = mapLibraryTreeNode(makeFileNode({ sourceFileId: null }))

    expect(result).toBeUndefined()
  })

  it('defaults missing fileClass to none for file nodes', () => {
    const result = mapLibraryTreeNode(withoutProperty(makeFileNode(), 'fileClass'))

    expect(result).not.toBeUndefined()
    if (result?.kind === 'file') {
      expect(result.fileClass).toBe('none')
    }
  })
})
