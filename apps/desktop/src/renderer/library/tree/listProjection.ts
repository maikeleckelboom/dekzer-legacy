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
  return canRevealBrowserTreeChildren(node)
}

export function getLoadedBrowserTreeChildren(node: BrowserTreeNode): readonly BrowserTreeNode[] {
  if (node.children.kind !== 'loaded') {
    return []
  }

  return node.children.nodes
}

export function canRevealBrowserTreeChildren(node: BrowserTreeNode): boolean {
  switch (node.children.kind) {
    case 'none':
    case 'unknown':
      return false
    case 'unmaterialized':
    case 'deferred':
    case 'loading':
    case 'failed':
      return true
    case 'loaded':
      return node.children.nodes.some((child) => child.role !== 'state')
  }
}

export function getBrowserTreeChildRows(node: BrowserTreeNode): readonly BrowserTreeNode[] {
  switch (node.children.kind) {
    case 'none':
    case 'unknown':
    case 'unmaterialized':
      return []
    case 'deferred':
    case 'loading':
    case 'failed':
      return [node.children.stateNode]
    case 'loaded':
      return node.children.nodes
  }
}

export function getBrowserTreeVisibleChildReadiness(
  node: BrowserTreeNode
): BrowserTreeNode | undefined {
  return node.children.kind === 'unknown' ? node.children.stateNode : undefined
}

export function canActivateBrowserTreeAction(node: BrowserTreeNode): boolean {
  return node.action?.state.kind === 'idle' || node.action?.state.kind === 'failed'
}

export function isBrowserTreeActionLoading(node: BrowserTreeNode): boolean {
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
    const children = getBrowserTreeChildRows(node)
    const hasExpandedTerminalState =
      options.expandedNodeIds.has(node.id) && hasOnlyStateChildren(node)
    const hasStableLocalBrowseTerminalState = hasLocalBrowseTerminalState(node)
    const isBranch =
      isBrowserTreeBranch(node) || hasExpandedTerminalState || hasStableLocalBrowseTerminalState
    const canReveal =
      canRevealBrowserTreeChildren(node) ||
      hasExpandedTerminalState ||
      hasStableLocalBrowseTerminalState
    const canActivate = canActivateBrowserTreeAction(node)
    const isActionLoading = isBrowserTreeActionLoading(node)
    const isExpanded =
      isBranch && options.expandedNodeIds.has(node.id) && hasMaterializedVisibleChildren(node)
    const isActionItem = node.action !== undefined && !isBranch
    const childReadinessNode = getBrowserTreeVisibleChildReadiness(node)

    options.visibleItems.push({
      id: node.id,
      node,
      ...(childReadinessNode === undefined ? {} : { childReadinessNode }),
      ...(options.parentId === undefined ? {} : { parentId: options.parentId }),
      level: options.level,
      visibleIndex: options.visibleItems.length,
      isBranch,
      canRevealChildren: canReveal,
      canActivateAction: canActivate,
      isActionLoading,
      isActionItem,
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

function hasMaterializedVisibleChildren(node: BrowserTreeNode): boolean {
  return getBrowserTreeChildRows(node).length > 0
}

function hasOnlyStateChildren(node: BrowserTreeNode): boolean {
  return (
    isDisclosureTerminalOwner(node) &&
    node.children.kind === 'loaded' &&
    node.children.nodes.length > 0 &&
    node.children.nodes.every((child) => child.role === 'state')
  )
}

function isDisclosureTerminalOwner(node: BrowserTreeNode): boolean {
  return (
    node.id.startsWith('navigation-row:') ||
    node.id.startsWith('source-directory:') ||
    node.id.startsWith('local-browse-entry:') ||
    node.id.startsWith('local-browse-item:')
  )
}

function hasLocalBrowseTerminalState(node: BrowserTreeNode): boolean {
  return (
    isStableLocalBrowseTerminalOwner(node) &&
    node.children.kind === 'loaded' &&
    node.children.nodes.length > 0 &&
    node.children.nodes.every((child) => child.role === 'state')
  )
}

function isStableLocalBrowseTerminalOwner(node: BrowserTreeNode): boolean {
  return node.id.startsWith('local-browse-entry:') || node.id.startsWith('local-browse-item:')
}

function getVisibleItemIndex(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): number {
  return visibleItems.findIndex((item) => item.id === nodeId)
}
