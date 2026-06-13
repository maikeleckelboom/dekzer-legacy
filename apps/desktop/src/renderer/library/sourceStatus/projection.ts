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
  | 'Still indexing'
  | 'No audio tracks'
  | 'Needs scan'
  | 'Maintenance needed'
  | 'Maintenance running'
  | 'Maintenance unavailable'
  | 'Not a music-source candidate'
  | 'Not in library yet'
  | 'Choose a narrower folder'
  | 'Protected location'
  | 'Already added'
  | 'Could not fully resolve this location'
  | 'Blocked'
  | 'Missing'
  | 'Offline/unavailable'
  | 'Partial'

export type StatusAction =
  | {
      readonly kind: 'addLocalPath'
      readonly label: 'Add as music source' | 'Add parent as music source'
      readonly resolvedPath: string
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
  readonly sourceLifecycle?: SourceLifecycleRecord
  readonly sourceReadiness?: SourceReadiness
  readonly sourceIntegrity?: ReadSourceIntegrityReply
  readonly sourceMaintenance?: ReadSourceMaintenanceReply
  readonly canAddLocalPath: boolean
  readonly scanStatus: LocalRootScanStatus
  readonly maintenanceRunState?: 'idle' | 'running' | 'completed' | 'failed'
  readonly removeSourceStatus: RemoveSourceStatus
  readonly refreshStatus: RootLifecycleRefreshStatus
  readonly canScan: boolean
  readonly canRemove: boolean
  readonly canRunMaintenance: boolean
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
    badge,
    tone: localBrowseTone(badge),
    detail: localBrowseDetail(context.localState, context.detail),
    actions:
      admission === undefined
        ? []
        : [
            {
              kind: 'addLocalPath',
              label: admission.label,
              resolvedPath: admission.resolvedPath,
              enabled: input.canAddLocalPath,
              ...(input.canAddLocalPath ? {} : { reason: 'A source action is already running.' })
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
    return 'Maintenance running'
  }
  if (
    input.sourceMaintenance?.status === 'unavailable' ||
    input.sourceMaintenance?.status === 'blocked' ||
    input.sourceMaintenance?.status === 'failed'
  ) {
    return 'Maintenance unavailable'
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

  if (input.sourceReadiness?.kind === 'empty') {
    return 'No audio tracks'
  }

  if (input.sourceReadiness?.kind === 'ready' || coverage === 'complete') {
    return 'Ready'
  }

  return 'Still indexing'
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
    parts.push('Maintenance is running for this source.')
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
    return 'No audio tracks found in this view.'
  }

  return input.sourceReadiness?.detail ?? 'Source status is current.'
}

function maintenanceSummary(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): string | undefined {
  const backlog = maintenanceBacklog(maintenance, integrity)
  if (backlog.total > 0) {
    const categories = backlog.categories
      .map((category) => `${category.label} ${category.count}`)
      .join(', ')

    if (categories.length > 0) {
      return `Preparing source. Pending work: ${categories}. ${backlog.total} ${
        backlog.total === 1 ? 'item' : 'items'
      } total.`
    }

    return `Preparing source. Analysis and identity work is pending. ${backlog.total} ${
      backlog.total === 1 ? 'item' : 'items'
    } total.`
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
  if (!input.canScan) {
    return undefined
  }

  const scanRunning = input.scanStatus === 'scanning'
  const label: 'Scan source' | 'Rescan source' =
    input.sourceLifecycle?.lastSuccessfulScanAtMs === undefined ? 'Scan source' : 'Rescan source'

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
  if (!input.canRunMaintenance) {
    return undefined
  }

  const scanRunning = input.scanStatus === 'scanning'
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
    enabled: !scanRunning && !maintenanceRunning && !maintenanceUnavailable,
    ...(scanRunning
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

function toneForBadge(badge: StatusBadge): StatusView['tone'] {
  switch (badge) {
    case 'Ready':
      return 'ready'
    case 'Scanning':
    case 'Still indexing':
      return 'active'
    case 'Needs scan':
    case 'Maintenance needed':
    case 'Maintenance unavailable':
    case 'Not a music-source candidate':
    case 'Not in library yet':
    case 'Choose a narrower folder':
    case 'Protected location':
    case 'Already added':
    case 'Could not fully resolve this location':
    case 'Partial':
    case 'No audio tracks':
      return 'warning'
    case 'Maintenance running':
      return 'active'
    case 'Blocked':
    case 'Missing':
    case 'Offline/unavailable':
      return 'danger'
  }
}

function localBrowseBadge(
  state: Extract<StatusContext, { readonly kind: 'localBrowse' }>['localState']
): StatusBadge {
  switch (state) {
    case 'eligible':
      return 'Not in library yet'
    case 'broadRoot':
      return 'Choose a narrower folder'
    case 'protected':
      return 'Protected location'
    case 'resolutionFailed':
      return 'Could not fully resolve this location'
    case 'alreadyAdded':
      return 'Already added'
    case 'localBrowseOnly':
      return 'Not a music-source candidate'
  }
}

function localBrowseTone(badge: StatusBadge): StatusView['tone'] {
  switch (badge) {
    case 'Not in library yet':
    case 'Not a music-source candidate':
      return 'muted'
    case 'Already added':
      return 'ready'
    case 'Protected location':
    case 'Could not fully resolve this location':
      return 'danger'
    default:
      return 'warning'
  }
}

function localBrowseDetail(
  state: Extract<StatusContext, { readonly kind: 'localBrowse' }>['localState'],
  detail: string | undefined
): string {
  switch (state) {
    case 'eligible':
      return 'Not in library yet.'
    case 'broadRoot':
      return 'Choose a narrower folder before adding it as a music source.'
    case 'protected':
      return 'Protected location.'
    case 'resolutionFailed':
      return detail ?? 'Could not fully resolve this location.'
    case 'alreadyAdded':
      return 'Already added as a library source. Use the managed source entry for scans and maintenance.'
    case 'localBrowseOnly':
      return 'Not a music-source candidate.'
  }
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

function hasMaintenanceBacklog(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): boolean {
  return maintenanceBacklog(maintenance, integrity).total > 0
}

function maintenanceBacklog(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): {
  readonly total: number
  readonly categories: readonly { readonly label: string; readonly count: number }[]
} {
  const hash =
    maintenance?.remainingHashCandidates ??
    integrity?.evidenceAndMaintenance.remainingHashCandidates ??
    0
  const probe =
    maintenance?.remainingProbeCandidates ??
    integrity?.evidenceAndMaintenance.remainingProbeCandidates ??
    0
  const promotion =
    maintenance?.remainingPlayableMediaPromotionCandidates ??
    integrity?.evidenceAndMaintenance.remainingPlayableMediaPromotionCandidates ??
    0
  const identity =
    (maintenance?.remainingTrackIdentityCandidateProductionCandidates ??
      integrity?.evidenceAndMaintenance.remainingTrackIdentityCandidateProductionCandidates ??
      0) +
    (maintenance?.remainingTrackIdentityDecisionProductionCandidates ??
      integrity?.evidenceAndMaintenance.remainingTrackIdentityDecisionProductionCandidates ??
      0)
  const attachment = maintenanceAttachmentRemaining(maintenance, integrity)

  const categories = [
    { label: 'hash', count: hash },
    { label: 'probe', count: probe },
    { label: 'promotion', count: promotion },
    { label: 'identity', count: identity },
    { label: 'attachment', count: attachment }
  ].filter((category) => category.count > 0)

  return {
    total: hash + probe + promotion + identity + attachment,
    categories
  }
}

function maintenanceAttachmentRemaining(
  maintenance: ReadSourceMaintenanceReply | undefined,
  integrity: ReadSourceIntegrityReply | undefined
): number {
  const staleLinks =
    maintenance?.attachmentLinks?.staleLinksCount ??
    integrity?.attachmentIntegrity?.staleLinksCount ??
    0
  const missingLinks =
    maintenance?.attachmentLinks?.sourceFilesMissingAttachmentLinksCount ??
    integrity?.attachmentIntegrity?.missingLinksCount ??
    0

  return staleLinks + missingLinks
}
