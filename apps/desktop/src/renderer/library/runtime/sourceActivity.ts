import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

import type { ScanProgressState } from '../boundary/boundaryEvents'

export type ProjectedSourceActivity = ReadSourceActivityReply

export type SourceActivityBacklogCategory = {
  readonly label: 'hash' | 'probe' | 'attachment' | 'promotion' | 'identity'
  readonly count: number
}

export function projectSourceActivity(input: {
  readonly activity?: ReadSourceActivityReply
  readonly scanProgress?: ScanProgressState
  readonly maintenanceRunState?: 'idle' | 'running' | 'completed' | 'failed'
}): ProjectedSourceActivity | undefined {
  const activity = input.activity
  if (activity === undefined) {
    return undefined
  }

  const scanActivity = scanActivityFromProgress(input.scanProgress) ?? activity.scanActivity
  const preparationActivity =
    input.maintenanceRunState === 'completed' &&
    activity.preparationActivity.lastRunStatus !== undefined
      ? { ...activity.preparationActivity, provenance: 'runResult' as const }
      : activity.preparationActivity

  return {
    ...activity,
    scanActivity,
    preparationActivity
  }
}

export function projectSourceActivityBySourceId(input: {
  readonly activityBySourceId: ReadonlyMap<string, ReadSourceActivityReply>
  readonly scanProgressByRootId: ReadonlyMap<string, ScanProgressState>
  readonly maintenanceRunStateBySourceId?: ReadonlyMap<
    string,
    'idle' | 'running' | 'completed' | 'failed'
  >
}): ReadonlyMap<string, ProjectedSourceActivity> {
  const next = new Map<string, ProjectedSourceActivity>()

  for (const [sourceId, activity] of input.activityBySourceId) {
    const scanProgress = input.scanProgressByRootId.get(sourceId)
    const maintenanceRunState = input.maintenanceRunStateBySourceId?.get(sourceId)
    const projected = projectSourceActivity({
      activity,
      ...(scanProgress === undefined ? {} : { scanProgress }),
      ...(maintenanceRunState === undefined ? {} : { maintenanceRunState })
    })

    if (projected !== undefined) {
      next.set(sourceId, projected)
    }
  }

  return next
}

export function sourceActivityBacklogCategories(
  activity: Pick<ReadSourceActivityReply, 'preparationActivity'>
): readonly SourceActivityBacklogCategory[] {
  const backlog = activity.preparationActivity.backlog
  const categories = [
    { label: 'hash', count: backlog.hash },
    { label: 'probe', count: backlog.probe },
    { label: 'attachment', count: backlog.attachment },
    { label: 'promotion', count: backlog.promotion },
    { label: 'identity', count: backlog.identity }
  ] satisfies readonly SourceActivityBacklogCategory[]

  return categories.filter((category) => category.count > 0)
}

export function sourceActivityBacklogTotal(
  activity: Pick<ReadSourceActivityReply, 'preparationActivity'>
): number {
  const backlog = activity.preparationActivity.backlog
  return backlog.hash + backlog.probe + backlog.attachment + backlog.promotion + backlog.identity
}

export function sourceActivityPreparationSummary(
  activity: Pick<ReadSourceActivityReply, 'preparationActivity'>
): string | undefined {
  const preparation = activity.preparationActivity
  const categories = sourceActivityBacklogCategories(activity)
  const categoryDetail = categories
    .map((category) => `${category.label} ${category.count}`)
    .join(', ')

  switch (preparation.state) {
    case 'running':
      return 'Preparing source.'
    case 'completedWithRemainingWork':
      return categoryDetail.length === 0
        ? 'Maintenance completed; pending work remains. Run maintenance processes a bounded batch.'
        : `Maintenance completed; pending work remains: ${categoryDetail}. Run maintenance processes a bounded batch.`
    case 'idle':
      return categoryDetail.length === 0
        ? undefined
        : `Preparation pending: ${categoryDetail}. Run maintenance processes a bounded batch.`
    case 'complete':
      return 'Preparation complete.'
    case 'failed':
      return 'Preparation failed.'
    case 'unavailable':
      return 'Preparation status unavailable.'
  }
}

export function sourceActivityScanSummary(
  activity: Pick<ReadSourceActivityReply, 'scanActivity'>
): string | undefined {
  const scan = activity.scanActivity

  switch (scan.state) {
    case 'running':
      return scanRunningDetail(scan.counters)
    case 'completed':
      return 'Scan completed.'
    case 'failed':
      return scan.detail ?? 'Scan failed.'
    case 'blocked':
      return scan.detail ?? 'Scan is blocked.'
    case 'cancelled':
      return scan.detail ?? 'Scan was cancelled.'
    case 'idle':
      return scan.detail
  }
}

function scanActivityFromProgress(
  progress: ScanProgressState | undefined
): ReadSourceActivityReply['scanActivity'] | undefined {
  if (progress === undefined || progress.kind === 'idle') {
    return undefined
  }

  switch (progress.kind) {
    case 'scanning':
      return {
        state: 'running',
        counters: {
          directoriesVisited: progress.directoriesVisited,
          filesVisited: progress.filesVisited,
          filesDiscovered: progress.filesDiscovered,
          mediaCandidates: progress.mediaCandidates,
          queuedWorkItems: progress.queuedWorkItems
        },
        scanRunId: progress.scanRunId
      }
    case 'completed':
      return {
        state: 'completed',
        counters: {
          filesDiscovered: progress.filesDiscovered,
          queuedWorkItems: progress.queuedWorkItems
        },
        scanRunId: progress.scanRunId,
        detail: 'Scan completed.'
      }
    case 'failed':
      return {
        state: 'failed',
        counters: {},
        detail: progress.detail ?? 'Scan failed.'
      }
    case 'blocked':
      return {
        state: 'blocked',
        counters: {},
        detail: progress.detail ?? 'Scan is blocked.'
      }
    case 'cancelled':
      return {
        state: 'cancelled',
        counters: {},
        scanRunId: progress.scanRunId,
        detail: progress.detail ?? 'Scan was cancelled.'
      }
  }
}

function scanRunningDetail(
  counters: ReadSourceActivityReply['scanActivity']['counters']
): string {
  const parts: string[] = []

  if (counters.filesDiscovered !== undefined && counters.filesDiscovered > 0) {
    parts.push(
      `${counters.filesDiscovered} ${counters.filesDiscovered === 1 ? 'file' : 'files'} discovered`
    )
  }

  if (counters.directoriesVisited !== undefined && counters.directoriesVisited > 0) {
    parts.push(
      `${counters.directoriesVisited} ${counters.directoriesVisited === 1 ? 'folder' : 'folders'} visited`
    )
  }

  if (counters.queuedWorkItems !== undefined && counters.queuedWorkItems > 0) {
    parts.push(
      `${counters.queuedWorkItems} ${counters.queuedWorkItems === 1 ? 'work item' : 'work items'} queued`
    )
  }

  return parts.length === 0 ? 'Scanning source.' : `Scanning source: ${parts.join(', ')}.`
}
