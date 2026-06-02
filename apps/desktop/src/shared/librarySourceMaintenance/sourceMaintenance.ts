import type {
  ReadSourceMaintenanceReply,
  RunSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

export const sourceMaintenanceChannels = {
  runSourceMaintenance: 'desktop:library-source-maintenance:run-source-maintenance',
  readSourceMaintenance: 'desktop:library-source-maintenance:read-source-maintenance'
} as const

export type SourceMaintenanceState =
  | 'completed'
  | 'ready'
  | 'hostUnavailable'
  | 'invalidRequest'
  | 'maintenanceFailed'
  | 'readFailed'

export type SourceMaintenanceErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'maintenanceFailed'
  | 'readFailed'

export type SourceMaintenanceError = {
  readonly code: SourceMaintenanceErrorCode
  readonly message: string
  readonly detail?: string
}

export type RunSourceMaintenanceRequest = {
  readonly sourceId: string
  readonly hashLimit?: number
  readonly attachmentLimit?: number
  readonly probeLimit?: number
}

export type ReadSourceMaintenanceRequest = {
  readonly sourceId: string
}

export type RunSourceMaintenanceResult =
  | {
      readonly state: 'completed'
      readonly result: RunSourceMaintenanceReply
    }
  | {
      readonly state: Exclude<SourceMaintenanceState, 'completed' | 'ready'>
      readonly error: SourceMaintenanceError
    }

export type ReadSourceMaintenanceResult =
  | {
      readonly state: 'ready'
      readonly snapshot: ReadSourceMaintenanceReply
    }
  | {
      readonly state: Exclude<SourceMaintenanceState, 'completed' | 'ready' | 'maintenanceFailed'>
      readonly error: SourceMaintenanceError
    }
