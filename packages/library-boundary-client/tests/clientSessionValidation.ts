import type {
  CommandOutcome,
  CommandReply,
  CommandRequest,
  ContentsReadRequest,
  LibraryBoundaryEvent,
  MaintainedSnapshotEvent,
  MaintainedSnapshotInvalidation,
  ProtocolError,
  HashSourceFilesBlake3Reply,
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply,
  ReadSourceFileAttachmentReply,
  ReadTrackIdentityReviewCandidatesReply,
  RegisterLocalRootReply,
  RunSourceMaintenanceReply,
  TrackIdentityDecisionCommandResult
} from '@dekzer/library-boundary-contract'

import {
  LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT,
  LibraryBoundaryClient,
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundarySession,
  LibraryBoundarySessionDrainInProgressError,
  LibraryBoundarySessionInvalidMaxEventsError,
  LibraryBoundarySessionStateError,
  LibraryBoundaryTransportError,
  type LibraryBoundaryTransport
} from '../src/index.js'

type EqualTypes<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends <Value>() => Value extends Right ? 1 : 2
    ? true
    : false

type AssertType<Condition extends true> = Condition

type RegisterLocalRootReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['registerLocalRoot']>>,
    RegisterLocalRootReply
  >
>

type RegisterLocalRootIdStaysString = AssertType<
  EqualTypes<RegisterLocalRootReply['rootId'], string>
>

type HashSourceFilesBlake3ReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['hashSourceFilesBlake3']>>,
    HashSourceFilesBlake3Reply
  >
>

type RunSourceMaintenanceReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['runSourceMaintenance']>>,
    RunSourceMaintenanceReply
  >
>

type AcceptTrackIdentityCandidateReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['acceptTrackIdentityCandidate']>>,
    TrackIdentityDecisionCommandResult
  >
>

type ReadSourceMaintenanceReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['readSourceMaintenance']>>,
    ReadSourceMaintenanceReply
  >
>

type ReadSourceIntegrityReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['readSourceIntegrity']>>,
    ReadSourceIntegrityReply
  >
>

type ReadSourceFileAttachmentReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['readSourceFileAttachment']>>,
    ReadSourceFileAttachmentReply
  >
>

type ReadTrackIdentityReviewCandidatesReturnIsGenerated = AssertType<
  EqualTypes<
    Awaited<ReturnType<LibraryBoundaryClient['readTrackIdentityReviewCandidates']>>,
    ReadTrackIdentityReviewCandidatesReply
  >
>

type ContentsReadAcceptsAudioBrowseProfile = AssertType<
  EqualTypes<
    Extract<ContentsReadRequest['policy'], { kind: 'audioBrowse' }>,
    { kind: 'audioBrowse' }
  >
>

const compileTimeAssertions: [
  RegisterLocalRootReturnIsGenerated,
  RegisterLocalRootIdStaysString,
  HashSourceFilesBlake3ReturnIsGenerated,
  RunSourceMaintenanceReturnIsGenerated,
  AcceptTrackIdentityCandidateReturnIsGenerated,
  ReadSourceMaintenanceReturnIsGenerated,
  ReadSourceIntegrityReturnIsGenerated,
  ReadSourceFileAttachmentReturnIsGenerated,
  ReadTrackIdentityReviewCandidatesReturnIsGenerated,
  ContentsReadAcceptsAudioBrowseProfile
] = [true, true, true, true, true, true, true, true, true, true]
void compileTimeAssertions

type Resolve<T> = (value: T | PromiseLike<T>) => void
type Reject = (reason?: unknown) => void

type Deferred<T> = {
  readonly promise: Promise<T>
  readonly resolve: Resolve<T>
  readonly reject: Reject
}

function deferred<T>(): Deferred<T> {
  let resolve: Resolve<T> | null = null
  let reject: Reject | null = null
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve
    reject = innerReject
  })

  return {
    promise,
    resolve: (value) => {
      must(resolve !== null, 'deferred resolve was not initialized')
      resolve(value)
    },
    reject: (reason) => {
      must(reject !== null, 'deferred reject was not initialized')
      reject(reason)
    }
  }
}

