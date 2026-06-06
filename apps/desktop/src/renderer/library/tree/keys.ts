import {
  getFirstChildVisibleNodeId,
  getFirstVisibleNodeId,
  getLastVisibleNodeId,
  getNextVisibleNodeId,
  getParentVisibleNodeId,
  getPreviousVisibleNodeId
} from './listProjection'
import type { BrowserTreeNodeId, BrowserTreeVisibleItem } from './types'

export const treeKeyboardKeys = {
  arrowDown: 'ArrowDown',
  arrowLeft: 'ArrowLeft',
  arrowRight: 'ArrowRight',
  arrowUp: 'ArrowUp',
  end: 'End',
  enter: 'Enter',
  escape: 'Escape',
  home: 'Home',
  legacySpace: 'Spacebar',
  space: ' ',
  spaceKey: 'Space'
} as const

export type TreeKeyboardIntent =
  | {
      readonly kind: 'none'
      readonly shouldPreventDefault: boolean
    }
  | {
      readonly kind: 'focus'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }
  | {
      readonly kind: 'expand'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }
  | {
      readonly kind: 'collapse'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }
  | {
      readonly kind: 'revealNode'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }
  | {
      readonly kind: 'activateAction'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }
  | {
      readonly kind: 'select'
      readonly nodeId: BrowserTreeNodeId
      readonly shouldPreventDefault: true
    }

export type ResolveTreeKeyboardIntentOptions = {
  readonly key: string
  readonly activeNodeId?: BrowserTreeNodeId
  readonly visibleItems: readonly BrowserTreeVisibleItem[]
}

export function resolveTreeKeyboardIntent(
  options: ResolveTreeKeyboardIntentOptions
): TreeKeyboardIntent {
  const activeItem =
    options.activeNodeId === undefined
      ? undefined
      : options.visibleItems.find((item) => item.id === options.activeNodeId)

  switch (options.key) {
    case treeKeyboardKeys.arrowUp:
      return resolveFocusIntent(
        activeItem === undefined
          ? getFirstVisibleNodeId(options.visibleItems)
          : getPreviousVisibleNodeId(options.visibleItems, activeItem.id)
      )

    case treeKeyboardKeys.arrowDown:
      return resolveFocusIntent(
        activeItem === undefined
          ? getFirstVisibleNodeId(options.visibleItems)
          : getNextVisibleNodeId(options.visibleItems, activeItem.id)
      )

    case treeKeyboardKeys.home:
      return resolveFocusIntent(getFirstVisibleNodeId(options.visibleItems))

    case treeKeyboardKeys.end:
      return resolveFocusIntent(getLastVisibleNodeId(options.visibleItems))

    case treeKeyboardKeys.arrowRight:
      if (activeItem === undefined) {
        return handledNoop()
      }

      if (!activeItem.canRevealChildren) {
        return handledNoop()
      }

      if (!activeItem.isExpanded || activeItem.canActivateAction || activeItem.isActionLoading) {
        return {
          kind: 'revealNode',
          nodeId: activeItem.id,
          shouldPreventDefault: true
        }
      }

      return resolveFocusIntent(getFirstChildVisibleNodeId(options.visibleItems, activeItem.id))

    case treeKeyboardKeys.arrowLeft:
      if (activeItem === undefined) {
        return handledNoop()
      }

      if (activeItem.isActionItem) {
        return resolveFocusIntent(getParentVisibleNodeId(options.visibleItems, activeItem.id))
      }

      if (activeItem.canRevealChildren && activeItem.isExpanded) {
        return {
          kind: 'collapse',
          nodeId: activeItem.id,
          shouldPreventDefault: true
        }
      }

      if (activeItem.canActivateAction || activeItem.isActionLoading) {
        return handledNoop()
      }

      return resolveFocusIntent(getParentVisibleNodeId(options.visibleItems, activeItem.id))

    case treeKeyboardKeys.enter:
      if (activeItem === undefined) {
        return handledNoop()
      }

      if (activeItem.isActionItem) {
        return {
          kind: 'activateAction',
          nodeId: activeItem.id,
          shouldPreventDefault: true
        }
      }

      return {
        kind: 'select',
        nodeId: activeItem.id,
        shouldPreventDefault: true
      }

    case treeKeyboardKeys.space:
    case treeKeyboardKeys.spaceKey:
    case treeKeyboardKeys.legacySpace:
      return unhandled()

    case treeKeyboardKeys.escape:
      return unhandled()

    default:
      return unhandled()
  }
}

function resolveFocusIntent(nodeId: BrowserTreeNodeId | undefined): TreeKeyboardIntent {
  if (nodeId === undefined) {
    return handledNoop()
  }

  return {
    kind: 'focus',
    nodeId,
    shouldPreventDefault: true
  }
}

function handledNoop(): TreeKeyboardIntent {
  return {
    kind: 'none',
    shouldPreventDefault: true
  }
}

function unhandled(): TreeKeyboardIntent {
  return {
    kind: 'none',
    shouldPreventDefault: false
  }
}
