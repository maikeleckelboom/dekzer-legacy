export const hierarchyReadChannels = {
  readChildren: 'desktop:library-hierarchy:read-children'
} as const

export type LibraryHierarchyReadChildrenState =
  | 'ready'
  | 'hostUnavailable'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryHierarchyReadChildrenErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'noTarget'
  | 'notFound'
  | 'invalidRequest'
  | 'readFailed'

export type LibraryHierarchyReadChildrenError = {
  readonly code: LibraryHierarchyReadChildrenErrorCode
  readonly message: string
}

export type LibraryHierarchyReadChildrenErrorState = Exclude<
  LibraryHierarchyReadChildrenState,
  'ready'
>

export type LibraryHierarchyReadChildrenEntryPoint =
  | {
      readonly kind: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'sourceLocation'
      readonly sourceLocationId: string
    }

export type LibraryHierarchyReadChildrenTarget =
  | {
      readonly kind: 'firstAvailableSource'
    }
  | {
      readonly kind: 'entryPoint'
      readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
      readonly label?: string
    }

export type LibraryHierarchyReadChildrenRequest = {
  readonly target?: LibraryHierarchyReadChildrenTarget
  readonly parentSourceDirectoryId?: string
  readonly offset?: number
  readonly limit?: number
}

export type LibraryHierarchyReadChildrenRoot = {
  readonly id: string
  readonly label?: string
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
}

export type LibraryHierarchyReadChildrenNodeKind = 'directory' | 'file'

export type LibraryHierarchyReadChildrenNodePresenceState = 'present' | 'missing' | 'removed'

export type LibraryHierarchyReadChildrenNode =
  | {
      readonly id: string
      readonly kind: 'directory'
      readonly label: string
      readonly sourceDirectoryId: string
      readonly parentSourceDirectoryId?: string
      readonly presenceState: LibraryHierarchyReadChildrenNodePresenceState
      readonly updatedAtMs: number
    }
  | {
      readonly id: string
      readonly kind: 'file'
      readonly label: string
      readonly sourceFileId: string
      readonly parentSourceDirectoryId?: string
      readonly presenceState: LibraryHierarchyReadChildrenNodePresenceState
      readonly updatedAtMs: number
    }

export type LibraryHierarchyReadChildrenWindow = {
  readonly root: LibraryHierarchyReadChildrenRoot
  readonly parentSourceDirectoryId?: string
  readonly offset: number
  readonly limit: number
  readonly totalRows: number
  readonly nodes: readonly LibraryHierarchyReadChildrenNode[]
}

export type LibraryHierarchyReadChildrenResult =
  | {
      readonly state: 'ready'
      readonly window: LibraryHierarchyReadChildrenWindow
    }
  | {
      readonly state: LibraryHierarchyReadChildrenErrorState
      readonly error: LibraryHierarchyReadChildrenError
    }
