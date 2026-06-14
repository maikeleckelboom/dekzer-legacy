import { libraryPublicationChannels } from '../../../src/shared/library/boundary/publicationPlane'
import { libraryControlChannels } from '../../../src/shared/library/boundary/controlPlane'
import { describe, expect, it } from 'vitest'

import { createRendererApi, exposeRendererApi } from '../../../src/preload/rendererApi'
import { type LibraryBoundaryHostStatus } from '../../../src/shared/library/boundary/status'
import {
  type LibraryViewStateReadResult,
  type LibraryViewStateWriteResult,
  type PersistedLibraryViewState
} from '../../../src/shared/library/viewState/persistence'
import { type ReadResult } from '../../../src/shared/library/hierarchy/read'
import { type NavigationReadRowsResult } from '../../../src/shared/library/navigation/read'

import type { LocalRootChoiceResult } from '../../../src/shared/library/roots/chooseLocal'
import type { ReadLocalRootsOutcome } from '../../../src/shared/library/roots/read'
import type { LocalRootScanResult } from '../../../src/shared/library/roots/scan'
import type { CancelRootScanResult } from '../../../src/shared/library/roots/cancel'
import type { UnregisterLocalRootResult } from '../../../src/shared/library/roots/unregister'
import { type ContentsReadResult } from '../../../src/shared/library/contents/read'

import type { ReadSourceLifecycleResult } from '../../../src/shared/library/source/lifecycle'
import type { SourceIntegrityReadResult } from '../../../src/shared/library/source/integrity'
import type { SourceActivityReadResult } from '../../../src/shared/library/source/activity'

import type {
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentResult
} from '../../../src/shared/library/attachmentIdentity/read'
import { type HashSourceFilesBlake3Result } from '../../../src/shared/library/source/fileHashing'
import {
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceResult
} from '../../../src/shared/library/source/maintenance'
import { type TrackIdentityDecisionCommandResult } from '../../../src/shared/library/trackIdentity/decisions'
import { type ReadCandidatesResult } from '../../../src/shared/library/trackIdentity/candidates'
import { type BoundaryEventDeliveryPayload } from '../../../src/shared/library/boundary/events'
import { emitStatus, testStatus } from '../../support/library/boundary'
import { firstAvailableSourceReadRequest } from '../../support/library/hierarchy'

