export type RegisteredLocalRoot = {
  readonly rootId: string
  readonly canonicalPath: string
}

export type ReadRegisteredLocalRootsResult = {
  readonly state: 'read'
  readonly roots: readonly RegisteredLocalRoot[]
}

export type ReadRegisteredLocalRootsErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadRegisteredLocalRootsError = {
  readonly code: ReadRegisteredLocalRootsErrorCode
  readonly message: string
}

export type ReadRegisteredLocalRootsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadRegisteredLocalRootsOutcome =
  | ReadRegisteredLocalRootsResult
  | {
      readonly state: ReadRegisteredLocalRootsErrorState
      readonly error: ReadRegisteredLocalRootsError
    }