function success(reply: CommandReply): CommandOutcome {
  return {
    type: 'success',
    payload: { reply }
  }
}

function protocolFailure(error: ProtocolError): CommandOutcome {
  return {
    type: 'error',
    payload: { error }
  }
}

function event(invalidation: MaintainedSnapshotInvalidation): LibraryBoundaryEvent {
  return {
    type: 'maintainedSnapshotInvalidated',
    payload: {
      eventSequence: 0,
      occurredAtMs: 0,
      invalidation
    } satisfies MaintainedSnapshotEvent
  }
}

function eventsReply(events: readonly LibraryBoundaryEvent[]): CommandOutcome {
  return success({
    type: 'libraryBoundaryEvents',
    payload: {
      type: 'readAfter',
      payload: {
        events: [...events],
        latestEventSequence: null,
        earliestRetainedSequence: null,
        gapDetected: false
      }
    }
  })
}

class RecordingTransport implements LibraryBoundaryTransport {
  readonly sentRequests: CommandRequest[] = []
  closeCount = 0
  private readonly outcomes: Array<CommandOutcome | Deferred<CommandOutcome> | Error> = []

  enqueueOutcome(outcome: CommandOutcome): void {
    this.outcomes.push(outcome)
  }

  enqueueEvents(events: readonly LibraryBoundaryEvent[]): void {
    this.enqueueOutcome(eventsReply(events))
  }

  enqueueRejection(cause: Error): void {
    this.outcomes.push(cause)
  }

  enqueueDeferredOutcome(): Deferred<CommandOutcome> {
    const pending = deferred<CommandOutcome>()
    this.outcomes.push(pending)
    return pending
  }

  async execute(request: CommandRequest): Promise<CommandOutcome> {
    this.sentRequests.push(request)
    const outcome = this.outcomes.shift()
    if (outcome === undefined) {
      throw new Error(`missing test outcome for ${request.type}`)
    }
    if (outcome instanceof Error) {
      throw outcome
    }
    if ('promise' in outcome) {
      return outcome.promise
    }

    return outcome
  }

  close(): void {
    this.closeCount += 1
  }
}

