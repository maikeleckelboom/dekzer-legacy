import type { NavigationRow } from '../../../shared/libraryNavigation/readRows'
import type { BrowserTreeRowRole } from '../tree/types'

export function browserRowRoleForNavigationRow(row: NavigationRow): BrowserTreeRowRole {
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
      return 'source'
    case 'locationGroup':
      return 'locationGroup'
    case 'location':
      return 'sourceLocation'
  }
}
