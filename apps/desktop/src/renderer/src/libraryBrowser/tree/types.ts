export type TreeNodeId = string

export type TreeNodeKind =
  | 'fixtureRoot'
  | 'folder'
  | 'playlistGroup'
  | 'preparation'
  | 'history'
  | 'trackGroup'

export type TreeNode = {
  readonly id: TreeNodeId
  readonly label: string
  readonly kind: TreeNodeKind
  readonly detail?: string
  readonly children?: readonly TreeNode[]
}

export type TreeFixture = {
  readonly name: string
  readonly detail: string
  readonly nodes: readonly TreeNode[]
}

export type TreeVisibleItem = {
  readonly id: TreeNodeId
  readonly node: TreeNode
  readonly parentId: TreeNodeId | null
  readonly level: number
  readonly visibleIndex: number
  readonly hasChildren: boolean
  readonly isExpanded: boolean
  readonly isSelected: boolean
  readonly isActive: boolean
  readonly ariaSetSize: number
  readonly ariaPosInSet: number
}
