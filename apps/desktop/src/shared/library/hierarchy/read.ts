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

export type NavigableChildScopeState =
  | 'unknown'
  | 'hasNavigableChildScopes'
  | 'noNavigableChildScopes'

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

export type FileClass = 'audio' | 'video' | 'image' | 'unsupported' | 'none'

export type DirectoryPlayableMediaState =
  | {
      readonly kind: 'unknown'
    }
  | {
      readonly kind: 'hasPlayableMediaDescendants'
    }
  | {
      readonly kind: 'noPlayableMediaDescendants'
    }

export type DirectoryImageMediaState =
  | {
      readonly kind: 'unknown'
    }
  | {
      readonly kind: 'hasImageMediaDescendants'
    }
  | {
      readonly kind: 'noImageMediaDescendants'
    }

export type DirectoryScanState = 'pending' | 'scanning' | 'complete' | 'failed' | 'blocked'

export type HierarchyCoverageState =
  | 'complete'
  | 'pending'
  | 'scanning'
  | 'blocked'
  | 'failed'
  | 'sourceUnavailable'
  | 'locationMissing'

export type HierarchyCoverage = {
  readonly state: HierarchyCoverageState
  readonly subtreeCoverageComplete: boolean
  readonly emptyResultAuthoritative: boolean
  readonly detail?: string
}

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
      readonly directoryPlayableMediaState: DirectoryPlayableMediaState
      readonly directoryImageMediaState: DirectoryImageMediaState
      readonly directoryScanState: DirectoryScanState
      readonly navigableChildScopeState: NavigableChildScopeState
      readonly updatedAtMs: number
    }
  | {
      readonly id: string
      readonly kind: 'file'
      readonly label: string
      readonly sourceId: string
      readonly fileId: string
      readonly parentDirectoryId?: string
      readonly fileClass: FileClass
      readonly presence: Presence
      readonly updatedAtMs: number
    }

export type ChildWindow = {
  readonly root: ReadRoot
  readonly parentDirectoryId?: string
  readonly offset: number
  readonly limit: number
  readonly totalRows: number
  readonly coverage: HierarchyCoverage
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
