import { contextBridge as rendererContext, ipcRenderer } from 'electron'

import { exposeDekzerRendererApi } from './libraryBoundary'

exposeDekzerRendererApi(rendererContext, ipcRenderer)
