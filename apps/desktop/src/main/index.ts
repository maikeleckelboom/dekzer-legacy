import { libraryPublicationChannels } from '../shared/library/boundary/publicationPlane'
import { app, BrowserWindow, dialog, ipcMain, shell } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'
import { createLibraryBoundaryHost } from './library/boundary/host'
import { registerReadChildrenIpc } from './library/hierarchy/read'
import { registerReadNavigationRowsIpc } from './library/navigation/read'
import { registerContentsReadIpc } from './library/contents/read'
import { registerReadSourceLifecycleIpc } from './library/source/lifecycle'
import { registerAttachmentIdentityReadIpc } from './library/attachmentIdentity/read'
import { registerSourceFileHashingIpc } from './library/source/fileHashing'
import { registerSourceMaintenanceIpc } from './library/source/maintenance'
import { registerTrackIdentityDecisionIpc } from './library/trackIdentity/decisions'
import { registerTrackIdentityReviewCandidatesIpc } from './library/trackIdentity/candidates'
import { registerLocalRootChoiceIpc } from './library/roots/chooseLocal'
import { registerLocalRootScanIpc } from './library/roots/scan'
import { registerCancelRootScanIpc } from './library/roots/cancel'
import { registerReadLocalRootsIpc } from './library/roots/read'
import { registerUnregisterLocalRootIpc } from './library/roots/unregister'
import { registerLibraryViewStateIpc } from './library/viewState/persistence'
import { BoundaryEventPump, registerBoundaryEventPumpIpc } from './library/boundary/eventPump'
import { HostStatusController, registerHostStatusIpc } from './library/boundary/status'

const appUserModelId = 'com.dekzer.desktop'
const windowTitle = 'Dekzer'
let hostStatusController: HostStatusController | undefined
let boundaryEventPump: BoundaryEventPump | undefined
let isQuittingAfterLibraryBoundaryHostStop = false

function createWindow(): void {
  const mainWindow = new BrowserWindow({
    width: 1120,
    height: 720,
    minWidth: 820,
    minHeight: 560,
    show: false,
    title: windowTitle,
    autoHideMenuBar: true,
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      sandbox: true
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.setName(windowTitle)

app.whenReady().then(() => {
  const host = createLibraryBoundaryHost({
    app,
    isDev: is.dev
  })
  hostStatusController = new HostStatusController(host)
  boundaryEventPump = new BoundaryEventPump(host)
  registerHostStatusIpc(ipcMain, hostStatusController)
  registerReadNavigationRowsIpc(ipcMain, host)
  registerReadChildrenIpc(ipcMain, host)
  registerReadSourceLifecycleIpc(ipcMain, host)
  registerAttachmentIdentityReadIpc(ipcMain, host)
  registerSourceFileHashingIpc(ipcMain, host)
  registerSourceMaintenanceIpc(ipcMain, host)
  registerTrackIdentityDecisionIpc(ipcMain, host)
  registerTrackIdentityReviewCandidatesIpc(ipcMain, host)
  registerContentsReadIpc(ipcMain, host)
  registerLocalRootChoiceIpc(ipcMain, host, {
    dialog,
    getParentWindow: getLibraryRootChoiceParentWindow
  })
  registerLocalRootScanIpc(ipcMain, host)
  registerCancelRootScanIpc(ipcMain, host)
  registerReadLocalRootsIpc(ipcMain, host)
  registerUnregisterLocalRootIpc(ipcMain, host)
  registerLibraryViewStateIpc(ipcMain, host)
  registerBoundaryEventPumpIpc(ipcMain, boundaryEventPump)
  hostStatusController.onStatusChanged((status) => {
    boundaryEventPump?.setHostStarted(status.state === 'started')
    for (const window of BrowserWindow.getAllWindows()) {
      window.webContents.send(libraryPublicationChannels.boundary.statusChanged, status)
    }
  })

  electronApp.setAppUserModelId(appUserModelId)

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  createWindow()
  void hostStatusController.start()

  app.on('activate', function () {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

function getLibraryRootChoiceParentWindow(): BrowserWindow | undefined {
  const focusedWindow = BrowserWindow.getFocusedWindow()

  if (focusedWindow) {
    return focusedWindow
  }

  return BrowserWindow.getAllWindows()[0]
}

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('before-quit', (event) => {
  if (
    isQuittingAfterLibraryBoundaryHostStop ||
    hostStatusController === undefined ||
    !hostStatusController.hasStarted
  ) {
    return
  }

  event.preventDefault()
  boundaryEventPump?.stop()
  void hostStatusController
    .stop()
    .catch((error: unknown) => {
      console.error('[library-boundary-host] failed to stop cleanly', error)
    })
    .finally(() => {
      isQuittingAfterLibraryBoundaryHostStop = true
      app.quit()
    })
})
