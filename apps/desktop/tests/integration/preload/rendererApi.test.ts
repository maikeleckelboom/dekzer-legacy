import { describe, expect, it } from 'vitest'

import { createRendererApi, exposeRendererApi } from '../../../src/preload/rendererApi'
import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../../../src/shared/libraryBoundary/status'
import {
  libraryViewStateChannels,
  type LibraryViewStateReadResult,
  type LibraryViewStateWriteResult,
  type PersistedLibraryViewState
} from '../../../src/shared/libraryViewState/viewState'
import {
  hierarchyReadChannels,
  type ReadResult
} from '../../../src/shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type NavigationReadRowsResult
} from '../../../src/shared/libraryNavigation/readRows'
import { rootChannels } from '../../../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../../../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../../../src/shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../../../src/shared/libraryRoots/runScan'
import type { CancelRootScanResult } from '../../../src/shared/libraryRoots/cancelScan'
import type { UnregisterLocalRootResult } from '../../../src/shared/libraryRoots/unregisterLocalRoot'
import {
  contentsReadChannels,
  type ContentsReadResult
} from '../../../src/shared/libraryContents/read'
import { sourceLifecycleReadChannels } from '../../../src/shared/librarySourceLifecycle/channels'
import type { ReadSourceLifecycleResult } from '../../../src/shared/librarySourceLifecycle/readSourceLifecycle'
import { attachmentIdentityReadChannels } from '../../../src/shared/libraryAttachmentIdentity/channels'
import type {
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentResult
} from '../../../src/shared/libraryAttachmentIdentity/read'
import {
  sourceFileHashingChannels,
  type HashSourceFilesBlake3Result
} from '../../../src/shared/librarySourceFileHashing/hashSourceFilesBlake3'
import {
  sourceMaintenanceChannels,
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceResult
} from '../../../src/shared/librarySourceMaintenance/sourceMaintenance'
import {
  trackIdentityDecisionChannels,
  type TrackIdentityDecisionCommandResult
} from '../../../src/shared/libraryTrackIdentityDecisions/decisionCommands'
import {
  channels as trackIdentityReviewChannels,
  type ReadCandidatesResult
} from '../../../src/shared/libraryTrackIdentityReview/candidates'
import {
  boundaryEventChannels,
  type BoundaryEventDeliveryPayload
} from '../../../src/shared/libraryBoundary/events'
import { emitStatus, testStatus } from '../../support/libraryBoundary'
import { firstAvailableSourceReadRequest } from '../../support/libraryHierarchy'

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
        mediaClasses: ['audio', 'video'] as const,
        rowProfile: { kind: 'primaryMedia' as const }
      },
      recursion: 'recursive' as const,
      limit: 100
    }
    const sourceLifecycleRequest = { sourceId: '7' }
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
    const sourceFileAttachmentResult: ReadSourceFileAttachmentResult = {
      state: 'ok',
      reply: {
        status: 'ok',
        attachmentLink: {
          attachmentId: '7',
          sourceFileId: '11',
          sourceId: '3',
          contentHashAlgorithm: 'blake3',
          contentHashValue: 'abc',
          fileKind: 'audio',
          linkStatus: 'current',
          createdAtMs: 100,
          updatedAtMs: 200
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
          sourceFilesWithCurrentBlake3FactsCount: 1,
          sourceFilesWithAttachmentLinksCount: 1,
          sourceFilesMissingAttachmentLinksCount: 0,
          unmaterializedBlake3FactsCount: 0
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
          skippedStaleFacts: 0,
          skippedNoBlake3: 0,
          skippedNoFacts: 0,
          remainingCandidates: 0
        },
        probe: {
          effectiveLimit: 3,
          probedCount: 1,
          skippedCount: 0,
          failedCount: 0,
          remainingCandidates: 0
        },
        primaryMediaPromotion: {
          effectiveLimit: 5,
          promotedCount: 1,
          refreshedCount: 0,
          skippedUnusableSource: 0,
          skippedUnsupportedMediaKind: 0,
          skippedNoFacts: 0,
          skippedStaleFacts: 0,
          skippedNoBlake3: 0,
          skippedNoProbeFacts: 0,
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
          skippedStalePrimaryMediaCandidates: 0,
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
        remainingPrimaryMediaPromotionCandidates: 0,
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
            candidateKind: 'exact_primary_media_content',
            candidateEvidenceBasis: 'current_primary_media_exact_blake3',
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
        remainingPrimaryMediaPromotionCandidates: 0,
        remainingTrackIdentityCandidateProductionCandidates: 0,
        remainingTrackIdentityDecisionProductionCandidates: 0
      }
    }
    const choiceResult: LocalRootChoiceResult = {
      state: 'registered',
      root: {
        rootId: '7',
        canonicalPath: 'C:/Music'
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
          canonicalPath: 'C:/Music',
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
        if (channel === hostStatusChannels.getStatus) {
          expect(args).toEqual([])
          return status
        }

        if (channel === navigationReadChannels.readRows) {
          receivedNavigationRequest = args[0]
          return navigationResult
        }

        if (channel === hierarchyReadChannels.readChildren) {
          receivedHierarchyRequest = args[0]
          return hierarchyResult
        }

        if (channel === contentsReadChannels.read) {
          receivedContentsRequest = args[0]
          return contentsResult
        }

        if (channel === sourceLifecycleReadChannels.readSourceLifecycle) {
          receivedSourceLifecycleRequest = args[0]
          return sourceLifecycleResult
        }

        if (channel === attachmentIdentityReadChannels.readSourceFileAttachment) {
          receivedSourceFileAttachmentRequest = args[0]
          return sourceFileAttachmentResult
        }

        if (channel === attachmentIdentityReadChannels.readAttachmentSourceFiles) {
          receivedAttachmentSourceFilesRequest = args[0]
          return attachmentSourceFilesResult
        }

        if (channel === attachmentIdentityReadChannels.readSourceAttachmentSummary) {
          receivedSourceAttachmentSummaryRequest = args[0]
          return sourceAttachmentSummaryResult
        }

        if (channel === sourceFileHashingChannels.hashSourceFilesBlake3) {
          receivedHashRequest = args[0]
          return hashResult
        }

        if (channel === sourceMaintenanceChannels.runSourceMaintenance) {
          receivedRunSourceMaintenanceRequest = args[0]
          return runSourceMaintenanceResult
        }

        if (channel === sourceMaintenanceChannels.readSourceMaintenance) {
          receivedReadSourceMaintenanceRequest = args[0]
          return readSourceMaintenanceResult
        }

        if (channel === trackIdentityDecisionChannels.acceptTrackIdentityCandidate) {
          receivedAcceptTrackIdentityCandidateRequest = args[0]
          return acceptTrackIdentityCandidateResult
        }

        if (channel === trackIdentityDecisionChannels.rejectTrackIdentityCandidate) {
          receivedRejectTrackIdentityCandidateRequest = args[0]
          return rejectTrackIdentityCandidateResult
        }

        if (channel === trackIdentityDecisionChannels.deferTrackIdentityCandidate) {
          receivedDeferTrackIdentityCandidateRequest = args[0]
          return deferTrackIdentityCandidateResult
        }

        if (channel === trackIdentityReviewChannels.readCandidates) {
          receivedReadTrackIdentityReviewCandidatesRequest = args[0]
          return readTrackIdentityReviewCandidatesResult
        }

        if (channel === rootChannels.chooseAndRegisterLocal) {
          receivedChoiceArgs = args
          return choiceResult
        }

        if (channel === rootChannels.runScan) {
          receivedScanRequest = args[0]
          return scanResult
        }

        if (channel === rootChannels.cancelScan) {
          receivedCancelScanRequest = args[0]
          return cancelScanResult
        }

        if (channel === rootChannels.readLocalRoots) {
          expect(args).toEqual([])
          return readLocalRootsResult
        }

        if (channel === rootChannels.unregisterLocalRoot) {
          return unregisterLocalRootResult
        }

        if (channel === libraryViewStateChannels.readViewState) {
          expect(args).toEqual([])
          return viewStateReadResult
        }

        if (channel === libraryViewStateChannels.writeViewState) {
          receivedViewStatePayload = args[0]
          return viewStateWriteResult
        }

        if (channel === boundaryEventChannels.subscribe) {
          expect(args).toEqual([])
          subscribeCount += 1
          return { kind: 'subscribed' }
        }

        if (channel === boundaryEventChannels.unsubscribe) {
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
      )({ absolutePath: 'C:/RendererMustNotControlThis' })
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
    for (const listener of listeners.get(boundaryEventChannels.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBe(eventPayload)

    unsubscribeEvents()
    await waitForMicrotasks()
    expect(unsubscribeCount).toBe(1)
    receivedEventPayload = undefined
    for (const listener of listeners.get(boundaryEventChannels.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBeUndefined()
  })
})

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
