import type { ComputedRef, Ref } from 'vue'
import { computed, nextTick, ref, watchEffect } from 'vue'

import { resolveTreeKeyboardIntent, type TreeKeyboardIntent } from './keys'
import { flattenVisibleTree, getFirstVisibleNodeId } from './listProjection'
import type { TreeContext } from './context'
import type { BrowserTreeNode, BrowserTreeNodeId, BrowserTreeVisibleItem } from './types'

export type UseTreeControllerOptions = {
  readonly nodes: ComputedRef<readonly BrowserTreeNode[]>
  readonly selectedNodeId: ComputedRef<BrowserTreeNodeId | undefined>
  readonly expandedNodeIds: ComputedRef<ReadonlySet<BrowserTreeNodeId>>
  readonly selectNode: (nodeId: BrowserTreeNodeId) => void
  readonly toggleNode: (nodeId: BrowserTreeNodeId) => void
  readonly activateAction: (nodeId: BrowserTreeNodeId) => void
  readonly prepareNode?: (nodeId: BrowserTreeNodeId) => void
  readonly cancelPrepareNode?: (nodeId: BrowserTreeNodeId) => void
}

export function useTreeController(options: UseTreeControllerOptions): TreeContext {
  const activeNodeId: Ref<BrowserTreeNodeId | undefined> = ref()
  const itemElements = new Map<BrowserTreeNodeId, HTMLElement>()

  const visibleItems = computed(() =>
    flattenVisibleTree({
      nodes: options.nodes.value,
      expandedNodeIds: options.expandedNodeIds.value,
      ...(options.selectedNodeId.value === undefined
        ? {}
        : { selectedNodeId: options.selectedNodeId.value }),
      ...(activeNodeId.value === undefined ? {} : { activeNodeId: activeNodeId.value })
    })
  )

  watchEffect(() => {
    const items = visibleItems.value

    if (items.length === 0) {
      activeNodeId.value = undefined
      return
    }

    if (activeNodeId.value !== undefined && items.some((item) => item.id === activeNodeId.value)) {
      return
    }

    if (
      options.selectedNodeId.value !== undefined &&
      items.some((item) => item.id === options.selectedNodeId.value)
    ) {
      activeNodeId.value = options.selectedNodeId.value
      return
    }

    activeNodeId.value = getFirstVisibleNodeId(items)
  })

  function getItemTabIndex(nodeId: BrowserTreeNodeId): 0 | -1 {
    return activeNodeId.value === nodeId ? 0 : -1
  }

  function setActiveNode(nodeId: BrowserTreeNodeId): void {
    if (visibleItems.value.some((item) => item.id === nodeId)) {
      activeNodeId.value = nodeId
    }
  }

  function registerItemElement(nodeId: BrowserTreeNodeId, element?: HTMLElement): void {
    if (element === undefined) {
      itemElements.delete(nodeId)
      return
    }

    itemElements.set(nodeId, element)
  }

  function focusNode(nodeId: BrowserTreeNodeId): void {
    setActiveNode(nodeId)

    void nextTick(() => {
      itemElements.get(nodeId)?.focus()
    })
  }

  function scrollNodeIntoView(nodeId: BrowserTreeNodeId): void {
    void nextTick(() => {
      itemElements.get(nodeId)?.scrollIntoView({ block: 'nearest' })
    })
  }

  function prepareNode(nodeId: BrowserTreeNodeId): void {
    if (visibleItems.value.some((item) => item.id === nodeId)) {
      options.prepareNode?.(nodeId)
    }
  }

  function cancelPrepareNode(nodeId: BrowserTreeNodeId): void {
    options.cancelPrepareNode?.(nodeId)
  }

  function resolveKeyboardIntent(item: BrowserTreeVisibleItem, key: string): TreeKeyboardIntent {
    return resolveTreeKeyboardIntent({
      key,
      activeNodeId: item.id,
      visibleItems: visibleItems.value
    })
  }

  function toggleNode(nodeId: BrowserTreeNodeId): void {
    const item = visibleItems.value.find((visibleItem) => visibleItem.id === nodeId)

    if (!item?.isBranch) {
      return
    }

    options.toggleNode(nodeId)
  }

  function expandNode(item: BrowserTreeVisibleItem): void {
    if (!item.isExpanded) {
      options.toggleNode(item.id)
    }
  }

  function revealNode(nodeId: BrowserTreeNodeId): void {
    focusNode(nodeId)

    const item = visibleItems.value.find((visibleItem) => visibleItem.id === nodeId)

    if (!item?.isBranch) {
      return
    }

    switch (item.node.children.kind) {
      case 'loaded':
        options.toggleNode(nodeId)
        return

      case 'deferred':
        expandNode(item)
        if (item.canActivateAction) {
          options.activateAction(nodeId)
        }
        return

      case 'unmaterialized':
        expandNode(item)
        if (item.canActivateAction) {
          options.activateAction(nodeId)
        }
        return

      case 'loading':
        expandNode(item)
        return

      case 'failed':
        if (item.canActivateAction) {
          expandNode(item)
          options.activateAction(nodeId)
        }
        return

      case 'unknown':
      case 'none':
        return
    }
  }

  function activateAction(nodeId: BrowserTreeNodeId): void {
    const item = visibleItems.value.find((visibleItem) => visibleItem.id === nodeId)

    if (item?.canActivateAction !== true) {
      return
    }

    options.activateAction(nodeId)
  }

  function activatePrimary(nodeId: BrowserTreeNodeId): void {
    const item = visibleItems.value.find((visibleItem) => visibleItem.id === nodeId)

    if (item === undefined) {
      return
    }

    if (item.isActionItem) {
      if (item.canActivateAction) {
        options.activateAction(nodeId)
      }

      return
    }

    options.selectNode(nodeId)

    if (!item.isBranch) {
      return
    }

    if (item.isExpanded) {
      options.toggleNode(nodeId)
      return
    }

    if (item.canActivateAction) {
      options.activateAction(nodeId)
      return
    }

    if (item.canRevealChildren) {
      options.toggleNode(nodeId)
      return
    }
  }

  return {
    visibleItems,
    activeNodeId,
    getItemTabIndex,
    registerItemElement,
    setActiveNode,
    focusNode,
    scrollNodeIntoView,
    prepareNode,
    cancelPrepareNode,
    selectNode: options.selectNode,
    toggleNode,
    revealNode,
    activateAction,
    activatePrimary,
    resolveKeyboardIntent
  }
}
