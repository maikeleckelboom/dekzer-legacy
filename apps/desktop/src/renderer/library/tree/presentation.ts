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
    case 'localBrowseRoot':
    case 'localBrowseDirectory':
      return 'folder.plain'
    case 'localBrowseFile':
      return resolveFileIcon(node.icon)
    case 'smartView':
      return 'navigation.view'
    case 'state':
      return resolveBrowserTreeStateIcon(node.icon)
    case 'action':
      return resolveActionIcon(node.icon)
  }
}

function resolveFileIcon(icon: BrowserTreeNode['icon']): IconRole {
  switch (icon) {
    case 'music':
      return 'media.audio'
    case 'video':
      return 'media.video'
    case 'image':
      return 'media.image'
    case 'cueSheet':
      return 'media.cueSheet'
    case 'warning':
      return 'state.warning'
    case 'loading':
      return 'state.loading'
    case 'metadata':
    case 'file':
    case 'state':
    default:
      return 'media.metadata'
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
