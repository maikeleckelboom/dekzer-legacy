import type {
  AcceptTrackIdentityCandidateRequest,
  CancelRootScanReply,
  CancelRootScanRequest,
  CommandReply,
  CommandRequest,
  CreatePlaylistReply,
  CreatePlaylistRequest,
  DeferTrackIdentityCandidateRequest,
  DeletePlaylistReply,
  DeletePlaylistRequest,
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
  ReadLibraryTreeChildrenReply,
  ReadLibraryTreeChildrenRequest,
  ReadSourceAttachmentSummaryReply,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceFileAttachmentReply,
  ReadSourceFileAttachmentRequest,
  ReadSourceLifecycleReply,
  ReadSourceLifecycleRequest,
  ReadSourceMaintenanceReply,
  ReadSourceMaintenanceRequest,
  ReadLocalRootsReply,
  ReadLocalRootsRequest,
  ReadNavigationNodeLibraryBrowserWindowReply,
  ReadNavigationNodeLibraryBrowserWindowRequest,
  ReadNavigationRowsReply,
  ReadNavigationRowsRequest,
  ContentsReadReply,
  ContentsReadRequest,
  RegisterLocalRootReply,
  RegisterLocalRootRequest,
  RejectTrackIdentityCandidateRequest,
  RenamePlaylistReply,
  RenamePlaylistRequest,
  RunSourceMaintenanceReply,
  RunSourceMaintenanceRequest,
  StartRootScanReply,
  StartRootScanRequest,
  SearchNavigationNodeLibraryBrowserWindowReply,
  SearchNavigationNodeLibraryBrowserWindowRequest,
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

  readNavigationNodeLibraryBrowserWindow(
    request: ReadNavigationNodeLibraryBrowserWindowRequest
  ): Promise<ReadNavigationNodeLibraryBrowserWindowReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'readNavigationNodeLibraryBrowserWindow',
          payload: request
        }
      },
      'snapshotRead',
      'navigationNodeLibraryBrowserWindow'
    )
  }

  searchNavigationNodeLibraryBrowserWindow(
    request: SearchNavigationNodeLibraryBrowserWindowRequest
  ): Promise<SearchNavigationNodeLibraryBrowserWindowReply> {
    return this.sendAndExpect(
      {
        type: 'snapshotRead',
        payload: {
          type: 'searchNavigationNodeLibraryBrowserWindow',
          payload: request
        }
      },
      'snapshotRead',
      'navigationNodeLibraryBrowserSearch'
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

  createPlaylist(request: CreatePlaylistRequest): Promise<CreatePlaylistReply> {
    return this.sendAndExpect(
      {
        type: 'playlistWrite',
        payload: { type: 'createPlaylist', payload: request }
      },
      'playlistWrite',
      'createPlaylist'
    )
  }

  renamePlaylist(request: RenamePlaylistRequest): Promise<RenamePlaylistReply> {
    return this.sendAndExpect(
      {
        type: 'playlistWrite',
        payload: { type: 'renamePlaylist', payload: request }
      },
      'playlistWrite',
      'renamePlaylist'
    )
  }

  deletePlaylist(request: DeletePlaylistRequest): Promise<DeletePlaylistReply> {
    return this.sendAndExpect(
      {
        type: 'playlistWrite',
        payload: { type: 'deletePlaylist', payload: request }
      },
      'playlistWrite',
      'deletePlaylist'
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
