export type BrowserTreeNodeId = string

export type BrowserTreeChildren =
  | {
      readonly kind: 'none'
    }
  | {
      readonly kind: 'unknown'
      readonly stateNode: BrowserTreeNode
    }
  | {
      readonly kind: 'deferred'
      readonly stateNode: BrowserTreeNode
    }
  | {
      readonly kind: 'loading'
      readonly stateNode: BrowserTreeNode
    }
  | {
      readonly kind: 'loaded'
      readonly nodes: readonly BrowserTreeNode[]
    }
  | {
      readonly kind: 'failed'
      readonly stateNode: BrowserTreeNode
    }

export type BrowserTreeActionState =
  | {
      readonly kind: 'idle'
      readonly detail?: string
    }
  | {
      readonly kind: 'loading'
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type BrowserTreeAction =
  | {
      readonly kind: 'loadChildren'
      readonly state: BrowserTreeActionState
    }
  | {
      readonly kind: 'loadMore'
      readonly state: BrowserTreeActionState
    }

export type BrowserTreeRowRole =
  | 'collectionView'
  | 'source'
  | 'sourceLocation'
  | 'literalDirectory'
  | 'preparationSurface'
  | 'playlistSurface'
  | 'smartView'
  | 'state'
  | 'action'

export type BrowserTreeIcon =
  | 'source'
  | 'navigation'
  | 'folder'
  | 'file'
  | 'music'
  | 'video'
  | 'image'
  | 'cueSheet'
  | 'playlist'
  | 'metadata'
  | 'more'
  | 'loading'
  | 'warning'
  | 'state'

export type BrowserTreeNode = {
  readonly id: BrowserTreeNodeId
  readonly label: string
  readonly role: BrowserTreeRowRole
  readonly detail?: string
  readonly icon?: BrowserTreeIcon
  readonly children: BrowserTreeChildren
  readonly action?: BrowserTreeAction
}

export type BrowserTreeVisibleItem = {
  readonly id: BrowserTreeNodeId
  readonly node: BrowserTreeNode
  readonly parentId?: BrowserTreeNodeId
  readonly level: number
  readonly visibleIndex: number
  readonly isBranch: boolean
  readonly canRevealChildren: boolean
  readonly canActivateAction: boolean
  readonly isActionLoading: boolean
  readonly isActionItem: boolean
  readonly isExpanded: boolean
  readonly isSelected: boolean
  readonly isActive: boolean
  readonly ariaSetSize: number
  readonly ariaPosInSet: number
}
