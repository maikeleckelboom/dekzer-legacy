export type BrowserTreeNodeId = string

export type BrowserTreeNodeKind =
  | 'fixtureRoot'
  | 'folder'
  | 'playlistGroup'
  | 'preparation'
  | 'history'
  | 'trackGroup'

export type BrowserTreeNode = {
  readonly id: BrowserTreeNodeId
  readonly label: string
  readonly kind: BrowserTreeNodeKind
  readonly detail?: string
  readonly children?: readonly BrowserTreeNode[]
}

export type BrowserTreeFixture = {
  readonly name: string
  readonly detail: string
  readonly nodes: readonly BrowserTreeNode[]
}

export type BrowserTreeVisibleItem = {
  readonly id: BrowserTreeNodeId
  readonly node: BrowserTreeNode
  readonly parentId: BrowserTreeNodeId | null
  readonly level: number
  readonly visibleIndex: number
  readonly hasChildren: boolean
  readonly isExpanded: boolean
  readonly isSelected: boolean
  readonly isActive: boolean
  readonly ariaSetSize: number
  readonly ariaPosInSet: number
}
