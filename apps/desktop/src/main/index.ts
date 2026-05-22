import { app, BrowserWindow, dialog, ipcMain, shell } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'
import { createLibraryBoundaryHost } from './libraryBoundary/host'
import { registerReadChildrenIpc } from './libraryHierarchy/readChildren'
import { registerReadNavigationRowsIpc } from './libraryNavigation/readRows'
import { registerLocalRootChoiceIpc } from './libraryRoots/chooseAndRegisterLocal'
import { registerLocalRootScanIpc } from './libraryRoots/runScan'
import { registerReadLocalRootsIpc } from './libraryRoots/readLocalRoots'
import {
  LibraryBoundaryHostStatusController,
  registerLibraryBoundaryHostStatusIpc
} from './libraryBoundary/status'
import { hostStatusChannels } from '../shared/libraryBoundary/status'

const appUserModelId = 'com.dekzer.desktop'
const windowTitle = 'Dekzer'
let libraryBoundaryHostStatusController: LibraryBoundaryHostStatusController | undefined
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
  libraryBoundaryHostStatusController = new LibraryBoundaryHostStatusController(host)
  registerLibraryBoundaryHostStatusIpc(ipcMain, libraryBoundaryHostStatusController)
  registerReadNavigationRowsIpc(ipcMain, host)
  registerReadChildrenIpc(ipcMain, host)
  registerLocalRootChoiceIpc(ipcMain, host, {
    dialog,
    getParentWindow: getLibraryRootChoiceParentWindow
  })
  registerLocalRootScanIpc(ipcMain, host)
  registerReadLocalRootsIpc(ipcMain, host)
  libraryBoundaryHostStatusController.onStatusChanged((status) => {
    for (const window of BrowserWindow.getAllWindows()) {
      window.webContents.send(hostStatusChannels.statusChanged, status)
    }
  })

  electronApp.setAppUserModelId(appUserModelId)

  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  createWindow()
  void libraryBoundaryHostStatusController.start()

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
    libraryBoundaryHostStatusController === undefined ||
    !libraryBoundaryHostStatusController.hasStarted
  ) {
    return
  }

  event.preventDefault()
  void libraryBoundaryHostStatusController
    .stop()
    .catch((error: unknown) => {
      console.error('[library-boundary-host] failed to stop cleanly', error)
    })
    .finally(() => {
      isQuittingAfterLibraryBoundaryHostStop = true
      app.quit()
    })
})
