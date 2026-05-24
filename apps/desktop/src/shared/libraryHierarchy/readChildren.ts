export const hierarchyReadChannels = {
  readChildren: 'desktop:library-hierarchy:read-children'
} as const

export type ReadState =
  | 'ready'
  | 'hostUnavailable'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type ReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type ReadError = {
  readonly code: ReadErrorCode
  readonly message: string
}

export type ReadErrorState = Exclude<ReadState, 'ready'>

export type EntryPoint =
  | {
      readonly kind: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'sourceLocation'
      readonly sourceLocationId: string
    }

export type ReadTarget =
  | {
      readonly kind: 'firstAvailableSource'
    }
  | {
      readonly kind: 'entryPoint'
      readonly entryPoint: EntryPoint
      readonly label?: string
    }

export type ReadRequest = {
  readonly target?: ReadTarget
  readonly parentDirectoryId?: string
  readonly offset?: number
  readonly limit?: number
}

export type ReadRoot = {
  readonly id: string
  readonly label?: string
  readonly entryPoint: EntryPoint
}

export type NodeKind = 'directory' | 'file'

export type Presence = 'present' | 'missing' | 'removed'

export type DirectoryMediaState =
  | {
      readonly kind: 'unknown'
    }
  | {
      readonly kind: 'hasMediaDescendants'
    }
  | {
      readonly kind: 'noMediaDescendants'
    }

export type DirectoryScanState = 'pending' | 'scanning' | 'complete' | 'failed' | 'blocked'

export type ChildRow =
  | {
      readonly id: string
      readonly kind: 'directory'
      readonly label: string
      readonly sourceId: string
      readonly directoryId: string
      readonly parentDirectoryId?: string
      readonly presence: Presence
      readonly hasChildDirectories: boolean
      readonly directoryMediaState: DirectoryMediaState
      readonly directoryScanState: DirectoryScanState
      readonly updatedAtMs: number
    }
  | {
      readonly id: string
      readonly kind: 'file'
      readonly label: string
      readonly sourceId: string
      readonly fileId: string
      readonly parentDirectoryId?: string
      readonly presence: Presence
      readonly updatedAtMs: number
    }

export type ChildWindow = {
  readonly root: ReadRoot
  readonly parentDirectoryId?: string
  readonly offset: number
  readonly limit: number
  readonly totalRows: number
  readonly nodes: readonly ChildRow[]
}

export type ReadResult =
  | {
      readonly state: 'ready'
      readonly window: ChildWindow
    }
  | {
      readonly state: ReadErrorState
      readonly error: ReadError
    }
