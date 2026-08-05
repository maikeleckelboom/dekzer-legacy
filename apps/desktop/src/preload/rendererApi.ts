import { libraryPublicationChannels } from '../shared/library/boundary/publicationPlane'
import { libraryControlChannels } from '../shared/library/boundary/controlPlane'
import { type LibraryBoundaryHostStatus } from '../shared/library/boundary/status'
import {
  type LibraryViewStateReadResult,
  type LibraryViewStateWriteResult,
  type PersistedLibraryViewState
} from '../shared/library/viewState/persistence'
import { type ReadRequest, type ReadResult } from '../shared/library/hierarchy/read'
import type {
  ReadLocalBrowseItemsOutcome,
  ReadLocalBrowseItemsRequest
} from '../shared/library/localBrowse/items'
import type { ReadLocalBrowseEntryPointsOutcome } from '../shared/library/localBrowse/entryPoints'
import {
  type NavigationReadRowsRequest,
  type NavigationReadRowsResult
} from '../shared/library/navigation/read'
import { type ContentsReadResult, type ContentsReadRequest } from '../shared/library/contents/read'
import type {
  SearchFilterReadRequest,
  SearchFilterReadResult
} from '../shared/library/searchFilter/read'

import type {
  ReadSourceLifecycleResult,
  ReadSourceLifecycleRequest
} from '../shared/library/source/lifecycle'
import type {
  ReadSourceIntegrityRequest,
  SourceIntegrityReadResult
} from '../shared/library/source/integrity'
import type {
  ReadSourceActivityRequest,
  SourceActivityReadResult
} from '../shared/library/source/activity'

