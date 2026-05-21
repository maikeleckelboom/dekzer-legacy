import type { BrowserTreeVisibleItem } from './types'

export type AriaBoolean = 'false' | 'true'

export function getTreeItemAriaExpanded(item: BrowserTreeVisibleItem): AriaBoolean | undefined {
  if (!item.canRevealChildren) {
    return undefined
  }

  return toAriaBoolean(item.isExpanded)
}

export function getTreeItemAriaSelected(item: BrowserTreeVisibleItem): AriaBoolean {
  return toAriaBoolean(item.isSelected)
}

function toAriaBoolean(value: boolean): AriaBoolean {
  return value ? 'true' : 'false'
}
