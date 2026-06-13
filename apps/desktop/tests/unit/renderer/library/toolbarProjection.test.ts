import { describe, expect, it } from 'vitest'

import {
  projectLibraryToolbar,
  type LibraryToolbarInput
} from '../../../../src/renderer/library/runtime/toolbarProjection'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'

describe('library toolbar projection', () => {
  it('shows only library start actions when nothing is selected', () => {
    const toolbar = projectLibraryToolbar(input({ selectedNodeId: undefined }))

    expect(toolbar.scope).toBe('libraryStart')
    expect(toolbar.addMusicFolder).toMatchObject({
      visible: true,
      enabled: true,
      label: 'Add music folder'
    })
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.browseProfile.visible).toBe(false)
  })

  it('scopes Local Files root to admission actions without indexed search', () => {
    const toolbar = projectLibraryToolbar(
      input({
        selectedNodeId: 'selected',
        binding: { kind: 'localBrowseSection' }
      })
    )

    expect(toolbar.scope).toBe('localAdmission')
    expect(toolbar.addMusicFolder.visible).toBe(true)
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.browseProfile).toMatchObject({
      visible: true,
      label: 'Local preview filter',
      title: 'Local preview filter: Audio'
    })
  })

  it('scopes admitted source rows to indexed controls only', () => {
    const toolbar = projectLibraryToolbar(
      input({
        selectedNodeId: 'selected',
        binding: sourceBinding()
      })
    )

    expect(toolbar.scope).toBe('indexedLibrary')
    expect(toolbar.addMusicFolder.visible).toBe(false)
    expect(toolbar.search).toMatchObject({
      visible: true,
      label: 'Search indexed library'
    })
    expect(toolbar.browseProfile).toMatchObject({
      visible: true,
      label: 'Indexed contents view'
    })
  })

  it('hides scoped controls for neutral read-state selections', () => {
    const toolbar = projectLibraryToolbar(
      input({
        selectedNodeId: 'selected',
        binding: {
          kind: 'readState',
          state: 'empty',
          ownerId: 'navigation',
          detail: 'No sources.'
        }
      })
    )

    expect(toolbar.scope).toBe('neutral')
    expect(toolbar.addMusicFolder.visible).toBe(false)
    expect(toolbar.search.visible).toBe(false)
    expect(toolbar.browseProfile.visible).toBe(false)
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
    selectedNodeId: 'selected',
    selectedBrowseProfileLabel: 'Audio',
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
