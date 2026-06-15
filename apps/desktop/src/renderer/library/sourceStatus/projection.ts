import type {
  ReadSourceActivityReply,
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

import type { LocalRootScanStatus, RemoveSourceStatus } from '../boundary/localRootActions'
import type { RootLifecycleRefreshStatus } from '../runtime/rootLifecycle'
import type { SourceReadiness } from '../runtime/sourceReadiness'
import type { SourceLifecycleRecord } from '../../../shared/library/source/lifecycle'
import type { StatusContext } from './context'
import { projectAddSourceStatusText } from '../addSource/projection'
import {
  libraryBrowseEmptyStateBadge,
  libraryBrowseEmptyStateLabel,
  projectLibrarySourceReadiness
} from '../libraryHome/projection'
import {
  defaultLibraryBrowseProfile,
  type LibraryBrowseProfile
} from '../libraryBrowseProfile/types'
import {
  hasSourceMaintenanceBacklog,
  sourceMaintenanceBacklog,
  type SourceMaintenanceBacklog
} from '../runtime/sourceMaintenanceSummary'
import {
  sourceActivityBacklogCategories,
  sourceActivityBacklogTotal,
  sourceActivityPreparationSummary,
  sourceActivityScanSummary,
  type ProjectedSourceActivity
} from '../runtime/sourceActivity'

export type StatusBadge =
  | 'Ready'
  | 'Scanning source'
  | 'Maintenance running'
  | 'Maintenance completed; pending work remains'
  | 'Maintenance current'
  | 'Indexing'
  | 'Still indexing'
  | 'No audio tracks in this view'
  | 'No playable media in this view'
  | 'No files in this source inventory view'
  | 'Needs scan'
  | 'Maintenance needed'
  | 'Maintenance unavailable'
  | 'Ready to add'
  | 'Restore source'
  | 'Choose a music folder'
  | 'Choose a specific folder'
  | 'Protected location'
  | 'Already added'
  | 'Blocked'
  | 'Missing'
  | 'Offline/unavailable'
  | 'Partial'

export type StatusAction =
  | {
      readonly kind: 'addLocalPath'
      readonly label: 'Add as music source' | 'Add parent as music source' | 'Restore source'
      readonly resolvedPath: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'showSource'
      readonly label: 'Show source'
      readonly sourceId: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'scanSource'
      readonly label: 'Scan source' | 'Rescan source'
      readonly sourceId: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'runMaintenance'
      readonly label: 'Run maintenance'
      readonly sourceId: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'removeSource'
      readonly label: 'Remove source'
      readonly sourceId: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'refreshStatus'
      readonly label: 'Refresh status'
      readonly sourceId: string
      readonly enabled: boolean
      readonly reason?: string
    }

export type ActiveSourceOperation =
  | {
      readonly kind: 'scan'
      readonly scope: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'scan' | 'remove'
      readonly scope: 'global'
    }

export type StatusView = {
  readonly role: StatusContext['kind']
  readonly title: string
  readonly badge?: StatusBadge
  readonly tone: 'ready' | 'active' | 'warning' | 'danger' | 'muted'
  readonly detail?: string
  readonly actions: readonly StatusAction[]
}

export type StatusViewInput = {
  readonly context: StatusContext
  readonly libraryBrowseProfile?: LibraryBrowseProfile
  readonly sourceLifecycle?: SourceLifecycleRecord
  readonly sourceReadiness?: SourceReadiness
  readonly sourceActivity?: ProjectedSourceActivity
  readonly sourceIntegrity?: ReadSourceIntegrityReply
  readonly sourceMaintenance?: ReadSourceMaintenanceReply
  readonly activeSourceOperation?: ActiveSourceOperation
  readonly canAddLocalPath: boolean
  readonly scanStatus: LocalRootScanStatus
  readonly maintenanceRunState?: 'idle' | 'running' | 'completed' | 'failed'
  readonly removeSourceStatus: RemoveSourceStatus
  readonly refreshStatus: RootLifecycleRefreshStatus
  readonly canScan: boolean
  readonly canRemove: boolean
  readonly canRunMaintenance: boolean
  readonly sourcePath?: string
}

export type SourceStatusDiagnosticTrace = {
  readonly sourceId?: string
  readonly sourcePath?: string
  readonly admission?:
    | ReadSourceActivityReply['admission']
    | 'active'
    | 'restorable'
    | 'notAdmitted'
  readonly browse?: ReadSourceActivityReply['browse']
  readonly scan?: ReadSourceActivityReply['scan']
  readonly preparation?: ReadSourceActivityReply['preparation']
  readonly duplicateStatus?: 'active' | 'restorable' | 'none'
  readonly maintenanceSnapshotSource?: ReturnType<typeof sourceMaintenanceBacklog>['source']
  readonly backlogCounts?: SourceMaintenanceBacklog['categories']
  readonly backlogTotal?: number
  readonly provenance?: ReadSourceActivityReply['preparation']['provenance']
  readonly lastRunStatus?: SourceMaintenanceBacklog['lastRunStatus']
}

export function projectStatusView(input: StatusViewInput): StatusView {
  const context = input.context

  switch (context.kind) {
    case 'none':
      return {
        role: 'none',
        title: 'Library contents',
        tone: 'muted',
        actions: []
      }
    case 'localBrowse':
      return localBrowseStatus(input, context)
    case 'registeredSource':
    case 'registeredDirectory':
    case 'registeredFile':
      return registeredStatus(input, context)
    case 'navigation':
      return {
        role: 'navigation',
        title: context.title,
        tone: 'muted',
        actions: [],
        ...(context.detail === undefined ? {} : { detail: context.detail })
      }
    case 'readState':
      return {
        role: 'readState',
        title: context.title,
        tone: 'muted',
        detail: context.detail,
        actions: []
      }
  }
}

function localBrowseStatus(
  input: StatusViewInput,
  context: Extract<StatusContext, { readonly kind: 'localBrowse' }>
): StatusView {
  const admission = context.admission
  const badge = localBrowseBadge(context.localState)

  return {
    role: 'localBrowse',
    title: context.title,
    tone: localBrowseTone(badge),
    detail: localBrowseDetail(context.localState, context.detail),
    actions: localBrowseActions(input, context, admission),
    ...(badge === undefined ? {} : { badge })
  }
}

function localBrowseActions(
  input: StatusViewInput,
  context: Extract<StatusContext, { readonly kind: 'localBrowse' }>,
  admission: Extract<StatusContext, { readonly kind: 'localBrowse' }>['admission']
): readonly StatusAction[] {
  if (context.localState === 'alreadyAdded') {
    return context.matchedSourceId === undefined
      ? []
      : [
          {
            kind: 'showSource',
            label: 'Show source',
            sourceId: context.matchedSourceId,
            enabled: true
          }
        ]
  }

  if (admission === undefined) {
    return []
  }

  return [
    {
      kind: 'addLocalPath',
      label: admission.label,
      resolvedPath: admission.resolvedPath,
      enabled: input.canAddLocalPath,
      ...(input.canAddLocalPath ? {} : { reason: 'A source action is already running.' })
    }
  ]
}

function registeredStatus(
  input: StatusViewInput,
  context: Extract<
    StatusContext,
    { readonly kind: 'registeredSource' | 'registeredDirectory' | 'registeredFile' }
  >
): StatusView {
  const sourceId = context.sourceId
  const badge = registeredBadge(input, sourceId)
  const inheritanceDetail =
    context.kind === 'registeredDirectory'
      ? 'Folder status follows its registered source.'
      : context.kind === 'registeredFile'
        ? 'File status follows its registered source.'
        : undefined

  return {
    role: context.kind,
    title: context.title,
    badge,
    tone: toneForBadge(badge),
    detail: compactDetail(input, inheritanceDetail),
    actions: registeredActions(input, sourceId)
  }
}

function registeredBadge(input: StatusViewInput, sourceId: string): StatusBadge {
  const activityBadge = registeredActivityBadge(input)
  if (activityBadge !== undefined) {
    return activityBadge
  }

  if (
    scanLockForSource(input, sourceId) === 'selectedSource' ||
    input.sourceLifecycle?.scanPhase === 'scanning'
  ) {
    return 'Indexing'
  }

  const availability = input.sourceIntegrity?.sourceAvailability.state
  if (availability === 'missing') {
    return 'Missing'
  }
  if (availability === 'blocked') {
    return 'Blocked'
  }
  if (availability === 'unavailable' || availability === 'notFound') {
    return 'Offline/unavailable'
  }
  if (availability === 'partial') {
    return 'Partial'
  }

  if (input.sourceLifecycle?.accessState === 'missing') {
    return 'Missing'
  }
  if (input.sourceLifecycle?.accessState === 'blocked') {
    return input.sourceLifecycle.accessIssueKind === 'unavailableMount'
      ? 'Offline/unavailable'
      : 'Blocked'
  }
  if (
    input.sourceLifecycle !== undefined &&
    input.sourceLifecycle.sourceClass !== 'internal' &&
    input.sourceLifecycle.mountStatus !== 'mounted' &&
    input.sourceLifecycle.mountStatus !== 'unknown'
  ) {
    return 'Offline/unavailable'
  }

  if (input.sourceLifecycle?.scanPhase === 'blocked') {
    return 'Blocked'
  }
  if (input.sourceLifecycle?.scanPhase === 'failed') {
    return 'Blocked'
  }
  if (input.sourceLifecycle?.scanPhase === 'partial') {
    return 'Partial'
  }

  const sourceReadinessBadge =
    input.sourceReadiness === undefined
      ? undefined
      : projectLibrarySourceReadiness({
          sourceReadiness: input.sourceReadiness,
          profile: input.libraryBrowseProfile ?? defaultLibraryBrowseProfile
        }).badge
  if (
    sourceReadinessBadge === 'Missing' ||
    sourceReadinessBadge === 'Blocked' ||
    sourceReadinessBadge === 'Offline/unavailable' ||
    sourceReadinessBadge === 'Indexing'
  ) {
    return sourceReadinessBadge
  }

  const coverage = input.sourceIntegrity?.coverageIntegrity.state
  if (coverage === 'locationMissing') {
    return 'Missing'
  }
  if (coverage === 'sourceUnavailable') {
    return 'Offline/unavailable'
  }
  if (coverage === 'blocked' || coverage === 'failed') {
    return 'Blocked'
  }
  if (coverage === 'incomplete') {
    return 'Partial'
  }
  if (coverage === 'pending') {
    return 'Needs scan'
  }
  if (coverage === 'scanning') {
    return 'Indexing'
  }

  if (input.maintenanceRunState === 'running' || input.sourceMaintenance?.status === 'running') {
    return 'Maintenance running'
  }
  if (
    input.sourceMaintenance?.status === 'unavailable' ||
    input.sourceMaintenance?.status === 'blocked' ||
    input.sourceMaintenance?.status === 'failed'
  ) {
    return 'Maintenance unavailable'
  }
  if (
    hasSourceMaintenanceBacklog({
      ...(input.sourceMaintenance === undefined ? {} : { maintenance: input.sourceMaintenance }),
      ...(input.sourceIntegrity === undefined ? {} : { integrity: input.sourceIntegrity }),
      ...(input.maintenanceRunState === undefined ? {} : { runState: input.maintenanceRunState })
    })
  ) {
    return 'Maintenance needed'
  }

  if (sourceReadinessBadge !== undefined) {
    return sourceReadinessBadge
  }

  if (
    input.sourceLifecycle !== undefined &&
    input.sourceLifecycle.scanPhase === 'idle' &&
    input.sourceLifecycle.lastSuccessfulScanAtMs === undefined
  ) {
    return 'Needs scan'
  }

  if (coverage === 'complete') {
    return 'Ready'
  }

  return 'Still indexing'
}

function registeredActivityBadge(input: StatusViewInput): StatusBadge | undefined {
  const activity = input.sourceActivity
  if (activity === undefined) {
    return undefined
  }

  if (activity.scan.state === 'running') {
    return 'Scanning source'
  }
  if (activity.scan.state === 'failed' || activity.scan.state === 'blocked') {
    return 'Blocked'
  }
  if (activity.scan.state === 'cancelled') {
    return 'Needs scan'
  }

  switch (activity.browse.state) {
    case 'missing':
      return 'Missing'
    case 'blocked':
      return 'Blocked'
    case 'unavailable':
      return 'Offline/unavailable'
    case 'indexing':
      return 'Scanning source'
    case 'needsScan':
    case 'ready':
    case 'empty':
      break
  }

  switch (activity.preparation.state) {
    case 'running':
      return 'Maintenance running'
    case 'completedWithRemainingWork':
      return 'Maintenance completed; pending work remains'
    case 'idle':
      return sourceActivityBacklogTotal(activity) > 0 ? 'Maintenance needed' : undefined
    case 'failed':
    case 'unavailable':
      return 'Maintenance unavailable'
    case 'complete':
      break
  }

  switch (activity.browse.state) {
    case 'needsScan':
      return 'Needs scan'
    case 'empty':
      return libraryBrowseEmptyStateBadge(input.libraryBrowseProfile ?? defaultLibraryBrowseProfile)
    case 'ready':
      return 'Ready'
  }
}

function compactDetail(input: StatusViewInput, prefix: string | undefined): string {
  const parts = [prefix].filter((part): part is string => part !== undefined)
  const activityDetail = sourceActivityDetail(input)
  const maintenanceDetail = activityDetail ?? maintenanceSummary(input)
  const healthDetail = healthSummary(input.sourceIntegrity, input.sourceLifecycle)

  if (activityDetail !== undefined) {
    parts.push(activityDetail)
  } else if (input.sourceMaintenance?.lastRun?.status === 'failed') {
    parts.push('Last maintenance failed.')
  } else if (
    input.maintenanceRunState === 'running' ||
    input.sourceMaintenance?.status === 'running'
  ) {
    parts.push('Maintenance running for this source.')
  } else if (input.sourceMaintenance?.status === 'unavailable') {
    parts.push('Maintenance is unavailable for this source.')
  } else if (input.sourceMaintenance?.status === 'blocked') {
    parts.push('Maintenance is blocked for this source.')
  } else if (input.sourceMaintenance?.status === 'failed') {
    parts.push('Maintenance status could not be read.')
  } else if (maintenanceDetail !== undefined) {
    parts.push(maintenanceDetail)
  }

  if (healthDetail !== undefined) {
    parts.push(healthDetail)
  }

  if (parts.length > 0) {
    return parts.join(' ')
  }

  if (input.sourceReadiness?.kind === 'empty') {
    return libraryBrowseEmptyStateLabel(input.libraryBrowseProfile ?? defaultLibraryBrowseProfile)
  }

  return input.sourceReadiness?.detail ?? 'Source status is current.'
}

function maintenanceSummary(input: StatusViewInput): string | undefined {
  const maintenance = input.sourceMaintenance
  const backlog = sourceMaintenanceBacklog({
    ...(maintenance === undefined ? {} : { maintenance }),
    ...(input.sourceIntegrity === undefined ? {} : { integrity: input.sourceIntegrity }),
    ...(input.maintenanceRunState === undefined ? {} : { runState: input.maintenanceRunState })
  })
  if (backlog.total > 0) {
    const categories = backlog.categories
      .map((category) => `${category.label} ${category.count}`)
      .join(', ')
    const prefix = maintenanceBacklogPrefix(backlog)
    const batchDetail = 'Run maintenance processes a bounded batch.'

    if (categories.length > 0) {
      return `${prefix}: ${categories}. ${batchDetail}`
    }

    return `${prefix}: analysis and identity work. ${batchDetail}`
  }

  return undefined
}

function sourceActivityDetail(input: StatusViewInput): string | undefined {
  const activity = input.sourceActivity
  if (activity === undefined) {
    return undefined
  }

  if (
    activity.scan.state === 'running' ||
    activity.scan.state === 'failed' ||
    activity.scan.state === 'blocked' ||
    activity.scan.state === 'cancelled'
  ) {
    return sourceActivityScanSummary(activity)
  }

  const preparation = sourceActivityPreparationSummary(activity)
  if (preparation !== undefined) {
    return preparation
  }

  return activity.browse.detail
}

function maintenanceBacklogPrefix(backlog: SourceMaintenanceBacklog): string {
  switch (backlog.source) {
    case 'runResult':
      return 'Maintenance completed; pending work remains'
    case 'integrityFallback':
      return 'Maintenance needed'
    case 'maintenance':
      return 'Maintenance needed'
    case 'unavailable':
      return 'Maintenance status unavailable'
  }
}

function healthSummary(
  integrity: ReadSourceIntegrityReply | undefined,
  lifecycle: SourceLifecycleRecord | undefined
): string | undefined {
  if (integrity?.coverageIntegrity.missingDirectoriesCount !== undefined) {
    const missing = integrity.coverageIntegrity.missingDirectoriesCount
    if (missing > 0) {
      return `${missing} ${missing === 1 ? 'folder is' : 'folders are'} missing.`
    }
  }

  if (integrity?.coverageIntegrity.blockedDirectoriesCount !== undefined) {
    const blocked = integrity.coverageIntegrity.blockedDirectoriesCount
    if (blocked > 0) {
      return `${blocked} blocked ${blocked === 1 ? 'folder' : 'folders'}.`
    }
  }

  if (lifecycle?.scanPhase === 'idle' && lifecycle.lastSuccessfulScanAtMs === undefined) {
    return 'Source status is stale until scan completes.'
  }

  return undefined
}

function registeredActions(input: StatusViewInput, sourceId: string): readonly StatusAction[] {
  return [
    scanAction(input, sourceId),
    maintenanceAction(input, sourceId),
    refreshAction(input, sourceId),
    removeAction(input, sourceId)
  ].filter((action): action is StatusAction => action !== undefined)
}

function scanAction(input: StatusViewInput, sourceId: string): StatusAction | undefined {
  if (!input.canScan) {
    return undefined
  }

  const scanLock = scanLockForSource(input, sourceId)
  const label: 'Scan source' | 'Rescan source' =
    input.sourceLifecycle?.lastSuccessfulScanAtMs === undefined ? 'Scan source' : 'Rescan source'

  return {
    kind: 'scanSource',
    label,
    sourceId,
    enabled: scanLock === 'none' && input.refreshStatus !== 'refreshing',
    ...(scanLock === 'selectedSource'
      ? { reason: 'A source scan is still running.' }
      : scanLock === 'anotherSource'
        ? { reason: 'Another source scan is running.' }
        : scanLock === 'unknownSource'
          ? { reason: 'A source scan is already running.' }
          : input.refreshStatus === 'refreshing'
            ? { reason: 'The library view is refreshing.' }
            : {})
  }
}

function maintenanceAction(input: StatusViewInput, sourceId: string): StatusAction | undefined {
  if (!input.canRunMaintenance) {
    return undefined
  }

  const scanLock = scanLockForSource(input, sourceId)
  const maintenanceRunning =
    input.maintenanceRunState === 'running' || input.sourceMaintenance?.status === 'running'
  const maintenanceUnavailable =
    input.sourceMaintenance?.status === 'unavailable' ||
    input.sourceMaintenance?.status === 'blocked' ||
    input.sourceMaintenance?.status === 'failed'

  return {
    kind: 'runMaintenance',
    label: 'Run maintenance',
    sourceId,
    enabled: scanLock !== 'selectedSource' && !maintenanceRunning && !maintenanceUnavailable,
    ...(scanLock === 'selectedSource'
      ? { reason: 'Wait for scan to finish.' }
      : maintenanceRunning
        ? { reason: 'Maintenance is running.' }
        : maintenanceUnavailable
          ? { reason: maintenanceUnavailableReason(input.sourceMaintenance?.status) }
          : {})
  }
}

function removeAction(input: StatusViewInput, sourceId: string): StatusAction | undefined {
  if (!input.canRemove) {
    return undefined
  }

  const removing = input.removeSourceStatus === 'removing'
  const scanLock = scanLockForSource(input, sourceId)

  return {
    kind: 'removeSource',
    label: 'Remove source',
    sourceId,
    enabled: !removing && scanLock === 'none' && input.refreshStatus !== 'refreshing',
    ...(removing
      ? { reason: 'A source removal is already in progress.' }
      : scanLock === 'selectedSource'
        ? { reason: 'A source scan is still running.' }
        : scanLock === 'anotherSource'
          ? { reason: 'Another source scan is running.' }
          : scanLock === 'unknownSource'
            ? { reason: 'A source scan is already running.' }
            : input.refreshStatus === 'refreshing'
              ? { reason: 'The library view is refreshing.' }
              : {})
  }
}

function refreshAction(input: StatusViewInput, sourceId: string): StatusAction {
  return {
    kind: 'refreshStatus',
    label: 'Refresh status',
    sourceId,
    enabled: input.refreshStatus !== 'refreshing',
    ...(input.refreshStatus === 'refreshing' ? { reason: 'The library view is refreshing.' } : {})
  }
}

function toneForBadge(badge: StatusBadge): StatusView['tone'] {
  switch (badge) {
    case 'Ready':
    case 'Maintenance current':
      return 'ready'
    case 'Scanning source':
    case 'Maintenance running':
    case 'Indexing':
    case 'Still indexing':
      return 'active'
    case 'Needs scan':
    case 'Maintenance needed':
    case 'Maintenance unavailable':
    case 'Maintenance completed; pending work remains':
    case 'Ready to add':
    case 'Restore source':
    case 'Choose a music folder':
    case 'Choose a specific folder':
    case 'Protected location':
    case 'Already added':
    case 'Partial':
    case 'No audio tracks in this view':
    case 'No playable media in this view':
    case 'No files in this source inventory view':
      return 'warning'
    case 'Blocked':
    case 'Missing':
    case 'Offline/unavailable':
      return 'danger'
  }
}

export function sourceStatusDiagnosticTrace(input: StatusViewInput): SourceStatusDiagnosticTrace {
  const backlog = sourceMaintenanceBacklog({
    ...(input.sourceMaintenance === undefined ? {} : { maintenance: input.sourceMaintenance }),
    ...(input.sourceIntegrity === undefined ? {} : { integrity: input.sourceIntegrity }),
    ...(input.maintenanceRunState === undefined ? {} : { runState: input.maintenanceRunState })
  })
  const context = input.context
  const sourceId =
    'sourceId' in context
      ? context.sourceId
      : context.kind === 'localBrowse'
        ? context.matchedSourceId
        : undefined
  const duplicateStatus =
    context.kind === 'localBrowse'
      ? context.localState === 'alreadyAdded'
        ? 'active'
        : context.localState === 'restorable'
          ? 'restorable'
          : 'none'
      : undefined
  const activity = input.sourceActivity
  const activityBacklogCounts =
    activity === undefined ? undefined : sourceActivityBacklogCategories(activity)
  const activityBacklogTotal =
    activity === undefined ? undefined : sourceActivityBacklogTotal(activity)
  const admission =
    activity?.admission ??
    (duplicateStatus === 'active'
      ? 'active'
      : duplicateStatus === 'restorable'
        ? 'restorable'
        : sourceId === undefined
          ? undefined
          : 'notAdmitted')
  const provenance =
    activity?.preparation.provenance ?? diagnosticProvenanceFromBacklog(backlog.source)

  return {
    ...(sourceId === undefined ? {} : { sourceId }),
    ...(input.sourcePath === undefined ? {} : { sourcePath: input.sourcePath }),
    ...(admission === undefined ? {} : { admission }),
    ...(activity === undefined ? {} : { browse: activity.browse }),
    ...(activity === undefined ? {} : { scan: activity.scan }),
    ...(activity === undefined ? {} : { preparation: activity.preparation }),
    ...(duplicateStatus === undefined ? {} : { duplicateStatus }),
    maintenanceSnapshotSource: backlog.source,
    backlogCounts: activityBacklogCounts ?? backlog.categories,
    backlogTotal: activityBacklogTotal ?? backlog.total,
    provenance,
    ...(activity?.preparation.lastRunStatus === undefined && backlog.lastRunStatus === undefined
      ? {}
      : { lastRunStatus: activity?.preparation.lastRunStatus ?? backlog.lastRunStatus })
  }
}

function diagnosticProvenanceFromBacklog(
  source: SourceMaintenanceBacklog['source']
): ReadSourceActivityReply['preparation']['provenance'] {
  switch (source) {
    case 'maintenance':
      return 'maintenanceSnapshot'
    case 'runResult':
      return 'runResult'
    case 'integrityFallback':
      return 'integrityFallback'
    case 'unavailable':
      return 'unavailable'
  }
}

function localBrowseBadge(
  state: Extract<StatusContext, { readonly kind: 'localBrowse' }>['localState']
): StatusBadge | undefined {
  return projectAddSourceStatusText({ localState: state }).badge
}

function localBrowseTone(badge: StatusBadge | undefined): StatusView['tone'] {
  switch (badge) {
    case undefined:
    case 'Ready to add':
    case 'Restore source':
    case 'Choose a music folder':
      return 'muted'
    case 'Already added':
      return 'ready'
    case 'Protected location':
    case 'Blocked':
      return 'danger'
    default:
      return 'warning'
  }
}

function localBrowseDetail(
  state: Extract<StatusContext, { readonly kind: 'localBrowse' }>['localState'],
  detail: string | undefined
): string {
  return projectAddSourceStatusText({
    localState: state,
    ...(detail === undefined ? {} : { detail })
  }).detail
}

function maintenanceUnavailableReason(
  status: ReadSourceMaintenanceReply['status'] | undefined
): string {
  switch (status) {
    case 'unavailable':
      return 'Maintenance is unavailable for this source.'
    case 'blocked':
      return 'Maintenance is blocked for this source.'
    case 'failed':
      return 'Maintenance status could not be read.'
    default:
      return 'Maintenance is unavailable.'
  }
}

type SourceScanLock = 'none' | 'selectedSource' | 'anotherSource' | 'unknownSource'

function scanLockForSource(input: StatusViewInput, sourceId: string): SourceScanLock {
  const operation = input.activeSourceOperation

  if (operation?.kind === 'scan') {
    if (operation.scope === 'source') {
      return operation.sourceId === sourceId ? 'selectedSource' : 'anotherSource'
    }

    return 'unknownSource'
  }

  return input.scanStatus === 'scanning' ? 'selectedSource' : 'none'
}
