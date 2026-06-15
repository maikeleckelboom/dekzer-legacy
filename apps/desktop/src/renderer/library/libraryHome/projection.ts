import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

import type { BrowserState, RowBinding } from '../state'
import type { SourceReadiness } from '../runtime/sourceReadiness'
import type { BrowserTreeNodeId } from '../tree/types'
import {
  defaultLibraryBrowseProfile,
  type LibraryBrowseProfile
} from '../libraryBrowseProfile/types'
import {
  sourceActivityBacklogTotal,
  sourceActivityPreparationSummary,
  sourceActivityScanSummary,
  type ProjectedSourceActivity
} from '../runtime/sourceActivity'
import { sourceMaintenanceBacklog } from '../runtime/sourceMaintenanceSummary'

export type LibraryHomeProductState =
  | 'noSources'
  | 'chooseSource'
  | 'indexing'
  | 'ready'
  | 'needsScan'
  | 'maintenanceNeeded'
  | 'missing'
  | 'blocked'
  | 'unavailable'
  | 'emptyCurrentView'

export type LibraryHomeTone = 'ready' | 'active' | 'warning' | 'danger' | 'muted'

export type LibraryHomeAction = {
  readonly kind: 'chooseMusicFolder'
  readonly label: 'Add Source'
}

export type LibraryHomeRow = {
  readonly id: string
  readonly label: string
  readonly detail: string
  readonly state: 'empty' | 'loading' | 'failed' | 'unsupported'
  readonly action?: LibraryHomeAction
}

export type LibraryHomeProjection = {
  readonly productState: LibraryHomeProductState
  readonly surfaceLabel: 'Library'
  readonly title: string
  readonly detail: string
  readonly rows: readonly LibraryHomeRow[]
}

export type LibrarySourceReadinessBadge =
  | 'Ready'
  | 'Indexing'
  | 'Needs scan'
  | 'Scanning source'
  | 'Preparing source'
  | 'Maintenance completed; pending work remains'
  | 'Preparation complete'
  | 'Missing'
  | 'Blocked'
  | 'Offline/unavailable'
  | 'Maintenance needed'
  | 'Maintenance unavailable'
  | 'No audio tracks in this view'
  | 'No playable media in this view'
  | 'No files in this source inventory view'

export type LibrarySourceReadinessProjection = {
  readonly productState: LibraryHomeProductState
  readonly badge: LibrarySourceReadinessBadge
  readonly tone: LibraryHomeTone
  readonly detail: string
}

export function projectLibraryHome(input: {
  readonly state: BrowserState
  readonly bindingsById?: ReadonlyMap<BrowserTreeNodeId, RowBinding>
  readonly sourceIntegrityBySourceId?: ReadonlyMap<string, ReadSourceIntegrityReply>
  readonly sourceMaintenanceBySourceId?: ReadonlyMap<string, ReadSourceMaintenanceReply>
  readonly sourceActivityBySourceId?: ReadonlyMap<string, ProjectedSourceActivity>
}): LibraryHomeProjection {
  const sourceBindings = admittedLibraryHomeRootBindings(input.bindingsById)
  const navigationReadResult = input.state.navigationReadResult
  const profile = input.state.libraryBrowseProfile ?? defaultLibraryBrowseProfile

  if (sourceBindings.length === 0) {
    if (navigationReadResult === undefined) {
      return {
        productState: 'indexing',
        surfaceLabel: 'Library',
        title: 'Library',
        detail: 'Loading your library.',
        rows: [
          {
            id: 'library-home:loading',
            label: 'Loading Library',
            detail: 'Loading your sources.',
            state: 'loading'
          }
        ]
      }
    }

    if (navigationReadResult.state !== 'ready') {
      return {
        productState: 'unavailable',
        surfaceLabel: 'Library',
        title: 'Library unavailable',
        detail: navigationReadResult.error.message,
        rows: [
          {
            id: 'library-home:unavailable',
            label: 'Offline/unavailable',
            detail: navigationReadResult.error.message,
            state: 'failed'
          }
        ]
      }
    }

    return {
      productState: 'noSources',
      surfaceLabel: 'Library',
      title: 'Library',
      detail: 'Add a source to start building your music library.',
      rows: [
        {
          id: 'library-home:no-sources',
          label: 'Add Source',
          detail: 'Choose where your music lives.',
          state: 'empty',
          action: {
            kind: 'chooseMusicFolder',
            label: 'Add Source'
          }
        }
      ]
    }
  }

  const summaries = sourceBindings.map(([nodeId, binding]) => {
    const sourceReadiness = input.state.sourceReadinessByNodeId?.get(nodeId)
    const sourceId = sourceIdForLibraryHomeRoot(binding)
    const sourceIntegrity =
      sourceId === undefined ? undefined : input.sourceIntegrityBySourceId?.get(sourceId)
    const sourceMaintenance =
      sourceId === undefined ? undefined : input.sourceMaintenanceBySourceId?.get(sourceId)
    const sourceActivity =
      sourceId === undefined ? undefined : input.sourceActivityBySourceId?.get(sourceId)

    return projectLibraryHomeRootReadiness({
      profile,
      ...(sourceReadiness === undefined ? {} : { sourceReadiness }),
      ...(sourceIntegrity === undefined ? {} : { sourceIntegrity }),
      ...(sourceMaintenance === undefined ? {} : { sourceMaintenance }),
      ...(sourceActivity === undefined ? {} : { sourceActivity })
    })
  })
  const dominant = dominantLibraryHomeState(summaries, profile)

  return {
    productState: dominant.productState,
    surfaceLabel: 'Library',
    title: dominant.title,
    detail: dominant.detail,
    rows: libraryHomeRows(summaries)
  }
}

