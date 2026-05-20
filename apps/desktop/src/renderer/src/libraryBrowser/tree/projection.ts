import type { BrowserTreeNode, BrowserTreeNodeId, BrowserTreeVisibleItem } from './types'

export type FlattenVisibleTreeOptions = {
  readonly nodes: readonly BrowserTreeNode[]
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly activeNodeId?: BrowserTreeNodeId
}

export function flattenVisibleTree(
  options: FlattenVisibleTreeOptions
): readonly BrowserTreeVisibleItem[] {
  const visibleItems: BrowserTreeVisibleItem[] = []

  appendVisibleNodes({
    nodes: options.nodes,
    level: 1,
    visibleItems,
    expandedNodeIds: options.expandedNodeIds,
    ...(options.selectedNodeId === undefined ? {} : { selectedNodeId: options.selectedNodeId }),
    ...(options.activeNodeId === undefined ? {} : { activeNodeId: options.activeNodeId })
  })

  return visibleItems
}

export function getFirstVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[]
): BrowserTreeNodeId | undefined {
  return visibleItems[0]?.id
}

export function getLastVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[]
): BrowserTreeNodeId | undefined {
  return visibleItems.at(-1)?.id
}

export function getNextVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNodeId | undefined {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return undefined
  }

  return visibleItems[visibleIndex + 1]?.id
}

export function getPreviousVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNodeId | undefined {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return undefined
  }

  return visibleItems[visibleIndex - 1]?.id
}

export function getParentVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNodeId | undefined {
  return visibleItems.find((item) => item.id === nodeId)?.parentId
}

export function getFirstChildVisibleNodeId(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNodeId | undefined {
  const visibleIndex = getVisibleItemIndex(visibleItems, nodeId)

  if (visibleIndex === -1) {
    return undefined
  }

  return visibleItems.slice(visibleIndex + 1).find((item) => item.parentId === nodeId)?.id
}

function appendVisibleNodes(options: {
  readonly nodes: readonly BrowserTreeNode[]
  readonly parentId?: BrowserTreeNodeId
  readonly level: number
  readonly visibleItems: BrowserTreeVisibleItem[]
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly activeNodeId?: BrowserTreeNodeId
}): void {
  const siblingCount = options.nodes.length

  options.nodes.forEach((node, nodeIndex) => {
    const children = getNodeChildren(node)
    const hasChildren = children.length > 0
    const isExpanded = hasChildren && options.expandedNodeIds.has(node.id)

    options.visibleItems.push({
      id: node.id,
      node,
      ...(options.parentId === undefined ? {} : { parentId: options.parentId }),
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
        ...(options.selectedNodeId === undefined ? {} : { selectedNodeId: options.selectedNodeId }),
        ...(options.activeNodeId === undefined ? {} : { activeNodeId: options.activeNodeId })
      })
    }
  })
}

function getVisibleItemIndex(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): number {
  return visibleItems.findIndex((item) => item.id === nodeId)
}

function getNodeChildren(node: BrowserTreeNode): readonly BrowserTreeNode[] {
  return node.children ?? []
}
