/// <reference types="vite/client" />

import type { DekzerRendererApi } from '../../shared/libraryBoundaryStatus'

declare global {
  interface Window {
    readonly dekzer: DekzerRendererApi
  }
}