export function projectLibrarySourceReadiness(input: {
  readonly sourceReadiness?: SourceReadiness
  readonly profile?: LibraryBrowseProfile
}): LibrarySourceReadinessProjection {
  const readiness = input.sourceReadiness
  const profile = input.profile ?? 'audio'

  if (readiness === undefined) {
    return {
      productState: 'indexing',
      badge: 'Indexing',
      tone: 'active',
      detail: 'Checking source readiness.'
    }
  }

  switch (readiness.kind) {
    case 'registered':
      return {
        productState: 'needsScan',
        badge: 'Needs scan',
        tone: 'warning',
        detail: 'Scan source to index your music.'
      }
    case 'scanning':
    case 'rescanRunning':
      return {
        productState: 'indexing',
        badge: 'Indexing',
        tone: 'active',
        detail: readiness.detail
      }
    case 'ready':
      return {
        productState: 'ready',
        badge: 'Ready',
        tone: 'ready',
        detail: readiness.detail
      }
    case 'empty':
      return {
        productState: 'emptyCurrentView',
        badge: libraryBrowseEmptyStateBadge(profile),
        tone: 'warning',
        detail: libraryBrowseEmptyStateLabel(profile)
      }
    case 'missing':
      return {
        productState: 'missing',
        badge: 'Missing',
        tone: 'danger',
        detail: readiness.detail
      }
    case 'unavailable':
      return {
        productState: 'unavailable',
        badge: 'Offline/unavailable',
        tone: 'danger',
        detail: readiness.detail
      }
    case 'blocked':
      return {
        productState: 'blocked',
        badge: 'Blocked',
        tone: 'danger',
        detail: readiness.detail
      }
    case 'failed':
      return {
        productState: 'blocked',
        badge: 'Blocked',
        tone: 'danger',
        detail: readiness.detail
      }
  }
}

function projectLibraryHomeRootReadiness(input: {
  readonly profile: LibraryBrowseProfile
  readonly sourceReadiness?: SourceReadiness
  readonly sourceIntegrity?: ReadSourceIntegrityReply
  readonly sourceMaintenance?: ReadSourceMaintenanceReply
  readonly sourceActivity?: ProjectedSourceActivity
}): LibrarySourceReadinessProjection {
  const activitySummary = projectLibraryHomeSourceActivity({
    profile: input.profile,
    ...(input.sourceActivity === undefined ? {} : { sourceActivity: input.sourceActivity })
  })
  if (activitySummary !== undefined) {
    return activitySummary
  }

  const summary = projectLibrarySourceReadiness({
    profile: input.profile,
    ...(input.sourceReadiness === undefined ? {} : { sourceReadiness: input.sourceReadiness })
  })

  if (summary.productState !== 'ready' && summary.productState !== 'emptyCurrentView') {
    return summary
  }

  const backlog = sourceMaintenanceBacklog({
    ...(input.sourceMaintenance === undefined ? {} : { maintenance: input.sourceMaintenance }),
    ...(input.sourceIntegrity === undefined ? {} : { integrity: input.sourceIntegrity })
  })
  if (backlog.total > 0) {
    return {
      productState: 'maintenanceNeeded',
      badge: 'Maintenance needed',
      tone: 'warning',
      detail: `${libraryHomeMaintenancePrefix(backlog.source)}. Run maintenance processes a bounded batch.`
    }
  }

  return summary
}