async function validatesRegisterLocalRootRequestAndReply(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'libraryRoots',
      payload: {
        type: 'registerLocalRoot',
        payload: {
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const reply = await client.registerLocalRoot({
    absolutePath: 'C:/Music'
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'libraryRoots',
      payload: {
        type: 'registerLocalRoot',
        payload: { absolutePath: 'C:/Music' }
      }
    } satisfies CommandRequest,
    'registerLocalRoot sends the generated boundary command'
  )
  equal(reply.rootId, 'root-1', 'registerLocalRoot unwraps the reply payload')
  equal(typeof reply.rootId, 'string', 'rootId remains a generated string id')
}

async function validatesHashSourceFilesBlake3RequestAndReply(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'sourceFileHash',
      payload: {
        type: 'hashSourceFilesBlake3',
        payload: {
          effectiveLimit: 1,
          outcomes: [
            {
              sourceFileId: '11',
              sourceId: '7',
              relativePath: 'a.flac',
              status: {
                type: 'hashed',
                payload: {
                  contentHashAlgorithm: 'blake3',
                  contentHashValue: 'abc',
                  acceptedArtifactId: '90',
                  workItemId: '91'
                }
              }
            }
          ],
          hashedCount: 1,
          skippedCount: 0,
          failedCount: 0,
          remainingCandidates: 0
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const reply = await client.hashSourceFilesBlake3({
    sourceId: '7',
    limit: 1
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'sourceFileHash',
      payload: {
        type: 'hashSourceFilesBlake3',
        payload: { sourceId: '7', limit: 1 }
      }
    } satisfies CommandRequest,
    'hashSourceFilesBlake3 sends the generated boundary command'
  )
  equal(reply.outcomes[0]?.sourceFileId, '11', 'hash outcome keeps string source file id')
  equal(reply.outcomes[0]?.status.type, 'hashed', 'hashSourceFilesBlake3 unwraps the reply payload')
}

async function validatesSourceMaintenanceRequestsAndReplies(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'sourceMaintenance',
      payload: {
        type: 'runSourceMaintenance',
        payload: {
          sourceId: '7',
          status: 'completed',
          effectiveLimits: {
            hashLimit: 8,
            attachmentLimit: 4,
            probeLimit: 4,
            promotionLimit: 4,
            identityCandidateLimit: 4,
            identityDecisionLimit: 4
          },
          hash: {
            effectiveLimit: 8,
            hashedCount: 1,
            skippedCount: 0,
            failedCount: 0,
            remainingCandidates: 0
          },
          attachmentMaterialization: {
            effectiveLimit: 4,
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
            effectiveLimit: 4,
            probedCount: 1,
            skippedCount: 0,
            failedCount: 0,
            remainingCandidates: 0
          },
          primaryMediaPromotion: {
            effectiveLimit: 4,
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
            effectiveLimit: 4,
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
            effectiveLimit: 4,
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
    })
  )
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'sourceMaintenance',
        payload: {
          sourceId: '7',
          status: 'idle',
          remainingHashCandidates: 0,
          remainingProbeCandidates: 0,
          remainingPrimaryMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0,
          attachmentLinks: {
            currentLinksCount: 1,
            staleLinksCount: 0,
            sourceFilesWithCurrentBlake3FactsCount: 1,
            sourceFilesWithAttachmentLinksCount: 1,
            sourceFilesMissingAttachmentLinksCount: 0,
            unmaterializedBlake3FactsCount: 0
          },
          lastRun: {
            status: 'completed',
            hash: {
              effectiveLimit: 8,
              hashedCount: 1,
              skippedCount: 0,
              failedCount: 0,
              remainingCandidates: 0
            },
            attachmentMaterialization: {
              effectiveLimit: 4,
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
              effectiveLimit: 4,
              probedCount: 1,
              skippedCount: 0,
              failedCount: 0,
              remainingCandidates: 0
            },
            primaryMediaPromotion: {
              effectiveLimit: 4,
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
              effectiveLimit: 4,
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
              effectiveLimit: 4,
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
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const runReply = await client.runSourceMaintenance({
    sourceId: '7',
    hashLimit: 8,
    attachmentLimit: 4,
    probeLimit: 4,
    promotionLimit: 4,
    identityCandidateLimit: 4,
    identityDecisionLimit: 4
  })
  const readReply = await client.readSourceMaintenance({ sourceId: '7' })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'sourceMaintenance',
      payload: {
        type: 'runSourceMaintenance',
        payload: {
          sourceId: '7',
          hashLimit: 8,
          attachmentLimit: 4,
          probeLimit: 4,
          promotionLimit: 4,
          identityCandidateLimit: 4,
          identityDecisionLimit: 4
        }
      }
    } satisfies CommandRequest,
    'runSourceMaintenance sends the generated boundary command'
  )
  deepEqual(
    transport.sentRequests[1],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readSourceMaintenance',
        payload: { sourceId: '7' }
      }
    } satisfies CommandRequest,
    'readSourceMaintenance sends the generated snapshot command'
  )
  equal(runReply.probe.probedCount, 1, 'runSourceMaintenance unwraps probe summary')
  equal(
    readReply.attachmentLinks?.currentLinksCount,
    1,
    'readSourceMaintenance unwraps attachment link summary'
  )
}

async function validatesSourceIntegrityReadRequestAndReply(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'sourceIntegrity',
        payload: {
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
          inventory: {
            countsByPresenceState: [{ presenceState: 'present', count: 2 }],
            countsByFileClass: [
              { fileClass: 'audio', count: 1 },
              { fileClass: 'image', count: 1 }
            ],
            countsByFileKind: [
              { fileKind: 'audio', count: 1 },
              { fileKind: 'image', count: 1 }
            ],
            mediaRelevantFilesCount: 2,
            presentMediaRelevantFilesCount: 2
          },
          evidenceAndMaintenance: {
            remainingHashCandidates: 2,
            remainingProbeCandidates: 1,
            remainingPrimaryMediaPromotionCandidates: 0,
            remainingTrackIdentityCandidateProductionCandidates: 0,
            remainingTrackIdentityDecisionProductionCandidates: 0
          },
          attachmentIntegrity: {
            currentLinksCount: 0,
            staleLinksCount: 0,
            missingLinksCount: 0,
            sourceFilesWithCurrentBlake3FactsCount: 0,
            sourceFilesWithAttachmentLinksCount: 0,
            unmaterializedBlake3FactsCount: 0
          },
          runtimeMaintenance: {
            state: 'idle'
          }
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const reply = await client.readSourceIntegrity({ sourceId: '7' })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readSourceIntegrity',
        payload: { sourceId: '7' }
      }
    } satisfies CommandRequest,
    'readSourceIntegrity sends the generated snapshot command'
  )
  equal(reply.sourceAvailability.state, 'mounted', 'readSourceIntegrity unwraps availability')
  equal(
    reply.inventory?.mediaRelevantFilesCount,
    2,
    'readSourceIntegrity unwraps inventory facet'
  )
  equal(
    reply.evidenceAndMaintenance.remainingHashCandidates,
    2,
    'readSourceIntegrity unwraps maintenance backlog facet'
  )
}

async function validatesTrackIdentityDecisionRequestsAndReplies(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'trackIdentityDecisions',
      payload: {
        type: 'acceptTrackIdentityCandidate',
        payload: {
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
    })
  )
  transport.enqueueOutcome(
    success({
      type: 'trackIdentityDecisions',
      payload: {
        type: 'rejectTrackIdentityCandidate',
        payload: {
          type: 'failed',
          payload: {
            type: 'candidateNotFound'
          }
        }
      }
    })
  )
  transport.enqueueOutcome(
    success({
      type: 'trackIdentityDecisions',
      payload: {
        type: 'deferTrackIdentityCandidate',
        payload: {
          type: 'written',
          payload: {
            decisionId: '12',
            candidateId: '8',
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
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const acceptReply = await client.acceptTrackIdentityCandidate({
    candidateId: '7',
    reason: 'same identity'
  })
  const rejectReply = await client.rejectTrackIdentityCandidate({
    candidateId: '999'
  })
  const deferReply = await client.deferTrackIdentityCandidate({
    candidateId: '8',
    reason: 'decide later'
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'trackIdentityDecisions',
      payload: {
        type: 'acceptTrackIdentityCandidate',
        payload: {
          candidateId: '7',
          reason: 'same identity'
        }
      }
    } satisfies CommandRequest,
    'acceptTrackIdentityCandidate sends only candidate id and decision intent'
  )
  deepEqual(
    transport.sentRequests[1],
    {
      type: 'trackIdentityDecisions',
      payload: {
        type: 'rejectTrackIdentityCandidate',
        payload: {
          candidateId: '999'
        }
      }
    } satisfies CommandRequest,
    'rejectTrackIdentityCandidate sends only candidate id and decision intent'
  )
  deepEqual(
    transport.sentRequests[2],
    {
      type: 'trackIdentityDecisions',
      payload: {
        type: 'deferTrackIdentityCandidate',
        payload: {
          candidateId: '8',
          reason: 'decide later'
        }
      }
    } satisfies CommandRequest,
    'deferTrackIdentityCandidate sends only candidate id and decision intent'
  )
  equal(acceptReply.type, 'written', 'accept reply unwraps the result')
  if (acceptReply.type === 'written') {
    equal(
      acceptReply.payload.effectiveDecision.effectiveDecisionPrecedence,
      'user',
      'effective decision precedence remains backend-owned in the reply'
    )
  }
  equal(rejectReply.type, 'failed', 'reject reply unwraps typed failure')
  equal(deferReply.type, 'written', 'defer reply unwraps the result')
}

async function validatesTrackIdentityReviewCandidateReads(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'trackIdentityReviewCandidates',
        payload: {
          status: 'ok',
          candidates: [
            {
              candidateId: '7',
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
                sourceSamples: [{ sourceId: '3', displayName: 'Local' }]
              },
              reviewState: 'userRejected',
              effectiveDecision: {
                decisionId: '11',
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
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const reply = await client.readTrackIdentityReviewCandidates({
    sourceId: '3',
    reviewState: 'userRejected',
    limit: 25
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readTrackIdentityReviewCandidates',
        payload: {
          sourceId: '3',
          reviewState: 'userRejected',
          limit: 25
        }
      }
    } satisfies CommandRequest,
    'readTrackIdentityReviewCandidates sends only sourceId, reviewState, and limit'
  )
  equal(reply.status, 'ok', 'review read unwraps status')
  equal(reply.candidates[0]?.reviewState, 'userRejected', 'review state is backend supplied')
  equal(
    reply.candidates[0]?.effectiveDecision?.userBlockingDecisionState,
    'rejected',
    'effective decision summary is preserved'
  )
}

async function validatesAttachmentIdentityReadRequestsAndReplies(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'sourceFileAttachment',
        payload: {
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
    })
  )
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'attachmentSourceFiles',
        payload: {
          status: 'ok',
          attachment: {
            attachmentId: '7',
            contentHashAlgorithm: 'blake3',
            contentHashValue: 'abc'
          },
          sourceFileLinks: [],
          effectiveLimit: 25,
          remainingSourceFileLinks: 0
        }
      }
    })
  )
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'sourceAttachmentSummary',
        payload: {
          status: 'ok',
          summary: {
            sourceId: '3',
            currentLinksCount: 1,
            staleLinksCount: 0,
            sourceFilesWithCurrentBlake3FactsCount: 1,
            sourceFilesWithAttachmentLinksCount: 1,
            sourceFilesMissingAttachmentLinksCount: 0,
            unmaterializedBlake3FactsCount: 0
          }
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const sourceFileReply = await client.readSourceFileAttachment({
    sourceFileId: '11'
  })
  const attachmentReply = await client.readAttachmentSourceFiles({
    attachmentId: '7',
    limit: 25
  })
  const summaryReply = await client.readSourceAttachmentSummary({
    sourceId: '3'
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readSourceFileAttachment',
        payload: { sourceFileId: '11' }
      }
    } satisfies CommandRequest,
    'readSourceFileAttachment sends the generated snapshot command'
  )
  deepEqual(
    transport.sentRequests[1],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readAttachmentSourceFiles',
        payload: { attachmentId: '7', limit: 25 }
      }
    } satisfies CommandRequest,
    'readAttachmentSourceFiles sends the generated snapshot command'
  )
  deepEqual(
    transport.sentRequests[2],
    {
      type: 'snapshotRead',
      payload: {
        type: 'readSourceAttachmentSummary',
        payload: { sourceId: '3' }
      }
    } satisfies CommandRequest,
    'readSourceAttachmentSummary sends the generated snapshot command'
  )
  equal(
    sourceFileReply.attachmentLink?.linkStatus,
    'current',
    'source-file attachment reply unwraps the link'
  )
  equal(
    attachmentReply.attachment?.attachmentId,
    '7',
    'attachment source-files reply unwraps the attachment identity'
  )
  equal(
    summaryReply.summary?.currentLinksCount,
    1,
    'source attachment summary reply unwraps counts'
  )
}

