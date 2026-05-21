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

export function isBrowserTreeLeaf(node: BrowserTreeNode): boolean {
  return node.children.kind === 'none'
}

export function isBrowserTreeBranch(node: BrowserTreeNode): boolean {
  return node.children.kind !== 'none'
}

export function getLoadedBrowserTreeChildren(node: BrowserTreeNode): readonly BrowserTreeNode[] {
  if (node.children.kind !== 'loaded') {
    return []
  }

  return node.children.children
}

export function canRevealBrowserTreeChildren(node: BrowserTreeNode): boolean {
  return getLoadedBrowserTreeChildren(node).length > 0
}

export function canRequestBrowserTreeChildren(node: BrowserTreeNode): boolean {
  return node.action?.state.kind === 'idle' || node.action?.state.kind === 'failed'
}

export function isLoadingBrowserTreeChildren(node: BrowserTreeNode): boolean {
  return node.action?.state.kind === 'loading'
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
    const children = getLoadedBrowserTreeChildren(node)
    const isBranch = isBrowserTreeBranch(node)
    const canReveal = canRevealBrowserTreeChildren(node)
    const canRequest = canRequestBrowserTreeChildren(node)
    const isLoadingChildren = isLoadingBrowserTreeChildren(node)
    const isExpanded = canReveal && options.expandedNodeIds.has(node.id)

    options.visibleItems.push({
      id: node.id,
      node,
      ...(options.parentId === undefined ? {} : { parentId: options.parentId }),
      level: options.level,
      visibleIndex: options.visibleItems.length,
      isBranch,
      canRevealChildren: canReveal,
      canRequestChildren: canRequest,
      isLoadingChildren,
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
