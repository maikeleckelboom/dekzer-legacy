export type LibraryHierarchyNodeId = string

export type LibraryHierarchyNodeKind =
  | 'fixtureRoot'
  | 'folder'
  | 'playlistGroup'
  | 'preparation'
  | 'history'
  | 'trackGroup'

export type LibraryHierarchyTreeNode = {
  readonly id: LibraryHierarchyNodeId
  readonly label: string
  readonly kind: LibraryHierarchyNodeKind
  readonly detail?: string
  readonly children?: readonly LibraryHierarchyTreeNode[]
}

export type LibraryHierarchyFixtureTree = {
  readonly name: string
  readonly detail: string
  readonly nodes: readonly LibraryHierarchyTreeNode[]
}
