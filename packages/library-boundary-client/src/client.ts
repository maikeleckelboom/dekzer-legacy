import type {
  AcceptTrackIdentityCandidateRequest,
  CancelRootScanReply,
  CancelRootScanRequest,
  CommandReply,
  CommandRequest,
  DeferTrackIdentityCandidateRequest,
  HashSourceFilesBlake3Reply,
  HashSourceFilesBlake3Request,
  LoadNavigationRowByStableKeyReply,
  LoadNavigationRowByStableKeyRequest,
  LoadNavigationRowReply,
  LoadNavigationRowRequest,
  ReadAttachmentSourceFilesReply,
  ReadAttachmentSourceFilesRequest,
  ReadLibraryBoundaryEventsAfterReply,
  ReadLibraryBoundaryEventsAfterRequest,
  ReadLocalBrowserChildrenReply,
  ReadLocalBrowserChildrenRequest,
  ReadLocalBrowserEntryPointsReply,
  ReadLocalBrowserEntryPointsRequest,
  ReadLibraryTreeChildrenReply,
  ReadLibraryTreeChildrenRequest,
  ReadTrackIdentityReviewCandidatesReply,
  ReadTrackIdentityReviewCandidatesRequest,
  ReadSourceAttachmentSummaryReply,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceFileAttachmentReply,
  ReadSourceFileAttachmentRequest,
  ReadSourceIntegrityReply,
  ReadSourceIntegrityRequest,
  ReadSourceLifecycleReply,
  ReadSourceLifecycleRequest,
  ReadSourceMaintenanceReply,
  ReadSourceMaintenanceRequest,
  SearchFilterReadReply,
  SearchFilterReadRequest,
  ReadLocalRootsReply,
  ReadLocalRootsRequest,
  ReadNavigationRowsReply,
  ReadNavigationRowsRequest,
  ContentsReadReply,
  ContentsReadRequest,
  RegisterLocalRootReply,
  RegisterLocalRootRequest,
  RejectTrackIdentityCandidateRequest,
  RunSourceMaintenanceReply,
  RunSourceMaintenanceRequest,
  StartRootScanReply,
  StartRootScanRequest,
  TrackIdentityDecisionCommandResult,
  UnregisterLocalRootReply,
  UnregisterLocalRootRequest
} from '@dekzer/library-boundary-contract'

import {
  executeLibraryBoundaryCommand,
  type LibraryBoundaryCommandReplyPayload,
  type LibraryBoundaryCommandReplyVariant
} from './commandExecutor.js'
import type { LibraryBoundaryCommandExecutor } from './transport.js'

type VariantPayload<
  Family extends CommandReply['type'],
  Variant extends LibraryBoundaryCommandReplyVariant<Family>
> = LibraryBoundaryCommandReplyPayload<Family, Variant>

export class LibraryBoundaryClient {
  readonly #executor: LibraryBoundaryCommandExecutor

  constructor(executor: LibraryBoundaryCommandExecutor) {
    this.#executor = executor
  }

  registerLocalRoot(request: RegisterLocalRootRequest): Promise<RegisterLocalRootReply> {
    return this.sendAndExpect(
      {
        type: 'libraryRoots',
        payload: { type: 'registerLocalRoot', payload: request }
      },
      'libraryRoots',
      'registerLocalRoot'
    )
  }

  startRootScan(request: StartRootScanRequest): Promise<StartRootScanReply> {
    return this.sendAndExpect(
      {
        type: 'libraryRoots',
        payload: { type: 'startRootScan', payload: request }
      },
      'libraryRoots',
      'startRootScan'
    )
  }

  readLocalRoots(request: ReadLocalRootsRequest): Promise<ReadLocalRootsReply> {
    return this.sendAndExpect(
      {
        type: 'libraryRoots',
        payload: { type: 'readLocalRoots', payload: request }
      },
      'libraryRoots',
      'readLocalRoots'
    )
  }

  unregisterLocalRoot(request: UnregisterLocalRootRequest): Promise<UnregisterLocalRootReply> {
    return this.sendAndExpect(
      {
        type: 'libraryRoots',
        payload: { type: 'unregisterLocalRoot', payload: request }
      },
      'libraryRoots',
      'unregisterLocalRoot'
    )
  }

  cancelRootScan(request: CancelRootScanRequest): Promise<CancelRootScanReply> {
    return this.sendAndExpect(
      {
        type: 'libraryRoots',
        payload: { type: 'cancelRootScan', payload: request }
      },
      'libraryRoots',
      'cancelRootScan'
    )
  }

  hashSourceFilesBlake3(
    request: HashSourceFilesBlake3Request
  ): Promise<HashSourceFilesBlake3Reply> {
    return this.sendAndExpect(
      {
        type: 'sourceFileHash',
        payload: { type: 'hashSourceFilesBlake3', payload: request }
      },
      'sourceFileHash',
      'hashSourceFilesBlake3'
    )
  }

  runSourceMaintenance(request: RunSourceMaintenanceRequest): Promise<RunSourceMaintenanceReply> {
    return this.sendAndExpect(
      {
        type: 'sourceMaintenance',
        payload: { type: 'runSourceMaintenance', payload: request }
      },
      'sourceMaintenance',
      'runSourceMaintenance'
    )
  }

  acceptTrackIdentityCandidate(
    request: AcceptTrackIdentityCandidateRequest
  ): Promise<TrackIdentityDecisionCommandResult> {
    return this.sendAndExpect(
      {
        type: 'trackIdentityDecisions',
        payload: { type: 'acceptTrackIdentityCandidate', payload: request }
      },
      'trackIdentityDecisions',
      'acceptTrackIdentityCandidate'
    )
  }

