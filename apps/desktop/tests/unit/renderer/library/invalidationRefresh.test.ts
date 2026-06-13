import { readFileSync } from 'node:fs'

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
} from '../../../../src/shared/library/boundary/eventParser'
import type {
  ContentsFileRow,
  ContentsReadRequest,
  ContentsReadResult
} from '../../../../src/shared/library/contents/read'

describe('classify', () => {
  it('maps known scopes and rejects unknown scopes', () => {
    expect(classifyInvalidationScope('navigationRows')).toBe('navigationRows')
    expect(classifyInvalidationScope('contents')).toBe('contents')
    expect(classifyInvalidationScope('sourceLifecycle')).toBe('unknown')
    expect(classifyInvalidationScope('searchFilter')).toBe('unknown')
  })
})

describe('buildInvalidationPlan', () => {
  it('coalesces invalidation batches', () => {
    const input = {
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('contents', '2'),
        invalidation('contents', '3'),
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
      refreshLocalBrowseEntryPoints: true,
      refreshExpandedBrowserWindows: true,
      refreshCurrentContents: true,
      refreshActiveSearchFilter: true,
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
      refreshLocalBrowseEntryPoints: false,
      refreshExpandedBrowserWindows: false,
      refreshCurrentContents: false,
      refreshActiveSearchFilter: false,
      clearAllContentsWarmSnapshots: false,
      refreshSourceLifecycleIds: [],
      acknowledgeGapAfterExecution: false,
      broadRecovery: false
    })
  })

  it('refreshes active search/filter for contents invalidation', () => {
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('contents', '1')],
      sourceLifecycleSourceIds: []
    })

    expect(plan.refreshActiveSearchFilter).toBe(true)
  })

  it('refreshes expanded browser windows for contents invalidation', () => {
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('contents', '1')],
      sourceLifecycleSourceIds: []
    })

    expect(plan.refreshExpandedBrowserWindows).toBe(true)
  })

  it('does not refresh active search/filter for navigation rows alone', () => {
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('navigationRows', '1')],
      sourceLifecycleSourceIds: []
    })

    expect(plan.refreshActiveSearchFilter).toBe(false)
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
      refreshExpandedBrowserWindows: true,
      refreshCurrentContents: false,
      refreshActiveSearchFilter: true
    })
  })

  it('does not refresh active search/filter or browser windows for started or progressed scan events', () => {
    const plan = buildScanPlan({
      events: [
        sourceScanEvent('7', 1, 'sourceScanStarted'),
        sourceScanEvent('7', 2, 'sourceScanProgressed')
      ],
      sourceLifecycleSourceIds: new Set(['7'])
    })

    expect(plan.refreshActiveSearchFilter).toBe(false)
    expect(plan.refreshExpandedBrowserWindows).toBe(false)
  })

  it('refreshes active search/filter and browser windows for terminal scan events', () => {
    const terminalKinds = [
      'sourceScanCompleted',
      'sourceScanFailed',
      'sourceScanBlocked',
      'sourceScanCancelled'
    ] as const

    for (const [index, kind] of terminalKinds.entries()) {
      const plan = buildScanPlan({
        events: [sourceScanEvent('7', index + 1, kind)],
        sourceLifecycleSourceIds: new Set(['7'])
      })

      expect(plan.refreshActiveSearchFilter).toBe(true)
      expect(plan.refreshExpandedBrowserWindows).toBe(true)
    }
  })
})