async function validatesContentsReadAudioBrowseRequestAndReply(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'snapshotRead',
      payload: {
        type: 'contents',
        payload: {
          result: {
            state: 'empty',
            scope: {
              type: 'source',
              payload: { sourceId: '7' }
            },
            policy: {
              kind: 'audioBrowse'
            },
            scopeDepth: 'recursive',
            rows: [],
            scopeCoverage: {
              state: 'complete',
              subtreeCoverageComplete: true,
              emptyResultAuthoritative: true
            },
            hasPolicyOmittedRows: false
          }
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const reply = await client.readContents({
    scope: {
      type: 'source',
      payload: { sourceId: '7' }
    },
    policy: {
      kind: 'audioBrowse'
    },
    scopeDepth: 'recursive',
    limit: 25
  })

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'snapshotRead',
      payload: {
        type: 'contentsRead',
        payload: {
          scope: {
            type: 'source',
            payload: { sourceId: '7' }
          },
          policy: {
            kind: 'audioBrowse'
          },
          scopeDepth: 'recursive',
          limit: 25
        }
      }
    } satisfies CommandRequest,
    'readContents sends audioBrowse through the existing snapshot command'
  )
  equal(reply.result.policy.kind, 'audioBrowse', 'contents reply preserves audioBrowse profile')
}