describe('preload renderer API', () => {
  it('exposes the library API and forwards calls over owned IPC channels', async () => {
    const status = testStatus()
    const hierarchyRequest = firstAvailableSourceReadRequest()
    const navigationRequest = { parentNavigationRowId: null }
    const contentsRequest = {
      scope: {
        kind: 'source' as const,
        sourceId: '7'
      },
      policy: {
        kind: 'playableMedia' as const,
        mediaKinds: ['audio', 'video'] as const
      },
      scopeDepth: 'recursive' as const,
      limit: 100
    }
    const sourceLifecycleRequest = { sourceId: '7' }
    const sourceIntegrityRequest = { sourceId: '7' }
    const sourceActivityRequest = { sourceId: '7' }
    const sourceFileAttachmentRequest = { sourceFileId: '11' }
    const attachmentSourceFilesRequest = { attachmentId: '7', limit: 25 }
    const sourceAttachmentSummaryRequest = { sourceId: '7' }
    const hashRequest = { sourceId: '7', limit: 4 }
    const runSourceMaintenanceRequest = {
      sourceId: '7',
      hashLimit: 4,
      attachmentLimit: 2,
      probeLimit: 3,
      promotionLimit: 5,
      identityCandidateLimit: 6,
      identityDecisionLimit: 7
    }
    const readSourceMaintenanceRequest = { sourceId: '7' }
    const acceptTrackIdentityCandidateRequest = {
      candidateId: '7',
      reason: 'same identity'
    }
    const rejectTrackIdentityCandidateRequest = { candidateId: '8' }
    const deferTrackIdentityCandidateRequest = {
      candidateId: '9',
      reason: 'decide later'
    }
    const readTrackIdentityReviewCandidatesRequest = {
      sourceId: '7',
      reviewState: 'userRejected' as const,
      limit: 25
    }
    const scanRequest = { rootId: 'root-1' }
    const persistedViewState: PersistedLibraryViewState = {
      version: 1,
      selectedNodeId: 'navigation-row:1',
      expandedNodeIds: ['navigation-row:1']
    }
    const navigationResult: NavigationReadRowsResult = {
      state: 'ready',
      rows: [
        {
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
        }
      ]
    }
    const hierarchyResult: ReadResult = {
      state: 'noTarget',
      error: {
        code: 'noTarget',
        message: 'No library source is available for a literal hierarchy read.'
      }
    }
    const contentsResult: ContentsReadResult = {
      state: 'noTarget',
      error: {
        code: 'noTarget',
        message: 'No contents scope was provided.'
      }
    }
    const sourceLifecycleResult: ReadSourceLifecycleResult = {
      state: 'ready',
      lifecycle: {
        sourceId: '7',
        sourceClass: 'externalMounted',
        isUserVisible: true,
        mountStatus: 'mounted',
        accessState: 'accessible',
        scanPhase: 'complete',
        lastScanStartedAtMs: 10,
        lastScanFinishedAtMs: 20,
        lastSuccessfulScanAtMs: 20,
        lastSeenAtMs: 9,
        updatedAtMs: 21
      }
    }
    const sourceIntegrityResult: SourceIntegrityReadResult = {
      state: 'ready',
      integrity: {
        sourceId: '7',
        sourceAvailability: {
          state: 'mounted'
        },
        coverageIntegrity: {
          state: 'complete',
          subtreeCoverageComplete: true,
          emptyResultAuthoritative: true,
          totalDirectoriesCount: 1,
          missingDirectoriesCount: 0,
          pendingDirectoriesCount: 0,
          scanningDirectoriesCount: 0,
          blockedDirectoriesCount: 0,
          failedDirectoriesCount: 0
        },
        evidenceAndMaintenance: {
          remainingHashCandidates: 0,
          remainingProbeCandidates: 0,
          remainingPlayableMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0
        },
        runtimeMaintenance: {
          state: 'idle'
        }
      }
    }
    const sourceActivityResult: SourceActivityReadResult = {
      state: 'ready',
      activity: {
        sourceId: '7',
        admissionState: 'active',
        browseReadiness: {
          state: 'ready',
          detail: 'Source is ready to browse.'
        },
        scanActivity: {
          state: 'completed',
          counters: {},
          detail: 'Scan completed.'
        },
        preparationActivity: {
          state: 'complete',
          backlog: {
            hash: 0,
            probe: 0,
            attachment: 0,
            promotion: 0,
            identity: 0
          },
          provenance: 'maintenanceSnapshot',
          boundedBatch: true
        }
      }
    }
    const sourceFileAttachmentResult: ReadSourceFileAttachmentResult = {
      state: 'ok',
      reply: {
        status: 'ok',
        attachmentLink: {
          sourceFileAttachmentLinkId: '5',
          attachmentId: '7',
          sourceFileId: '11',
          sourceId: '3',
          contentHashAlgorithm: 'blake3',
          contentHashValue: 'abc',
          sourceDisplayName: 'Local',
          sourceClass: 'externalMounted',
          parentSourceDirectoryId: '2',
          name: 'track.flac',
          relativePath: 'Album/track.flac',
          sizeBytes: 123,
          mtimeNs: 456,
          fileKind: 'audio',
          fileClass: 'audio',
          presenceState: 'present',
          hasCurrentBlake3Observation: true,
          linkStatus: 'current',
          sourceMountStatus: 'mounted',
          sourceAccessState: 'accessible',
          sourceScanPhase: 'complete',
          sourceAvailabilityState: 'mounted',
          occurrenceStatus: 'available',
          createdAtMs: 100,
          updatedAtMs: 200,
          sourceFileUpdatedAtMs: 300
        }
      }
    }
    const attachmentSourceFilesResult: ReadAttachmentSourceFilesResult = {
      state: 'notFound',
      reply: {
        status: 'notFound',
        sourceFileLinks: [],
        effectiveLimit: 25,
        remainingSourceFileLinks: 0
      }
    }
    const sourceAttachmentSummaryResult: ReadSourceAttachmentSummaryResult = {
      state: 'ok',
      reply: {
        status: 'ok',
        summary: {
          sourceId: '7',
          currentLinksCount: 1,
          staleLinksCount: 0,
          sourceFilesWithCurrentBlake3ObservationsCount: 1,
          sourceFilesWithAttachmentLinksCount: 1,
          sourceFilesMissingAttachmentLinksCount: 0,
          unmaterializedBlake3ObservationsCount: 0
        }
      }
    }
    const hashResult: HashSourceFilesBlake3Result = {
      state: 'completed',
      result: {
        effectiveLimit: 4,
        outcomes: [],
        hashedCount: 0,
        skippedCount: 0,
        failedCount: 0,
        remainingCandidates: 0
      }
    }
    const runSourceMaintenanceResult: RunSourceMaintenanceResult = {
      state: 'completed',
      result: {
        sourceId: '7',
        status: 'completed',
        effectiveLimits: {
          hashLimit: 4,
          attachmentLimit: 2,
          probeLimit: 3,
          promotionLimit: 5,
          identityCandidateLimit: 6,
          identityDecisionLimit: 7
        },
        hash: {
          effectiveLimit: 4,
          hashedCount: 1,
          skippedCount: 0,
          failedCount: 0,
          remainingCandidates: 0
        },
        attachmentMaterialization: {
          effectiveLimit: 2,
          attachmentsCreated: 1,
          attachmentsRefreshed: 0,
          linksCreated: 1,
          linksReplaced: 0,
          linksRefreshed: 0,
          skippedStaleObservations: 0,
          skippedNoBlake3: 0,
          skippedNoObservations: 0,
          remainingCandidates: 0
        },
        probe: {
          effectiveLimit: 3,
          probedCount: 1,
          skippedCount: 0,
          failedCount: 0,
          remainingCandidates: 0
        },
        playableMediaPromotion: {
          effectiveLimit: 5,
          promotedCount: 1,
          refreshedCount: 0,
          skippedUnusableSource: 0,
          skippedUnsupportedMediaKind: 0,
          skippedNoObservations: 0,
          skippedStaleObservations: 0,
          skippedNoBlake3: 0,
          skippedNoProbeObservations: 0,
          skippedMissingAttachmentLink: 0,
          skippedStaleAttachmentLink: 0,
          remainingCandidates: 0
        },
        trackIdentityCandidates: {
          effectiveLimit: 6,
          candidatesCreated: 1,
          candidatesRefreshed: 0,
          membersCreated: 1,
          membersRefreshed: 0,
          evidenceCreated: 1,
          evidenceRefreshed: 0,
          candidatesMarkedStale: 0,
          skippedStalePlayableMedia: 0,
          remainingCandidates: 0
        },
        trackIdentityDecisions: {
          effectiveLimit: 7,
          decisionsCreated: 1,
          decisionEvidenceCreated: 1,
          skippedStaleCandidates: 0,
          skippedExistingCurrentDecisions: 0,
          skippedUserBlockedCandidates: 0,
          remainingCandidates: 0
        },
        remainingHashCandidates: 0,
        remainingProbeCandidates: 0,
        remainingPlayableMediaPromotionCandidates: 0,
        remainingTrackIdentityCandidateProductionCandidates: 0,
        remainingTrackIdentityDecisionProductionCandidates: 0
      }
    }
    const acceptTrackIdentityCandidateResult: TrackIdentityDecisionCommandResult = {
      state: 'completed',
      result: {
        type: 'written',
        payload: {
          decisionId: '11',
          candidateId: '7',
          decisionState: 'accepted',
          decisionSource: 'user_local_v0',
          evidenceSnapshotCount: 1,
          decisionCreated: true,
          effectiveDecision: {
            effectiveDecisionId: '11',
            effectiveDecisionState: 'accepted',
            effectiveDecisionSource: 'user_local_v0',
            effectiveDecisionCurrentStatus: 'current',
            effectiveDecisionPrecedence: 'user',
            userBlockingDecisionState: 'none'
          }
        }
      }
    }
    const rejectTrackIdentityCandidateResult: TrackIdentityDecisionCommandResult = {
      state: 'completed',
      result: {
        type: 'failed',
        payload: { type: 'candidateNotFound' }
      }
    }
    const deferTrackIdentityCandidateResult: TrackIdentityDecisionCommandResult = {
      state: 'completed',
      result: {
        type: 'written',
        payload: {
          decisionId: '12',
          candidateId: '9',
          decisionState: 'deferred',
          decisionSource: 'user_local_v0',
          evidenceSnapshotCount: 0,
          decisionCreated: true,
          effectiveDecision: {
            effectiveDecisionId: '12',
            effectiveDecisionState: 'deferred',
            effectiveDecisionSource: 'user_local_v0',
            effectiveDecisionCurrentStatus: 'stale',
            effectiveDecisionPrecedence: 'user',
            userBlockingDecisionState: 'deferred'
          }
        }
      }
    }
    const readTrackIdentityReviewCandidatesResult: ReadCandidatesResult = {
      state: 'ready',
      result: {
        status: 'ok',
        candidates: [
          {
            candidateId: '11',
            candidateKind: 'exact_playable_media_content',
            candidateEvidenceBasis: 'current_playable_media_exact_blake3',
            candidateStatus: 'active',
            evidenceKeyAlgorithm: 'blake3',
            evidenceKeyValue: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            evidenceSummary: {
              memberCount: 1,
              evidenceCount: 1,
              currentEvidenceCount: 1
            },
            sourceSummary: {
              sourceCount: 1,
              sourceSamples: [{ sourceId: '7', displayName: 'Local' }]
            },
            reviewState: 'userRejected',
            effectiveDecision: {
              decisionId: '12',
              decisionState: 'rejected',
              decisionSource: 'user_local_v0',
              decisionBasis: 'explicit_user_local_decision_v0',
              currentStatus: 'current',
              createdAtMs: 100,
              userBlockingDecisionState: 'rejected',
              maskedSystemDecisionId: '10'
            },
            createdAtMs: 80,
            updatedAtMs: 90
          }
        ]
      }
    }
    const readSourceMaintenanceResult: ReadSourceMaintenanceResult = {
      state: 'ready',
      snapshot: {
        sourceId: '7',
        status: 'idle',
        remainingHashCandidates: 0,
        remainingProbeCandidates: 0,
        remainingPlayableMediaPromotionCandidates: 0,
        remainingTrackIdentityCandidateProductionCandidates: 0,
        remainingTrackIdentityDecisionProductionCandidates: 0
      }
    }
    const choiceResult: LocalRootChoiceResult = {
      state: 'registered',
      root: {
        rootId: '7',
        admittedRootPath: 'C:/Music'
      }
    }
    const scanResult: LocalRootScanResult = {
      state: 'started',
      scanRunId: 'scan-1'
    }
    const cancelScanResult: CancelRootScanResult = {
      state: 'accepted',
      status: 'accepted'
    }
    const readLocalRootsResult: ReadLocalRootsOutcome = {
      state: 'read',
      roots: [
        {
          rootId: '7',
          admittedRootPath: 'C:/Music',
          availability: 'available'
        }
      ]
    }
    const unregisterLocalRootResult: UnregisterLocalRootResult = {
      state: 'unregistered',
      unregistered: true
    }
    const viewStateReadResult: LibraryViewStateReadResult = {
      state: 'ready',
      viewState: {
        version: 1,
        selectedNodeId: 'navigation-row:1',
        expandedNodeIds: ['navigation-row:1', 'source-directory:2']
      }
    }
    const viewStateWriteResult: LibraryViewStateWriteResult = { state: 'written' }
    let receivedChoiceArgs: readonly unknown[] | undefined
    let receivedHierarchyRequest: unknown
    let receivedNavigationRequest: unknown
    let receivedScanRequest: unknown
    let receivedCancelScanRequest: unknown
    let receivedContentsRequest: unknown
    let receivedSourceLifecycleRequest: unknown
    let receivedSourceIntegrityRequest: unknown
    let receivedSourceActivityRequest: unknown
    let receivedSourceFileAttachmentRequest: unknown
    let receivedAttachmentSourceFilesRequest: unknown
    let receivedSourceAttachmentSummaryRequest: unknown
    let receivedHashRequest: unknown
    let receivedRunSourceMaintenanceRequest: unknown
    let receivedReadSourceMaintenanceRequest: unknown
    let receivedAcceptTrackIdentityCandidateRequest: unknown
    let receivedRejectTrackIdentityCandidateRequest: unknown
    let receivedDeferTrackIdentityCandidateRequest: unknown
    let receivedReadTrackIdentityReviewCandidatesRequest: unknown
    let receivedViewStatePayload: unknown
    let subscribeCount = 0
    let unsubscribeCount = 0
    const listeners = new Map<string, Set<(event: unknown, payload: unknown) => void>>()
    const ipcRenderer = {
      invoke: async (channel: string, ...args: readonly unknown[]): Promise<unknown> => {
        if (channel === libraryControlChannels.boundary.getStatus) {
          expect(args).toEqual([])
          return status
        }

        if (channel === libraryControlChannels.navigation.read) {
          receivedNavigationRequest = args[0]
          return navigationResult
        }

        if (channel === libraryControlChannels.hierarchy.read) {
          receivedHierarchyRequest = args[0]
          return hierarchyResult
        }

        if (channel === libraryControlChannels.contents.read) {
          receivedContentsRequest = args[0]
          return contentsResult
        }

        if (channel === libraryControlChannels.source.lifecycle) {
          receivedSourceLifecycleRequest = args[0]
          return sourceLifecycleResult
        }

        if (channel === libraryControlChannels.source.integrity) {
          receivedSourceIntegrityRequest = args[0]
          return sourceIntegrityResult
        }

        if (channel === libraryControlChannels.source.activity) {
          receivedSourceActivityRequest = args[0]
          return sourceActivityResult
        }

        if (channel === libraryControlChannels.attachmentIdentity.readSourceFileAttachment) {
          receivedSourceFileAttachmentRequest = args[0]
          return sourceFileAttachmentResult
        }

        if (channel === libraryControlChannels.attachmentIdentity.readAttachmentSourceFiles) {
          receivedAttachmentSourceFilesRequest = args[0]
          return attachmentSourceFilesResult
        }

        if (channel === libraryControlChannels.attachmentIdentity.readSourceAttachmentSummary) {
          receivedSourceAttachmentSummaryRequest = args[0]
          return sourceAttachmentSummaryResult
        }

        if (channel === libraryControlChannels.source.fileHashing) {
          receivedHashRequest = args[0]
          return hashResult
        }

        if (channel === libraryControlChannels.source.maintenance.run) {
          receivedRunSourceMaintenanceRequest = args[0]
          return runSourceMaintenanceResult
        }

        if (channel === libraryControlChannels.source.maintenance.read) {
          receivedReadSourceMaintenanceRequest = args[0]
          return readSourceMaintenanceResult
        }

        if (channel === libraryControlChannels.trackIdentity.decisions.accept) {
          receivedAcceptTrackIdentityCandidateRequest = args[0]
          return acceptTrackIdentityCandidateResult
        }

        if (channel === libraryControlChannels.trackIdentity.decisions.reject) {
          receivedRejectTrackIdentityCandidateRequest = args[0]
          return rejectTrackIdentityCandidateResult
        }

        if (channel === libraryControlChannels.trackIdentity.decisions.defer) {
          receivedDeferTrackIdentityCandidateRequest = args[0]
          return deferTrackIdentityCandidateResult
        }

        if (channel === libraryControlChannels.trackIdentity.candidates.read) {
          receivedReadTrackIdentityReviewCandidatesRequest = args[0]
          return readTrackIdentityReviewCandidatesResult
        }

        if (channel === libraryControlChannels.roots.chooseLocal) {
          receivedChoiceArgs = args
          return choiceResult
        }

        if (channel === libraryControlChannels.roots.scan) {
          receivedScanRequest = args[0]
          return scanResult
        }

        if (channel === libraryControlChannels.roots.cancel) {
          receivedCancelScanRequest = args[0]
          return cancelScanResult
        }

        if (channel === libraryControlChannels.roots.read) {
          expect(args).toEqual([])
          return readLocalRootsResult
        }

        if (channel === libraryControlChannels.roots.unregister) {
          return unregisterLocalRootResult
        }

        if (channel === libraryControlChannels.viewState.read) {
          expect(args).toEqual([])
          return viewStateReadResult
        }

        if (channel === libraryControlChannels.viewState.write) {
          receivedViewStatePayload = args[0]
          return viewStateWriteResult
        }

        if (channel === libraryControlChannels.boundary.events.subscribe) {
          expect(args).toEqual([])
          subscribeCount += 1
          return { kind: 'subscribed' }
        }

        if (channel === libraryControlChannels.boundary.events.unsubscribe) {
          expect(args).toEqual([])
          unsubscribeCount += 1
          return { kind: 'unsubscribed' }
        }

        throw new Error(`Unexpected preload invoke channel ${channel}.`)
      },
      on: (channel: string, listener: (event: unknown, payload: unknown) => void): void => {
        const channelListeners = listeners.get(channel) ?? new Set()
        channelListeners.add(listener)
        listeners.set(channel, channelListeners)
      },
      off: (channel: string, listener: (event: unknown, payload: unknown) => void): void => {
        listeners.get(channel)?.delete(listener)
      }
    }
    const exposedApis = new Map<string, unknown>()

    exposeRendererApi(
      {
        exposeInMainWorld(apiKey, api): void {
          exposedApis.set(apiKey, api)
        }
      },
      ipcRenderer
    )
    expect(exposedApis.get('dekzer')).toBeDefined()

    const api = createRendererApi(ipcRenderer)

    await expect(api.library.host.getStatus()).resolves.toBe(status)
    await expect(
      (
        api.library.roots.chooseAndRegisterLocal as (
          request?: unknown
        ) => Promise<LocalRootChoiceResult>
      )({ requestedPath: 'C:/RendererMustNotControlThis' })
    ).resolves.toBe(choiceResult)
    expect(receivedChoiceArgs).toEqual([])
    await expect(api.library.roots.runScan(scanRequest)).resolves.toBe(scanResult)
    expect(receivedScanRequest).toBe(scanRequest)
    await expect(api.library.roots.cancelScan({ scanRunId: 'scan-1' })).resolves.toBe(
      cancelScanResult
    )
    expect(receivedCancelScanRequest).toEqual({ scanRunId: 'scan-1' })
    await expect(api.library.roots.readLocalRoots()).resolves.toBe(readLocalRootsResult)
    await expect(api.library.roots.unregisterLocalRoot({ rootId: '7' })).resolves.toBe(
      unregisterLocalRootResult
    )
    await expect(api.library.navigation.readRows(navigationRequest)).resolves.toBe(navigationResult)
    expect(receivedNavigationRequest).toBe(navigationRequest)
    await expect(api.library.hierarchy.readChildren(hierarchyRequest)).resolves.toBe(
      hierarchyResult
    )
    expect(receivedHierarchyRequest).toBe(hierarchyRequest)
    await expect(api.library.contents.read(contentsRequest)).resolves.toBe(contentsResult)
    expect(receivedContentsRequest).toBe(contentsRequest)
    await expect(
      api.library.sourceLifecycle.readSourceLifecycle(sourceLifecycleRequest)
    ).resolves.toBe(sourceLifecycleResult)
    expect(receivedSourceLifecycleRequest).toBe(sourceLifecycleRequest)
    await expect(
      api.library.sourceIntegrity.readSourceIntegrity(sourceIntegrityRequest)
    ).resolves.toBe(sourceIntegrityResult)
    expect(receivedSourceIntegrityRequest).toBe(sourceIntegrityRequest)
    await expect(
      api.library.sourceActivity.readSourceActivity(sourceActivityRequest)
    ).resolves.toBe(sourceActivityResult)
    expect(receivedSourceActivityRequest).toBe(sourceActivityRequest)
    await expect(
      api.library.attachmentIdentity.readSourceFileAttachment(sourceFileAttachmentRequest)
    ).resolves.toBe(sourceFileAttachmentResult)
    expect(receivedSourceFileAttachmentRequest).toBe(sourceFileAttachmentRequest)
    await expect(
      api.library.attachmentIdentity.readAttachmentSourceFiles(attachmentSourceFilesRequest)
    ).resolves.toBe(attachmentSourceFilesResult)
    expect(receivedAttachmentSourceFilesRequest).toBe(attachmentSourceFilesRequest)
    await expect(
      api.library.attachmentIdentity.readSourceAttachmentSummary(sourceAttachmentSummaryRequest)
    ).resolves.toBe(sourceAttachmentSummaryResult)
    expect(receivedSourceAttachmentSummaryRequest).toBe(sourceAttachmentSummaryRequest)
    await expect(api.library.hashing.hashSourceFilesBlake3(hashRequest)).resolves.toBe(hashResult)
    expect(receivedHashRequest).toBe(hashRequest)
    await expect(
      api.library.sourceMaintenance.runSourceMaintenance(runSourceMaintenanceRequest)
    ).resolves.toBe(runSourceMaintenanceResult)
    expect(receivedRunSourceMaintenanceRequest).toBe(runSourceMaintenanceRequest)
    await expect(
      api.library.sourceMaintenance.readSourceMaintenance(readSourceMaintenanceRequest)
    ).resolves.toBe(readSourceMaintenanceResult)
    expect(receivedReadSourceMaintenanceRequest).toBe(readSourceMaintenanceRequest)
    await expect(
      api.library.trackIdentityDecisions.acceptTrackIdentityCandidate(
        acceptTrackIdentityCandidateRequest
      )
    ).resolves.toBe(acceptTrackIdentityCandidateResult)
    expect(receivedAcceptTrackIdentityCandidateRequest).toBe(acceptTrackIdentityCandidateRequest)
    await expect(
      api.library.trackIdentityDecisions.rejectTrackIdentityCandidate(
        rejectTrackIdentityCandidateRequest
      )
    ).resolves.toBe(rejectTrackIdentityCandidateResult)
    expect(receivedRejectTrackIdentityCandidateRequest).toBe(rejectTrackIdentityCandidateRequest)
    await expect(
      api.library.trackIdentityDecisions.deferTrackIdentityCandidate(
        deferTrackIdentityCandidateRequest
      )
    ).resolves.toBe(deferTrackIdentityCandidateResult)
    expect(receivedDeferTrackIdentityCandidateRequest).toBe(deferTrackIdentityCandidateRequest)
    await expect(
      api.library.trackIdentityReview.readTrackIdentityReviewCandidates(
        readTrackIdentityReviewCandidatesRequest
      )
    ).resolves.toBe(readTrackIdentityReviewCandidatesResult)
    expect(receivedReadTrackIdentityReviewCandidatesRequest).toBe(
      readTrackIdentityReviewCandidatesRequest
    )
    await expect(api.library.viewState.readViewState()).resolves.toBe(viewStateReadResult)
    await expect(api.library.viewState.writeViewState(persistedViewState)).resolves.toBe(
      viewStateWriteResult
    )
    expect(receivedViewStatePayload).toBe(persistedViewState)

    let receivedStatus: LibraryBoundaryHostStatus | null = null
    const unsubscribe = api.library.host.onStatusChanged((changedStatus) => {
      receivedStatus = changedStatus
    })

    emitStatus(listeners, status)
    expect(receivedStatus).toBe(status)

    receivedStatus = null
    unsubscribe()
    emitStatus(listeners, status)
    expect(receivedStatus).toBeNull()

    const eventPayload: BoundaryEventDeliveryPayload = {
      kind: 'batch',
      events: [],
      latestEventSequence: null,
      earliestRetainedSequence: null,
      gapDetected: false
    }
    let receivedEventPayload: BoundaryEventDeliveryPayload | undefined
    const unsubscribeEvents = api.library.events.subscribe((payload) => {
      receivedEventPayload = payload
    })

    await waitForMicrotasks()
    expect(subscribeCount).toBe(1)
    for (const listener of listeners.get(libraryPublicationChannels.boundary.events.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBe(eventPayload)

    unsubscribeEvents()
    await waitForMicrotasks()
    expect(unsubscribeCount).toBe(1)
    receivedEventPayload = undefined
    for (const listener of listeners.get(libraryPublicationChannels.boundary.events.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBeUndefined()
  })
})

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
