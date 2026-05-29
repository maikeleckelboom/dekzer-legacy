import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../shared/libraryBoundary/status'
import {
  libraryViewStateChannels,
  type LibraryViewStateReadResult,
  type LibraryViewStateWriteResult,
  type PersistedLibraryViewState
} from '../shared/libraryViewState/viewState'
import {
  hierarchyReadChannels,
  type ReadRequest,
  type ReadResult
} from '../shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type NavigationReadRowsRequest,
  type NavigationReadRowsResult
} from '../shared/libraryNavigation/readRows'
import {
  contentsReadChannels,
  type ContentsReadResult,
  type ContentsReadRequest
} from '../shared/libraryContents/read'
import { rootChannels } from '../shared/libraryRoots/channels'
import { boundaryEventChannels } from '../shared/libraryBoundary/events'
import type {
  BoundaryEventReadAfterReply,
  BoundaryEventReadAfterRequest,
  BoundaryEventReadPendingReply,
  BoundaryEventReadPendingRequest
} from '../shared/libraryBoundary/events'
import type { LocalRootChoiceResult } from '../shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../shared/libraryRoots/readLocalRoots'
import type { LocalRootScanRequest, LocalRootScanResult } from '../shared/libraryRoots/runScan'
import type {
  UnregisterLocalRootRequest,
  UnregisterLocalRootResult
} from '../shared/libraryRoots/unregisterLocalRoot'
import type { RendererApi } from '../shared/rendererApi'

type IpcRendererEventLike = unknown

export type RendererApiPreloadIpcRenderer = {
  invoke(channel: string, ...args: readonly unknown[]): Promise<unknown>
  on(
    channel: string,
    listener: (event: IpcRendererEventLike, status: LibraryBoundaryHostStatus) => void
  ): void
  off(
    channel: string,
    listener: (event: IpcRendererEventLike, status: LibraryBoundaryHostStatus) => void
  ): void
}

export type RendererApiPreloadContextBridge = {
  exposeInMainWorld(apiKey: string, api: RendererApi): void
}

export function exposeRendererApi(
  rendererContext: RendererApiPreloadContextBridge,
  ipcRenderer: RendererApiPreloadIpcRenderer
): void {
  rendererContext.exposeInMainWorld('dekzer', createRendererApi(ipcRenderer))
}

export function createRendererApi(ipcRenderer: RendererApiPreloadIpcRenderer): RendererApi {
  return {
    library: {
      host: {
        async getStatus(): Promise<LibraryBoundaryHostStatus> {
          return (await ipcRenderer.invoke(
            hostStatusChannels.getStatus
          )) as LibraryBoundaryHostStatus
        },
        onStatusChanged(callback) {
          const listener = (
            _event: IpcRendererEventLike,
            status: LibraryBoundaryHostStatus
          ): void => {
            callback(status)
          }

          ipcRenderer.on(hostStatusChannels.statusChanged, listener)

          return () => {
            ipcRenderer.off(hostStatusChannels.statusChanged, listener)
          }
        }
      },
      navigation: {
        async readRows(request: NavigationReadRowsRequest): Promise<NavigationReadRowsResult> {
          return (await ipcRenderer.invoke(
            navigationReadChannels.readRows,
            request
          )) as NavigationReadRowsResult
        }
      },
      hierarchy: {
        async readChildren(request: ReadRequest): Promise<ReadResult> {
          return (await ipcRenderer.invoke(
            hierarchyReadChannels.readChildren,
            request
          )) as ReadResult
        }
      },
      contents: {
        async read(request: ContentsReadRequest): Promise<ContentsReadResult> {
          return (await ipcRenderer.invoke(
            contentsReadChannels.read,
            request
          )) as ContentsReadResult
        }
      },
      roots: {
        async chooseAndRegisterLocal(): Promise<LocalRootChoiceResult> {
          return (await ipcRenderer.invoke(
            rootChannels.chooseAndRegisterLocal
          )) as LocalRootChoiceResult
        },
        async runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult> {
          return (await ipcRenderer.invoke(rootChannels.runScan, request)) as LocalRootScanResult
        },
        async readLocalRoots(): Promise<ReadLocalRootsOutcome> {
          return (await ipcRenderer.invoke(rootChannels.readLocalRoots)) as ReadLocalRootsOutcome
        },
        async unregisterLocalRoot(
          request: UnregisterLocalRootRequest
        ): Promise<UnregisterLocalRootResult> {
          return (await ipcRenderer.invoke(
            rootChannels.unregisterLocalRoot,
            request
          )) as UnregisterLocalRootResult
        }
      },
      viewState: {
        readViewState: async (): Promise<LibraryViewStateReadResult> => {
          return (await ipcRenderer.invoke(
            libraryViewStateChannels.readViewState
          )) as LibraryViewStateReadResult
        },
        writeViewState: async (
          viewState: PersistedLibraryViewState
        ): Promise<LibraryViewStateWriteResult> => {
          return (await ipcRenderer.invoke(
            libraryViewStateChannels.writeViewState,
            viewState
          )) as LibraryViewStateWriteResult
        }
      },
      events: {
        async readPending(
          request: BoundaryEventReadPendingRequest
        ): Promise<BoundaryEventReadPendingReply> {
          return (await ipcRenderer.invoke(
            boundaryEventChannels.readPending,
            request
          )) as BoundaryEventReadPendingReply
        },
        async readAfter(
          request: BoundaryEventReadAfterRequest
        ): Promise<BoundaryEventReadAfterReply> {
          return (await ipcRenderer.invoke(
            boundaryEventChannels.readAfter,
            request
          )) as BoundaryEventReadAfterReply
        }
      }
    }
  }
}
