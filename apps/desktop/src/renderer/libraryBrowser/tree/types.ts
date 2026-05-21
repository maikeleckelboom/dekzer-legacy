export type BrowserTreeNodeId = string

export type BrowserTreeNodeKind =
  | 'fixtureRoot'
  | 'source'
  | 'folder'
  | 'file'
  | 'playlistGroup'
  | 'preparation'
  | 'history'
  | 'trackGroup'

export type BrowserTreeChildrenState =
  | {
      readonly kind: 'leaf'
    }
  | {
      readonly kind: 'loaded'
      readonly children: readonly BrowserTreeNode[]
    }
  | {
      readonly kind: 'unloaded'
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

export type BrowserTreeNode = {
  readonly id: BrowserTreeNodeId
  readonly label: string
  readonly kind: BrowserTreeNodeKind
  readonly detail?: string
  readonly childrenState: BrowserTreeChildrenState
}

export type BrowserTreeFixture = {
  readonly name: string
  readonly detail: string
  readonly nodes: readonly BrowserTreeNode[]
}

export type BrowserTreeVisibleItem = {
  readonly id: BrowserTreeNodeId
  readonly node: BrowserTreeNode
  readonly parentId?: BrowserTreeNodeId
  readonly level: number
  readonly visibleIndex: number
  readonly isBranch: boolean
  readonly canRevealChildren: boolean
  readonly canRequestChildren: boolean
  readonly isLoadingChildren: boolean
  readonly isExpanded: boolean
  readonly isSelected: boolean
  readonly isActive: boolean
  readonly ariaSetSize: number
  readonly ariaPosInSet: number
}
