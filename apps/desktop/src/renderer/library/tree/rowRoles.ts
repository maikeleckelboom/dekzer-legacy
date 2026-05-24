import type { NavigationRow } from '../../../shared/libraryNavigation/readRows'
import type { BrowserTreeRowRole } from './types'

export function browserRowRoleForNavigationRow(row: NavigationRow): BrowserTreeRowRole {
  if (row.selectorKind === 'source') {
    return 'source'
  }

  if (row.selectorKind === 'sourceLocation') {
    return 'sourceLocation'
  }

  switch (row.rowKind) {
    case 'view':
    case 'collectionGroup':
      return 'collectionView'
    case 'playlist':
      return 'playlistSurface'
    case 'prepPolicyGroup':
    case 'prepPolicyScope':
      return 'preparationSurface'
    case 'source':
    case 'location':
      return 'source'
    default:
      return 'collectionView'
  }
}
