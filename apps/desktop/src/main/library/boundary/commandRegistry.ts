import { libraryControlChannels } from '../../../shared/library/boundary/controlPlane'
import {
  readAttachmentSourceFilesThroughHost,
  readSourceAttachmentSummaryThroughHost,
  readSourceFileAttachmentThroughHost
} from '../attachmentIdentity/read'
import { readContentsThroughHost } from '../contents/read'
import { readThroughHost } from '../hierarchy/read'
import { readItems as readLocalBrowseItems } from '../localBrowse/items'
import { readEntryPoints as readLocalBrowseEntryPoints } from '../localBrowse/entryPoints'
import { readNavigationRowsThroughHost } from '../navigation/read'
import { analyzePlayableMediaThroughHost } from '../musicalAnalysis/analyze'
import { readSearchFilterThroughHost } from '../searchFilter/read'
import { cancelRootScanThroughHost, type CancelScanLogger } from '../roots/cancel'
import { chooseAndRegisterLocalRoot, type LocalRootChoiceDependencies } from '../roots/chooseLocal'
import { readLocalRootsThroughHost } from '../roots/read'
import { registerLocalRoot } from '../roots/register'
import { runLocalRootScanThroughHost, type ScanLogger } from '../roots/scan'
import { unregisterLocalRootThroughHost } from '../roots/unregister'
import {
  hashSourceFilesBlake3ThroughHost,
  type SourceFileHashingLogger
} from '../source/fileHashing'
import { readSourceLifecycleThroughHost } from '../source/lifecycle'
import { readSourceIntegrityThroughHost } from '../source/integrity'
import { readSourceActivityThroughHost } from '../source/activity'
import {
  readSourceMaintenanceThroughHost,
  runSourceMaintenanceThroughHost
} from '../source/maintenance'
import { readCandidatesThroughHost } from '../trackIdentity/candidates'
import {
  acceptTrackIdentityCandidateThroughHost,
  deferTrackIdentityCandidateThroughHost,
  rejectTrackIdentityCandidateThroughHost,
  type TrackIdentityDecisionCommandLogger
} from '../trackIdentity/decisions'
import { readViewStateFromHost, writeViewStateThroughHost } from '../viewState/persistence'
import {
  type BoundaryEventPump,
  subscribeBoundaryEventsForSender,
  unsubscribeBoundaryEventsForSender
} from './eventPump'
import type { LibraryBoundaryHost } from './host'
import type { HostStatusController } from './status'
import type { ReadLocalBrowseItemsRequest } from '../../../shared/library/localBrowse/items'
import type { LocalRootRegistrationRequest } from '../../../shared/library/roots/register'
import type { UnregisterLocalRootRequest } from '../../../shared/library/roots/unregister'

export type LibraryControlPlaneIpcMain = {
  handle(channel: string, listener: (event: unknown, ...args: readonly unknown[]) => unknown): void
}

export type LibraryControlPlaneIpcEvent = {
  readonly sender: Parameters<BoundaryEventPump['subscribe']>[0]
}

export type RegisterLibraryIpcCommandsOptions = {
  readonly ipcMain: LibraryControlPlaneIpcMain
  readonly host: LibraryBoundaryHost
  readonly hostStatusController: HostStatusController
  readonly boundaryEventPump: BoundaryEventPump
  readonly localRootChoiceDependencies: LocalRootChoiceDependencies
  readonly localRootScanLogger?: ScanLogger
  readonly cancelRootScanLogger?: CancelScanLogger
  readonly sourceFileHashingLogger?: SourceFileHashingLogger
  readonly trackIdentityDecisionLogger?: TrackIdentityDecisionCommandLogger
}

