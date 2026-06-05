import type {
  AppMaintainedSnapshotInvalidatedEvent,
  AppSourceScanEvent
} from '../../../shared/libraryBoundary/eventParser'
import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { SourceLifecycleReadController } from '../boundary/sourceLifecycleRead'
import type { BrowserTreeNodeId } from '../tree/types'

export type RendererInvalidationScope =
  | 'navigationRows'
  | 'libraryBrowser'
  | 'contents'
  | 'sourceLifecycle'
  | 'unknown'

export type LibraryRefreshPlan = {
  readonly refreshRootHierarchy: boolean
  readonly refreshNavigationRows: boolean
  readonly refreshExpandedBrowserWindows: boolean
  readonly refreshCurrentContents: boolean
  readonly clearAllContentsWarmSnapshots: boolean
  readonly refreshSourceLifecycleIds: ReadonlySet<string>
  readonly acknowledgeGapAfterExecution: boolean
  readonly broadRecovery: boolean
}

export type MaintainedSnapshotInvalidationRefreshPlanInput = {
  readonly invalidations: readonly AppMaintainedSnapshotInvalidatedEvent[]
  readonly sourceLifecycleSourceIds?: Iterable<string>
}

export type SourceScanRefreshPlanInput = {
  readonly events: readonly AppSourceScanEvent[]
  readonly sourceLifecycleSourceIds?: ReadonlySet<string>
}

export type InvalidationRefreshDependencies = {
  readonly hierarchyRead: Pick<
    LibraryHierarchyReadController,
    'refreshNavigationRows' | 'refreshBrowserWindows' | 'refresh'
  >
  readonly sourceLifecycleRead?: Pick<SourceLifecycleReadController, 'refreshSourceLifecycles'>
  readonly sourceLifecycleSourceIds?: ReadonlySet<string>
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly clearContentsWarmSnapshots?: () => void
  readonly refreshContentsForCurrentSelection?: () => Promise<boolean> | boolean | void
}

export function classifyInvalidationScope(scope: string): RendererInvalidationScope {
  switch (scope) {
    case 'navigationRows':
      return 'navigationRows'
    case 'libraryBrowser':
      return 'libraryBrowser'
    default:
      return 'unknown'
  }
}

export function buildMaintainedSnapshotInvalidationRefreshPlan(
  input: MaintainedSnapshotInvalidationRefreshPlanInput
): LibraryRefreshPlan {
  let refreshNavigationRows = false
  let refreshExpandedBrowserWindows = false
  let refreshCurrentContents = false
  let clearAllContentsWarmSnapshots = false
  const refreshSourceLifecycleIds = new Set<string>()

  for (const event of input.invalidations) {
    switch (classifyInvalidationScope(event.invalidation.scope)) {
      case 'navigationRows':
        refreshNavigationRows = true
        addSourceLifecycleIds(refreshSourceLifecycleIds, input.sourceLifecycleSourceIds)
        break
      case 'libraryBrowser':
        refreshExpandedBrowserWindows = true
        refreshCurrentContents = true
        clearAllContentsWarmSnapshots = true
        addSourceLifecycleIds(refreshSourceLifecycleIds, input.sourceLifecycleSourceIds)
        break
      case 'contents':
      case 'sourceLifecycle':
      case 'unknown':
        break
    }
  }

  return refreshPlan({
    refreshNavigationRows,
    refreshExpandedBrowserWindows,
    refreshCurrentContents,
    clearAllContentsWarmSnapshots,
    refreshSourceLifecycleIds
  })
}

export function buildSourceScanRefreshPlan(input: SourceScanRefreshPlanInput): LibraryRefreshPlan {
  const refreshSourceLifecycleIds = new Set<string>()
  const visibleSourceIds = input.sourceLifecycleSourceIds

  if (visibleSourceIds !== undefined) {
    for (const event of input.events) {
      if (visibleSourceIds.has(event.rootId)) {
        refreshSourceLifecycleIds.add(event.rootId)
      }
    }
  }

  return refreshPlan({ refreshSourceLifecycleIds })
}

export function buildGapRecoveryRefreshPlan(): LibraryRefreshPlan {
  return refreshPlan({
    refreshRootHierarchy: true,
    clearAllContentsWarmSnapshots: true,
    acknowledgeGapAfterExecution: true,
    broadRecovery: true
  })
}

