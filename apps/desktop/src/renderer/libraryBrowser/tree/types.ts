export type BrowserTreeNodeId = string

export type BrowserTreeChildren =
  | {
      readonly kind: 'none'
    }
  | {
      readonly kind: 'loaded'
      readonly nodes: readonly BrowserTreeNode[]
    }
  | {
      readonly kind: 'deferred'
      readonly detail?: string
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

export type BrowserTreeIcon =
  | 'source'
  | 'navigation'
  | 'folder'
  | 'folderOpen'
  | 'file'
  | 'music'
  | 'more'
  | 'loading'
  | 'warning'
  | 'state'

export type BrowserTreeNode = {
  readonly id: BrowserTreeNodeId
  readonly label: string
  readonly badgeLabel?: string
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
