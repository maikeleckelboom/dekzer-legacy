import type { AppMaintainedSnapshotInvalidatedEvent } from '../../../shared/libraryBoundary/eventParser'
import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { SourceLifecycleReadController } from '../boundary/sourceLifecycleRead'
import type { BrowserTreeNodeId } from '../tree/types'

export type InvalidationRefreshDependencies = {
  readonly hierarchyRead: Pick<
    LibraryHierarchyReadController,
    'refreshNavigationRows' | 'refreshBrowserWindows'
  >
  readonly sourceLifecycleRead?: Pick<SourceLifecycleReadController, 'refreshSourceLifecycles'>
  readonly sourceLifecycleSourceIds?: ReadonlySet<string>
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly refreshContentsForCurrentSelection: () => void
}

export async function refreshHierarchyForMaintainedSnapshotInvalidation(
  event: AppMaintainedSnapshotInvalidatedEvent,
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  switch (event.invalidation.scope) {
    case 'navigationRows':
      return refreshNavigationRowsAndKnownSourceLifecycles(dependencies)
    case 'libraryBrowser': {
      const refreshedBrowser = await dependencies.hierarchyRead.refreshBrowserWindows(
        dependencies.expandedNodeIds
      )
      const refreshedLifecycle = await refreshKnownSourceLifecycles(dependencies)
      dependencies.refreshContentsForCurrentSelection()
      return refreshedBrowser && refreshedLifecycle
    }
    default:
      return true
  }
}

async function refreshNavigationRowsAndKnownSourceLifecycles(
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  const refreshedNavigation = await dependencies.hierarchyRead.refreshNavigationRows()
  const refreshedLifecycle = await refreshKnownSourceLifecycles(dependencies)
  return refreshedNavigation && refreshedLifecycle
}

async function refreshKnownSourceLifecycles(
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  if (
    dependencies.sourceLifecycleRead === undefined ||
    dependencies.sourceLifecycleSourceIds === undefined
  ) {
    return true
  }

  return dependencies.sourceLifecycleRead.refreshSourceLifecycles(
    dependencies.sourceLifecycleSourceIds
  )
}
