import { describe, expect, it } from 'vitest'

import { isValidViewState } from '../../../../src/main/library/viewState/persistence'

describe('library view-state persistence', () => {
  it('accepts only strict version 3 canonical view state', () => {
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 'node-1',
        selectedAddSourceNodeId: 'add-source:section',
        expandedLibraryNodeIds: ['node-1'],
        expandedAddSourceNodeIds: ['add-source:section'],
        libraryBrowseProfile: 'audio',
        addSourceView: 'preview'
      })
    ).toBe(true)
  })

  it('rejects stale versions and generic profile residue', () => {
    expect(
      isValidViewState({
        version: 1,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        libraryBrowseProfile: 'audio',
        addSourceView: 'preview'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 2,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        libraryBrowseProfile: 'audio',
        addSourceView: 'preview'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        profile: 'audio'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 3,
        selectedNodeId: 'node-1',
        expandedNodeIds: ['node-1'],
        libraryBrowseProfile: 'audio',
        addSourceView: 'preview'
      })
    ).toBe(false)
  })

  it('rejects stale Add Source view-state vocabulary', () => {
    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'addSource',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        localPreviewMode: 'musicEvidence'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'addSource',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        addSourceView: 'musicEvidence'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 3,
        activeSurface: 'addSource',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        addSourceView: 'advancedInventory'
      })
    ).toBe(false)
  })
})
