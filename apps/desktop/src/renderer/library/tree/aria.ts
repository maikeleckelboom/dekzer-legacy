import type { BrowserTreeVisibleItem } from './types'

export type AriaBoolean = 'false' | 'true'

export function getTreeItemAriaExpanded(item: BrowserTreeVisibleItem): AriaBoolean | undefined {
  if (!item.isBranch) {
    return undefined
  }

  if (item.canRevealChildren) {
    return toAriaBoolean(item.isExpanded)
  }

  if (item.canActivateAction || item.isActionLoading) {
    return 'false'
  }

  return undefined
}

export function getTreeItemAriaSelected(item: BrowserTreeVisibleItem): AriaBoolean {
  return toAriaBoolean(item.isSelected)
}

function toAriaBoolean(value: boolean): AriaBoolean {
  return value ? 'true' : 'false'
}