function projectLibraryHomeSourceActivity(input: {
  readonly profile: LibraryBrowseProfile
  readonly sourceActivity?: ProjectedSourceActivity
}): LibrarySourceReadinessProjection | undefined {
  const activity = input.sourceActivity
  if (activity === undefined) {
    return undefined
  }

  switch (activity.browse.state) {
    case 'missing':
      return {
        productState: 'missing',
        badge: 'Missing',
        tone: 'danger',
        detail: activity.browse.detail ?? 'This source is missing.'
      }
    case 'blocked':
      return {
        productState: 'blocked',
        badge: 'Blocked',
        tone: 'danger',
        detail: activity.browse.detail ?? 'This source is blocked.'
      }
    case 'unavailable':
      return {
        productState: 'unavailable',
        badge: 'Offline/unavailable',
        tone: 'danger',
        detail: activity.browse.detail ?? 'This source is offline or unavailable.'
      }
    case 'indexing':
      return {
        productState: 'indexing',
        badge: 'Scanning source',
        tone: 'active',
        detail: sourceActivityScanSummary(activity) ?? 'Scanning source.'
      }
    case 'needsScan':
    case 'ready':
    case 'empty':
      break
  }

  if (activity.scan.state === 'running') {
    return {
      productState: 'indexing',
      badge: 'Scanning source',
      tone: 'active',
      detail: sourceActivityScanSummary(activity) ?? 'Scanning source.'
    }
  }
  if (activity.scan.state === 'failed' || activity.scan.state === 'blocked') {
    return {
      productState: 'blocked',
      badge: 'Blocked',
      tone: 'danger',
      detail: sourceActivityScanSummary(activity) ?? 'Scan is blocked.'
    }
  }
  if (activity.scan.state === 'cancelled') {
    return {
      productState: 'needsScan',
      badge: 'Needs scan',
      tone: 'warning',
      detail: sourceActivityScanSummary(activity) ?? 'Scan was cancelled.'
    }
  }

  switch (activity.preparation.state) {
    case 'running':
      return {
        productState: 'maintenanceNeeded',
        badge: 'Preparing source',
        tone: 'active',
        detail: sourceActivityPreparationSummary(activity) ?? 'Preparing source.'
      }
    case 'completedWithRemainingWork':
      return {
        productState: 'maintenanceNeeded',
        badge: 'Maintenance completed; pending work remains',
        tone: 'warning',
        detail:
          sourceActivityPreparationSummary(activity) ??
          'Maintenance completed; pending work remains.'
      }
    case 'idle':
      if (sourceActivityBacklogTotal(activity) > 0) {
        return {
          productState: 'maintenanceNeeded',
          badge: 'Maintenance needed',
          tone: 'warning',
          detail:
            sourceActivityPreparationSummary(activity) ??
            'Maintenance needed. Run maintenance processes a bounded batch.'
        }
      }
      break
    case 'failed':
      return {
        productState: 'maintenanceNeeded',
        badge: 'Maintenance needed',
        tone: 'warning',
        detail: sourceActivityPreparationSummary(activity) ?? 'Maintenance needed.'
      }
    case 'unavailable':
      return {
        productState: 'maintenanceNeeded',
        badge: 'Maintenance unavailable',
        tone: 'warning',
        detail: sourceActivityPreparationSummary(activity) ?? 'Maintenance status unavailable.'
      }
    case 'complete':
      break
  }

  switch (activity.browse.state) {
    case 'needsScan':
      return {
        productState: 'needsScan',
        badge: 'Needs scan',
        tone: 'warning',
        detail: activity.browse.detail ?? 'Scan source to index your music.'
      }
    case 'empty':
      return {
        productState: 'emptyCurrentView',
        badge: libraryBrowseEmptyStateBadge(input.profile),
        tone: 'warning',
        detail: libraryBrowseEmptyStateLabel(input.profile)
      }
    case 'ready':
      return {
        productState: 'ready',
        badge: 'Ready',
        tone: 'ready',
        detail: sourceActivityPreparationSummary(activity) ?? 'Ready to browse.'
      }
  }
}

function libraryHomeMaintenancePrefix(
  source: ReturnType<typeof sourceMaintenanceBacklog>['source']
): string {
  switch (source) {
    case 'maintenance':
      return 'Maintenance needed'
    case 'integrityFallback':
      return 'Maintenance needed'
    case 'runResult':
      return 'Maintenance completed; pending work remains'
    case 'unavailable':
      return 'Maintenance status unavailable'
  }
}

export function libraryBrowseEmptyStateLabel(profile: LibraryBrowseProfile): string {
  switch (profile) {
    case 'audio':
      return 'No audio tracks in this view.'
    case 'playable':
      return 'No playable media in this view.'
    case 'allFiles':
      return 'No files in this source inventory view.'
  }
}

function admittedLibraryHomeRootBindings(
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding> | undefined
): readonly (readonly [BrowserTreeNodeId, Extract<RowBinding, { readonly kind: 'source' }>])[] {
  if (bindingsById === undefined) {
    return []
  }

  return [...bindingsById.entries()].flatMap(([nodeId, binding]) =>
    isAdmittedLibraryHomeRootBinding(binding) ? [[nodeId, binding] as const] : []
  )
}

