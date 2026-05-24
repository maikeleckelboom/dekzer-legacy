import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundary/status'
import type { NavigationReadRowsResult } from '../../../shared/libraryNavigation/readRows'
import type {
  LocalRootChoiceStatus,
  LocalRootScanStatus,
  LocalRootScanSummary,
  RemoveSourceStatus
} from '../boundary/localRootActions'
import type { RootLifecycleRefreshStatus } from '../runtime/rootLifecycle'

export type LibraryOperationFeedbackKind =
  | 'checkingHost'
  | 'hostUnavailable'
  | 'chooseRoot'
  | 'choosingRoot'
  | 'rootChoiceFailed'
  | 'rootChoiceCanceled'
  | 'scanningRoot'
  | 'scanFailed'
  | 'scanComplete'
  | 'refreshingView'
  | 'refreshFailed'
  | 'navigationLoading'
  | 'navigationFailed'
  | 'hierarchyLoading'
  | 'hierarchyFailed'
  | 'noSources'
  | 'removingSource'
  | 'removeFailed'
  | 'ready'

export type LibraryOperationFeedbackTone = 'idle' | 'loading' | 'success' | 'warning' | 'error'

export type LibraryOperationFeedback = {
  readonly kind: LibraryOperationFeedbackKind
  readonly tone: LibraryOperationFeedbackTone
  readonly title: string
  readonly detail?: string
}

export type OperationFeedbackInputs = {
  readonly hostStatus: LibraryBoundaryHostStatus | undefined
  readonly rootChoiceStatus: LocalRootChoiceStatus
  readonly registeredRootPath: string | undefined
  readonly scanStatus: LocalRootScanStatus
  readonly scanSummary: LocalRootScanSummary | undefined
  readonly refreshStatus: RootLifecycleRefreshStatus
  readonly navigationReadIsLoading: boolean
  readonly hierarchyReadIsLoading: boolean
  readonly navigationReadRequestError: string | undefined
  readonly hierarchyReadRequestError: string | undefined
  readonly navigationReadResult: NavigationReadRowsResult | undefined
  readonly removeSourceStatus: RemoveSourceStatus
  readonly removeSourceFailureMessage?: string
}

type FeedbackMeta = {
  readonly tone: LibraryOperationFeedbackTone
  readonly title: string
  readonly detail: string
}

const meta: Record<LibraryOperationFeedbackKind, FeedbackMeta> = {
  checkingHost: {
    tone: 'loading',
    title: 'Starting library',
    detail: 'Setting up the library.'
  },
  hostUnavailable: {
    tone: 'warning',
    title: 'Library unavailable',
    detail: 'The library is not available right now.'
  },
  chooseRoot: {
    tone: 'idle',
    title: 'Choose a music folder',
    detail: 'No local root is registered yet.'
  },
  choosingRoot: {
    tone: 'loading',
    title: 'Choosing music folder',
    detail: 'Opening folder picker.'
  },
  rootChoiceFailed: {
    tone: 'error',
    title: 'Unable to add music folder',
    detail: 'The music folder could not be registered. Try again.'
  },
  rootChoiceCanceled: {
    tone: 'warning',
    title: 'Folder selection canceled',
    detail: 'Choose a music folder to add it to your library.'
  },
  scanningRoot: {
    tone: 'loading',
    title: 'Scanning music folder',
    detail: 'Scanning your music folder for files.'
  },
  scanFailed: {
    tone: 'error',
    title: 'Scan failed',
    detail: 'Unable to scan the registered folder. Try rescanning.'
  },
  scanComplete: {
    tone: 'success',
    title: 'Scan complete',
    detail: 'Music folder scanned successfully.'
  },
  refreshingView: {
    tone: 'loading',
    title: 'Refreshing library',
    detail: 'Updating your library contents.'
  },
  refreshFailed: {
    tone: 'warning',
    title: 'Refresh incomplete',
    detail: 'Scan complete, but the library view could not refresh.'
  },
  navigationLoading: {
    tone: 'loading',
    title: 'Loading library',
    detail: 'Loading your library.'
  },
  navigationFailed: {
    tone: 'error',
    title: 'Library load failed',
    detail: 'Could not load your library.'
  },
  hierarchyLoading: {
    tone: 'loading',
    title: 'Loading folder contents',
    detail: 'Loading folder contents.'
  },
  hierarchyFailed: {
    tone: 'error',
    title: 'Folder contents failed to load',
    detail: 'Could not load folder contents.'
  },
  noSources: {
    tone: 'warning',
    title: 'No library sources',
    detail: 'Add a music folder to start building your library.'
  },
  removingSource: {
    tone: 'loading',
    title: 'Removing source',
    detail: 'Dekzer is removing the current source. Your files stay on disk.'
  },
  removeFailed: {
    tone: 'error',
    title: 'Unable to remove source',
    detail: 'The source could not be removed. Try again.'
  },
  ready: {
    tone: 'success',
    title: 'Library ready',
    detail: 'Your music library is ready.'
  }
}

