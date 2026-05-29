export type LocalRootScanState = 'started' | 'hostUnavailable' | 'invalidRequest' | 'scanFailed'

export type LocalRootScanErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'scanFailed'

export type LocalRootScanError = {
  readonly code: LocalRootScanErrorCode
  readonly message: string
  readonly detail?: string
}

export type LocalRootScanErrorState = Exclude<LocalRootScanState, 'started'>

export type LocalRootScanRequest = {
  readonly rootId: string
}

export type LocalRootScanResult =
  | {
      readonly state: 'started'
      readonly scanRunId: string
    }
  | {
      readonly state: LocalRootScanErrorState
      readonly error: LocalRootScanError
    }
