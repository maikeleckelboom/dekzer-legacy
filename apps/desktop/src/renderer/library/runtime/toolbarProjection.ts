import type { RowBinding } from '../state'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'

export type LibraryToolbarScope = 'libraryStart' | 'indexedLibrary' | 'localAdmission' | 'neutral'

export type LibraryToolbarModel = {
  readonly scope: LibraryToolbarScope
  readonly search: ToolbarControl
  readonly browseProfile: ToolbarControl
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
  readonly selectedBrowseProfileLabel: string
  readonly addMusicFolderLabel: string
  readonly canAddMusicFolder: boolean
}

export function projectLibraryToolbar(input: LibraryToolbarInput): LibraryToolbarModel {
  const scope = toolbarScope(input.projection, input.selectedNodeId)
  const browseProfileScopeLabel =
    scope === 'localAdmission' ? 'Local preview filter' : 'Indexed contents view'

  return {
    scope,
    search: {
      visible: scope === 'indexedLibrary',
      enabled: scope === 'indexedLibrary',
      label: 'Search indexed library',
      title: 'Search indexed library',
      placeholder: 'Search indexed library'
    },
    browseProfile: {
      visible: scope === 'indexedLibrary' || scope === 'localAdmission',
      enabled: scope === 'indexedLibrary' || scope === 'localAdmission',
      label: browseProfileScopeLabel,
      title: `${browseProfileScopeLabel}: ${input.selectedBrowseProfileLabel}`
    },
    addMusicFolder: {
      visible: scope === 'libraryStart' || scope === 'localAdmission',
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
    binding.kind === 'localBrowseSection' ||
    binding.kind === 'localBrowseEntryPoint' ||
    binding.kind === 'localBrowseItem' ||
    binding.kind === 'localBrowseMore'
  )
}

function isIndexedLibraryBinding(binding: RowBinding): boolean {
  return (
    binding.kind === 'source' ||
    binding.kind === 'directory' ||
    binding.kind === 'file' ||
    binding.kind === 'more'
  )
}
