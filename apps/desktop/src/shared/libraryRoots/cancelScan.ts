import type { CancelRootScanStatus } from '@dekzer/library-boundary-contract'

export type CancelRootScanState =
  | 'accepted'
  | 'notFound'
  | 'alreadyTerminal'
  | 'notCancelable'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'cancelFailed'

export type CancelRootScanErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'cancelFailed'

export type CancelRootScanError = {
  readonly code: CancelRootScanErrorCode
  readonly message: string
  readonly detail?: string
}

export type CancelRootScanErrorState = Exclude<
  CancelRootScanState,
  'accepted' | 'notFound' | 'alreadyTerminal' | 'notCancelable'
>

export type CancelRootScanRequest = {
  readonly scanRunId: string
}

export type CancelRootScanResult =
  | CancelRootScanAcceptedResult
  | CancelRootScanNotFoundResult
  | CancelRootScanAlreadyTerminalResult
  | CancelRootScanNotCancelableResult
  | CancelRootScanErrorResult

export type CancelRootScanAcceptedResult = {
  readonly state: 'accepted'
  readonly status: CancelRootScanStatus
}

export type CancelRootScanNotFoundResult = {
  readonly state: 'notFound'
  readonly status: CancelRootScanStatus
}

export type CancelRootScanAlreadyTerminalResult = {
  readonly state: 'alreadyTerminal'
  readonly status: CancelRootScanStatus
}

export type CancelRootScanNotCancelableResult = {
  readonly state: 'notCancelable'
  readonly status: CancelRootScanStatus
}

export type CancelRootScanErrorResult = {
  readonly state: CancelRootScanErrorState
  readonly error: CancelRootScanError
}
