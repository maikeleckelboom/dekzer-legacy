import type {
  LibraryBrowserViewStateReadResult,
  LibraryBrowserViewStateWriteResult,
  PersistedLibraryBrowserViewState
} from '../../../shared/libraryBrowser/viewState'

export type ViewStateApi = {
  readonly library: {
    readonly browser: {
      readonly viewState: {
        readonly readViewState: () => Promise<LibraryBrowserViewStateReadResult>
        readonly writeViewState: (
          state: PersistedLibraryBrowserViewState
        ) => Promise<LibraryBrowserViewStateWriteResult>
      }
    }
  }
}

export type ViewStateStore = {
  readonly save: (state: PersistedLibraryBrowserViewState) => void
  readonly load: () => Promise<LibraryBrowserViewStateReadResult>
}

function getRendererApi(): ViewStateApi {
  return (window as unknown as { readonly dekzer: ViewStateApi }).dekzer
}

function normalizeViewStateForPersist(
  state: PersistedLibraryBrowserViewState
): PersistedLibraryBrowserViewState {
  return {
    version: 1,
    ...(state.selectedNodeId === undefined ? {} : { selectedNodeId: state.selectedNodeId }),
    expandedNodeIds: [...new Set(state.expandedNodeIds)]
  }
}

export function createViewStateStore(api: ViewStateApi = getRendererApi()): ViewStateStore {
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
    save(state: PersistedLibraryBrowserViewState): void {
      pendingState = normalizeViewStateForPersist(state)
      drain()
    },

    load(): Promise<LibraryBrowserViewStateReadResult> {
      return api.library.browser.viewState.readViewState()
    }
  }
}
