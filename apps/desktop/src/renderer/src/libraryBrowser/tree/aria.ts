import type { TreeVisibleItem } from './types'

export type AriaBoolean = 'false' | 'true'

export function getTreeItemAriaExpanded(item: TreeVisibleItem): AriaBoolean | undefined {
  if (!item.hasChildren) {
    return undefined
  }

  return toAriaBoolean(item.isExpanded)
}

export function getTreeItemAriaSelected(item: TreeVisibleItem): AriaBoolean {
  return toAriaBoolean(item.isSelected)
}

function toAriaBoolean(value: boolean): AriaBoolean {
  return value ? 'true' : 'false'
}
