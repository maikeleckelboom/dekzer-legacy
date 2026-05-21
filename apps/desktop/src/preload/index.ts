import { contextBridge as rendererContext, ipcRenderer } from 'electron'

import { exposeRendererApi } from './rendererApi'

exposeRendererApi(rendererContext, ipcRenderer)
