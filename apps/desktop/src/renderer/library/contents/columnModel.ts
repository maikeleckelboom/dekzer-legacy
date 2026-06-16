import type { RowBinding } from '../state'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNode, BrowserTreeNodeId } from '../tree/types'

export type ColRow = {
  readonly id: BrowserTreeNodeId
  readonly label: string
  readonly detail?: string
  readonly icon: BrowserTreeNode['icon']
  readonly role: BrowserTreeNode['role']
  readonly active: boolean
  readonly inPath: boolean
  readonly canActivate: boolean
}

export type Column = {
  readonly id: string
  readonly title: string
  readonly rows: readonly ColRow[]
}

export type Columns = {
  readonly columns: readonly Column[]
}

export function projectColumns(options: {
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId?: BrowserTreeNodeId
}): Columns {
  const projection = options.projection

  if (projection === undefined) {
    return {
      columns: [
        {
          id: 'column:library',
          title: 'Library',
          rows: []
        }
      ]
    }
  }

  const path = findNodePath(projection.nodes, options.selectedNodeId)
  const pathIds = new Set(path.map((node) => node.id))
  const cols: Column[] = [
    {
      id: 'column:library',
      title: 'Library',
      rows: projectColRows({
        nodes: projection.nodes,
        selectedNodeId: options.selectedNodeId,
        pathIds,
        bindingsById: projection.bindingsById
      })
    }
  ]

  for (const node of path) {
    const nodes = childNodes(node)

    if (nodes.length === 0) {
      continue
    }

    cols.push({
      id: `column:${node.id}`,
      title: node.label,
      rows: projectColRows({
        nodes,
        selectedNodeId: options.selectedNodeId,
        pathIds,
        bindingsById: projection.bindingsById
      })
    })
  }

  return { columns: cols }
}

function projectColRows(options: {
  readonly nodes: readonly BrowserTreeNode[]
  readonly selectedNodeId: BrowserTreeNodeId | undefined
  readonly pathIds: ReadonlySet<BrowserTreeNodeId>
  readonly bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
}): readonly ColRow[] {
  return options.nodes.map((node) => ({
    id: node.id,
    label: node.label,
    ...(node.detail === undefined ? {} : { detail: node.detail }),
    icon: node.icon,
    role: node.role,
    active: options.selectedNodeId === node.id,
    inPath: options.pathIds.has(node.id),
    canActivate: options.bindingsById.has(node.id)
  }))
}

function findNodePath(
  nodes: readonly BrowserTreeNode[],
  selectedNodeId: BrowserTreeNodeId | undefined
): readonly BrowserTreeNode[] {
  if (selectedNodeId === undefined) {
    return []
  }

  for (const node of nodes) {
    if (node.id === selectedNodeId) {
      return [node]
    }

    const childPath = findNodePath(childNodes(node), selectedNodeId)

    if (childPath.length > 0) {
      return [node, ...childPath]
    }
  }

  return []
}

function childNodes(node: BrowserTreeNode): readonly BrowserTreeNode[] {
  switch (node.children.kind) {
    case 'loaded':
      return node.children.nodes
    case 'unknown':
    case 'deferred':
    case 'loading':
    case 'failed':
      return [node.children.stateNode]
    case 'none':
    case 'unmaterialized':
      return []
  }
}
