import { ref } from 'vue'
import type { Ref } from 'vue'

import type { LocalBrowseItemFilter } from '../../../shared/library/localBrowse/items'

export type AddSourceView = 'preview' | 'inventory'

export type AddSourceViewOption = {
  readonly key: AddSourceView
  readonly label: string
}

export type AddSourceViewController = {
  readonly view: Ref<AddSourceView>
  readonly setView: (view: AddSourceView) => boolean
  readonly restoreView: (view: unknown) => boolean
}

export const defaultAddSourceView: AddSourceView = 'preview'

export const addSourceViewOptions: readonly AddSourceViewOption[] = [
  { key: 'preview', label: 'Preview' },
  { key: 'inventory', label: 'Inventory' }
]

export function createAddSourceViewController(
  initialView: AddSourceView = defaultAddSourceView
): AddSourceViewController {
  const view = ref<AddSourceView>(initialView)

  function setView(nextView: AddSourceView): boolean {
    if (view.value === nextView) {
      return false
    }

    view.value = nextView
    return true
  }

  function restoreView(value: unknown): boolean {
    return isAddSourceView(value) ? setView(value) : false
  }

  return {
    view,
    setView,
    restoreView
  }
}

export function isAddSourceView(value: unknown): value is AddSourceView {
  return value === 'preview' || value === 'inventory'
}

export function addSourceViewLabel(view: AddSourceView): string {
  return addSourceViewOptions.find((option) => option.key === view)?.label ?? 'Preview'
}

export function addSourceSurfaceLabel(view: AddSourceView): 'Preview' | 'Inventory' {
  return view === 'inventory' ? 'Inventory' : 'Preview'
}

export function mapAddSourceViewToLocalBrowseItemFilter(
  view: AddSourceView
): LocalBrowseItemFilter {
  return view === 'inventory' ? 'allFiles' : 'audio'
}
