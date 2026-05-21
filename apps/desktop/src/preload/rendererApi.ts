import {
  libraryBoundaryHostStatusIpcChannels,
  type LibraryBoundaryHostStatus
} from '../shared/libraryBoundary/status'
import {
  libraryHierarchyReadChildrenIpcChannels,
  type LibraryHierarchyReadChildrenRequest,
  type LibraryHierarchyReadChildrenResult
} from '../shared/libraryHierarchy/readChildren'
import {
  libraryRootsIpcChannels,
  type LocalRootRegistrationRequest,
  type LocalRootRegistrationResult
} from '../shared/libraryRoots/registerLocalRoot'
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
            libraryBoundaryHostStatusIpcChannels.getStatus
          )) as LibraryBoundaryHostStatus
        },
        onStatusChanged(callback) {
          const listener = (
            _event: IpcRendererEventLike,
            status: LibraryBoundaryHostStatus
          ): void => {
            callback(status)
          }

          ipcRenderer.on(libraryBoundaryHostStatusIpcChannels.statusChanged, listener)

          return () => {
            ipcRenderer.off(libraryBoundaryHostStatusIpcChannels.statusChanged, listener)
          }
        }
      },
      hierarchy: {
        async readChildren(
          request: LibraryHierarchyReadChildrenRequest
        ): Promise<LibraryHierarchyReadChildrenResult> {
          return (await ipcRenderer.invoke(
            libraryHierarchyReadChildrenIpcChannels.readChildren,
            request
          )) as LibraryHierarchyReadChildrenResult
        }
      },
      roots: {
        async registerLocal(
          request: LocalRootRegistrationRequest
        ): Promise<LocalRootRegistrationResult> {
          return (await ipcRenderer.invoke(
            libraryRootsIpcChannels.registerLocal,
            request
          )) as LocalRootRegistrationResult
        }
      }
    }
  }
}
