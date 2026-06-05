import { describe, expect, it, vi } from 'vitest'

import {
  buildGapRecoveryRefreshPlan,
  buildMaintainedSnapshotInvalidationRefreshPlan,
  buildSourceScanRefreshPlan,
  classifyInvalidationScope,
  executeLibraryRefreshPlan,
  refreshHierarchyForMaintainedSnapshotInvalidation,
  type InvalidationRefreshDependencies
} from '../../../../src/renderer/library/runtime/invalidationRefresh'
import {
  createBoundaryEventsController,
  type BoundaryEventApi
} from '../../../../src/renderer/library/boundary/boundaryEvents'
import {
  createLibraryHierarchyReadController,
  type LibraryHierarchyReadApi
} from '../../../../src/renderer/library/boundary/hierarchyRead'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type {
  AppMaintainedSnapshotInvalidatedEvent,
  AppSourceScanEvent
} from '../../../../src/shared/libraryBoundary/eventParser'
import type { BoundaryEventDeliveryPayload } from '../../../../src/shared/libraryBoundary/events'
import type {
  ChildRow,
  HierarchyCoverage,
  ReadRequest,
  ReadResult
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type { NavigationReadRowsResult } from '../../../../src/shared/libraryNavigation/readRows'

describe('library invalidation refresh planning', () => {
  it('classifies current renderer invalidation scopes without inventing future scopes', () => {
    expect(classifyInvalidationScope('navigationRows')).toBe('navigationRows')
    expect(classifyInvalidationScope('libraryBrowser')).toBe('libraryBrowser')
    expect(classifyInvalidationScope('LibraryBrowser')).toBe('unknown')
    expect(classifyInvalidationScope('contents')).toBe('unknown')
    expect(classifyInvalidationScope('sourceLifecycle')).toBe('unknown')
  })

  it('builds deterministic coalesced plans for maintained invalidation batches', () => {
    const input = {
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('libraryBrowser', '2'),
        invalidation('libraryBrowser', '3'),
        invalidation('unsupportedScope', '4')
      ],
      sourceLifecycleSourceIds: ['7', '7', '9']
    }

    const first = buildMaintainedSnapshotInvalidationRefreshPlan(input)
    const second = buildMaintainedSnapshotInvalidationRefreshPlan(input)

    expect(planSnapshot(first)).toEqual(planSnapshot(second))
    expect(planSnapshot(first)).toEqual({
      refreshRootHierarchy: false,
      refreshNavigationRows: true,
      refreshExpandedBrowserWindows: true,
      refreshCurrentContents: true,
      clearAllContentsWarmSnapshots: true,
      refreshSourceLifecycleIds: ['7', '9'],
      acknowledgeGapAfterExecution: false,
      broadRecovery: false
    })
  })

  it('keeps unknown maintained invalidation scopes as safe no-ops', () => {
    const plan = buildMaintainedSnapshotInvalidationRefreshPlan({
      invalidations: [invalidation('futureScope', '1')],
      sourceLifecycleSourceIds: ['7']
    })

    expect(planSnapshot(plan)).toEqual({
      refreshRootHierarchy: false,
      refreshNavigationRows: false,
      refreshExpandedBrowserWindows: false,
      refreshCurrentContents: false,
      clearAllContentsWarmSnapshots: false,
      refreshSourceLifecycleIds: [],
      acknowledgeGapAfterExecution: false,
      broadRecovery: false
    })
  })

  it('dedupes source scan lifecycle refresh ids for visible source ids only', () => {
    const plan = buildSourceScanRefreshPlan({
      events: [
        sourceScanEvent('7', 1),
        sourceScanEvent('7', 2),
        sourceScanEvent('8', 3),
        sourceScanEvent('9', 4)
      ],
      sourceLifecycleSourceIds: new Set(['7', '9'])
    })

    expect(planSnapshot(plan)).toMatchObject({
      refreshSourceLifecycleIds: ['7', '9'],
      refreshNavigationRows: false,
      refreshExpandedBrowserWindows: false,
      refreshCurrentContents: false
    })
  })

  it('represents gap recovery as a broad plan with post-execution acknowledgement', () => {
    expect(planSnapshot(buildGapRecoveryRefreshPlan())).toEqual({
      refreshRootHierarchy: true,
      refreshNavigationRows: false,
      refreshExpandedBrowserWindows: false,
      refreshCurrentContents: false,
      clearAllContentsWarmSnapshots: true,
      refreshSourceLifecycleIds: [],
      acknowledgeGapAfterExecution: true,
      broadRecovery: true
    })
  })
})

