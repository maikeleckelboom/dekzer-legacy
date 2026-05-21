import {
  libraryBoundaryHostStatusIpcChannels,
  type DekzerRendererApi,
  type LibraryBoundaryHostStatus
} from '../shared/libraryBoundary/status'
import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadRequest,
  type LibraryHierarchyReadResult
} from '../shared/libraryHierarchy/read'

type IpcRendererEventLike = unknown

export type LibraryBoundaryPreloadIpcRenderer = {
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

export type LibraryBoundaryPreloadContextBridge = {
  exposeInMainWorld(apiKey: string, api: DekzerRendererApi): void
}

export function exposeDekzerRendererApi(
  rendererContext: LibraryBoundaryPreloadContextBridge,
  ipcRenderer: LibraryBoundaryPreloadIpcRenderer
): void {
  rendererContext.exposeInMainWorld('dekzer', createDekzerRendererApi(ipcRenderer))
}

export function createDekzerRendererApi(
  ipcRenderer: LibraryBoundaryPreloadIpcRenderer
): DekzerRendererApi {
  return {
    libraryBoundary: {
      async getStatus(): Promise<LibraryBoundaryHostStatus> {
        return (await ipcRenderer.invoke(
          libraryBoundaryHostStatusIpcChannels.getStatus
        )) as LibraryBoundaryHostStatus
      },
      async readLiteralHierarchyChildren(
        request: LibraryHierarchyReadRequest
      ): Promise<LibraryHierarchyReadResult> {
        return (await ipcRenderer.invoke(
          libraryHierarchyReadIpcChannels.readLiteralHierarchyChildren,
          request
        )) as LibraryHierarchyReadResult
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
    }
  }
}