export async function executeLibraryRefreshPlan(
  plan: LibraryRefreshPlan,
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  let succeeded = true

  if (plan.refreshRootHierarchy) {
    succeeded = (await runRefresh(dependencies.hierarchyRead.refresh)) && succeeded
  }

  if (plan.refreshNavigationRows && !plan.refreshRootHierarchy) {
    succeeded = (await runRefresh(dependencies.hierarchyRead.refreshNavigationRows)) && succeeded
  }

  if (plan.refreshExpandedBrowserWindows) {
    succeeded =
      (await runRefresh(() =>
        dependencies.hierarchyRead.refreshBrowserWindows(dependencies.expandedNodeIds)
      )) && succeeded
  }

  if (plan.refreshSourceLifecycleIds.size > 0) {
    const refreshedSourceLifecycles = await runRefresh(() => {
      const refreshSourceLifecycles = dependencies.sourceLifecycleRead?.refreshSourceLifecycles
      return refreshSourceLifecycles?.(plan.refreshSourceLifecycleIds) ?? Promise.resolve(true)
    })
    succeeded = refreshedSourceLifecycles && succeeded
  }

  if (plan.refreshCurrentContents) {
    if (plan.clearAllContentsWarmSnapshots) {
      succeeded = runSync(dependencies.clearContentsWarmSnapshots) && succeeded
    }
    succeeded =
      (await runCurrentContentsRefresh(dependencies.refreshContentsForCurrentSelection)) &&
      succeeded
  } else if (plan.clearAllContentsWarmSnapshots) {
    succeeded = runSync(dependencies.clearContentsWarmSnapshots) && succeeded
  }

  return succeeded
}

export async function refreshHierarchyForMaintainedSnapshotInvalidation(
  event: AppMaintainedSnapshotInvalidatedEvent,
  dependencies: InvalidationRefreshDependencies
): Promise<boolean> {
  const plan = buildMaintainedSnapshotInvalidationRefreshPlan({
    invalidations: [event],
    ...(dependencies.sourceLifecycleSourceIds === undefined
      ? {}
      : { sourceLifecycleSourceIds: dependencies.sourceLifecycleSourceIds })
  })

  return executeLibraryRefreshPlan(plan, dependencies)
}

function refreshPlan(
  input: Partial<
    Omit<LibraryRefreshPlan, 'refreshSourceLifecycleIds'> & {
      readonly refreshSourceLifecycleIds: Iterable<string>
    }
  >
): LibraryRefreshPlan {
  return {
    refreshRootHierarchy: input.refreshRootHierarchy ?? false,
    refreshNavigationRows: input.refreshNavigationRows ?? false,
    refreshExpandedBrowserWindows: input.refreshExpandedBrowserWindows ?? false,
    refreshCurrentContents: input.refreshCurrentContents ?? false,
    clearAllContentsWarmSnapshots: input.clearAllContentsWarmSnapshots ?? false,
    refreshSourceLifecycleIds: new Set(input.refreshSourceLifecycleIds ?? []),
    acknowledgeGapAfterExecution: input.acknowledgeGapAfterExecution ?? false,
    broadRecovery: input.broadRecovery ?? false
  }
}

function addSourceLifecycleIds(target: Set<string>, sourceIds: Iterable<string> | undefined): void {
  if (sourceIds === undefined) {
    return
  }

  for (const sourceId of sourceIds) {
    if (sourceId.length > 0) {
      target.add(sourceId)
    }
  }
}

async function runRefresh(refresh: (() => Promise<boolean>) | undefined): Promise<boolean> {
  if (refresh === undefined) {
    return true
  }

  try {
    return await refresh()
  } catch {
    return false
  }
}

async function runCurrentContentsRefresh(
  refresh: (() => Promise<boolean> | boolean | void) | undefined
): Promise<boolean> {
  if (refresh === undefined) {
    return true
  }

  try {
    const refreshed = await refresh()
    return refreshed !== false
  } catch {
    return false
  }
}

function runSync(work: (() => void) | undefined): boolean {
  if (work === undefined) {
    return true
  }

  try {
    work()
    return true
  } catch {
    return false
  }
}
