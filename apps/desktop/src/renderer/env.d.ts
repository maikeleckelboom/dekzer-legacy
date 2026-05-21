/// <reference types="vite/client" />

import type { RendererApi } from '../shared/rendererApi'

declare global {
  interface Window {
    readonly dekzer: RendererApi
  }
}
