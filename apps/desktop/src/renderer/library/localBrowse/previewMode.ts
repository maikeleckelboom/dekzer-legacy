import { ref } from 'vue'
import type { Ref } from 'vue'

import type { LocalBrowseItemFilter } from '../../../shared/library/localBrowse/items'

export type LocalPreviewMode = 'musicEvidence' | 'advancedInventory'

export type LocalPreviewModeOption = {
  readonly key: LocalPreviewMode
  readonly label: string
}

export type LocalPreviewModeController = {
  readonly mode: Ref<LocalPreviewMode>
  readonly setMode: (mode: LocalPreviewMode) => boolean
  readonly restoreMode: (mode: unknown) => boolean
}

export const defaultLocalPreviewMode: LocalPreviewMode = 'musicEvidence'

export const localPreviewModeOptions: readonly LocalPreviewModeOption[] = [
  { key: 'musicEvidence', label: 'Music Preview' },
  { key: 'advancedInventory', label: 'Inventory' }
]

export function createLocalPreviewModeController(
  initialMode: LocalPreviewMode = defaultLocalPreviewMode
): LocalPreviewModeController {
  const mode = ref<LocalPreviewMode>(initialMode)

  function setMode(nextMode: LocalPreviewMode): boolean {
    if (mode.value === nextMode) {
      return false
    }

    mode.value = nextMode
    return true
  }

  function restoreMode(value: unknown): boolean {
    return isLocalPreviewMode(value) ? setMode(value) : false
  }

  return {
    mode,
    setMode,
    restoreMode
  }
}

export function isLocalPreviewMode(value: unknown): value is LocalPreviewMode {
  return value === 'musicEvidence' || value === 'advancedInventory'
}

export function localPreviewModeLabel(mode: LocalPreviewMode): string {
  return localPreviewModeOptions.find((option) => option.key === mode)?.label ?? 'Music Preview'
}

export function localPreviewSurfaceLabel(
  mode: LocalPreviewMode
): 'Source Preview' | 'Source Inventory' {
  return mode === 'advancedInventory' ? 'Source Inventory' : 'Source Preview'
}

export function mapLocalPreviewModeToLocalBrowseItemFilter(
  mode: LocalPreviewMode
): LocalBrowseItemFilter {
  return mode === 'advancedInventory' ? 'allFiles' : 'audio'
}