import type {
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from '../shared/library/attachmentIdentity/read'
import {
  type HashSourceFilesBlake3Request,
  type HashSourceFilesBlake3Result
} from '../shared/library/source/fileHashing'
import {
  type ReadSourceMaintenanceRequest,
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceRequest,
  type RunSourceMaintenanceResult
} from '../shared/library/source/maintenance'
import {
  type TrackIdentityDecisionRequest,
  type TrackIdentityDecisionCommandResult
} from '../shared/library/trackIdentity/decisions'
import {
  type ReadCandidatesRequest,
  type ReadCandidatesResult
} from '../shared/library/trackIdentity/candidates'
import type {
  MusicalAnalysisRequest,
  MusicalAnalysisResult
} from '../shared/library/musicalAnalysis/analyze'

import type {
  BoundaryEventDeliveryPayload,
  BoundaryEventSubscribeResult,
  BoundaryEventUnsubscribeResult
} from '../shared/library/boundary/events'
import type { LocalRootChoiceResult } from '../shared/library/roots/chooseLocal'
import type { ReadLocalRootsOutcome } from '../shared/library/roots/read'
import type {
  LocalRootRegistrationRequest,
  LocalRootRegistrationResult
} from '../shared/library/roots/register'
import type { LocalRootScanRequest, LocalRootScanResult } from '../shared/library/roots/scan'
import type { CancelRootScanRequest, CancelRootScanResult } from '../shared/library/roots/cancel'
import type {
  UnregisterLocalRootRequest,
  UnregisterLocalRootResult
} from '../shared/library/roots/unregister'
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
            libraryControlChannels.boundary.getStatus
          )) as LibraryBoundaryHostStatus
        },
        onStatusChanged(callback) {
          const listener = (_event: IpcRendererEventLike, payload: unknown): void => {
            callback(payload as LibraryBoundaryHostStatus)
          }

          ipcRenderer.on(libraryPublicationChannels.boundary.statusChanged, listener)

          return () => {
            ipcRenderer.off(libraryPublicationChannels.boundary.statusChanged, listener)
          }
        }
      },
      localBrowse: {
        async readEntryPoints(): Promise<ReadLocalBrowseEntryPointsOutcome> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.localBrowse.entryPoints.read
          )) as ReadLocalBrowseEntryPointsOutcome
        },
        async readItems(
          request: ReadLocalBrowseItemsRequest
        ): Promise<ReadLocalBrowseItemsOutcome> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.localBrowse.items.read,
            request
          )) as ReadLocalBrowseItemsOutcome
        }
      },
      navigation: {
        async readRows(request: NavigationReadRowsRequest): Promise<NavigationReadRowsResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.navigation.read,
            request
          )) as NavigationReadRowsResult
        }
      },
      hierarchy: {
        async readChildren(request: ReadRequest): Promise<ReadResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.hierarchy.read,
            request
          )) as ReadResult
        }
      },
      sourceLifecycle: {
        async readSourceLifecycle(
          request: ReadSourceLifecycleRequest
        ): Promise<ReadSourceLifecycleResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.lifecycle,
            request
          )) as ReadSourceLifecycleResult
        }
      },
      sourceIntegrity: {
        async readSourceIntegrity(
          request: ReadSourceIntegrityRequest
        ): Promise<SourceIntegrityReadResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.integrity,
            request
          )) as SourceIntegrityReadResult
        }
      },
      sourceActivity: {
        async readSourceActivity(
          request: ReadSourceActivityRequest
        ): Promise<SourceActivityReadResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.activity,
            request
          )) as SourceActivityReadResult
        }
      },
      attachmentIdentity: {
        async readSourceFileAttachment(
          request: ReadSourceFileAttachmentRequest
        ): Promise<ReadSourceFileAttachmentResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.attachmentIdentity.readSourceFileAttachment,
            request
          )) as ReadSourceFileAttachmentResult
        },
        async readAttachmentSourceFiles(
          request: ReadAttachmentSourceFilesRequest
        ): Promise<ReadAttachmentSourceFilesResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.attachmentIdentity.readAttachmentSourceFiles,
            request
          )) as ReadAttachmentSourceFilesResult
        },
        async readSourceAttachmentSummary(
          request: ReadSourceAttachmentSummaryRequest
        ): Promise<ReadSourceAttachmentSummaryResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.attachmentIdentity.readSourceAttachmentSummary,
            request
          )) as ReadSourceAttachmentSummaryResult
        }
      },
      hashing: {
        async hashSourceFilesBlake3(
          request: HashSourceFilesBlake3Request
        ): Promise<HashSourceFilesBlake3Result> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.fileHashing,
            request
          )) as HashSourceFilesBlake3Result
        }
      },
      sourceMaintenance: {
        async runSourceMaintenance(
          request: RunSourceMaintenanceRequest
        ): Promise<RunSourceMaintenanceResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.maintenance.run,
            request
          )) as RunSourceMaintenanceResult
        },
        async readSourceMaintenance(
          request: ReadSourceMaintenanceRequest
        ): Promise<ReadSourceMaintenanceResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.source.maintenance.read,
            request
          )) as ReadSourceMaintenanceResult
        }
      },
      musicalAnalysis: {
        async analyzePlayableMedia(
          request: MusicalAnalysisRequest
        ): Promise<MusicalAnalysisResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.musicalAnalysis.analyzePlayableMedia,
            request
          )) as MusicalAnalysisResult
        }
      },
      trackIdentityDecisions: {
        async acceptTrackIdentityCandidate(
          request: TrackIdentityDecisionRequest
        ): Promise<TrackIdentityDecisionCommandResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.trackIdentity.decisions.accept,
            request
          )) as TrackIdentityDecisionCommandResult
        },
        async rejectTrackIdentityCandidate(
          request: TrackIdentityDecisionRequest
        ): Promise<TrackIdentityDecisionCommandResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.trackIdentity.decisions.reject,
            request
          )) as TrackIdentityDecisionCommandResult
        },
        async deferTrackIdentityCandidate(
          request: TrackIdentityDecisionRequest
        ): Promise<TrackIdentityDecisionCommandResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.trackIdentity.decisions.defer,
            request
          )) as TrackIdentityDecisionCommandResult
        }
      },
      trackIdentityReview: {
        async readTrackIdentityReviewCandidates(
          request: ReadCandidatesRequest
        ): Promise<ReadCandidatesResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.trackIdentity.candidates.read,
            request
          )) as ReadCandidatesResult
        }
      },
      contents: {
        async read(request: ContentsReadRequest): Promise<ContentsReadResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.contents.read,
            request
          )) as ContentsReadResult
        }
      },
      searchFilter: {
        async read(request: SearchFilterReadRequest): Promise<SearchFilterReadResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.searchFilter.read,
            request
          )) as SearchFilterReadResult
        }
      },
      roots: {
        async chooseAndRegisterLocal(): Promise<LocalRootChoiceResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.chooseLocal
          )) as LocalRootChoiceResult
        },
        async registerLocalPath(
          request: LocalRootRegistrationRequest
        ): Promise<LocalRootRegistrationResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.registerLocalPath,
            request
          )) as LocalRootRegistrationResult
        },
        async runScan(request: LocalRootScanRequest): Promise<LocalRootScanResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.scan,
            request
          )) as LocalRootScanResult
        },
        async cancelScan(request: CancelRootScanRequest): Promise<CancelRootScanResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.cancel,
            request
          )) as CancelRootScanResult
        },
        async readLocalRoots(): Promise<ReadLocalRootsOutcome> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.read
          )) as ReadLocalRootsOutcome
        },
        async unregisterLocalRoot(
          request: UnregisterLocalRootRequest
        ): Promise<UnregisterLocalRootResult> {
          return (await ipcRenderer.invoke(
            libraryControlChannels.roots.unregister,
            request
          )) as UnregisterLocalRootResult
        }
      },
      viewState: {
        readViewState: async (): Promise<LibraryViewStateReadResult> => {
          return (await ipcRenderer.invoke(
            libraryControlChannels.viewState.read
          )) as LibraryViewStateReadResult
        },
        writeViewState: async (
          viewState: PersistedLibraryViewState
        ): Promise<LibraryViewStateWriteResult> => {
          return (await ipcRenderer.invoke(
            libraryControlChannels.viewState.write,
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

          ipcRenderer.on(libraryPublicationChannels.boundary.events.batch, listener)
          void ipcRenderer
            .invoke(libraryControlChannels.boundary.events.subscribe)
            .then((result) => {
              const subscribeResult = result as BoundaryEventSubscribeResult
              if (!active) {
                void ipcRenderer
                  .invoke(libraryControlChannels.boundary.events.unsubscribe)
                  .catch(() => undefined)
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
            ipcRenderer.off(libraryPublicationChannels.boundary.events.batch, listener)
            void ipcRenderer
              .invoke(libraryControlChannels.boundary.events.unsubscribe)
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
