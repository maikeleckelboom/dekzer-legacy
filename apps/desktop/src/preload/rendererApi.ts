import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../shared/libraryBoundary/status'
import {
  hierarchyReadChannels,
  type LibraryHierarchyReadChildrenRequest,
  type LibraryHierarchyReadChildrenResult
} from '../shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type LibraryNavigationReadRowsRequest,
  type LibraryNavigationReadRowsResult
} from '../shared/libraryNavigation/readRows'
import { rootChannels } from '../shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootScanRequest, LocalRootScanResult } from '../shared/libraryRoots/runScan'
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
        async readRows(
          request: LibraryNavigationReadRowsRequest
        ): Promise<LibraryNavigationReadRowsResult> {
          return (await ipcRenderer.invoke(
            navigationReadChannels.readRows,
            request
          )) as LibraryNavigationReadRowsResult
        }
      },
      hierarchy: {
        async readChildren(
          request: LibraryHierarchyReadChildrenRequest
        ): Promise<LibraryHierarchyReadChildrenResult> {
          return (await ipcRenderer.invoke(
            hierarchyReadChannels.readChildren,
            request
          )) as LibraryHierarchyReadChildrenResult
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
        }
      }
    }
  }
}
