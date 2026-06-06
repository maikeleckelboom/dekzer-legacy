import { describe, expect, it, vi } from 'vitest'

import { createContentsReadController } from '../../../../src/renderer/library/boundary/contentsRead'
import type { RowBinding } from '../../../../src/renderer/library/state'
import {
  buildGapPlan,
  buildInvalidationPlan,
  buildScanPlan,
  classifyInvalidationScope,
  executeRefreshPlan,
  type RefreshPlanDeps
} from '../../../../src/renderer/library/runtime/invalidationRefresh'
import type {
  AppMaintainedSnapshotInvalidatedEvent,
  AppSourceScanEvent
} from '../../../../src/shared/libraryBoundary/eventParser'
import type {
  ContentsFileRow,
  ContentsReadRequest,
  ContentsReadResult
} from '../../../../src/shared/libraryContents/read'

describe('classify', () => {
  it('maps known scopes and rejects unknown scopes', () => {
    expect(classifyInvalidationScope('navigationRows')).toBe('navigationRows')
    expect(classifyInvalidationScope('libraryBrowser')).toBe('libraryBrowser')
    expect(classifyInvalidationScope('LibraryBrowser')).toBe('unknown')
    expect(classifyInvalidationScope('contents')).toBe('unknown')
    expect(classifyInvalidationScope('sourceLifecycle')).toBe('unknown')
  })
})

