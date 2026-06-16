import { ref } from 'vue'
import type { Ref } from 'vue'

export type ViewMode = 'list' | 'columns'

export type ViewModeOption = {
  readonly key: ViewMode
  readonly label: string
}

export type ViewModeController = {
  readonly view: Ref<ViewMode>
  readonly setView: (view: ViewMode) => boolean
  readonly restoreView: (view: unknown) => boolean
}

export const defaultViewMode: ViewMode = 'list'

export const viewModeOptions: readonly ViewModeOption[] = [
  { key: 'list', label: 'List' },
  { key: 'columns', label: 'Columns' }
]

export function createViewModeController(
  initialView: ViewMode = defaultViewMode
): ViewModeController {
  const view = ref<ViewMode>(initialView)

  function setView(nextView: ViewMode): boolean {
    if (view.value === nextView) {
      return false
    }

    view.value = nextView
    return true
  }

  function restoreView(value: unknown): boolean {
    return setView(parseViewMode(value))
  }

  return {
    view,
    setView,
    restoreView
  }
}

export function isViewMode(value: unknown): value is ViewMode {
  return value === 'list' || value === 'columns'
}

export function parseViewMode(value: unknown): ViewMode {
  return isViewMode(value) ? value : defaultViewMode
}

export function viewModeLabel(view: ViewMode): string {
  return viewModeOptions.find((option) => option.key === view)?.label ?? 'List'
}