async function validatesProtocolErrorsArePreserved(): Promise<void> {
  const protocolError: ProtocolError = {
    type: 'invalidRequest',
    payload: { detail: 'fixture invalid request' }
  }
  const transport = new RecordingTransport()
  transport.enqueueOutcome(protocolFailure(protocolError))
  const client = new LibraryBoundaryClient(transport)

  const error = await rejects(
    () => client.registerLocalRoot({ absolutePath: '' }),
    LibraryBoundaryProtocolError,
    'protocol error outcomes reject with a typed client error'
  )

  equal(error.protocolError, protocolError, 'protocol error payload object is preserved')
}

async function validatesReplyFamilyMismatch(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(eventsReply([]))
  const client = new LibraryBoundaryClient(transport)

  const error = await rejects(
    () => client.registerLocalRoot({ absolutePath: 'C:/Music' }),
    LibraryBoundaryReplyMismatchError,
    'unexpected reply family is rejected'
  )

  equal(error.expectedFamily, 'libraryRoots', 'expected family is captured')
  equal(error.actualFamily, 'libraryBoundaryEvents', 'actual family is captured')
}

async function validatesReplyVariantMismatch(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    success({
      type: 'libraryRoots',
      payload: {
        type: 'startRootScan',
        payload: {
          scanRunId: 'scan-1'
        }
      }
    })
  )
  const client = new LibraryBoundaryClient(transport)

  const error = await rejects(
    () => client.registerLocalRoot({ absolutePath: 'C:/Music' }),
    LibraryBoundaryReplyMismatchError,
    'unexpected reply variant is rejected'
  )

  equal(error.expectedVariant, 'registerLocalRoot', 'expected variant is captured')
  equal(error.actualVariant, 'startRootScan', 'actual variant is captured')
}

