import type {
  CancelRootScanReply,
  CancelRootScanRequest,
  CommandReply,
  CommandRequest,
  CreatePlaylistReply,
  CreatePlaylistRequest,
  DeletePlaylistReply,
  DeletePlaylistRequest,
  LoadNavigationRowByStableKeyReply,
  LoadNavigationRowByStableKeyRequest,
  LoadNavigationRowReply,
  LoadNavigationRowRequest,
  ReadLibraryBoundaryEventsAfterReply,
  ReadLibraryBoundaryEventsAfterRequest,
  ReadLibraryTreeChildrenReply,
  ReadLibraryTreeChildrenRequest,
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
  RenamePlaylistReply,
  RenamePlaylistRequest,
  StartRootScanReply,
  StartRootScanRequest,
  SearchNavigationNodeLibraryBrowserWindowReply,
  SearchNavigationNodeLibraryBrowserWindowRequest,
  UnregisterLocalRootReply,
  UnregisterLocalRootRequest
} from "@dekzer/library-boundary-contract";

import {
  executeLibraryBoundaryCommand,
  type LibraryBoundaryCommandReplyPayload,
  type LibraryBoundaryCommandReplyVariant
} from "./commandExecutor.js";
import type { LibraryBoundaryCommandExecutor } from "./transport.js";

type VariantPayload<
  Family extends CommandReply["type"],
  Variant extends LibraryBoundaryCommandReplyVariant<Family>
> = LibraryBoundaryCommandReplyPayload<Family, Variant>;

export class LibraryBoundaryClient {
  readonly #executor: LibraryBoundaryCommandExecutor;

  constructor(executor: LibraryBoundaryCommandExecutor) {
    this.#executor = executor;
  }

  registerLocalRoot(
    request: RegisterLocalRootRequest
  ): Promise<RegisterLocalRootReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "registerLocalRoot", payload: request }
      },
      "libraryRoots",
      "registerLocalRoot"
    );
  }

  startRootScan(request: StartRootScanRequest): Promise<StartRootScanReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "startRootScan", payload: request }
      },
      "libraryRoots",
      "startRootScan"
    );
  }

  readLocalRoots(
    request: ReadLocalRootsRequest
  ): Promise<ReadLocalRootsReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "readLocalRoots", payload: request }
      },
      "libraryRoots",
      "readLocalRoots"
    );
  }

  unregisterLocalRoot(
    request: UnregisterLocalRootRequest
  ): Promise<UnregisterLocalRootReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "unregisterLocalRoot", payload: request }
      },
      "libraryRoots",
      "unregisterLocalRoot"
    );
  }

  cancelRootScan(
    request: CancelRootScanRequest
  ): Promise<CancelRootScanReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "cancelRootScan", payload: request }
      },
      "libraryRoots",
      "cancelRootScan"
    );
  }

  readNavigationRows(
    request: ReadNavigationRowsRequest
  ): Promise<ReadNavigationRowsReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: { type: "readNavigationRows", payload: request }
      },
      "snapshotRead",
      "navigationRows"
    );
  }

  loadNavigationRow(
    request: LoadNavigationRowRequest
  ): Promise<LoadNavigationRowReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: { type: "loadNavigationRow", payload: request }
      },
      "snapshotRead",
      "navigationRow"
    );
  }

  loadNavigationRowByStableKey(
    request: LoadNavigationRowByStableKeyRequest
  ): Promise<LoadNavigationRowByStableKeyReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: { type: "loadNavigationRowByStableKey", payload: request }
      },
      "snapshotRead",
      "navigationRowByStableKey"
    );
  }

  readLibraryTreeChildren(
    request: ReadLibraryTreeChildrenRequest
  ): Promise<ReadLibraryTreeChildrenReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: {
          type: "readLibraryTreeChildren",
          payload: request
        }
      },
      "snapshotRead",
      "libraryTreeChildren"
    );
  }

  readNavigationNodeLibraryBrowserWindow(
    request: ReadNavigationNodeLibraryBrowserWindowRequest
  ): Promise<ReadNavigationNodeLibraryBrowserWindowReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: {
          type: "readNavigationNodeLibraryBrowserWindow",
          payload: request
        }
      },
      "snapshotRead",
      "navigationNodeLibraryBrowserWindow"
    );
  }

  searchNavigationNodeLibraryBrowserWindow(
    request: SearchNavigationNodeLibraryBrowserWindowRequest
  ): Promise<SearchNavigationNodeLibraryBrowserWindowReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: {
          type: "searchNavigationNodeLibraryBrowserWindow",
          payload: request
        }
      },
      "snapshotRead",
      "navigationNodeLibraryBrowserSearch"
    );
  }

  readContents(
    request: ContentsReadRequest
  ): Promise<ContentsReadReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: {
          type: "contentsRead",
          payload: request
        }
      },
      "snapshotRead",
      "contents"
    );
  }

  createPlaylist(
    request: CreatePlaylistRequest
  ): Promise<CreatePlaylistReply> {
    return this.sendAndExpect(
      {
        type: "playlistWrite",
        payload: { type: "createPlaylist", payload: request }
      },
      "playlistWrite",
      "createPlaylist"
    );
  }

  renamePlaylist(
    request: RenamePlaylistRequest
  ): Promise<RenamePlaylistReply> {
    return this.sendAndExpect(
      {
        type: "playlistWrite",
        payload: { type: "renamePlaylist", payload: request }
      },
      "playlistWrite",
      "renamePlaylist"
    );
  }

  deletePlaylist(
    request: DeletePlaylistRequest
  ): Promise<DeletePlaylistReply> {
    return this.sendAndExpect(
      {
        type: "playlistWrite",
        payload: { type: "deletePlaylist", payload: request }
      },
      "playlistWrite",
      "deletePlaylist"
    );
  }

  readAfterBoundaryEvents(
    request: ReadLibraryBoundaryEventsAfterRequest
  ): Promise<ReadLibraryBoundaryEventsAfterReply> {
    return this.sendAndExpect(
      {
        type: "libraryBoundaryEvents",
        payload: { type: "readAfter", payload: request }
      },
      "libraryBoundaryEvents",
      "readAfter"
    );
  }

  private async sendAndExpect<
    Family extends CommandReply["type"],
    Variant extends LibraryBoundaryCommandReplyVariant<Family>
  >(
    request: CommandRequest,
    expectedFamily: Family,
    expectedVariant: Variant
  ): Promise<VariantPayload<Family, Variant>> {
    return executeLibraryBoundaryCommand(
      this.#executor,
      request,
      expectedFamily,
      expectedVariant
    );
  }
}

export function createLibraryBoundaryClient(
  executor: LibraryBoundaryCommandExecutor
): LibraryBoundaryClient {
  return new LibraryBoundaryClient(executor);
}
