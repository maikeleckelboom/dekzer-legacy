import type {
  AppMaintainedSnapshotInvalidatedEvent,
  AppSourceScanEvent
} from '../../../shared/library/boundary/eventParser'
import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { SourceLifecycleReadController } from '../boundary/sourceLifecycleRead'
import type { BrowserTreeNodeId } from '../tree/types'

export type InvalidationScope = 'navigationRows' | 'contents' | 'sourceLifecycle' | 'unknown'

export type RefreshPlan = {
  readonly refreshRootHierarchy: boolean
  readonly refreshNavigationRows: boolean
  readonly refreshExpandedBrowserWindows: boolean
  readonly refreshCurrentContents: boolean
  readonly refreshActiveSearchFilter: boolean
  readonly clearAllContentsWarmSnapshots: boolean
  readonly refreshSourceLifecycleIds: ReadonlySet<string>
  readonly acknowledgeGapAfterExecution: boolean
  readonly broadRecovery: boolean
}

export type InvalidationPlanInput = {
  readonly invalidations: readonly AppMaintainedSnapshotInvalidatedEvent[]
  readonly sourceLifecycleSourceIds?: Iterable<string>
}

export type ScanPlanInput = {
  readonly events: readonly AppSourceScanEvent[]
  readonly sourceLifecycleSourceIds?: ReadonlySet<string>
}

export type RefreshPlanDeps = {
  readonly hierarchyRead: Pick<
    LibraryHierarchyReadController,
    'refreshNavigationRows' | 'refreshBrowserWindows' | 'refresh'
  >
  readonly sourceLifecycleRead?: Pick<SourceLifecycleReadController, 'refreshSourceLifecycles'>
  readonly sourceLifecycleSourceIds?: ReadonlySet<string>
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly clearContentsWarmSnapshots?: () => void
  readonly refreshContentsForCurrentSelection?: () => Promise<boolean> | boolean | void
  readonly refreshActiveSearchFilter?: () => Promise<boolean> | boolean | void
}

export function classifyInvalidationScope(scope: string): InvalidationScope {
  switch (scope) {
    case 'navigationRows':
      return 'navigationRows'
    case 'contents':
      return 'contents'
    default:
      return 'unknown'
  }
}

export function buildInvalidationPlan(input: InvalidationPlanInput): RefreshPlan {
  let refreshNavigationRows = false
  let refreshExpandedBrowserWindows = false
  let refreshCurrentContents = false
  let refreshActiveSearchFilter = false
  let clearAllContentsWarmSnapshots = false
  const refreshSourceLifecycleIds = new Set<string>()

  for (const event of input.invalidations) {
    switch (classifyInvalidationScope(event.invalidation.scope)) {
      case 'navigationRows':
        refreshNavigationRows = true
        addSourceLifecycleIds(refreshSourceLifecycleIds, input.sourceLifecycleSourceIds)
        break
      case 'contents':
        refreshExpandedBrowserWindows = true
        refreshCurrentContents = true
        refreshActiveSearchFilter = true
        clearAllContentsWarmSnapshots = true
        addSourceLifecycleIds(refreshSourceLifecycleIds, input.sourceLifecycleSourceIds)
        break
      case 'sourceLifecycle':
      case 'unknown':
        break
    }
  }

  return refreshPlan({
    refreshNavigationRows,
    refreshExpandedBrowserWindows,
    refreshCurrentContents,
    refreshActiveSearchFilter,
    clearAllContentsWarmSnapshots,
    refreshSourceLifecycleIds
  })
}

export function buildScanPlan(input: ScanPlanInput): RefreshPlan {
  const refreshSourceLifecycleIds = new Set<string>()
  const visibleSourceIds = input.sourceLifecycleSourceIds
  let refreshExpandedBrowserWindows = false
  let refreshActiveSearchFilter = false

  if (visibleSourceIds !== undefined) {
    for (const event of input.events) {
      if (visibleSourceIds.has(event.rootId)) {
        refreshSourceLifecycleIds.add(event.rootId)
      }
    }
  }

  for (const event of input.events) {
    if (isTerminalSourceScanEvent(event)) {
      refreshExpandedBrowserWindows = true
      refreshActiveSearchFilter = true
      break
    }
  }

  return refreshPlan({
    refreshExpandedBrowserWindows,
    refreshSourceLifecycleIds,
    refreshActiveSearchFilter
  })
}

export function buildGapPlan(): RefreshPlan {
  return refreshPlan({
    refreshRootHierarchy: true,
    refreshActiveSearchFilter: true,
    clearAllContentsWarmSnapshots: true,
    acknowledgeGapAfterExecution: true,
    broadRecovery: true
  })
}

export async function executeRefreshPlan(
  plan: RefreshPlan,
  dependencies: RefreshPlanDeps
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

  if (plan.refreshActiveSearchFilter) {
    succeeded =
      (await runOptionalBooleanRefresh(dependencies.refreshActiveSearchFilter)) && succeeded
  }

  return succeeded
}

function refreshPlan(
  input: Partial<
    Omit<RefreshPlan, 'refreshSourceLifecycleIds'> & {
      readonly refreshSourceLifecycleIds: Iterable<string>
    }
  >
): RefreshPlan {
  return {
    refreshRootHierarchy: input.refreshRootHierarchy ?? false,
    refreshNavigationRows: input.refreshNavigationRows ?? false,
    refreshExpandedBrowserWindows: input.refreshExpandedBrowserWindows ?? false,
    refreshCurrentContents: input.refreshCurrentContents ?? false,
    refreshActiveSearchFilter: input.refreshActiveSearchFilter ?? false,
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
  return runOptionalBooleanRefresh(refresh)
}

async function runOptionalBooleanRefresh(
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

function isTerminalSourceScanEvent(event: AppSourceScanEvent): boolean {
  switch (event.kind) {
    case 'sourceScanCompleted':
    case 'sourceScanFailed':
    case 'sourceScanBlocked':
    case 'sourceScanCancelled':
      return true
    case 'sourceScanStarted':
    case 'sourceScanProgressed':
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
