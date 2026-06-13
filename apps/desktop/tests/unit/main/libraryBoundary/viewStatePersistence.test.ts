import { describe, expect, it } from 'vitest'

import { isValidViewState } from '../../../../src/main/library/viewState/persistence'

describe('library view-state persistence', () => {
  it('accepts only strict version 2 canonical view state', () => {
    expect(
      isValidViewState({
        version: 2,
        activeSurface: 'libraryBrowse',
        selectedLibraryNodeId: 'node-1',
        selectedAddSourceNodeId: 'local-browse:section',
        expandedLibraryNodeIds: ['node-1'],
        expandedAddSourceNodeIds: ['local-browse:section'],
        libraryBrowseProfile: 'audio',
        localPreviewMode: 'musicEvidence'
      })
    ).toBe(true)
  })

  it('rejects stale version 1 and generic profile residue', () => {
    expect(
      isValidViewState({
        version: 1,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        libraryBrowseProfile: 'audio',
        localPreviewMode: 'musicEvidence'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 2,
        activeSurface: 'libraryBrowse',
        expandedLibraryNodeIds: [],
        expandedAddSourceNodeIds: [],
        profile: 'audio'
      })
    ).toBe(false)

    expect(
      isValidViewState({
        version: 2,
        selectedNodeId: 'node-1',
        expandedNodeIds: ['node-1'],
        libraryBrowseProfile: 'audio',
        localPreviewMode: 'musicEvidence'
      })
    ).toBe(false)
  })
})