describe('buildGapPlan', () => {
  it('broad plan with post-execution acknowledgement', () => {
    expect(planSnapshot(buildGapPlan())).toEqual({
      refreshRootHierarchy: true,
      refreshNavigationRows: false,
      refreshLocalBrowseEntryPoints: true,
      refreshExpandedBrowserWindows: false,
      refreshCurrentContents: false,
      refreshActiveSearchFilter: true,
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
      refreshContentsForCurrentSelection: vi.fn(async () => true),
      refreshActiveSearchFilter: vi.fn(async () => true)
    })
    const sourceLifecycleSourceIds = deps.sourceLifecycleSourceIds
    const plan = buildInvalidationPlan({
      invalidations: [
        invalidation('navigationRows', '1'),
        invalidation('navigationRows', '2'),
        invalidation('contents', '3'),
        invalidation('contents', '4')
      ],
      ...(sourceLifecycleSourceIds === undefined ? {} : { sourceLifecycleSourceIds })
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(true)

    expect(deps.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(deps.refreshLocalBrowseEntryPoints).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(deps.expandedNodeIds)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(
      new Set(['7', '9'])
    )
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(deps.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
    expect(deps.refreshActiveSearchFilter).toHaveBeenCalledTimes(1)
  })

  it('continues independent refreshes after one failure', async () => {
    const deps = testDeps({
      sourceLifecycleSourceIds: new Set(['7']),
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true),
      refreshActiveSearchFilter: vi.fn(async () => true)
    })
    vi.mocked(deps.hierarchyRead.refreshNavigationRows).mockRejectedValueOnce(
      new Error('navigation failed')
    )
    const sourceLifecycleSourceIds = deps.sourceLifecycleSourceIds
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('navigationRows', '1'), invalidation('contents', '2')],
      ...(sourceLifecycleSourceIds === undefined ? {} : { sourceLifecycleSourceIds })
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(deps.hierarchyRead.refreshNavigationRows).toHaveBeenCalledTimes(1)
    expect(deps.refreshLocalBrowseEntryPoints).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(deps.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
    expect(deps.refreshActiveSearchFilter).toHaveBeenCalledTimes(1)
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
    expect(deps.refreshLocalBrowseEntryPoints).toHaveBeenCalledTimes(1)
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(plan.acknowledgeGapAfterExecution).toBe(true)
  })

  it('terminal scan plans refresh browser windows but not navigation rows or contents', async () => {
    const deps = testDeps({
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) }
    })
    const plan = buildScanPlan({
      events: [sourceScanEvent('7', 1), sourceScanEvent('8', 2)],
      sourceLifecycleSourceIds: new Set(['7'])
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(true)

    expect(deps.hierarchyRead.refreshNavigationRows).not.toHaveBeenCalled()
    expect(deps.refreshLocalBrowseEntryPoints).not.toHaveBeenCalled()
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledWith(deps.expandedNodeIds)
    expect(deps.refreshContentsForCurrentSelection).not.toHaveBeenCalled()
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledWith(new Set(['7']))
    expect(deps.refreshActiveSearchFilter).toHaveBeenCalledTimes(1)
  })

  it('continues independent refreshes after search/filter refresh fails', async () => {
    const deps = testDeps({
      sourceLifecycleRead: { refreshSourceLifecycles: vi.fn(async () => true) },
      clearContentsWarmSnapshots: vi.fn(),
      refreshContentsForCurrentSelection: vi.fn(async () => true),
      refreshActiveSearchFilter: vi.fn(async () => {
        throw new Error('search failed')
      })
    })
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('contents', '1')],
      sourceLifecycleSourceIds: ['7']
    })

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(deps.hierarchyRead.refreshBrowserWindows).toHaveBeenCalledTimes(1)
    expect(deps.sourceLifecycleRead?.refreshSourceLifecycles).toHaveBeenCalledTimes(1)
    expect(deps.clearContentsWarmSnapshots).toHaveBeenCalledTimes(1)
    expect(deps.refreshContentsForCurrentSelection).toHaveBeenCalledTimes(1)
    expect(deps.refreshActiveSearchFilter).toHaveBeenCalledTimes(1)
  })

  it('search/filter refresh false result contributes to final failure', async () => {
    const deps = testDeps({
      refreshActiveSearchFilter: vi.fn(async () => false)
    })
    const plan = buildGapPlan()

    await expect(executeRefreshPlan(plan, deps)).resolves.toBe(false)

    expect(deps.hierarchyRead.refresh).toHaveBeenCalledTimes(1)
    expect(deps.refreshActiveSearchFilter).toHaveBeenCalledTimes(1)
  })

  it('missing search/filter dependency does not fail execution', async () => {
    const deps = testDeps()
    const depsWithoutSearchFilter = {
      hierarchyRead: deps.hierarchyRead,
      expandedNodeIds: deps.expandedNodeIds
    }
    const plan = buildGapPlan()

    await expect(executeRefreshPlan(plan, depsWithoutSearchFilter)).resolves.toBe(true)
  })

  it('refreshes current contents through the owner', async () => {
    const refreshContentsForCurrentSelection = vi.fn(async () => false)
    const deps = testDeps({
      refreshContentsForCurrentSelection
    })
    const plan = buildInvalidationPlan({
      invalidations: [invalidation('contents', '1')],
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
          scopeCoverage: {
            state: 'scanning',
            emptyResultAuthoritative: false
          }
        }
      }
    })

    const plan = buildInvalidationPlan({
      invalidations: [invalidation('contents', '1')],
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
        policy: { kind: 'audioBrowse' },
        scopeDepth: 'recursive',
        limit: 100
      },
      {
        scope: { kind: 'source', sourceId: '7' },
        policy: { kind: 'audioBrowse' },
        scopeDepth: 'recursive',
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

describe('panel runtime wiring', () => {
  it('uses the search/filter controller invalidation signal in refresh dependencies', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain("import { useSearchFilterRead } from './runtime/searchFilterState'")
    expect(panel).toContain('const searchFilterRead = useSearchFilterRead()')
    expect(panel).toContain(
      'refreshActiveSearchFilter: () => searchFilterRead.invalidationSignal()'
    )
  })

  it('wires expanded disclosure reconciliation from panel state', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain(
      "import { createDisclosureReconciler } from './runtime/disclosureReconciliation'"
    )
    expect(panel).toContain('const disclosureReconciler = createDisclosureReconciler')
    expect(panel).toContain('disclosureReconciler.reconcile({')
    expect(panel).toContain('expandedNodeIds: expandedIds')
    expect(panel).not.toContain('requestNodeChildrenIfExpandable')
  })

  it('wires local browse refresh from host start and refresh dependencies', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain("import { useLocalBrowseController } from './localBrowse/controller'")
    expect(panel).toContain('const libraryBrowseProfile = createLibraryBrowseProfileController()')
    expect(panel).toContain('const localPreviewMode = createLocalPreviewModeController()')
    expect(panel).toContain('const localBrowse = useLocalBrowseController(undefined, {')
    expect(panel).toContain('localPreviewMode: localPreviewMode.mode')
    expect(panel).not.toContain(
      'const localBrowse = useLocalBrowseController(undefined, {\n  profile'
    )
    expect(panel).toContain('void localBrowse.refreshEntryPoints()')
    expect(panel).toContain('refreshLocalBrowseEntryPoints: () => localBrowse.refreshEntryPoints()')
  })

  it('keeps library profile and local preview mode watchers separate', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain('() => libraryBrowseProfile.profile.value')
    expect(panel).toContain('await hierarchyRead.refreshBrowserWindows(expandedNodeIds.value)')
    expect(panel).toContain('requestContentsForCurrentSelection({ force: true })')
    expect(panel).toContain('() => localPreviewMode.mode.value')
    expect(panel).toContain(
      'await localBrowse.refreshBrowserWindows(expandedNodeIds.value, browserProjection.value)'
    )
    expect(panel).toContain('libraryBrowseProfile: libraryBrowseProfile.profile.value')
    expect(panel).toContain('localPreviewMode: localPreviewMode.mode.value')
  })

  it('clears failed disclosure ledger entries from scan completion and manual retries', () => {
    const panel = readRendererSource('panel.vue')

    expect(panel).toContain("event.kind === 'sourceScanCompleted'")
    expect(panel).toContain('disclosureReconciler.clearFailed()')
    expect(panel).toContain('disclosureReconciler.clearFailedForNode(nodeId)')
    expect(panel).toContain('disclosureReconciler.clearFailedForNode(action.nodeId)')
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
    refreshLocalBrowseEntryPoints: vi.fn(),
    refreshContentsForCurrentSelection: vi.fn(),
    refreshActiveSearchFilter: vi.fn(),
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

function sourceScanEvent(
  rootId: string,
  eventSequence: number,
  kind: AppSourceScanEvent['kind'] = 'sourceScanCompleted'
): AppSourceScanEvent {
  return {
    eventSequence,
    occurredAtMs: 1000 + eventSequence,
    kind,
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
      policy: { kind: 'audioBrowse' },
      scopeDepth: 'recursive',
      rows,
      scopeCoverage: {
        state: state === 'partial' ? 'scanning' : 'complete',
        subtreeCoverageComplete: state === 'ready',
        emptyResultAuthoritative: false
      },
      hasPolicyOmittedRows: false
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
  readonly refreshLocalBrowseEntryPoints: boolean
  readonly refreshExpandedBrowserWindows: boolean
  readonly refreshCurrentContents: boolean
  readonly refreshActiveSearchFilter: boolean
  readonly clearAllContentsWarmSnapshots: boolean
  readonly refreshSourceLifecycleIds: readonly string[]
  readonly acknowledgeGapAfterExecution: boolean
  readonly broadRecovery: boolean
} {
  return {
    refreshRootHierarchy: plan.refreshRootHierarchy,
    refreshNavigationRows: plan.refreshNavigationRows,
    refreshLocalBrowseEntryPoints: plan.refreshLocalBrowseEntryPoints,
    refreshExpandedBrowserWindows: plan.refreshExpandedBrowserWindows,
    refreshCurrentContents: plan.refreshCurrentContents,
    refreshActiveSearchFilter: plan.refreshActiveSearchFilter,
    clearAllContentsWarmSnapshots: plan.clearAllContentsWarmSnapshots,
    refreshSourceLifecycleIds: [...plan.refreshSourceLifecycleIds],
    acknowledgeGapAfterExecution: plan.acknowledgeGapAfterExecution,
    broadRecovery: plan.broadRecovery
  }
}

function readRendererSource(relativePath: string): string {
  return readFileSync(
    new URL(`../../../../src/renderer/library/${relativePath}`, import.meta.url),
    {
      encoding: 'utf8'
    }
  )
}
