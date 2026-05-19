import type {
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
  ReadLibraryBoundaryEventsReply,
  ReadLibraryBoundaryEventsRequest,
  ReadLiteralHierarchyChildrenReply,
  ReadLiteralHierarchyChildrenRequest,
  ReadNavigationNodeLibraryBrowserWindowReply,
  ReadNavigationNodeLibraryBrowserWindowRequest,
  ReadNavigationRowsReply,
  ReadNavigationRowsRequest,
  RegisterLocalRootReply,
  RegisterLocalRootRequest,
  RenamePlaylistReply,
  RenamePlaylistRequest,
  RunRootScanReply,
  RunRootScanRequest,
  SearchNavigationNodeLibraryBrowserWindowReply,
  SearchNavigationNodeLibraryBrowserWindowRequest
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

  runRootScan(request: RunRootScanRequest): Promise<RunRootScanReply> {
    return this.sendAndExpect(
      {
        type: "libraryRoots",
        payload: { type: "runRootScan", payload: request }
      },
      "libraryRoots",
      "runRootScan"
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

  readLiteralHierarchyChildren(
    request: ReadLiteralHierarchyChildrenRequest
  ): Promise<ReadLiteralHierarchyChildrenReply> {
    return this.sendAndExpect(
      {
        type: "snapshotRead",
        payload: {
          type: "readLiteralHierarchyChildren",
          payload: request
        }
      },
      "snapshotRead",
      "literalHierarchyChildren"
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

  readPendingBoundaryEvents(
    request: ReadLibraryBoundaryEventsRequest
  ): Promise<ReadLibraryBoundaryEventsReply> {
    return this.sendAndExpect(
      {
        type: "libraryBoundaryEvents",
        payload: { type: "readPending", payload: request }
      },
      "libraryBoundaryEvents",
      "readPending"
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
