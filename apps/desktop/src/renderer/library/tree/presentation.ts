import type { IconComponent } from '../../icons/types'
import {
  FolderIcon,
  FolderOpenIcon,
  ListMusicIcon,
  LoadingIcon,
  MoreIcon,
  NavigationIcon,
  SourceIcon,
  StateIcon,
  WarningIcon
} from '../../icons/lucide'
import type { BrowserTreeNode } from './types'

export function resolveBrowserTreeRowIcon(
  node: BrowserTreeNode,
  isExpanded: boolean
): IconComponent | undefined {
  switch (node.role) {
    case 'collectionView':
      return NavigationIcon
    case 'source':
      return SourceIcon
    case 'sourceLocation':
      return isExpanded ? FolderOpenIcon : FolderIcon
    case 'literalDirectory':
      return isExpanded ? FolderOpenIcon : FolderIcon
    case 'preparationSurface':
      return StateIcon
    case 'playlistSurface':
      return ListMusicIcon
    case 'smartView':
      return NavigationIcon
    case 'state':
      return resolveStateIcon(node.icon)
    case 'action':
      return resolveActionIcon(node.icon)
  }
}

function resolveStateIcon(icon: BrowserTreeNode['icon']): IconComponent {
  switch (icon) {
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    default:
      return StateIcon
  }
}

function resolveActionIcon(icon: BrowserTreeNode['icon']): IconComponent {
  switch (icon) {
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    default:
      return MoreIcon
  }
}
