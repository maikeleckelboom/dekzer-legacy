import { contextBridge as rendererContext, ipcRenderer } from 'electron'

import { exposeDekzerRendererApi } from './libraryBoundaryPreload'

exposeDekzerRendererApi(rendererContext, ipcRenderer)
