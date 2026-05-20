import { contextBridge as rendererContext, ipcRenderer } from 'electron'

import { createDesktopApi } from './libraryBoundaryPreload'

rendererContext.exposeInMainWorld('desktop', createDesktopApi(ipcRenderer))
