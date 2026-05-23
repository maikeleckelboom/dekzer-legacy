export type UnregisterLocalRootRequest = {
  readonly rootId: string
}

export type UnregisterLocalRootState = 'unregistered' | 'hostUnavailable' | 'invalidRequest'

export type UnregisterLocalRootErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'

export type UnregisterLocalRootError = {
  readonly code: UnregisterLocalRootErrorCode
  readonly message: string
}

export type UnregisterLocalRootErrorState = Exclude<UnregisterLocalRootState, 'unregistered'>

export type UnregisterLocalRootResult =
  | {
      readonly state: 'unregistered'
      readonly unregistered: boolean
    }
  | {
      readonly state: UnregisterLocalRootErrorState
      readonly error: UnregisterLocalRootError
    }
