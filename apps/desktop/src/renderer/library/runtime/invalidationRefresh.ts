import type { AppMaintainedSnapshotInvalidatedEvent } from '../../../shared/libraryBoundary/eventParser'
import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { BrowserTreeNodeId } from '../tree/types'

export type InvalidationRefreshDependencies = {
  readonly hierarchyRead: Pick<
    LibraryHierarchyReadController,
    'refreshNavigationRows' | 'refreshLoadedBrowserWindows' | 'refreshBrowserWindowsForNodeIds'
  >
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly refreshContentsForCurrentSelection: () => void
}

export async function refreshHierarchyForMaintainedSnapshotInvalidation(
  event: AppMaintainedSnapshotInvalidatedEvent,
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  switch (event.invalidation.scope) {
    case 'navigationRows':
      return dependencies.hierarchyRead.refreshNavigationRows()
    case 'libraryBrowser': {
      const refreshedLoaded = await dependencies.hierarchyRead.refreshLoadedBrowserWindows()
      const refreshedExpanded = await dependencies.hierarchyRead.refreshBrowserWindowsForNodeIds(
        dependencies.expandedNodeIds
      )
      dependencies.refreshContentsForCurrentSelection()
      return refreshedLoaded && refreshedExpanded
    }
    default:
      return true
  }
}
