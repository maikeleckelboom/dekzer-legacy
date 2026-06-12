import type { NavigationRow } from '../../../shared/library/navigation/read'
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
      return 'collectionView'
    case 'source':
    case 'location':
      return 'source'
    default:
      return 'collectionView'
  }
}