async function validatesExecutorRejectionBecomesTransportFailure(): Promise<void> {
  const transport = new RecordingTransport()
  const cause = new Error('fixture transport failure')
  transport.enqueueRejection(cause)
  const client = new LibraryBoundaryClient(transport)

  const error = await rejects(
    () => client.registerLocalRoot({ absolutePath: 'C:/Music' }),
    LibraryBoundaryTransportError,
    'executor rejection is surfaced as transport failure'
  )

  equal(error.cause, cause, 'transport failure keeps the original cause')
}

async function validatesSessionPumpFlow(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueEvents([
    event({ scope: 'navigationRows', revision: '1' }),
    event({ scope: 'libraryBrowser', revision: '5' })
  ])
  const session = new LibraryBoundarySession(transport)
  let listenerBatch: unknown = null
  session.subscribeInvalidations((batch) => {
    listenerBatch = batch
  })

  const batch = await session.pumpEvents(8)

  deepEqual(
    transport.sentRequests[0],
    {
      type: 'libraryBoundaryEvents',
      payload: {
        type: 'readAfter',
        payload: { lastSeenEventSequence: null, maxEvents: 8 }
      }
    } satisfies CommandRequest,
    'session pump sends the explicit event cursor read command'
  )
  deepEqual(
    batch.changedScopes,
    ['navigationRows', 'libraryBrowser'],
    'session pump reports changed scopes'
  )
  deepEqual(
    batch.changedRevisions,
    [
      { scope: 'navigationRows', revision: '1' },
      { scope: 'libraryBrowser', revision: '5' }
    ],
    'session pump reports changed revisions'
  )
  equal(
    batch.lastSeenRevisions.get('libraryBrowser'),
    '5',
    'session pump returns last-seen revisions'
  )
  equal(
    session.getLastSeenRevision('navigationRows'),
    '1',
    'session records the last seen revision by scope'
  )
  equal(listenerBatch, batch, 'invalidation listener receives the pump batch')
}

