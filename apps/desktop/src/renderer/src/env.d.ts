/// <reference types="vite/client" />

import type { DesktopApi } from '../../shared/libraryBoundaryStatus'

declare global {
  interface Window {
    readonly desktop: DesktopApi
  }
}
