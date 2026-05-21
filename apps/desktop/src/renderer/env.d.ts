/// <reference types="vite/client" />

import type { DekzerRendererApi } from '../shared/libraryBoundary/status'

declare global {
  interface Window {
    readonly dekzer: DekzerRendererApi
  }
}
