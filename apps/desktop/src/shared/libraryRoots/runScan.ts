export type LocalRootScanState = 'scanned' | 'hostUnavailable' | 'invalidRequest' | 'scanFailed'

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
}

export type LocalRootScanErrorState = Exclude<LocalRootScanState, 'scanned'>

export type LocalRootScanRequest = {
  readonly rootId: string
}

export type LocalRootScanResult =
  | {
      readonly state: 'scanned'
      readonly rootId: string
      readonly scanRunId: string
      readonly discoveredFileCount: number
      readonly queuedSourceWorkItems: number
    }
  | {
      readonly state: LocalRootScanErrorState
      readonly error: LocalRootScanError
    }
