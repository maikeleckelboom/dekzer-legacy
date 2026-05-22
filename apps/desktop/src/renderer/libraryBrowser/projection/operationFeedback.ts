import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundary/status'
import type { NavigationReadRowsResult } from '../../../shared/libraryNavigation/readRows'
import type {
  LocalRootChoiceStatus,
  LocalRootScanStatus,
  LocalRootScanSummary
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
}

type FeedbackMeta = {
  readonly tone: LibraryOperationFeedbackTone
  readonly title: string
  readonly detail: string
}

const meta: Record<LibraryOperationFeedbackKind, FeedbackMeta> = {
  checkingHost: {
    tone: 'loading',
    title: 'Checking library host',
    detail: 'Waiting for library boundary host status.'
  },
  hostUnavailable: {
    tone: 'warning',
    title: 'Library host unavailable',
    detail: 'The library boundary host is not in a usable state.'
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
    title: 'Scanning local root',
    detail: 'Dekzer is scanning the registered library root.'
  },
  scanFailed: {
    tone: 'error',
    title: 'Scan failed',
    detail: 'Unable to scan the registered folder. Try rescanning.'
  },
  scanComplete: {
    tone: 'success',
    title: 'Scan complete',
    detail: 'The registered root was scanned successfully.'
  },
  refreshingView: {
    tone: 'loading',
    title: 'Refreshing library view',
    detail: 'Reading maintained navigation rows and hierarchy children.'
  },
  refreshFailed: {
    tone: 'warning',
    title: 'Refresh incomplete',
    detail: 'Scan complete, but the library view could not refresh.'
  },
  navigationLoading: {
    tone: 'loading',
    title: 'Loading navigation',
    detail: 'Reading persisted library navigation rows.'
  },
  navigationFailed: {
    tone: 'error',
    title: 'Navigation read failed',
    detail: 'Unable to request library navigation rows.'
  },
  hierarchyLoading: {
    tone: 'loading',
    title: 'Loading hierarchy',
    detail: 'Reading literal hierarchy children.'
  },
  hierarchyFailed: {
    tone: 'error',
    title: 'Hierarchy read failed',
    detail: 'Unable to request library hierarchy children.'
  },
  noSources: {
    tone: 'warning',
    title: 'No library sources',
    detail: 'No persisted library navigation rows are available.'
  },
  ready: {
    tone: 'success',
    title: 'Library ready',
    detail: 'Showing persisted local library rows.'
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
      detail: host.lastError?.message ?? 'The library boundary host encountered a failure.'
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
    return 'The registered root was scanned successfully.'
  }

  return [
    'Scan complete.',
    `${summary.discoveredFileCount} ${summary.discoveredFileCount === 1 ? 'file' : 'files'} discovered.`,
    `${summary.queuedSourceWorkItems} source work ${summary.queuedSourceWorkItems === 1 ? 'item' : 'items'} queued.`
  ].join(' ')
}
