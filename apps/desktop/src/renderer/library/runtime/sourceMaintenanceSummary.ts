import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

export type SourceMaintenanceBacklog = {
  readonly source: SourceMaintenanceBacklogSource
  readonly total: number
  readonly categories: readonly { readonly label: string; readonly count: number }[]
  readonly lastRunStatus?: NonNullable<ReadSourceMaintenanceReply['lastRun']>['status']
}

export type SourceMaintenanceBacklogSource =
  | 'maintenance'
  | 'integrityFallback'
  | 'runResult'
  | 'unavailable'

export function hasSourceMaintenanceBacklog(input: {
  readonly maintenance?: ReadSourceMaintenanceReply
  readonly integrity?: ReadSourceIntegrityReply
  readonly runState?: 'idle' | 'running' | 'completed' | 'failed'
}): boolean {
  return sourceMaintenanceBacklog(input).total > 0
}

export function sourceMaintenanceBacklog(input: {
  readonly maintenance?: ReadSourceMaintenanceReply
  readonly integrity?: ReadSourceIntegrityReply
  readonly runState?: 'idle' | 'running' | 'completed' | 'failed'
}): SourceMaintenanceBacklog {
  const maintenance = input.maintenance
  const integrity = input.integrity
  const source = maintenanceSource(input)
  const hash =
    maintenance === undefined
      ? (integrity?.evidenceAndMaintenance.remainingHashCandidates ?? 0)
      : maintenance.remainingHashCandidates
  const probe =
    maintenance === undefined
      ? (integrity?.evidenceAndMaintenance.remainingProbeCandidates ?? 0)
      : maintenance.remainingProbeCandidates
  const promotion =
    maintenance === undefined
      ? (integrity?.evidenceAndMaintenance.remainingPlayableMediaPromotionCandidates ?? 0)
      : maintenance.remainingPlayableMediaPromotionCandidates
  const identity =
    maintenance === undefined
      ? (integrity?.evidenceAndMaintenance.remainingTrackIdentityCandidateProductionCandidates ??
          0) +
        (integrity?.evidenceAndMaintenance.remainingTrackIdentityDecisionProductionCandidates ?? 0)
      : maintenance.remainingTrackIdentityCandidateProductionCandidates +
        maintenance.remainingTrackIdentityDecisionProductionCandidates
  const attachment =
    maintenance === undefined
      ? integrityAttachmentRemaining(integrity)
      : maintenanceAttachmentRemaining(maintenance)

  const categories = [
    { label: 'hash', count: hash },
    { label: 'probe', count: probe },
    { label: 'promotion', count: promotion },
    { label: 'identity', count: identity },
    { label: 'attachment', count: attachment }
  ].filter((category) => category.count > 0)

  return {
    source,
    total: hash + probe + promotion + identity + attachment,
    categories,
    ...(maintenance?.lastRun?.status === undefined
      ? {}
      : { lastRunStatus: maintenance.lastRun.status })
  }
}

function maintenanceSource(input: {
  readonly maintenance?: ReadSourceMaintenanceReply
  readonly integrity?: ReadSourceIntegrityReply
  readonly runState?: 'idle' | 'running' | 'completed' | 'failed'
}): SourceMaintenanceBacklogSource {
  if (input.maintenance !== undefined) {
    return input.runState === 'completed' ? 'runResult' : 'maintenance'
  }

  if (input.integrity !== undefined) {
    return 'integrityFallback'
  }

  return 'unavailable'
}

function maintenanceAttachmentRemaining(maintenance: ReadSourceMaintenanceReply): number {
  const staleLinks = maintenance.attachmentLinks?.staleLinksCount ?? 0
  const missingLinks = maintenance.attachmentLinks?.sourceFilesMissingAttachmentLinksCount ?? 0

  return staleLinks + missingLinks
}

function integrityAttachmentRemaining(integrity: ReadSourceIntegrityReply | undefined): number {
  const staleLinks = integrity?.attachmentIntegrity?.staleLinksCount ?? 0
  const missingLinks = integrity?.attachmentIntegrity?.missingLinksCount ?? 0

  return staleLinks + missingLinks
}
