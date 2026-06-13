import { describe, expect, it } from 'vitest'

import { projectAddSourceProjection } from '../../../../src/renderer/library/addSource/projection'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type {
  LocalBrowseEntryPoint,
  LocalBrowseOperation
} from '../../../../src/shared/library/localBrowse/entryPoints'
import type { LocalBrowseItem } from '../../../../src/shared/library/localBrowse/items'

describe('Add Source projection', () => {
  it('owns the default Add Source start semantics', () => {
    const projection = projectAddSourceProjection({
      entryPointsState: {
        kind: 'ready'
      }
    })

    expect(projection.productState).toBe('suggestionsReady')
    expect(projection.surfaceLabel).toBe('Add Source')
    expect(projection.title).toBe('Add Source')
    expect(projection.detail).toContain('Choose where your music lives')
    expect(projection.detail).toContain('Inventory is for inspection')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Suggested folders',
        action: {
          kind: 'chooseMusicFolder',
          label: 'Add music folder'
        }
      })
    ])
  })

  it('keeps Inventory explicit and non-default', () => {
    const projection = projectAddSourceProjection({ addSourceView: 'inventory' })

    expect(projection.productState).toBe('inventory')
    expect(projection.surfaceLabel).toBe('Inventory')
    expect(projection.detail).toContain('without adding or indexing')
    expect(projectAddSourceProjection({}).surfaceLabel).toBe('Add Source')
  })

  it('projects an eligible selected folder as a preview to inspect before adding', () => {
    const projection = projectAddSourceProjection({
      selectedNodeId: 'selected',
      projection: projectionForBinding({
        kind: 'localBrowseEntryPoint',
        entry: localEntry([admission('defaultMusicFolder', 'C:/Music')]),
        target: {
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          label: 'Music'
        }
      })
    })

    expect(projection.productState).toBe('selectedFolderPreview')
    expect(projection.surfaceLabel).toBe('Preview')
    expect(projection.title).toBe('Music')
    expect(projection.rows).toEqual([
      expect.objectContaining({
        label: 'Preview not loaded',
        action: {
          kind: 'loadLocalBrowseChildren',
          nodeId: 'selected',
          label: 'Load preview'
        }
      })
    ])
  })

  it('uses boundary duplicate identity for the already-added state', () => {
    const projection = projectAddSourceProjection({
      selectedNodeId: 'selected',
      projection: projectionForBinding({
        kind: 'localBrowseItem',
        item: {
          ...localItem([]),
          status: 'duplicateOfAdmittedSource',
          matchedSourceId: '7'
        },
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Admitted',
          label: 'Admitted'
        }
      })
    })

    expect(projection.productState).toBe('alreadyAdded')
    expect(projection.detail).toBe('Already added as a music source.')
  })

  it('asks for a specific folder inside broad roots', () => {
    const projection = projectAddSourceProjection({
      selectedNodeId: 'selected',
      projection: projectionForBinding({
        kind: 'localBrowseEntryPoint',
        entry: localEntry([{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }], {
          identity: { entryPointKind: 'systemDriveRoot', resolvedPath: 'C:/' },
          displayName: 'System Drive'
        }),
        target: {
          entryPointKind: 'systemDriveRoot',
          resolvedRootPath: 'C:/',
          label: 'System Drive'
        }
      })
    })

    expect(projection.productState).toBe('tooBroad')
    expect(projection.detail).toBe('Choose a specific folder inside this drive.')
  })

  it('uses calm guidance for folders that are not useful to add yet', () => {
    const projection = projectAddSourceProjection({
      selectedNodeId: 'selected',
      projection: projectionForBinding({
        kind: 'localBrowseItem',
        item: localItem([{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]),
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Empty',
          label: 'Empty'
        }
      })
    })

    expect(projection.productState).toBe('notUseful')
    expect(projection.detail).toContain('does not look useful as a music source yet')
    expect(`${projection.title} ${projection.detail}`).not.toContain('not a music-source candidate')
  })
})

function projectionForBinding(binding: RowBinding): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [],
    bindingsById: new Map([['selected', binding]])
  }
}

function localEntry(
  operations: readonly LocalBrowseOperation[],
  overrides: Partial<LocalBrowseEntryPoint> = {}
): LocalBrowseEntryPoint {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedPath: 'C:/Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    availableOperations: operations,
    failure: null,
    ...overrides
  }
}

function localItem(operations: readonly LocalBrowseOperation[]): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:/Music',
      resolvedItemPath: 'C:/Music/Empty'
    },
    itemKind: 'directory',
    displayName: 'Empty',
    status: 'available',
    platform: 'windows',
    fileKind: null,
    mediaRelevance: null,
    availableOperations: operations,
    failure: null
  }
}

function admission(
  requestKind: Extract<
    LocalBrowseOperation,
    { readonly kind: 'requestSourceAdmission' }
  >['requestKind'],
  resolvedPath: string
): LocalBrowseOperation {
  return {
    kind: 'requestSourceAdmission',
    requestKind,
    resolvedPath
  }
}