describe('buildInvalidationPlan', () => {
  it('coalesces invalidation batches', () => {
    const input = {
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('libraryBrowser', '2'),
        invalidation('libraryBrowser', '3'),
        invalidation('unsupportedScope', '4')
      ],
      sourceLifecycleSourceIds: ['7', '7', '9']
    }

    const first = buildInvalidationPlan(input)
    const second = buildInvalidationPlan(input)

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

  it('unknown scopes are no-ops', () => {
    const plan = buildInvalidationPlan({
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
})

describe('buildScanPlan', () => {
  it('dedupes visible source lifecycle ids', () => {
    const plan = buildScanPlan({
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
})

describe('buildGapPlan', () => {
  it('broad plan with post-execution acknowledgement', () => {
    expect(planSnapshot(buildGapPlan())).toEqual({
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

describe('executeRefreshPlan', () => {
  it('runs each refresh owner at most once', async () => {
    const deps = testDeps({
      sourceLifecycleSourceIds: new Set(['7', '7', '9']),
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true)
    })
    const sourceLifecycleSourceIds = deps.sourceLifecycleSourceIds
    const plan = buildInvalidationPlan({
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('navigationRows', '2'),
        invalidation('libraryBrowser', '3'),
        invalidation('libraryBrowser', '4')
      ],
      ...(sourceLifecycleSourceIds === undefined ? {} : { sourceLifecycleSourceIds })
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(true)

    expect(deps.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(
      new Set(['navigation-row:7', 'source-directory:12'])
    )
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(
      new Set(['7', '9'])
    )
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(deps.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('continues independent refreshes after one failure', async () => {
    const deps = testDeps({
      sourceLifecycleSourceIds: new Set(['7']),
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true)
    })
    vi.mocked(deps.hierarchyRead.refreshNavigationRows).mockRejectedValueOnce(
      new Error('navigation failed')
    )
    const sourceLifecycleSourceIds = deps.sourceLifecycleSourceIds
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('navigationRows', '1'), invalidation('libraryBrowser', '2')],
      ...(sourceLifecycleSourceIds === undefined ? {} : { sourceLifecycleSourceIds })
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(deps.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(deps.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('gap recovery prefers full hierarchy refresh and clears warm contents', async () => {
    const deps = testDeps({
      hierarchyRead: {
        refresh: vi.fn(async () => false),
        refreshNavigationRows: vi.fn(async () => true),
        refreshBrowserWindows: vi.fn(async () => true)
      },
      clearContentsWarmSnapshots: vi.fn()
    })
    const plan = buildGapPlan()

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(deps.hierarchyRead.refresh).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(deps.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(plan.acknowledgeGapAfterExecution).toBe(true)
  })

  it('scan plans do not refresh hierarchy or contents', async () => {
    const deps = testDeps({
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) }
    })
    const plan = buildScanPlan({
      events: [sourceScanEvent('7', 1), sourceScanEvent('8', 2)],
      sourceLifecycleSourceIds: new Set(['7'])
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(true)

    expect(deps.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(deps.hierarchyRead.refreshBrowserWindows).not.toHaveBeenCalled()
    expect(deps.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(new Set(['7']))
  })

  it('refreshes current contents through the owner', async () => {
    const refreshContentsForCurrentSelection = vi.fn(async () => false)
    const deps = testDeps({
      refreshContentsForCurrentSelection
    })
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('libraryBrowser', '1')],
      sourceLifecycleSourceIds: []
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
  })

  it('refreshes a selected mid-scan source from zero known rows when invalidation arrives', async () => {
    const responses: ContentsReadResult[] = [
      readyContentsResult([], 'partial'),
      readyContentsResult([audioRow('source-file:1', 'track.wav')], 'ready')
    ]
    const requests: ContentsReadRequest[] = []
    const contentsRead = createContentsReadController({
      read: async (request) => {
        requests.push(request)
        const response = responses.shift()
        if (response === undefined) {
          throw new Error('Unexpected contents read.')
        }
        return response
      }
    })
    const selectedSourceBinding: RowBinding = {
      kind: 'source',
      navigationRow: {
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
      },
      target: {
        navigationRowId: '7',
        entryPoint: { kind: 'source', sourceId: '7' },
        label: 'Source Fixture'
      }
    }

    contentsRead.start()
    await expect(contentsRead.readForBinding(selectedSourceBinding)).resolves.toBe(true)
    expect(contentsRead.state.value).toMatchObject({
      kind: 'ready',
      result: {
        state: 'ready',
        result: {
          state: 'partial',
          rows: [],
          coverage: {
            state: 'scanning',
            emptyResultAuthoritative: false
          }
        }
      }
    })

    const plan = buildInvalidationPlan({
      invalidations: [invalidation('libraryBrowser', '1')],
      sourceLifecycleSourceIds: ['7']
    })
    await expect(
      executeRefreshPlan(
        plan,
        testDeps({
          clearContentsWarmSnapshots: contentsRead.clearWarmSnapshots,
          refreshContentsForCurrentSelection: () =>
            contentsRead.readForBinding(selectedSourceBinding, { force: true })
        })
      )
    ).resolves.toBe(true)

    expect(requests).toEqual([
      {
        scope: { kind: 'source', sourceId: '7' },
        policy: { kind: 'playableMediaBrowse' },
        recursion: 'recursive',
        limit: 100
      },
      {
        scope: { kind: 'source', sourceId: '7' },
        policy: { kind: 'playableMediaBrowse' },
        recursion: 'recursive',
        limit: 100
      }
    ])
    expect(contentsRead.state.value).toMatchObject({
      kind: 'ready',
      result: {
        state: 'ready',
        result: {
          state: 'ready',
          rows: [{ id: 'source-file:1', label: 'track.wav' }]
        }
      }
    })
  })
})

function testDeps(overrides: Partial<RefreshPlanDeps> = {}): RefreshPlanDeps {
  const { hierarchyRead, ...rest } = overrides

  return {
    hierarchyRead: {
      refresh: vi.fn(async () => true),
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

function readyContentsResult(
  rows: readonly ContentsFileRow[],
  state: 'partial' | 'ready'
): ContentsReadResult {
  return {
    state: 'ready',
    result: {
      state,
      scope: { kind: 'source', sourceId: '7' },
      policy: { kind: 'playableMediaBrowse' },
      recursion: 'recursive',
      rows,
      coverage: {
        state: state === 'partial' ? 'scanning' : 'complete',
        recursiveScopeComplete: state === 'ready',
        emptyResultAuthoritative: false
      },
      hasRowsOmittedByPolicy: false
    }
  }
}

function audioRow(id: string, label: string): ContentsFileRow {
  return {
    id,
    sourceId: '7',
    sourceFileId: id,
    label,
    relativePath: label,
    fileName: label,
    fileClass: 'audio',
    fileKind: 'audio',
    presence: 'present',
    updatedAtMs: 100
  }
}

function planSnapshot(plan: ReturnType<typeof buildGapPlan>): {
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