export function registerLibraryIpcCommands(options: RegisterLibraryIpcCommandsOptions): void {
  const {
    ipcMain,
    host,
    hostStatusController,
    boundaryEventPump,
    localRootChoiceDependencies,
    localRootScanLogger,
    cancelRootScanLogger,
    sourceFileHashingLogger,
    trackIdentityDecisionLogger
  } = options

  ipcMain.handle(libraryControlChannels.boundary.getStatus, () => hostStatusController.getStatus())
  ipcMain.handle(libraryControlChannels.boundary.events.subscribe, (event) =>
    subscribeBoundaryEventsForSender(
      boundaryEventPump,
      (event as LibraryControlPlaneIpcEvent).sender
    )
  )
  ipcMain.handle(libraryControlChannels.boundary.events.unsubscribe, (event) =>
    unsubscribeBoundaryEventsForSender(
      boundaryEventPump,
      (event as LibraryControlPlaneIpcEvent).sender
    )
  )
  ipcMain.handle(libraryControlChannels.navigation.read, (_event, request) =>
    readNavigationRowsThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.localBrowse.entryPoints.read, () =>
    readLocalBrowseEntryPoints(host)
  )
  ipcMain.handle(libraryControlChannels.localBrowse.items.read, (_event, request) =>
    readLocalBrowseItems(host, request as ReadLocalBrowseItemsRequest)
  )
  ipcMain.handle(libraryControlChannels.musicalAnalysis.analyzePlayableMedia, (_event, request) =>
    analyzePlayableMediaThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.hierarchy.read, (_event, request) =>
    readThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.source.lifecycle, (_event, request) =>
    readSourceLifecycleThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.source.integrity, (_event, request) =>
    readSourceIntegrityThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.source.activity, (_event, request) =>
    readSourceActivityThroughHost(host, request)
  )
  ipcMain.handle(
    libraryControlChannels.attachmentIdentity.readSourceFileAttachment,
    (_event, request) => readSourceFileAttachmentThroughHost(host, request)
  )
  ipcMain.handle(
    libraryControlChannels.attachmentIdentity.readAttachmentSourceFiles,
    (_event, request) => readAttachmentSourceFilesThroughHost(host, request)
  )
  ipcMain.handle(
    libraryControlChannels.attachmentIdentity.readSourceAttachmentSummary,
    (_event, request) => readSourceAttachmentSummaryThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.source.fileHashing, (_event, request) =>
    hashSourceFilesBlake3ThroughHost(host, request, sourceFileHashingLogger)
  )
  ipcMain.handle(libraryControlChannels.source.maintenance.run, (_event, request) =>
    runSourceMaintenanceThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.source.maintenance.read, (_event, request) =>
    readSourceMaintenanceThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.trackIdentity.decisions.accept, (_event, request) =>
    acceptTrackIdentityCandidateThroughHost(host, request, trackIdentityDecisionLogger)
  )
  ipcMain.handle(libraryControlChannels.trackIdentity.decisions.reject, (_event, request) =>
    rejectTrackIdentityCandidateThroughHost(host, request, trackIdentityDecisionLogger)
  )
  ipcMain.handle(libraryControlChannels.trackIdentity.decisions.defer, (_event, request) =>
    deferTrackIdentityCandidateThroughHost(host, request, trackIdentityDecisionLogger)
  )
  ipcMain.handle(libraryControlChannels.trackIdentity.candidates.read, (_event, request) =>
    readCandidatesThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.contents.read, (_event, request) =>
    readContentsThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.searchFilter.read, (_event, request) =>
    readSearchFilterThroughHost(host, request)
  )
  ipcMain.handle(libraryControlChannels.roots.chooseLocal, () =>
    chooseAndRegisterLocalRoot(host, localRootChoiceDependencies)
  )
  ipcMain.handle(libraryControlChannels.roots.registerLocalPath, (_event, request) =>
    registerLocalRoot(host, request as LocalRootRegistrationRequest)
  )
  ipcMain.handle(libraryControlChannels.roots.scan, (_event, request) =>
    runLocalRootScanThroughHost(host, request, localRootScanLogger)
  )
  ipcMain.handle(libraryControlChannels.roots.cancel, (_event, request) =>
    cancelRootScanThroughHost(host, request, cancelRootScanLogger)
  )
  ipcMain.handle(libraryControlChannels.roots.read, () => readLocalRootsThroughHost(host))
  ipcMain.handle(libraryControlChannels.roots.unregister, (_event, request) =>
    unregisterLocalRootThroughHost(host, request as UnregisterLocalRootRequest)
  )
  ipcMain.handle(libraryControlChannels.viewState.read, () => readViewStateFromHost(host))
  ipcMain.handle(libraryControlChannels.viewState.write, (_event, viewState) =>
    writeViewStateThroughHost(host, viewState)
  )
}