function isAdmittedLibraryHomeRootBinding(
  binding: RowBinding
): binding is Extract<RowBinding, { readonly kind: 'source' }> {
  return (
    binding.kind === 'source' &&
    (binding.target.entryPoint.kind === 'source' ||
      binding.target.entryPoint.kind === 'sourceLocation')
  )
}

function sourceIdForLibraryHomeRoot(
  binding: Extract<RowBinding, { readonly kind: 'source' }>
): string | undefined {
  return binding.target.entryPoint.kind === 'source'
    ? binding.target.entryPoint.sourceId
    : undefined
}

function dominantLibraryHomeState(
  summaries: readonly LibrarySourceReadinessProjection[],
  profile: LibraryBrowseProfile
): Pick<LibraryHomeProjection, 'productState' | 'title' | 'detail'> {
  if (summaries.some((summary) => summary.productState === 'missing')) {
    return {
      productState: 'missing',
      title: 'Library needs attention',
      detail: 'A source is missing. Select it to review status and actions.'
    }
  }

  if (summaries.some((summary) => summary.productState === 'blocked')) {
    return {
      productState: 'blocked',
      title: 'Library needs attention',
      detail: 'A source is blocked. Select it to review status and actions.'
    }
  }

  if (summaries.some((summary) => summary.productState === 'unavailable')) {
    return {
      productState: 'unavailable',
      title: 'Library needs attention',
      detail: 'A source is offline or unavailable. Select it to review status and actions.'
    }
  }

  if (summaries.some((summary) => summary.productState === 'indexing')) {
    return {
      productState: 'indexing',
      title: 'Library indexing',
      detail: 'Still indexing. More tracks may appear as scanning finishes.'
    }
  }

  if (summaries.some((summary) => summary.productState === 'needsScan')) {
    return {
      productState: 'needsScan',
      title: 'Library needs scan',
      detail: 'Choose a source, then run Scan source to index your music.'
    }
  }

  if (summaries.some((summary) => summary.productState === 'maintenanceNeeded')) {
    return {
      productState: 'maintenanceNeeded',
      title: 'Library preparation pending',
      detail: 'Run maintenance processes a bounded batch.'
    }
  }

  if (summaries.every((summary) => summary.productState === 'emptyCurrentView')) {
    const emptyLabel = libraryBrowseEmptyStateLabel(profile)
    return {
      productState: 'emptyCurrentView',
      title: emptyLabel,
      detail: emptyLabel
    }
  }

  return {
    productState: 'ready',
    title: 'Library ready',
    detail: 'Your library is ready. Select a source, folder, or search your music.'
  }
}

function libraryHomeRows(
  summaries: readonly LibrarySourceReadinessProjection[]
): readonly LibraryHomeRow[] {
  const counts = new Map<string, { summary: LibrarySourceReadinessProjection; count: number }>()

  for (const summary of summaries) {
    const prior = counts.get(summary.badge)
    if (prior === undefined) {
      counts.set(summary.badge, { summary, count: 1 })
    } else {
      prior.count += 1
    }
  }

  if (counts.size === 0) {
    return [
      {
        id: 'library-home:choose-source',
        label: 'Choose a source',
        detail: 'Choose a source to browse your music.',
        state: 'empty'
      }
    ]
  }

  return [...counts.values()].map(({ summary, count }) => ({
    id: `library-home:${summary.badge.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-')}`,
    label: summary.badge,
    detail:
      count === 1
        ? sourceSummaryDetail(summary)
        : `${count} sources. ${sourceSummaryDetail(summary)}`,
    state: rowStateForTone(summary.tone)
  }))
}

function sourceSummaryDetail(summary: LibrarySourceReadinessProjection): string {
  switch (summary.productState) {
    case 'ready':
      return 'Ready to browse.'
    case 'indexing':
      return 'Still indexing. More tracks may appear as scanning finishes.'
    case 'needsScan':
      return 'Scan source to index your music.'
    case 'missing':
      return 'This source is missing.'
    case 'blocked':
      return 'This source is blocked.'
    case 'unavailable':
      return 'This source is offline or unavailable.'
    case 'emptyCurrentView':
      return summary.detail
    case 'maintenanceNeeded':
      return summary.detail
    case 'noSources':
    case 'chooseSource':
      return summary.detail
  }
}

export function libraryBrowseEmptyStateBadge(
  profile: LibraryBrowseProfile
): LibrarySourceReadinessBadge {
  switch (profile) {
    case 'audio':
      return 'No audio tracks in this view'
    case 'playable':
      return 'No playable media in this view'
    case 'allFiles':
      return 'No files in this source inventory view'
  }
}

function rowStateForTone(tone: LibraryHomeTone): LibraryHomeRow['state'] {
  switch (tone) {
    case 'active':
      return 'loading'
    case 'danger':
      return 'failed'
    case 'warning':
      return 'unsupported'
    case 'ready':
    case 'muted':
      return 'empty'
  }
}
