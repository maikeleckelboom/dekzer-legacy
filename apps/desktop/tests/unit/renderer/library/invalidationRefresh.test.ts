import { describe, expect, it, vi } from 'vitest'

import {
  refreshHierarchyForMaintainedSnapshotInvalidation,
  type InvalidationRefreshDependencies
} from '../../../../src/renderer/library/runtime/invalidationRefresh'
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
    expect(dependencies.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
  })

  it('refreshes browser windows once with expanded ids for libraryBrowser invalidation', async () => {
    const dependencies = testDependencies()

    await expect(
      refreshHierarchyForMaintainedSnapshotInvalidation(
        invalidation('libraryBrowser', '2'),
        dependencies
      )
    ).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(
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
    expect(dependencies.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
  })
})

function testDependencies(): InvalidationRefreshDependencies {
  return {
    hierarchyRead: {
      refreshNavigationRows: vi.fn(async () => true),
      refreshBrowserWindows: vi.fn(async () => true)
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
