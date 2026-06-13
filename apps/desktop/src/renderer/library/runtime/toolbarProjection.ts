import type { RowBinding } from '../state'
import { sourceAdmissionOperation } from '../localBrowse/projection'
import type { LocalPreviewMode } from '../localBrowse/previewMode'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'

export type LibraryToolbarScope = 'libraryStart' | 'indexedLibrary' | 'localAdmission' | 'neutral'

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
  readonly visible: boolean
  readonly enabled: boolean
  readonly label: string
  readonly reason?: string
}

export type LibraryToolbarInput = {
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId: BrowserTreeNodeId | undefined
  readonly selectedLibraryBrowseProfileLabel: string
  readonly selectedLocalPreviewModeLabel: string
  readonly localPreviewMode: LocalPreviewMode
  readonly addMusicFolderLabel: string
  readonly canAddMusicFolder: boolean
}

export function projectLibraryToolbar(input: LibraryToolbarInput): LibraryToolbarModel {
  const scope = toolbarScope(input.projection, input.selectedNodeId)
  const selectedBinding =
    input.selectedNodeId === undefined
      ? undefined
      : input.projection?.bindingsById.get(input.selectedNodeId)
  const selectedLocalAdmissionAvailable =
    selectedBinding === undefined ? false : hasLocalSourceAdmission(selectedBinding)
  const showFolderPicker =
    scope === 'libraryStart' ||
    (scope === 'localAdmission' &&
      input.localPreviewMode !== 'advancedInventory' &&
      !selectedLocalAdmissionAvailable)

  return {
    scope,
    search: {
      visible: scope === 'indexedLibrary',
      enabled: scope === 'indexedLibrary',
      label: 'Search indexed library',
      title: 'Search indexed library',
      placeholder: 'Search indexed library'
    },
    libraryBrowseProfile: {
      visible: scope === 'indexedLibrary',
      enabled: scope === 'indexedLibrary',
      label: 'Indexed contents view',
      title: `Indexed contents view: ${input.selectedLibraryBrowseProfileLabel}`
    },
    localPreviewMode: {
      visible: scope === 'localAdmission',
      enabled: scope === 'localAdmission',
      label: 'Local preview mode',
      title: `Local preview mode: ${input.selectedLocalPreviewModeLabel}`
    },
    addMusicFolder: {
      visible: showFolderPicker,
      enabled: input.canAddMusicFolder,
      label: input.addMusicFolderLabel,
      ...(input.canAddMusicFolder ? {} : { reason: 'A source action is already running.' })
    }
  }
}

function toolbarScope(
  projection: BrowserProjection | undefined,
  selectedNodeId: BrowserTreeNodeId | undefined
): LibraryToolbarScope {
  if (selectedNodeId === undefined) {
    return 'libraryStart'
  }

  const binding = projection?.bindingsById.get(selectedNodeId)

  if (binding === undefined) {
    return 'neutral'
  }

  if (isLocalAdmissionBinding(binding)) {
    return 'localAdmission'
  }

  if (isIndexedLibraryBinding(binding)) {
    return 'indexedLibrary'
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
