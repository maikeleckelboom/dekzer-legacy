export type LocalRoot = {
  readonly rootId: string
  readonly canonicalPath: string
  readonly availability: LocalRootAvailability
}

export type LocalRootAvailability = 'available' | 'unavailable'

export type ReadLocalRootsResult = {
  readonly state: 'read'
  readonly roots: readonly LocalRoot[]
}

export type ReadLocalRootsErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadLocalRootsError = {
  readonly code: ReadLocalRootsErrorCode
  readonly message: string
}

export type ReadLocalRootsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadLocalRootsOutcome =
  | ReadLocalRootsResult
  | {
      readonly state: ReadLocalRootsErrorState
      readonly error: ReadLocalRootsError
    }
