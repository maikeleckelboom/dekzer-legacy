import type {
  LibraryViewStateReadResult,
  LibraryViewStateWriteResult,
  PersistedLibraryViewState
} from '../../../shared/library/viewState/persistence'
import { isLibraryBrowseProfile } from '../libraryBrowseProfile/types'
import { isLocalPreviewMode } from '../localBrowse/previewMode'

export type ViewStateApi = {
  readonly library: {
    readonly viewState: {
      readonly readViewState: () => Promise<LibraryViewStateReadResult>
      readonly writeViewState: (
        state: PersistedLibraryViewState
      ) => Promise<LibraryViewStateWriteResult>
    }
  }
}

export type ViewStateStore = {
  readonly save: (state: PersistedLibraryViewState) => void
  readonly load: () => Promise<LibraryViewStateReadResult>
}

function getRendererApi(): ViewStateApi {
  return (window as unknown as { readonly dekzer: ViewStateApi }).dekzer
}

function normalizeViewStateForPersist(state: PersistedLibraryViewState): PersistedLibraryViewState {
  return {
    version: 2,
    ...(state.selectedNodeId === undefined ? {} : { selectedNodeId: state.selectedNodeId }),
    expandedNodeIds: [...new Set(state.expandedNodeIds)],
    ...(isLibraryBrowseProfile(state.libraryBrowseProfile)
      ? { libraryBrowseProfile: state.libraryBrowseProfile }
      : {}),
    ...(isLocalPreviewMode(state.localPreviewMode)
      ? { localPreviewMode: state.localPreviewMode }
      : {})
  }
}

export function createViewStateStore(api: ViewStateApi = getRendererApi()): ViewStateStore {
  let pendingState: PersistedLibraryViewState | undefined
  let writeInFlight = false

  function drain(): void {
    if (writeInFlight || pendingState === undefined) {
      return
    }

    const state = pendingState
    pendingState = undefined
    writeInFlight = true

    api.library.viewState.writeViewState(state).then(
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
    save(state: PersistedLibraryViewState): void {
      pendingState = normalizeViewStateForPersist(state)
      drain()
    },

    load(): Promise<LibraryViewStateReadResult> {
      return api.library.viewState.readViewState()
    }
  }
}
