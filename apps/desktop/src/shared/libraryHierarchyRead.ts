export const libraryHierarchyReadIpcChannels = {
  readLiteralHierarchyChildren: 'desktop:library-hierarchy:read-literal-children'
} as const

export type LibraryHierarchyReadState =
  | 'ready'
  | 'hostUnavailable'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryHierarchyReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryHierarchyReadError = {
  readonly code: LibraryHierarchyReadErrorCode
  readonly message: string
}

export type LibraryHierarchyReadErrorState = Exclude<LibraryHierarchyReadState, 'ready'>

export type LibraryHierarchyReadEntryPoint =
  | {
      readonly kind: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'sourceLocation'
      readonly sourceLocationId: string
    }

export type LibraryHierarchyReadTarget =
  | {
      readonly kind: 'firstAvailableSource'
    }
  | {
      readonly kind: 'entryPoint'
      readonly entryPoint: LibraryHierarchyReadEntryPoint
      readonly label?: string
    }

export type LibraryHierarchyReadRequest = {
  readonly target?: LibraryHierarchyReadTarget
  readonly parentSourceDirectoryId?: string
  readonly offset?: number
  readonly limit?: number
}

export type LibraryHierarchyReadRoot = {
  readonly id: string
  readonly label?: string
  readonly entryPoint: LibraryHierarchyReadEntryPoint
}

export type LibraryHierarchyReadNodeKind = 'directory' | 'file'

export type LibraryHierarchyReadNodePresenceState = 'present' | 'missing' | 'removed'

export type LibraryHierarchyReadNode =
  | {
      readonly id: string
      readonly kind: 'directory'
      readonly label: string
      readonly sourceDirectoryId: string
      readonly parentSourceDirectoryId?: string
      readonly presenceState: LibraryHierarchyReadNodePresenceState
      readonly updatedAtMs: number
    }
  | {
      readonly id: string
      readonly kind: 'file'
      readonly label: string
      readonly sourceFileId: string
      readonly parentSourceDirectoryId?: string
      readonly presenceState: LibraryHierarchyReadNodePresenceState
      readonly updatedAtMs: number
    }

export type LibraryHierarchyReadWindow = {
  readonly root: LibraryHierarchyReadRoot
  readonly parentSourceDirectoryId?: string
  readonly offset: number
  readonly limit: number
  readonly totalRows: number
  readonly nodes: readonly LibraryHierarchyReadNode[]
}

export type LibraryHierarchyReadResult =
  | {
      readonly state: 'ready'
      readonly window: LibraryHierarchyReadWindow
    }
  | {
      readonly state: LibraryHierarchyReadErrorState
      readonly error: LibraryHierarchyReadError
    }
