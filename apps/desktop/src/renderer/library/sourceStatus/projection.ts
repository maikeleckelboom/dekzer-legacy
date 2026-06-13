import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

import type { LocalRootScanStatus, RemoveSourceStatus } from '../boundary/localRootActions'
import type { RootLifecycleRefreshStatus } from '../runtime/rootLifecycle'
import type { SourceReadiness } from '../runtime/sourceReadiness'
import type { SourceLifecycleRecord } from '../../../shared/library/source/lifecycle'
import type { StatusContext } from './context'

export type StatusBadge =
  | 'Ready'
  | 'Scanning'
  | 'Needs scan'
  | 'Maintenance needed'
  | 'Blocked'
  | 'Missing'
  | 'Offline/unavailable'
  | 'Partial'
  | 'Unknown'

export type StatusAction =
  | {
      readonly kind: 'addLocalPath'
      readonly label: 'Add this folder' | 'Add parent folder'
      readonly resolvedPath: string
      readonly enabled: boolean
      readonly reason?: string
    }
  | {
      readonly kind: 'scanSource'
      readonly label: 'Scan' | 'Rescan'
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

export type StatusView = {
  readonly kind: StatusContext['kind']
  readonly title: string
  readonly badge?: StatusBadge
  readonly badgeTone: 'ready' | 'active' | 'warning' | 'danger' | 'muted'
  readonly detail?: string
  readonly actions: readonly StatusAction[]
}

export type StatusViewInput = {
  readonly context: StatusContext
  readonly sourceLifecycle?: SourceLifecycleRecord
  readonly sourceReadiness?: SourceReadiness
  readonly sourceIntegrity?: ReadSourceIntegrityReply
  readonly sourceMaintenance?: ReadSourceMaintenanceReply
  readonly localAddEnabled: boolean
  readonly scanStatus: LocalRootScanStatus
  readonly maintenanceRunState?: 'idle' | 'running' | 'completed' | 'failed'
  readonly removeSourceStatus: RemoveSourceStatus
  readonly refreshStatus: RootLifecycleRefreshStatus
  readonly scanSupported: boolean
  readonly removeSupported: boolean
  readonly maintenanceSupported: boolean
}

export function projectStatusView(input: StatusViewInput): StatusView {
  const context = input.context

  switch (context.kind) {
    case 'none':
      return {
        kind: 'none',
        title: 'Library contents',
        badgeTone: 'muted',
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
        kind: 'navigation',
        title: context.title,
        badge: 'Unknown',
        badgeTone: 'muted',
        detail: context.detail ?? 'Navigation row.',
        actions: []
      }
    case 'readState':
      return {
        kind: 'readState',
        title: context.title,
        badge: 'Unknown',
        badgeTone: 'muted',
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

  return {
    kind: 'localBrowse',
    title: context.title,
    badge: 'Unknown',
    badgeTone: 'muted',
    detail:
      admission === undefined
        ? 'Local browse only. Add a folder before scan or maintenance.'
        : 'Local browse admission available.',
    actions:
      admission === undefined
        ? []
        : [
            {
              kind: 'addLocalPath',
              label: admission.label,
              resolvedPath: admission.resolvedPath,
              enabled: input.localAddEnabled,
              ...(input.localAddEnabled ? {} : { reason: 'A source action is already running.' })
            }
          ]
  }
}

function registeredStatus(
  input: StatusViewInput,
  context: Extract<
    StatusContext,
    { readonly kind: 'registeredSource' | 'registeredDirectory' | 'registeredFile' }
  >
): StatusView {
  const badge = registeredBadge(input)
  const sourceId = context.sourceId
  const sourceScopedDetail =
    context.kind === 'registeredDirectory'
      ? 'Folder status follows its registered source.'
      : context.kind === 'registeredFile'
        ? 'File status follows its registered source.'
        : undefined

  return {
    kind: context.kind,
    title: context.title,
    badge,
    badgeTone: badgeTone(badge),
    detail: compactDetail(input, sourceScopedDetail),
    actions: registeredActions(input, sourceId)
  }
}

function registeredBadge(input: StatusViewInput): StatusBadge {
  if (input.scanStatus === 'scanning' || input.sourceLifecycle?.scanPhase === 'scanning') {
    return 'Scanning'
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
    return 'Scanning'
  }

  if (input.maintenanceRunState === 'running' || input.sourceMaintenance?.status === 'running') {
    return 'Maintenance needed'
  }
  if (hasMaintenanceBacklog(input.sourceMaintenance, input.sourceIntegrity)) {
    return 'Maintenance needed'
  }

  if (
    input.sourceLifecycle !== undefined &&
    input.sourceLifecycle.scanPhase === 'idle' &&
    input.sourceLifecycle.lastSuccessfulScanAtMs === undefined
  ) {
    return 'Needs scan'
  }

  if (input.sourceReadiness?.kind === 'ready' || coverage === 'complete') {
    return 'Ready'
  }

  return 'Unknown'
}

function compactDetail(input: StatusViewInput, prefix: string | undefined): string {
  const parts = [prefix].filter((part): part is string => part !== undefined)
  const maintenanceDetail = maintenanceSummary(input.sourceMaintenance, input.sourceIntegrity)
  const healthDetail = healthSummary(input.sourceIntegrity, input.sourceLifecycle)

  if (input.sourceMaintenance?.lastRun?.status === 'failed') {
    parts.push('Last maintenance failed.')
  } else if (
    input.maintenanceRunState === 'running' ||
    input.sourceMaintenance?.status === 'running'
  ) {
    parts.push('Maintenance is running.')
  } else if (maintenanceDetail !== undefined) {
    parts.push(maintenanceDetail)
  }

  if (healthDetail !== undefined) {
    parts.push(healthDetail)
  }

  if (parts.length > 0) {
    return parts.join(' ')
  }

  return input.sourceReadiness?.detail ?? 'Source status is current.'
}

function maintenanceSummary(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): string | undefined {
  const remaining = maintenanceRemaining(maintenance, integrity)
  if (remaining > 0) {
    return `${remaining} maintenance ${remaining === 1 ? 'item' : 'items'} pending.`
  }

  const staleLinks =
    maintenance?.attachmentLinks?.staleLinksCount ??
    integrity?.attachmentIntegrity?.staleLinksCount ??
    0
  const missingLinks =
    maintenance?.attachmentLinks?.sourceFilesMissingAttachmentLinksCount ??
    integrity?.attachmentIntegrity?.missingLinksCount ??
    0
  if (staleLinks + missingLinks > 0) {
    return 'Attachment integrity needs maintenance.'
  }

  return undefined
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
  if (!input.scanSupported) {
    return undefined
  }

  const scanRunning = input.scanStatus === 'scanning'
  const label: 'Scan' | 'Rescan' =
    input.sourceLifecycle?.lastSuccessfulScanAtMs === undefined ? 'Scan' : 'Rescan'

  return {
    kind: 'scanSource',
    label,
    sourceId,
    enabled: !scanRunning && input.refreshStatus !== 'refreshing',
    ...(scanRunning
      ? { reason: 'A scan is running.' }
      : input.refreshStatus === 'refreshing'
        ? { reason: 'The library view is refreshing.' }
        : {})
  }
}

function maintenanceAction(input: StatusViewInput, sourceId: string): StatusAction | undefined {
  if (!input.maintenanceSupported) {
    return undefined
  }

  const scanRunning = input.scanStatus === 'scanning'
  const maintenanceRunning =
    input.maintenanceRunState === 'running' || input.sourceMaintenance?.status === 'running'

  return {
    kind: 'runMaintenance',
    label: 'Run maintenance',
    sourceId,
    enabled: !scanRunning && !maintenanceRunning,
    ...(scanRunning
      ? { reason: 'Wait for scan to finish.' }
      : maintenanceRunning
        ? { reason: 'Maintenance is running.' }
        : {})
  }
}

function removeAction(input: StatusViewInput, sourceId: string): StatusAction | undefined {
  if (!input.removeSupported) {
    return undefined
  }

  const removing = input.removeSourceStatus === 'removing'
  const scanRunning = input.scanStatus === 'scanning'

  return {
    kind: 'removeSource',
    label: 'Remove source',
    sourceId,
    enabled: !removing && !scanRunning && input.refreshStatus !== 'refreshing',
    ...(removing
      ? { reason: 'A source removal is already in progress.' }
      : scanRunning
        ? { reason: 'A source scan is still running.' }
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

function badgeTone(badge: StatusBadge): StatusView['badgeTone'] {
  switch (badge) {
    case 'Ready':
      return 'ready'
    case 'Scanning':
      return 'active'
    case 'Needs scan':
    case 'Maintenance needed':
    case 'Partial':
    case 'Unknown':
      return 'warning'
    case 'Blocked':
    case 'Missing':
    case 'Offline/unavailable':
      return 'danger'
  }
}

function hasMaintenanceBacklog(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): boolean {
  return maintenanceRemaining(maintenance, integrity) > 0
}

function maintenanceRemaining(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): number {
  return (
    (maintenance?.remainingHashCandidates ??
      integrity?.evidenceAndMaintenance.remainingHashCandidates ??
      0) +
    (maintenance?.remainingProbeCandidates ??
      integrity?.evidenceAndMaintenance.remainingProbeCandidates ??
      0) +
    (maintenance?.remainingPlayableMediaPromotionCandidates ??
      integrity?.evidenceAndMaintenance.remainingPlayableMediaPromotionCandidates ??
      0) +
    (maintenance?.remainingTrackIdentityCandidateProductionCandidates ??
      integrity?.evidenceAndMaintenance.remainingTrackIdentityCandidateProductionCandidates ??
      0) +
    (maintenance?.remainingTrackIdentityDecisionProductionCandidates ??
      integrity?.evidenceAndMaintenance.remainingTrackIdentityDecisionProductionCandidates ??
      0)
  )
}
