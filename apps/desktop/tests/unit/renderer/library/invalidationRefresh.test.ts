import { describe, expect, it, vi } from 'vitest'

import { refreshHierarchyForMaintainedSnapshotInvalidation } from '../../../../src/renderer/library/runtime/invalidationRefresh'
import type { AppMaintainedSnapshotInvalidatedEvent } from '../../../../src/shared/libraryBoundary/eventParser'

describe('refreshHierarchyForMaintainedSnapshotInvalidation', () => {
  it('refreshes navigation rows for navigationRows invalidation', async () => {
    const dependencies = testDependencies()

    await expect(
      refreshHierarchyForMaintainedSnapshotInvalidation(
        invalidation('navigationRows', '1'),
        dependencies
      )
    ).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshLoadedBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshBrowserWindowsForNodeIds).not.toHaveBeenCalled()
    expect(dependencies.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
  })

  it('refreshes loaded and expanded browser windows for libraryBrowser invalidation', async () => {
    const dependencies = testDependencies()

    await expect(
      refreshHierarchyForMaintainedSnapshotInvalidation(
        invalidation('libraryBrowser', '2'),
        dependencies
      )
    ).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshLoadedBrowserWindows).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshBrowserWindowsForNodeIds).toHaveBeenCalledWith(
      new Set(['navigation-row:7', 'source-directory:12'])
    )
    expect(dependencies.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('does not alias unsupported future scopes', async () => {
    const dependencies = testDependencies()

    await expect(
      refreshHierarchyForMaintainedSnapshotInvalidation(
        invalidation('LibraryBrowser', '3'),
        dependencies
      )
    ).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshLoadedBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshBrowserWindowsForNodeIds).not.toHaveBeenCalled()
    expect(dependencies.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
  })
})

function testDependencies() {
  return {
    hierarchyRead: {
      refreshNavigationRows: vi.fn(async () => true),
      refreshLoadedBrowserWindows: vi.fn(async () => true),
      refreshBrowserWindowsForNodeIds: vi.fn(async () => true)
    },
    expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12']),
    refreshContentsForCurrentSelection: vi.fn()
  }
}

function invalidation(scope: string, revision: string): AppMaintainedSnapshotInvalidatedEvent {
  return {
    eventSequence: Number(revision),
    occurredAtMs: 1000 + Number(revision),
    invalidation: {
      scope,
      revision
    }
  }
}
