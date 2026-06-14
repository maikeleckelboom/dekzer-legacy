import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

export type SourceMaintenanceBacklog = {
  readonly total: number
  readonly categories: readonly { readonly label: string; readonly count: number }[]
}

export function hasSourceMaintenanceBacklog(input: {
  readonly maintenance?: ReadSourceMaintenanceReply
  readonly integrity?: ReadSourceIntegrityReply
}): boolean {
  return sourceMaintenanceBacklog(input).total > 0
}

export function sourceMaintenanceBacklog(input: {
  readonly maintenance?: ReadSourceMaintenanceReply
  readonly integrity?: ReadSourceIntegrityReply
}): SourceMaintenanceBacklog {
  const maintenance = input.maintenance
  const integrity = input.integrity
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
    total: hash + probe + promotion + identity + attachment,
    categories
  }
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
