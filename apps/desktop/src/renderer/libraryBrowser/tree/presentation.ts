import type { IconComponent } from '../../icons'
import {
  FileIcon,
  FileTextIcon,
  FolderIcon,
  FolderOpenIcon,
  ImageIcon,
  ListMusicIcon,
  LoadingIcon,
  MoreIcon,
  MusicIcon,
  NavigationIcon,
  SourceIcon,
  StateIcon,
  VideoIcon,
  WarningIcon
} from '../../icons'
import type { BrowserTreeNode } from './types'

export function resolveBrowserTreeRowIcon(
  node: BrowserTreeNode,
  isExpanded: boolean
): IconComponent | undefined {
  switch (node.role) {
    case 'collectionView':
      return NavigationIcon
    case 'locationGroup':
      return isExpanded ? FolderOpenIcon : FolderIcon
    case 'source':
      return SourceIcon
    case 'sourceLocation':
      return isExpanded ? FolderOpenIcon : FolderIcon
    case 'literalDirectory':
      return isExpanded ? FolderOpenIcon : FolderIcon
    case 'literalFile':
      return resolveLiteralFileIcon(node.icon)
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

function resolveLiteralFileIcon(
  icon: BrowserTreeNode['icon']
): IconComponent {
  switch (icon) {
    case 'music':
      return MusicIcon
    case 'video':
      return VideoIcon
    case 'image':
      return ImageIcon
    case 'cueSheet':
      return FileTextIcon
    case 'playlist':
      return ListMusicIcon
    case 'metadata':
      return FileTextIcon
    case 'file':
    default:
      return FileIcon
  }
}

function resolveStateIcon(
  icon: BrowserTreeNode['icon']
): IconComponent {
  switch (icon) {
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    default:
      return StateIcon
  }
}

function resolveActionIcon(
  icon: BrowserTreeNode['icon']
): IconComponent {
  switch (icon) {
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    default:
      return MoreIcon
  }
}