describe('executeLibraryRefreshPlan', () => {
  it('executes each refresh ownership at most once for a coalesced maintained batch', async () => {
    const dependencies = testDependencies({
      sourceLifecycleSourceIds: new Set(['7', '7', '9']),
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true)
    })
    const plan = buildMaintainedSnapshotInvalidationRefreshPlan({
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('navigationRows', '2'),
        invalidation('libraryBrowser', '3'),
        invalidation('libraryBrowser', '4')
      ],
      sourceLifecycleSourceIds: dependencies.sourceLifecycleSourceIds
    })

    await expect(executeLibraryRefreshPlan(plan, dependencies)).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(
      new Set(['navigation-row:7', 'source-directory:12'])
    )
    expect(dependencies.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(dependencies.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(
      new Set(['7', '9'])
    )
    expect(dependencies.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(dependencies.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('continues independent refreshes when one refresh fails', async () => {
    const dependencies = testDependencies({
      sourceLifecycleSourceIds: new Set(['7']),
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true)
    })
    vi.mocked(dependencies.hierarchyRead.refreshNavigationRows).mockRejectedValueOnce(
      new Error('navigation failed')
    )
    const plan = buildMaintainedSnapshotInvalidationRefreshPlan({
      invalidations: [invalidation('navigationRows', '1'), invalidation('libraryBrowser', '2')],
      sourceLifecycleSourceIds: dependencies.sourceLifecycleSourceIds
    })

    await expect(executeLibraryRefreshPlan(plan, dependencies)).resolves.toBe(false)

    expect(dependencies.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(dependencies.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(dependencies.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(dependencies.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('executes gap recovery through the same plan and keeps acknowledgement policy visible', async () => {
    const dependencies = testDependencies({
      refresh: vi.fn(async () => false),
      clearContentsWarmSnapshots: vi.fn()
    })
    const plan = buildGapRecoveryRefreshPlan()

    await expect(executeLibraryRefreshPlan(plan, dependencies)).resolves.toBe(false)

    expect(dependencies.hierarchyRead.refresh).toHaveBeenCalledTimes(1)
    expect(dependencies.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(plan.acknowledgeGapAfterExecution).toBe(true)
  })

  it('executes source scan plans without refreshing hierarchy or contents surfaces', async () => {
    const dependencies = testDependencies({
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) }
    })
    const plan = buildSourceScanRefreshPlan({
      events: [sourceScanEvent('7', 1), sourceScanEvent('8', 2)],
      sourceLifecycleSourceIds: new Set(['7'])
    })

    await expect(executeLibraryRefreshPlan(plan, dependencies)).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(dependencies.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(dependencies.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
    expect(dependencies.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(
      new Set(['7'])
    )
  })
})

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

  it('keeps navigation rows visible while navigationRows invalidation refresh is pending', async () => {
    const navigationRefresh = deferred<NavigationReadRowsResult>()
    let navigationReadCount = 0
    const hierarchyRead = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => {
          navigationReadCount += 1
          return navigationReadCount === 1
            ? navigationSourceReadRowsResult()
            : navigationRefresh.promise
        },
        readChildren: async () => initialSourceReadResult()
      })
    )

    await expect(hierarchyRead.refresh()).resolves.toBe(true)
    expect(topNodeIds(hierarchyRead)).toEqual(['navigation-row:7'])

    const refresh = refreshHierarchyForMaintainedSnapshotInvalidation(
      invalidation('navigationRows', '5'),
      {
        hierarchyRead,
        expandedNodeIds: new Set(),
        refreshContentsForCurrentSelection: vi.fn()
      }
    )
    await waitForMicrotasks()

    expect(topNodeIds(hierarchyRead)).toEqual(['navigation-row:7'])
    expect(loadedChildIds(hierarchyRead, 'navigation-row:7')).toEqual(['source-directory:12'])

    navigationRefresh.resolve(navigationSourceReadRowsResult())
    await expect(refresh).resolves.toBe(true)
    expect(topNodeIds(hierarchyRead)).toEqual(['navigation-row:7'])
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

  it('refreshes known source lifecycles separately from hierarchy branch refresh', async () => {
    const sourceLifecycleRead = {
      refreshSourceLifecycles: vi.fn(async () => true)
    }
    const dependencies: InvalidationRefreshDependencies = {
      ...testDependencies(),
      sourceLifecycleRead,
      sourceLifecycleSourceIds: new Set(['7'])
    }

    await expect(
      refreshHierarchyForMaintainedSnapshotInvalidation(
        invalidation('libraryBrowser', '4'),
        dependencies
      )
    ).resolves.toBe(true)

    expect(dependencies.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(
      new Set(['navigation-row:7', 'source-directory:12'])
    )
    expect(sourceLifecycleRead.refreshSourceLifecycles).toHaveBeenCalledWith(new Set(['7']))
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

  it('uses delivered maintained invalidations to refresh the loaded tree without reload', async () => {
    const events = testEventApi()
    const boundaryEvents = createBoundaryEventsController(events)
    const refreshRead = deferred<Extract<ReadResult, { state: 'ready' }>>()
    const refreshContentsForCurrentSelection = vi.fn()
    const readRequests: ReadRequest[] = []
    let readCount = 0
    const hierarchyRead = createLibraryHierarchyReadController(
      testLibraryApi({
        readRows: async () => navigationSourceReadRowsResult(),
        readChildren: async (request) => {
          readRequests.push(structuredClone(request))
          readCount += 1

          if (readCount === 1) {
            return initialSourceReadResult()
          }

          return refreshRead.promise
        }
      })
    )

    await expect(hierarchyRead.refresh()).resolves.toBe(true)
    expect(loadedChildIds(hierarchyRead, 'navigation-row:7')).toEqual(['source-directory:12'])
    readRequests.length = 0

    boundaryEvents.start()
    events.deliver({
      kind: 'batch',
      events: [
        {
          type: 'sourceScanEvent',
          payload: {
            eventSequence: 1,
            occurredAtMs: 1000,
            kind: 'sourceScanCompleted',
            rootId: 'root-1',
            scanRunId: 'scan-1',
            phase: 'scanning',
            directoriesVisited: 1,
            filesVisited: 2,
            filesDiscovered: 3,
            mediaCandidates: 4,
            queuedWorkItems: 5,
            detail: null
          }
        }
      ],
      latestEventSequence: 1,
      earliestRetainedSequence: 1,
      gapDetected: false
    })

    expect(boundaryEvents.scanProgress.value.get('root-1')).toMatchObject({
      kind: 'completed',
      filesDiscovered: 3,
      queuedWorkItems: 5
    })
    expect(boundaryEvents.consumeSourceScanEvents().map((event) => event.rootId)).toEqual([
      'root-1'
    ])
    expect(boundaryEvents.consumeMaintainedSnapshotInvalidations()).toEqual([])
    expect(readRequests).toEqual([])
    expect(loadedChildIds(hierarchyRead, 'navigation-row:7')).toEqual(['source-directory:12'])

    events.deliver({
      kind: 'batch',
      events: [
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 2,
            occurredAtMs: 1001,
            invalidation: { scope: 'libraryBrowser', revision: '8' }
          }
        }
      ],
      latestEventSequence: 2,
      earliestRetainedSequence: 1,
      gapDetected: false
    })

    const refresh = Promise.all(
      boundaryEvents.consumeMaintainedSnapshotInvalidations().map((event) =>
        refreshHierarchyForMaintainedSnapshotInvalidation(event, {
          hierarchyRead,
          expandedNodeIds: new Set(['navigation-row:7']),
          refreshContentsForCurrentSelection
        })
      )
    )
    await waitForMicrotasks()

    expect(readRequests.map((request) => request.parentDirectoryId ?? 'root')).toEqual(['root'])
    expect(loadedChildIds(hierarchyRead, 'navigation-row:7')).toEqual(['source-directory:12'])
    expect(refreshContentsForCurrentSelection).not.toHaveBeenCalled()

    refreshRead.resolve(refreshedSourceReadResult())
    await expect(refresh).resolves.toEqual([true])

    expect(loadedChildIds(hierarchyRead, 'navigation-row:7')).toEqual(['source-directory:13'])
    expect(refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
    boundaryEvents.stop()
  })
})

function testDependencies(
  overrides: Partial<InvalidationRefreshDependencies> & {
    readonly refresh?: () => Promise<boolean>
  } = {}
): InvalidationRefreshDependencies {
  const { hierarchyRead, refresh, ...rest } = overrides

  return {
    hierarchyRead: {
      ...(refresh === undefined ? {} : { refresh }),
      refreshNavigationRows: vi.fn(async () => true),
      refreshBrowserWindows: vi.fn(async () => true),
      ...hierarchyRead
    },
    expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12']),
    refreshContentsForCurrentSelection: vi.fn(),
    ...rest
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

function sourceScanEvent(rootId: string, eventSequence: number): AppSourceScanEvent {
  return {
    eventSequence,
    occurredAtMs: 1000 + eventSequence,
    kind: 'sourceScanCompleted',
    rootId,
    scanRunId: `scan-${eventSequence}`,
    phase: 'scanning',
    directoriesVisited: 1,
    filesVisited: 2,
    filesDiscovered: 3,
    mediaCandidates: 4,
    queuedWorkItems: 5,
    detail: null
  }
}

function planSnapshot(plan: ReturnType<typeof buildGapRecoveryRefreshPlan>): {
  readonly refreshRootHierarchy: boolean
  readonly refreshNavigationRows: boolean
  readonly refreshExpandedBrowserWindows: boolean
  readonly refreshCurrentContents: boolean
  readonly clearAllContentsWarmSnapshots: boolean
  readonly refreshSourceLifecycleIds: readonly string[]
  readonly acknowledgeGapAfterExecution: boolean
  readonly broadRecovery: boolean
} {
  return {
    refreshRootHierarchy: plan.refreshRootHierarchy,
    refreshNavigationRows: plan.refreshNavigationRows,
    refreshExpandedBrowserWindows: plan.refreshExpandedBrowserWindows,
    refreshCurrentContents: plan.refreshCurrentContents,
    clearAllContentsWarmSnapshots: plan.clearAllContentsWarmSnapshots,
    refreshSourceLifecycleIds: [...plan.refreshSourceLifecycleIds],
    acknowledgeGapAfterExecution: plan.acknowledgeGapAfterExecution,
    broadRecovery: plan.broadRecovery
  }
}

function testEventApi(): BoundaryEventApi & {
  readonly deliver: (payload: BoundaryEventDeliveryPayload) => void
} {
  let callback: ((payload: BoundaryEventDeliveryPayload) => void) | undefined
  const subscribe = vi.fn((next: (payload: BoundaryEventDeliveryPayload) => void) => {
    callback = next
    return () => undefined
  })

  return {
    subscribe,
    deliver(payload): void {
      callback?.(payload)
    }
  }
}

function testLibraryApi(options: {
  readonly readRows: LibraryHierarchyReadApi['navigation']['readRows']
  readonly readChildren: (request: ReadRequest) => Promise<ReadResult>
}): LibraryHierarchyReadApi {
  return {
    host: {
      getStatus: async () => ({
        state: 'started',
        environment: 'development',
        binaryPolicy: {
          kind: 'developmentBinary',
          source: 'environmentOverride'
        },
        lastError: null
      }),
      onStatusChanged: () => () => undefined
    },
    navigation: {
      readRows: options.readRows
    },
    hierarchy: {
      readChildren: options.readChildren
    }
  }
}

function navigationSourceReadRowsResult(): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: [
      {
        navigationRowId: '7',
        stableKey: 'source:7',
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: 'Source Fixture',
        siblingPosition: 0,
        selectable: true,
        selectorKind: 'source',
        selectorPayload: '7',
        updatedAtMs: 100,
        rowVersion: '1'
      }
    ]
  }
}

function initialSourceReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return sourceReadResult([directoryNode('12', 'Album')])
}

function refreshedSourceReadResult(): Extract<ReadResult, { state: 'ready' }> {
  return sourceReadResult([directoryNode('13', 'Refreshed Album'), fileNode('99', 'new.wav')])
}

function sourceReadResult(nodes: readonly ChildRow[]): Extract<ReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: { kind: 'source', sourceId: '7' }
      },
      offset: 0,
      limit: 50,
      totalRows: nodes.length,
      coverage: completeCoverage(),
      nodes
    }
  }
}

function directoryNode(
  directoryId: string,
  label: string
): Extract<ChildRow, { kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    presence: 'present',
    hasChildDirectories: true,
    directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'complete',
    childRowState: 'hasChildRows',
    updatedAtMs: 100
  }
}

function fileNode(fileId: string, label: string): Extract<ChildRow, { kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    mediaClass: 'audio',
    presence: 'present',
    updatedAtMs: 100
  }
}

function completeCoverage(): HierarchyCoverage {
  return {
    state: 'complete',
    recursiveScopeComplete: true,
    emptyResultAuthoritative: false
  }
}

function loadedChildIds(
  controller: ReturnType<typeof createLibraryHierarchyReadController>,
  nodeId: string
): readonly string[] {
  const node = findNode(controller.browserProjection.value?.nodes ?? [], nodeId)

  return node?.children.kind === 'loaded' ? node.children.nodes.map((child) => child.id) : []
}

function topNodeIds(
  controller: ReturnType<typeof createLibraryHierarchyReadController>
): readonly string[] {
  return controller.browserProjection.value?.nodes.map((node) => node.id) ?? []
}

function findNode(nodes: readonly BrowserTreeNode[], nodeId: string): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.children.kind === 'loaded') {
      const child = findNode(node.children.nodes, nodeId)

      if (child !== undefined) {
        return child
      }
    }
  }

  return undefined
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolveDeferred: (value: T) => void = () => undefined
  const promise = new Promise<T>((resolve) => {
    resolveDeferred = resolve
  })

  return {
    promise,
    resolve: resolveDeferred
  }
}

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
