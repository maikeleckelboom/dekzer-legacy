import type { RowBinding } from '../state'
import { sourceAdmissionOperation } from '../localBrowse/projection'
import type { AddSourceView } from '../addSource/view'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { LibraryPanelSurface } from '../../../shared/library/viewState/persistence'

export type LibraryToolbarScope = LibraryPanelSurface | 'neutral'

export type LibraryToolbarModel = {
  readonly scope: LibraryToolbarScope
  readonly search: ToolbarControl
  readonly viewMode: ToolbarControl
  readonly libraryBrowseProfile: ToolbarControl
  readonly addSourceView: ToolbarControl
  readonly addMusicFolder: ToolbarAction
}

export type ToolbarControl = {
  readonly visible: boolean
  readonly enabled: boolean
  readonly label: string
  readonly title: string
  readonly placeholder?: string
}

export type ToolbarAction = {
  readonly kind: 'openAddSource' | 'chooseMusicFolder'
  readonly visible: boolean
  readonly enabled: boolean
  readonly label: string
  readonly reason?: string
}

export type LibraryToolbarInput = {
  readonly activeSurface: LibraryPanelSurface
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId: BrowserTreeNodeId | undefined
  readonly selectedViewModeLabel: string
  readonly selectedLibraryBrowseProfileLabel: string
  readonly selectedAddSourceViewLabel: string
  readonly addSourceView: AddSourceView
  readonly addMusicFolderLabel: string
  readonly canAddMusicFolder: boolean
}

export function projectLibraryToolbar(input: LibraryToolbarInput): LibraryToolbarModel {
  const scope = toolbarScope(input.activeSurface, input.projection, input.selectedNodeId)
  const selectedBinding =
    input.selectedNodeId === undefined
      ? undefined
      : input.projection?.bindingsById.get(input.selectedNodeId)
  const selectedLocalAdmissionAvailable =
    scope === 'addSource' && selectedBinding !== undefined
      ? hasLocalSourceAdmission(selectedBinding)
      : false
  const searchScope = searchScopeDescription(selectedBinding)
  const showOpenAddSource = scope === 'libraryBrowse'
  const showFolderPicker =
    scope === 'addSource' && input.addSourceView !== 'inventory' && !selectedLocalAdmissionAvailable

  return {
    scope,
    search: {
      visible: scope === 'libraryBrowse',
      enabled: scope === 'libraryBrowse',
      label: 'Search indexed library',
      title: `Search indexed library ${searchScope.titleSuffix}`,
      placeholder: `Search ${searchScope.placeholder}`
    },
    viewMode: {
      visible: scope === 'libraryBrowse',
      enabled: scope === 'libraryBrowse',
      label: 'View',
      title: `View: ${input.selectedViewModeLabel}`
    },
    libraryBrowseProfile: {
      visible: scope === 'libraryBrowse',
      enabled: scope === 'libraryBrowse',
      label: 'Indexed contents view',
      title: `Indexed contents view: ${input.selectedLibraryBrowseProfileLabel}`
    },
    addSourceView: {
      visible: scope === 'addSource',
      enabled: scope === 'addSource',
      label: 'Add Source view',
      title: `Add Source view: ${input.selectedAddSourceViewLabel}`
    },
    addMusicFolder: {
      kind: showOpenAddSource ? 'openAddSource' : 'chooseMusicFolder',
      visible: showOpenAddSource || showFolderPicker,
      enabled: showOpenAddSource || input.canAddMusicFolder,
      label: showOpenAddSource ? 'Add Source' : input.addMusicFolderLabel,
      ...(showOpenAddSource || input.canAddMusicFolder
        ? {}
        : { reason: 'A source action is already running.' })
    }
  }
}

function searchScopeDescription(binding: RowBinding | undefined): {
  readonly placeholder: string
  readonly titleSuffix: string
} {
  switch (binding?.kind) {
    case 'source':
      return { placeholder: 'inside selected source', titleSuffix: 'inside selected source' }
    case 'directory':
      return { placeholder: 'inside selected folder', titleSuffix: 'inside selected folder' }
    default:
      return { placeholder: 'library-wide', titleSuffix: 'library-wide' }
  }
}

function toolbarScope(
  activeSurface: LibraryPanelSurface,
  projection: BrowserProjection | undefined,
  selectedNodeId: BrowserTreeNodeId | undefined
): LibraryToolbarScope {
  if (activeSurface === 'libraryBrowse') {
    return 'libraryBrowse'
  }

  if (activeSurface === 'addSource') {
    return 'addSource'
  }

  if (selectedNodeId === undefined) {
    return 'neutral'
  }

  const binding = projection?.bindingsById.get(selectedNodeId)

  if (binding === undefined) {
    return 'neutral'
  }

  if (isLocalAdmissionBinding(binding)) {
    return 'addSource'
  }

  if (isIndexedLibraryBinding(binding)) {
    return 'libraryBrowse'
  }

  return 'neutral'
}

function isLocalAdmissionBinding(binding: RowBinding): boolean {
  return (
    binding.kind === 'addSourceSection' ||
    binding.kind === 'localBrowseEntryPoint' ||
    binding.kind === 'localBrowseItem' ||
    binding.kind === 'localBrowseMore'
  )
}

function hasLocalSourceAdmission(binding: RowBinding): boolean {
  switch (binding.kind) {
    case 'localBrowseEntryPoint':
      return sourceAdmissionOperation(binding.entry.availableOperations) !== undefined
    case 'localBrowseItem':
      return sourceAdmissionOperation(binding.item.availableOperations) !== undefined
    default:
      return false
  }
}

function isIndexedLibraryBinding(binding: RowBinding): boolean {
  return (
    binding.kind === 'source' ||
    binding.kind === 'directory' ||
    binding.kind === 'file' ||
    binding.kind === 'more'
  )
}
