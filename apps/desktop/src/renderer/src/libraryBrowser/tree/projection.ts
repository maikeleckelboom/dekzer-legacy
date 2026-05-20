import type { TreeNode, TreeNodeId, TreeVisibleItem } from './types'

export type FlattenVisibleTreeOptions = {
  readonly nodes: readonly TreeNode[]
  readonly expandedNodeIds: ReadonlySet<TreeNodeId>
  readonly selectedNodeId: TreeNodeId | null
  readonly activeNodeId?: TreeNodeId | null
}

export function flattenVisibleTree(options: FlattenVisibleTreeOptions): readonly TreeVisibleItem[] {
  const visibleItems: TreeVisibleItem[] = []
  const activeNodeId = options.activeNodeId ?? null

  appendVisibleNodes({
    nodes: options.nodes,
    parentId: null,
    level: 1,
    visibleItems,
    expandedNodeIds: options.expandedNodeIds,
    selectedNodeId: options.selectedNodeId,
    activeNodeId
  })

  return visibleItems
}

export function getFirstVisibleNodeId(visibleItems: readonly TreeVisibleItem[]): TreeNodeId | null {
  return visibleItems[0]?.id ?? null
}

export function getLastVisibleNodeId(visibleItems: readonly TreeVisibleItem[]): TreeNodeId | null {
  return visibleItems.at(-1)?.id ?? null
}

export function getNextVisibleNodeId(
  visibleItems: readonly TreeVisibleItem[],
  nodeId: TreeNodeId
): TreeNodeId | null {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return null
  }

  return visibleItems[visibleIndex + 1]?.id ?? null
}

export function getPreviousVisibleNodeId(
  visibleItems: readonly TreeVisibleItem[],
  nodeId: TreeNodeId
): TreeNodeId | null {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return null
  }

  return visibleItems[visibleIndex - 1]?.id ?? null
}

export function getParentVisibleNodeId(
  visibleItems: readonly TreeVisibleItem[],
  nodeId: TreeNodeId
): TreeNodeId | null {
  return visibleItems.find((item) => item.id === nodeId)?.parentId ?? null
}

export function getFirstChildVisibleNodeId(
  visibleItems: readonly TreeVisibleItem[],
  nodeId: TreeNodeId
): TreeNodeId | null {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return null
  }

  return visibleItems.slice(visibleIndex + 1).find((item) => item.parentId === nodeId)?.id ?? null
}

function appendVisibleNodes(options: {
  readonly nodes: readonly TreeNode[]
  readonly parentId: TreeNodeId | null
  readonly level: number
  readonly visibleItems: TreeVisibleItem[]
  readonly expandedNodeIds: ReadonlySet<TreeNodeId>
  readonly selectedNodeId: TreeNodeId | null
  readonly activeNodeId: TreeNodeId | null
}): void {
  const siblingCount = options.nodes.length

  options.nodes.forEach((node, nodeIndex) => {
    const children = getNodeChildren(node)
    const hasChildren = children.length > 0
    const isExpanded = hasChildren && options.expandedNodeIds.has(node.id)

    options.visibleItems.push({
      id: node.id,
      node,
      parentId: options.parentId,
      level: options.level,
      visibleIndex: options.visibleItems.length,
      hasChildren,
      isExpanded,
      isSelected: options.selectedNodeId === node.id,
      isActive: options.activeNodeId === node.id,
      ariaSetSize: siblingCount,
      ariaPosInSet: nodeIndex + 1
    })

    if (isExpanded) {
      appendVisibleNodes({
        nodes: children,
        parentId: node.id,
        level: options.level + 1,
        visibleItems: options.visibleItems,
        expandedNodeIds: options.expandedNodeIds,
        selectedNodeId: options.selectedNodeId,
        activeNodeId: options.activeNodeId
      })
    }
  })
}

function getVisibleItemIndex(visibleItems: readonly TreeVisibleItem[], nodeId: TreeNodeId): number {
  return visibleItems.findIndex((item) => item.id === nodeId)
}

function getNodeChildren(node: TreeNode): readonly TreeNode[] {
  return node.children ?? []
}
