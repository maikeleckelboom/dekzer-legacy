export type SourceLifecycleSourceClass = 'internal' | 'externalMounted' | 'removableMounted'

export type SourceLifecycleMountStatus =
  | 'unknown'
  | 'mounted'
  | 'unmounted'
  | 'ejectRequested'
  | 'ejectPending'

export type SourceLifecycleAccessState = 'accessible' | 'missing' | 'blocked' | 'unknown'

export type SourceLifecycleIssueKind =
  | 'missing'
  | 'notDirectory'
  | 'permissionDenied'
  | 'privacyPermissionRequired'
  | 'unavailableMount'
  | 'resourceBusy'
  | 'staleNetworkHandle'
  | 'symlinkLoop'
  | 'symlinkEscapeBlocked'
  | 'unsupportedPath'
  | 'invalidPath'
  | 'ioInterrupted'
  | 'timedOut'
  | 'unknownIo'

export type SourceLifecycleScanPhase =
  | 'idle'
  | 'scanning'
  | 'complete'
  | 'partial'
  | 'blocked'
  | 'failed'

export type SourceLifecycleRecord = {
  readonly sourceId: string
  readonly sourceClass: SourceLifecycleSourceClass
  readonly isUserVisible: boolean
  readonly mountStatus: SourceLifecycleMountStatus
  readonly accessState: SourceLifecycleAccessState
  readonly accessIssueKind?: SourceLifecycleIssueKind
  readonly scanPhase: SourceLifecycleScanPhase
  readonly scanIssueKind?: SourceLifecycleIssueKind
  readonly lastScanStartedAtMs?: number
  readonly lastScanFinishedAtMs?: number
  readonly lastSuccessfulScanAtMs?: number
  readonly lastSeenAtMs?: number
  readonly updatedAtMs: number
}

export type ReadSourceLifecycleRequest = {
  readonly sourceId: string
}

export type ReadSourceLifecycleState =
  | 'ready'
  | 'notFound'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'readFailed'

export type ReadSourceLifecycleErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'notFound'
  | 'readFailed'

export type ReadSourceLifecycleError = {
  readonly code: ReadSourceLifecycleErrorCode
  readonly message: string
}

export type ReadSourceLifecycleErrorState = Exclude<ReadSourceLifecycleState, 'ready'>

export type ReadSourceLifecycleResult =
  | {
      readonly state: 'ready'
      readonly lifecycle: SourceLifecycleRecord
    }
  | {
      readonly state: ReadSourceLifecycleErrorState
      readonly error: ReadSourceLifecycleError
    }
