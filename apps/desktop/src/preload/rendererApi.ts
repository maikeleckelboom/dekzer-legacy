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
import { sourceLifecycleReadChannels } from '../shared/librarySourceLifecycle/channels'
import type {
  ReadSourceLifecycleResult,
  ReadSourceLifecycleRequest
} from '../shared/librarySourceLifecycle/readSourceLifecycle'
import { attachmentIdentityReadChannels } from '../shared/libraryAttachmentIdentity/channels'
import type {
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from '../shared/libraryAttachmentIdentity/read'
import {
  sourceFileHashingChannels,
  type HashSourceFilesBlake3Request,
  type HashSourceFilesBlake3Result
} from '../shared/librarySourceFileHashing/hashSourceFilesBlake3'
import {
  sourceMaintenanceChannels,
  type ReadSourceMaintenanceRequest,
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceRequest,
  type RunSourceMaintenanceResult
} from '../shared/librarySourceMaintenance/sourceMaintenance'
import {
  trackIdentityDecisionWriteChannels,
  type TrackIdentityDecisionWriteRequest,
  type TrackIdentityDecisionWriteResult
} from '../shared/libraryTrackIdentityDecisionWrite/decisionWrite'
import { rootChannels } from '../shared/libraryRoots/channels'
import { boundaryEventChannels } from '../shared/libraryBoundary/events'
import type {
  BoundaryEventDeliveryPayload,
  BoundaryEventSubscribeResult,
  BoundaryEventUnsubscribeResult
} from '../shared/libraryBoundary/events'
import type { LocalRootChoiceResult } from '../shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../shared/libraryRoots/readLocalRoots'
import type { LocalRootScanRequest, LocalRootScanResult } from '../shared/libraryRoots/runScan'
import type { CancelRootScanRequest, CancelRootScanResult } from '../shared/libraryRoots/cancelScan'
import type {
  UnregisterLocalRootRequest,
  UnregisterLocalRootResult
} from '../shared/libraryRoots/unregisterLocalRoot'
import type { RendererApi } from '../shared/rendererApi'

type IpcRendererEventLike = unknown

export type RendererApiPreloadIpcRenderer = {
  invoke(channel: string, ...args: readonly unknown[]): Promise<unknown>
  on(channel: string, listener: (event: IpcRendererEventLike, payload: unknown) => void): void
  off(channel: string, listener: (event: IpcRendererEventLike, payload: unknown) => void): void
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
          const listener = (_event: IpcRendererEventLike, payload: unknown): void => {
            callback(payload as LibraryBoundaryHostStatus)
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
      sourceLifecycle: {
        async readSourceLifecycle(
          request: ReadSourceLifecycleRequest
        ): Promise<ReadSourceLifecycleResult> {
          return (await ipcRenderer.invoke(
            sourceLifecycleReadChannels.readSourceLifecycle,
            request
          )) as ReadSourceLifecycleResult
        }
      },
      attachmentIdentity: {
        async readSourceFileAttachment(
          request: ReadSourceFileAttachmentRequest
        ): Promise<ReadSourceFileAttachmentResult> {
          return (await ipcRenderer.invoke(
            attachmentIdentityReadChannels.readSourceFileAttachment,
            request
          )) as ReadSourceFileAttachmentResult
        },
        async readAttachmentSourceFiles(
          request: ReadAttachmentSourceFilesRequest
        ): Promise<ReadAttachmentSourceFilesResult> {
          return (await ipcRenderer.invoke(
            attachmentIdentityReadChannels.readAttachmentSourceFiles,
            request
          )) as ReadAttachmentSourceFilesResult
        },
        async readSourceAttachmentSummary(
          request: ReadSourceAttachmentSummaryRequest
        ): Promise<ReadSourceAttachmentSummaryResult> {
          return (await ipcRenderer.invoke(
            attachmentIdentityReadChannels.readSourceAttachmentSummary,
            request
          )) as ReadSourceAttachmentSummaryResult
        }
      },
      hashing: {
        async hashSourceFilesBlake3(
          request: HashSourceFilesBlake3Request
        ): Promise<HashSourceFilesBlake3Result> {
          return (await ipcRenderer.invoke(
            sourceFileHashingChannels.hashSourceFilesBlake3,
            request
          )) as HashSourceFilesBlake3Result
        }
      },
      sourceMaintenance: {
        async runSourceMaintenance(
          request: RunSourceMaintenanceRequest
        ): Promise<RunSourceMaintenanceResult> {
          return (await ipcRenderer.invoke(
            sourceMaintenanceChannels.runSourceMaintenance,
            request
          )) as RunSourceMaintenanceResult
        },
        async readSourceMaintenance(
          request: ReadSourceMaintenanceRequest
        ): Promise<ReadSourceMaintenanceResult> {
          return (await ipcRenderer.invoke(
            sourceMaintenanceChannels.readSourceMaintenance,
            request
          )) as ReadSourceMaintenanceResult
        }
      },
      trackIdentityDecisions: {
        async acceptTrackIdentityCandidate(
          request: TrackIdentityDecisionWriteRequest
        ): Promise<TrackIdentityDecisionWriteResult> {
          return (await ipcRenderer.invoke(
            trackIdentityDecisionWriteChannels.acceptTrackIdentityCandidate,
            request
          )) as TrackIdentityDecisionWriteResult
        },
        async rejectTrackIdentityCandidate(
          request: TrackIdentityDecisionWriteRequest
        ): Promise<TrackIdentityDecisionWriteResult> {
          return (await ipcRenderer.invoke(
            trackIdentityDecisionWriteChannels.rejectTrackIdentityCandidate,
            request
          )) as TrackIdentityDecisionWriteResult
        },
        async deferTrackIdentityCandidate(
          request: TrackIdentityDecisionWriteRequest
        ): Promise<TrackIdentityDecisionWriteResult> {
          return (await ipcRenderer.invoke(
            trackIdentityDecisionWriteChannels.deferTrackIdentityCandidate,
            request
          )) as TrackIdentityDecisionWriteResult
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
        async cancelScan(request: CancelRootScanRequest): Promise<CancelRootScanResult> {
          return (await ipcRenderer.invoke(
            rootChannels.cancelScan,
            request
          )) as CancelRootScanResult
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
        subscribe(callback) {
          let active = true
          const listener = (_event: IpcRendererEventLike, payload: unknown): void => {
            callback(payload as BoundaryEventDeliveryPayload)
          }

          ipcRenderer.on(boundaryEventChannels.batch, listener)
          void ipcRenderer
            .invoke(boundaryEventChannels.subscribe)
            .then((result) => {
              const subscribeResult = result as BoundaryEventSubscribeResult
              if (!active) {
                void ipcRenderer.invoke(boundaryEventChannels.unsubscribe).catch(() => undefined)
                return
              }
              if (subscribeResult.kind === 'failed') {
                callback({ kind: 'failed', detail: subscribeResult.detail })
              }
            })
            .catch((error: unknown) => {
              if (!active) {
                return
              }
              callback({
                kind: 'failed',
                detail:
                  error instanceof Error ? error.message : 'boundary event subscription failed'
              })
            })

          return () => {
            active = false
            ipcRenderer.off(boundaryEventChannels.batch, listener)
            void ipcRenderer
              .invoke(boundaryEventChannels.unsubscribe)
              .then((result) => {
                const unsubscribeResult = result as BoundaryEventUnsubscribeResult
                if (unsubscribeResult.kind === 'failed') {
                  callback({ kind: 'failed', detail: unsubscribeResult.detail })
                }
              })
              .catch(() => undefined)
          }
        }
      }
    }
  }
}
