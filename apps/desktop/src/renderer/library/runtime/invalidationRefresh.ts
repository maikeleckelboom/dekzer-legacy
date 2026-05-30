import type { AppMaintainedSnapshotInvalidatedEvent } from '../../../shared/libraryBoundary/eventParser'
import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { BrowserTreeNodeId } from '../tree/types'

export type InvalidationRefreshDependencies = {
  readonly hierarchyRead: Pick<
    LibraryHierarchyReadController,
    'refreshNavigationRows' | 'refreshBrowserWindows'
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
      const refreshedBrowser = await dependencies.hierarchyRead.refreshBrowserWindows(
        dependencies.expandedNodeIds
      )
      dependencies.refreshContentsForCurrentSelection()
      return refreshedBrowser
    }
    default:
      return true
  }
}