  rejectTrackIdentityCandidate(
    request: RejectTrackIdentityCandidateRequest
  ): Promise<TrackIdentityDecisionCommandResult> {
    return this.sendAndExpect(
      {
        type: 'trackIdentityDecisions',
        payload: { type: 'rejectTrackIdentityCandidate', payload: request }
      },
      'trackIdentityDecisions',
      'rejectTrackIdentityCandidate'
    )
  }

  deferTrackIdentityCandidate(
    request: DeferTrackIdentityCandidateRequest
  ): Promise<TrackIdentityDecisionCommandResult> {
    return this.sendAndExpect(
      {
        type: 'trackIdentityDecisions',
        payload: { type: 'deferTrackIdentityCandidate', payload: request }
      },
      'trackIdentityDecisions',
      'deferTrackIdentityCandidate'
    )
  }

  readNavigationRows(request: ReadNavigationRowsRequest): Promise<ReadNavigationRowsReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: { type: 'readNavigationRows', payload: request }
      },
      'snapshotRead',
      'navigationRows'
    )
  }

  readLocalBrowserEntryPoints(
    request: ReadLocalBrowserEntryPointsRequest
  ): Promise<ReadLocalBrowserEntryPointsReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: { type: 'readLocalBrowserEntryPoints', payload: request }
      },
      'snapshotRead',
      'localBrowserEntryPoints'
    )
  }

  readLocalBrowserChildren(
    request: ReadLocalBrowserChildrenRequest
  ): Promise<ReadLocalBrowserChildrenReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: { type: 'readLocalBrowserChildren', payload: request }
      },
      'snapshotRead',
      'localBrowserChildren'
    )
  }

  loadNavigationRow(request: LoadNavigationRowRequest): Promise<LoadNavigationRowReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: { type: 'loadNavigationRow', payload: request }
      },
      'snapshotRead',
      'navigationRow'
    )
  }

  loadNavigationRowByStableKey(
    request: LoadNavigationRowByStableKeyRequest
  ): Promise<LoadNavigationRowByStableKeyReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: { type: 'loadNavigationRowByStableKey', payload: request }
      },
      'snapshotRead',
      'navigationRowByStableKey'
    )
  }

  readLibraryTreeChildren(
    request: ReadLibraryTreeChildrenRequest
  ): Promise<ReadLibraryTreeChildrenReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readLibraryTreeChildren',
          payload: request
        }
      },
      'snapshotRead',
      'libraryTreeChildren'
    )
  }

  readSourceLifecycle(request: ReadSourceLifecycleRequest): Promise<ReadSourceLifecycleReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readSourceLifecycle',
          payload: request
        }
      },
      'snapshotRead',
      'sourceLifecycle'
    )
  }

  readSourceIntegrity(request: ReadSourceIntegrityRequest): Promise<ReadSourceIntegrityReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readSourceIntegrity',
          payload: request
        }
      },
      'snapshotRead',
      'sourceIntegrity'
    )
  }

  readSourceMaintenance(
    request: ReadSourceMaintenanceRequest
  ): Promise<ReadSourceMaintenanceReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readSourceMaintenance',
          payload: request
        }
      },
      'snapshotRead',
      'sourceMaintenance'
    )
  }

  readSourceFileAttachment(
    request: ReadSourceFileAttachmentRequest
  ): Promise<ReadSourceFileAttachmentReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readSourceFileAttachment',
          payload: request
        }
      },
      'snapshotRead',
      'sourceFileAttachment'
    )
  }

  readAttachmentSourceFiles(
    request: ReadAttachmentSourceFilesRequest
  ): Promise<ReadAttachmentSourceFilesReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readAttachmentSourceFiles',
          payload: request
        }
      },
      'snapshotRead',
      'attachmentSourceFiles'
    )
  }

  readSourceAttachmentSummary(
    request: ReadSourceAttachmentSummaryRequest
  ): Promise<ReadSourceAttachmentSummaryReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readSourceAttachmentSummary',
          payload: request
        }
      },
      'snapshotRead',
      'sourceAttachmentSummary'
    )
  }

  readTrackIdentityReviewCandidates(
    request: ReadTrackIdentityReviewCandidatesRequest
  ): Promise<ReadTrackIdentityReviewCandidatesReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readTrackIdentityReviewCandidates',
          payload: request
        }
      },
      'snapshotRead',
      'trackIdentityReviewCandidates'
    )
  }

  readContents(request: ContentsReadRequest): Promise<ContentsReadReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'contentsRead',
          payload: request
        }
      },
      'snapshotRead',
      'contents'
    )
  }

  readSearchFilter(request: SearchFilterReadRequest): Promise<SearchFilterReadReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'searchFilterRead',
          payload: request
        }
      },
      'snapshotRead',
      'searchFilter'
    )
  }

  readAfterBoundaryEvents(
    request: ReadLibraryBoundaryEventsAfterRequest
  ): Promise<ReadLibraryBoundaryEventsAfterReply> {
    return this.sendAndExpect(
      {
        type: 'libraryBoundaryEvents',
        payload: { type: 'readAfter', payload: request }
      },
      'libraryBoundaryEvents',
      'readAfter'
    )
  }

  private async sendAndExpect<
    Family extends CommandReply['type'],
    Variant extends LibraryBoundaryCommandReplyVariant<Family>
  >(
    request: CommandRequest,
    expectedFamily: Family,
    expectedVariant: Variant
  ): Promise<VariantPayload<Family, Variant>> {
    return executeLibraryBoundaryCommand(this.#executor, request, expectedFamily, expectedVariant)
  }
}

export function createLibraryBoundaryClient(
  executor: LibraryBoundaryCommandExecutor
): LibraryBoundaryClient {
  return new LibraryBoundaryClient(executor)
}