export function deriveOperationFeedback(inputs: OperationFeedbackInputs): LibraryOperationFeedback {
  const host = inputs.hostStatus

  if (host === undefined) {
    return build('checkingHost')
  }

  if (host.state !== 'started') {
    return hostNotReadyFeedback(host)
  }

  if (inputs.registeredRootPath === undefined && inputs.navigationReadResult?.state !== 'ready') {
    return noRootFeedback(inputs.rootChoiceStatus)
  }

  if (inputs.rootChoiceStatus === 'choosing') {
    return build('choosingRoot')
  }

  if (inputs.scanStatus === 'scanning') {
    return build('scanningRoot')
  }

  if (inputs.removeSourceStatus === 'removing') {
    return build('removingSource')
  }

  if (inputs.refreshStatus === 'refreshing') {
    return build('refreshingView')
  }

  if (inputs.navigationReadIsLoading) {
    return build('navigationLoading')
  }

  if (inputs.hierarchyReadIsLoading) {
    return build('hierarchyLoading')
  }

  if (inputs.rootChoiceStatus === 'failed') {
    return build('rootChoiceFailed')
  }

  if (inputs.scanStatus === 'failed') {
    return build('scanFailed')
  }

  if (inputs.removeSourceStatus === 'failed') {
    return build(
      'removeFailed',
      inputs.removeSourceFailureMessage === undefined
        ? undefined
        : { detail: inputs.removeSourceFailureMessage }
    )
  }

  if (inputs.refreshStatus === 'failed') {
    return build('refreshFailed')
  }

  if (inputs.navigationReadRequestError !== undefined) {
    return build('navigationFailed', { detail: inputs.navigationReadRequestError })
  }

  if (inputs.hierarchyReadRequestError !== undefined) {
    return build('hierarchyFailed', { detail: inputs.hierarchyReadRequestError })
  }

  if (inputs.rootChoiceStatus === 'canceled') {
    return build('rootChoiceCanceled')
  }

  if (inputs.navigationReadResult?.state === 'ready') {
    return inputs.navigationReadResult.rows.length > 0 ? build('ready') : build('noSources')
  }

  if (inputs.scanStatus === 'scanned') {
    return build('scanComplete', {
      detail: scanDetail(inputs.scanSummary)
    })
  }

  return build('chooseRoot')
}

function hostNotReadyFeedback(host: LibraryBoundaryHostStatus): LibraryOperationFeedback {
  if (host.state === 'failed') {
    return build('hostUnavailable', {
      tone: 'error',
      detail: host.lastError?.message ?? 'The library encountered an error.'
    })
  }

  if (host.state === 'starting') {
    return build('checkingHost')
  }

  return build('hostUnavailable')
}

function noRootFeedback(status: LocalRootChoiceStatus): LibraryOperationFeedback {
  if (status === 'choosing') return build('choosingRoot')
  if (status === 'failed') return build('rootChoiceFailed')
  if (status === 'canceled') return build('rootChoiceCanceled')
  return build('chooseRoot')
}

function build(
  kind: LibraryOperationFeedbackKind,
  overrides?: Partial<Pick<LibraryOperationFeedback, 'tone' | 'detail'>>
): LibraryOperationFeedback {
  const m = meta[kind]
  return {
    kind,
    tone: overrides?.tone ?? m.tone,
    title: m.title,
    detail: overrides?.detail ?? m.detail
  }
}

function scanDetail(summary: LocalRootScanSummary | undefined): string {
  if (summary === undefined) {
    return 'Music folder scanned successfully.'
  }

  return [
    'Scan complete.',
    `${summary.discoveredFileCount} ${summary.discoveredFileCount === 1 ? 'file' : 'files'} discovered.`,
    `${summary.queuedSourceWorkItems} ${summary.queuedSourceWorkItems === 1 ? 'item' : 'items'} queued.`
  ].join(' ')
}
