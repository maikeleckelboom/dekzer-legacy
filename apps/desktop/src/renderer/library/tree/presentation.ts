import type { IconRole } from '../../icons/types'
import type { BrowserTreeNode } from './types'

export function resolveBrowserTreeRowIcon(node: BrowserTreeNode): IconRole | undefined {
  switch (node.role) {
    case 'collectionView':
      return 'navigation.collection'
    case 'source':
      return 'source.local'
    case 'sourceLocation':
    case 'literalDirectory':
      return 'folder.plain'
    case 'preparationSurface':
      return 'state.unknown'
    case 'playlistSurface':
      return 'media.playlist'
    case 'smartView':
      return 'navigation.view'
    case 'state':
      return resolveBrowserTreeStateIcon(node.icon)
    case 'action':
      return resolveActionIcon(node.icon)
  }
}

export function resolveBrowserTreeStateIcon(icon: BrowserTreeNode['icon']): IconRole {
  switch (icon) {
    case 'loading':
      return 'state.loading'
    case 'warning':
      return 'state.warning'
    default:
      return 'state.unknown'
  }
}

function resolveActionIcon(icon: BrowserTreeNode['icon']): IconRole {
  switch (icon) {
    case 'loading':
      return 'state.loading'
    case 'warning':
      return 'state.warning'
    default:
      return 'action.more'
  }
}