async function validatesStaleDuplicateInvalidationsAreIgnored(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueEvents([
    event({ scope: 'navigationRows', revision: '1' }),
    event({ scope: 'navigationRows', revision: '3' }),
    event({ scope: 'navigationRows', revision: '2' })
  ])
  transport.enqueueEvents([
    event({ scope: 'navigationRows', revision: '3' }),
    event({ scope: 'navigationRows', revision: '1' }),
    event({ scope: 'libraryBrowser', revision: '4' })
  ])
  const session = new LibraryBoundarySession(transport)

  const first = await session.pumpEvents(8)
  const second = await session.pumpEvents(8)

  equal(
    first.invalidations[0]?.revision,
    '3',
    'duplicate scope invalidations keep the highest fresh revision'
  )
  deepEqual(
    second.changedScopes,
    ['libraryBrowser'],
    'stale duplicate revisions are ignored by scope'
  )
  equal(
    session.getLastSeenRevision('navigationRows'),
    '3',
    'stale duplicate revisions do not roll back last-seen state'
  )
}

async function validatesConcurrentDrainsAreRejected(): Promise<void> {
  const transport = new RecordingTransport()
  const pending = transport.enqueueDeferredOutcome()
  const session = new LibraryBoundarySession(transport)

  const firstPump = session.pumpEvents(8)
  throws(
    () => session.pumpEvents(8),
    LibraryBoundarySessionDrainInProgressError,
    'overlapping event drain is rejected'
  )

  pending.resolve(eventsReply([]))
  await firstPump
}

function validatesInvalidMaxEvents(): void {
  const transport = new RecordingTransport()
  const session = new LibraryBoundarySession(transport)

  for (const maxEvents of [0, -1, 1.5, Number.NaN]) {
    throws(
      () => session.pumpEvents(maxEvents),
      LibraryBoundarySessionInvalidMaxEventsError,
      `invalid maxEvents ${String(maxEvents)} is rejected`
    )
  }

  equal(transport.sentRequests.length, 0, 'invalid maxEvents is rejected before sending a command')
}

async function validatesCloseAndClosedState(): Promise<void> {
  const transport = new RecordingTransport()
  const session = new LibraryBoundarySession(transport)

  await session.close()
  await session.close()

  equal(session.state, 'closed', 'close marks the session closed')
  equal(transport.closeCount, 1, 'close calls injected close only once')
  throws(
    () => session.pumpEvents(1),
    LibraryBoundarySessionStateError,
    'closed session rejects pump'
  )
  throws(
    () => session.subscribeInvalidations(() => undefined),
    LibraryBoundarySessionStateError,
    'closed session rejects new listeners'
  )
}

