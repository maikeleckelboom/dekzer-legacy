import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundary/status'
import type { NavigationReadRowsResult } from '../../../shared/libraryNavigation/readRows'
import type { LocalRootChoiceStatus, LocalRootScanStatus } from '../boundary/localRootActions'
import type { RootLifecycleRefreshStatus } from '../runtime/rootLifecycle'

export type LibraryOperationFeedbackTone = 'idle' | 'loading' | 'success' | 'warning' | 'error'

export type LibraryOperationFeedback = {
  readonly tone: LibraryOperationFeedbackTone
  readonly title: string
  readonly detail?: string
}

export type OperationFeedbackInputs = {
  readonly hostStatus: LibraryBoundaryHostStatus | undefined
  readonly rootChoiceStatus: LocalRootChoiceStatus
  readonly registeredRootPath: string | undefined
  readonly scanStatus: LocalRootScanStatus
  readonly refreshStatus: RootLifecycleRefreshStatus
  readonly navigationReadIsLoading: boolean
  readonly hierarchyReadIsLoading: boolean
  readonly navigationReadRequestError: string | undefined
  readonly hierarchyReadRequestError: string | undefined
  readonly navigationReadResult: NavigationReadRowsResult | undefined
}

export function deriveOperationFeedback(inputs: OperationFeedbackInputs): LibraryOperationFeedback {
  const host = inputs.hostStatus

  if (host === undefined) {
    return {
      tone: 'loading',
      title: 'Checking library host',
      detail: 'Waiting for library boundary host status.'
    }
  }

  if (host.state !== 'started') {
    return hostNotStartedFeedback(host)
  }

  if (inputs.registeredRootPath === undefined) {
    if (inputs.rootChoiceStatus === 'choosing') {
      return {
        tone: 'loading',
        title: 'Choosing music folder',
        detail: 'Opening folder picker.'
      }
    }

    if (inputs.rootChoiceStatus === 'failed') {
      return {
        tone: 'error',
        title: 'Unable to add music folder',
        detail: 'The music folder could not be registered. Try again.'
      }
    }

    if (inputs.rootChoiceStatus === 'canceled') {
      return {
        tone: 'warning',
        title: 'Folder selection canceled',
        detail: 'Choose a music folder to add it to your library.'
      }
    }

    return {
      tone: 'idle',
      title: 'Choose a music folder',
      detail: 'No local root is registered yet.'
    }
  }

  if (inputs.rootChoiceStatus === 'choosing') {
    return {
      tone: 'loading',
      title: 'Choosing music folder',
      detail: 'Opening folder picker.'
    }
  }

  if (inputs.scanStatus === 'scanning') {
    return {
      tone: 'loading',
      title: 'Scanning local root',
      detail: 'Dekzer is scanning the registered library root.'
    }
  }

  if (inputs.refreshStatus === 'refreshing') {
    return {
      tone: 'loading',
      title: 'Refreshing library view',
      detail: 'Reading maintained navigation rows and hierarchy children.'
    }
  }

  if (inputs.navigationReadIsLoading) {
    return {
      tone: 'loading',
      title: 'Loading navigation',
      detail: 'Reading persisted library navigation rows.'
    }
  }

  if (inputs.hierarchyReadIsLoading) {
    return {
      tone: 'loading',
      title: 'Loading hierarchy',
      detail: 'Reading literal hierarchy children.'
    }
  }

  if (inputs.rootChoiceStatus === 'failed') {
    return {
      tone: 'error',
      title: 'Unable to add music folder',
      detail: 'The music folder could not be registered. Try again.'
    }
  }

  if (inputs.scanStatus === 'failed') {
    return {
      tone: 'error',
      title: 'Scan failed',
      detail: 'Unable to scan the registered folder. Try rescanning.'
    }
  }

  if (inputs.refreshStatus === 'failed') {
    return {
      tone: 'warning',
      title: 'Refresh incomplete',
      detail: 'Scan complete, but the library view could not refresh.'
    }
  }

  if (inputs.navigationReadRequestError !== undefined) {
    return {
      tone: 'error',
      title: 'Navigation read failed',
      detail: inputs.navigationReadRequestError
    }
  }

  if (inputs.hierarchyReadRequestError !== undefined) {
    return {
      tone: 'error',
      title: 'Hierarchy read failed',
      detail: inputs.hierarchyReadRequestError
    }
  }

  if (inputs.rootChoiceStatus === 'canceled') {
    return {
      tone: 'warning',
      title: 'Folder selection canceled',
      detail: 'Choose a music folder to add it to your library.'
    }
  }

  if (inputs.scanStatus === 'scanned') {
    return {
      tone: 'success',
      title: 'Scan complete',
      detail: 'The registered root was scanned successfully.'
    }
  }

  if (inputs.refreshStatus === 'refreshed') {
    return {
      tone: 'success',
      title: 'Library view refreshed',
      detail: 'Showing maintained local library rows.'
    }
  }

  if (inputs.rootChoiceStatus === 'registered') {
    return {
      tone: 'success',
      title: 'Folder added',
      detail: 'Ready to scan this folder.'
    }
  }

  if (inputs.navigationReadResult?.state === 'ready') {
    if (inputs.navigationReadResult.rows.length === 0) {
      return {
        tone: 'warning',
        title: 'No library sources',
        detail: 'No persisted library navigation rows are available.'
      }
    }

    return {
      tone: 'success',
      title: 'Library ready',
      detail: 'Showing persisted local library rows.'
    }
  }

  return {
    tone: 'idle',
    title: 'Library panel ready',
    detail: 'Choose a music folder and scan to get started.'
  }
}

function hostNotStartedFeedback(host: LibraryBoundaryHostStatus): LibraryOperationFeedback {
  switch (host.state) {
    case 'idle':
      return {
        tone: 'warning',
        title: 'Library host idle',
        detail: 'The library boundary host has not started yet.'
      }
    case 'starting':
      return {
        tone: 'loading',
        title: 'Library host starting',
        detail: 'Waiting for the local library host.'
      }
    case 'stopping':
      return {
        tone: 'warning',
        title: 'Library host stopping',
        detail: 'The library boundary host is stopping.'
      }
    case 'stopped':
      return {
        tone: 'warning',
        title: 'Library host stopped',
        detail: 'The library boundary host has stopped.'
      }
    case 'failed':
      return {
        tone: 'error',
        title: 'Library host failed',
        detail: host.lastError?.message ?? 'The library boundary host encountered a failure.'
      }
    default:
      return {
        tone: 'warning',
        title: 'Library host unavailable',
        detail: 'The library boundary host is not in a usable state.'
      }
  }
}
