import type { RowBinding } from '../state'
import { sourceAdmissionOperation } from '../localBrowse/projection'
import type { LocalPreviewMode } from '../localBrowse/previewMode'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { LibraryPanelSurface } from '../../../shared/library/viewState/persistence'

export type LibraryToolbarScope = LibraryPanelSurface | 'neutral'

export type LibraryToolbarModel = {
  readonly scope: LibraryToolbarScope
  readonly search: ToolbarControl
  readonly libraryBrowseProfile: ToolbarControl
  readonly localPreviewMode: ToolbarControl
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
  readonly selectedLibraryBrowseProfileLabel: string
  readonly selectedLocalPreviewModeLabel: string
  readonly localPreviewMode: LocalPreviewMode
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
  const showOpenAddSource = scope === 'libraryBrowse'
  const showFolderPicker =
    scope === 'addSource' &&
    input.localPreviewMode !== 'advancedInventory' &&
    !selectedLocalAdmissionAvailable

  return {
    scope,
    search: {
      visible: scope === 'libraryBrowse',
      enabled: scope === 'libraryBrowse',
      label: 'Search indexed library',
      title: 'Search indexed library',
      placeholder: 'Search indexed library'
    },
    libraryBrowseProfile: {
      visible: scope === 'libraryBrowse',
      enabled: scope === 'libraryBrowse',
      label: 'Indexed contents view',
      title: `Indexed contents view: ${input.selectedLibraryBrowseProfileLabel}`
    },
    localPreviewMode: {
      visible: scope === 'addSource',
      enabled: scope === 'addSource',
      label: 'Local preview mode',
      title: `Local preview mode: ${input.selectedLocalPreviewModeLabel}`
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