async function validatesListenerErrorsAreRetainedWithLimit(): Promise<void> {
  const transport = new RecordingTransport()
  const listenerFailures = Array.from(
    { length: LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT + 1 },
    (_, index) => new Error(`listener failure ${index}`)
  )
  const reportedErrors: unknown[] = []
  transport.enqueueEvents([event({ scope: 'libraryBrowser', revision: '9' })])
  const session = new LibraryBoundarySession(transport, {
    onListenerError: (report) => {
      reportedErrors.push(report.error)
    }
  })
  for (const failure of listenerFailures) {
    session.subscribeInvalidations(() => {
      throw failure
    })
  }

  await session.pumpEvents(8)
  const drained = session.takeListenerErrors()

  equal(
    reportedErrors.length,
    listenerFailures.length,
    'listener error hook observes every listener failure'
  )
  equal(
    drained.retentionLimit,
    LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT,
    'listener error drain exposes the retention limit'
  )
  equal(
    drained.retainedReportCount,
    LIBRARY_BOUNDARY_LISTENER_ERROR_RETENTION_LIMIT,
    'listener error reports are bounded'
  )
  equal(drained.droppedReportCount, 1, 'oldest listener error is dropped')
  equal(
    drained.reports[0]?.error,
    listenerFailures[1],
    'retained listener errors keep the newest reports'
  )
}

async function validatesFailedPumpState(): Promise<void> {
  const transport = new RecordingTransport()
  transport.enqueueOutcome(
    protocolFailure({
      type: 'hostFailure',
      payload: { detail: 'fixture host failure' }
    })
  )
  const session = new LibraryBoundarySession(transport)

  await rejects(
    () => session.pumpEvents(8),
    LibraryBoundaryProtocolError,
    'event protocol failure remains typed'
  )

  equal(session.state, 'failed', 'failed pump marks the session failed')
  throws(
    () => session.pumpEvents(8),
    LibraryBoundarySessionStateError,
    'failed session rejects more pumps'
  )
  await session.close()
  equal(session.state, 'closed', 'failed session can still be closed')
}

function must(condition: boolean, message: string): asserts condition {
  if (!condition) {
    throw new Error(message)
  }
}

function equal<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${String(expected)}, received ${String(actual)}`)
  }
}

function deepEqual(actual: unknown, expected: unknown, message: string): void {
  const actualJson = JSON.stringify(actual)
  const expectedJson = JSON.stringify(expected)
  if (actualJson !== expectedJson) {
    throw new Error(`${message}: expected ${expectedJson}, received ${actualJson}`)
  }
}

function throws<ErrorType extends Error>(
  action: () => unknown,
  errorType: new (...args: never[]) => ErrorType,
  message: string
): ErrorType {
  try {
    action()
  } catch (error) {
    if (error instanceof errorType) {
      return error
    }

    throw new Error(`${message}: wrong error ${String(error)}`)
  }

  throw new Error(`${message}: no error thrown`)
}

async function rejects<ErrorType extends Error>(
  action: () => Promise<unknown>,
  errorType: new (...args: never[]) => ErrorType,
  message: string
): Promise<ErrorType> {
  try {
    await action()
  } catch (error) {
    if (error instanceof errorType) {
      return error
    }

    throw new Error(`${message}: wrong error ${String(error)}`)
  }

  throw new Error(`${message}: no error thrown`)
}

await validatesRegisterLocalRootRequestAndReply()
await validatesHashSourceFilesBlake3RequestAndReply()
await validatesSourceMaintenanceRequestsAndReplies()
await validatesSourceIntegrityReadRequestAndReply()
await validatesTrackIdentityDecisionRequestsAndReplies()
await validatesTrackIdentityReviewCandidateReads()
await validatesAttachmentIdentityReadRequestsAndReplies()
await validatesContentsReadAudioBrowseRequestAndReply()
await validatesProtocolErrorsArePreserved()
await validatesReplyFamilyMismatch()
await validatesReplyVariantMismatch()
await validatesExecutorRejectionBecomesTransportFailure()
await validatesSessionPumpFlow()
await validatesStaleDuplicateInvalidationsAreIgnored()
await validatesConcurrentDrainsAreRejected()
validatesInvalidMaxEvents()
await validatesCloseAndClosedState()
await validatesListenerErrorsAreRetainedWithLimit()
await validatesFailedPumpState()

console.log('boundary client/session validation passed')
