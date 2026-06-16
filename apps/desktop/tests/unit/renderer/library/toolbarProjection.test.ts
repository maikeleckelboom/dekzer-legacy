import { describe, expect, it } from 'vitest'

import {
  projectLibraryToolbar,
  type LibraryToolbarInput
} from '../../../../src/renderer/library/runtime/toolbarProjection'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'

describe('library toolbar projection', () => {
  it('shows Library Browse controls and an Add Source path without local preview controls', () => {
    const toolbar = projectLibraryToolbar(
      input({ activeSurface: 'libraryBrowse', selectedNodeId: undefined })
    )

    expect(toolbar.scope).toBe('libraryBrowse')
    expect(toolbar.addMusicFolder).toMatchObject({
      kind: 'openAddSource',
      visible: true,
      enabled: true,
      label: 'Add Source'
    })
    expect(toolbar.search.visible).toBe(true)
    expect(toolbar.search.placeholder).toBe('Search library-wide')
    expect(toolbar.viewMode).toMatchObject({
      visible: true,
      label: 'View',
      title: 'View: List'
    })
    expect(toolbar.libraryBrowseProfile.visible).toBe(true)
    expect(toolbar.addSourceView.visible).toBe(false)
  })

  it('scopes Add Source root to local preview controls without indexed search', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'addSource',
        selectedNodeId: 'selected',
        binding: { kind: 'addSourceSection' }
      })
    )

    expect(toolbar.scope).toBe('addSource')
    expect(toolbar.addMusicFolder.visible).toBe(true)
    expect(toolbar.addMusicFolder.kind).toBe('chooseMusicFolder')
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.viewMode.visible).toBe(false)
    expect(toolbar.libraryBrowseProfile.visible).toBe(false)
    expect(toolbar.addSourceView).toMatchObject({
      visible: true,
      label: 'Add Source view',
      title: 'Add Source view: Preview'
    })
  })

  it('keeps Inventory explicit and hides folder picker/search', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'addSource',
        selectedNodeId: 'selected',
        binding: { kind: 'addSourceSection' },
        addSourceView: 'inventory',
        selectedAddSourceViewLabel: 'Inventory'
      })
    )

    expect(toolbar.scope).toBe('addSource')
    expect(toolbar.addMusicFolder.visible).toBe(false)
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.viewMode.visible).toBe(false)
    expect(toolbar.libraryBrowseProfile.visible).toBe(false)
    expect(toolbar.addSourceView).toMatchObject({
      visible: true,
      title: 'Add Source view: Inventory'
    })
  })

  it('does not duplicate add actions for an eligible local preview selection', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'addSource',
        selectedNodeId: 'selected',
        binding: {
          kind: 'localBrowseEntryPoint',
          entry: {
            identity: {
              entryPointKind: 'music',
              resolvedPath: 'C:/Music'
            },
            displayName: 'Music',
            status: 'available',
            platform: 'windows',
            availableOperations: [
              { kind: 'browseChildren' },
              { kind: 'chooseDescendant' },
              {
                kind: 'requestSourceAdmission',
                requestKind: 'defaultMusicFolder',
                resolvedPath: 'C:/Music'
              }
            ],
            failure: null
          },
          target: {
            entryPointKind: 'music',
            resolvedRootPath: 'C:/Music',
            label: 'Music'
          }
        }
      })
    )

    expect(toolbar.scope).toBe('addSource')
    expect(toolbar.addMusicFolder.visible).toBe(false)
    expect(toolbar.addSourceView.visible).toBe(true)
  })

  it('scopes admitted source rows to indexed controls only', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'libraryBrowse',
        selectedNodeId: 'selected',
        binding: sourceBinding()
      })
    )

    expect(toolbar.scope).toBe('libraryBrowse')
    expect(toolbar.addMusicFolder).toMatchObject({
      kind: 'openAddSource',
      visible: true,
      label: 'Add Source'
    })
    expect(toolbar.search).toMatchObject({
      visible: true,
      label: 'Search indexed library',
      placeholder: 'Search inside selected source'
    })
    expect(toolbar.viewMode).toMatchObject({
      visible: true,
      title: 'View: List'
    })
    expect(toolbar.libraryBrowseProfile).toMatchObject({
      visible: true,
      label: 'Indexed contents view'
    })
    expect(toolbar.addSourceView.visible).toBe(false)
  })

  it('labels search as selected-folder scoped for admitted folders', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'libraryBrowse',
        selectedNodeId: 'selected',
        binding: {
          kind: 'directory',
          sourceId: '7',
          directoryId: '12',
          entryPoint: { kind: 'source', sourceId: '7' },
          label: 'Album'
        }
      })
    )

    expect(toolbar.search).toMatchObject({
      visible: true,
      placeholder: 'Search inside selected folder'
    })
  })

  it('hides scoped controls for neutral read-state selections', () => {
    const toolbar = projectLibraryToolbar(
      input({
        activeSurface: 'addSource',
        selectedNodeId: 'selected',
        binding: {
          kind: 'readState',
          state: 'empty',
          parentNodeId: 'navigation',
          detail: 'No sources.'
        }
      })
    )

    expect(toolbar.scope).toBe('addSource')
    expect(toolbar.addMusicFolder.visible).toBe(true)
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.viewMode.visible).toBe(false)
    expect(toolbar.libraryBrowseProfile.visible).toBe(false)
    expect(toolbar.addSourceView.visible).toBe(true)
  })
})

function input(
  overrides: Partial<LibraryToolbarInput> & {
    readonly binding?: RowBinding
  } = {}
): LibraryToolbarInput {
  return {
    projection:
      overrides.binding === undefined ? undefined : projectionForBinding(overrides.binding),
    activeSurface: 'libraryBrowse',
    selectedNodeId: 'selected',
    selectedViewModeLabel: 'List',
    selectedLibraryBrowseProfileLabel: 'Audio',
    selectedAddSourceViewLabel: 'Preview',
    addSourceView: 'preview',
    addMusicFolderLabel: 'Add music folder',
    canAddMusicFolder: true,
    ...overrides
  }
}

function projectionForBinding(binding: RowBinding): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [],
    bindingsById: new Map([['selected', binding]])
  }
}

function sourceBinding(): RowBinding {
  return {
    kind: 'source',
    navigationRow: {
      navigationRowId: '7',
      stableKey: 'source:7',
      parentNavigationRowId: null,
      family: 'sources',
      rowKind: 'source',
      displayName: 'Music',
      siblingPosition: 0,
      selectable: true,
      selectorKind: 'source',
      selectorPayload: '7',
      updatedAtMs: 100,
      rowVersion: '1'
    },
    target: {
      navigationRowId: '7',
      entryPoint: { kind: 'source', sourceId: '7' },
      label: 'Music'
    }
  }
}
