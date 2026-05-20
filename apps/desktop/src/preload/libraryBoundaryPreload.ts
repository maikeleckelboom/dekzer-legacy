import {
  libraryBoundaryHostStatusIpcChannels,
  type DekzerRendererApi,
  type LibraryBoundaryHostStatus
} from '../shared/libraryBoundaryStatus'

type IpcRendererEventLike = unknown

export type LibraryBoundaryPreloadIpcRenderer = {
  invoke(channel: string): Promise<unknown>
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
