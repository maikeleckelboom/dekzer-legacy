import type {
  LibraryBrowserViewStateReadResult,
  PersistedLibraryBrowserViewState
} from '../../../shared/libraryBrowser/viewState'
import type { RendererApi } from '../../../shared/rendererApi'

export type ViewStatePersistence = {
  readonly schedulePersist: (state: PersistedLibraryBrowserViewState) => void
  readonly read: () => Promise<LibraryBrowserViewStateReadResult>
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

export function createViewStatePersistence(
  api: RendererApi = getRendererApi()
): ViewStatePersistence {
  let pendingState: PersistedLibraryBrowserViewState | undefined
  let writeInFlight = false

  function drain(): void {
    if (writeInFlight || pendingState === undefined) {
      return
    }

    const state = pendingState
    pendingState = undefined
    writeInFlight = true

    api.library.browser.viewState.writeViewState(state).then(
      () => {
        writeInFlight = false
        drain()
      },
      () => {
        writeInFlight = false
        drain()
      }
    )
  }

  return {
    schedulePersist(state: PersistedLibraryBrowserViewState): void {
      pendingState = state
      drain()
    },

    read(): Promise<LibraryBrowserViewStateReadResult> {
      return api.library.browser.viewState.readViewState()
    }
  }
}
