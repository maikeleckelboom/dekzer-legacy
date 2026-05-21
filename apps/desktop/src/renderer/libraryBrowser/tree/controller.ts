import type { ComputedRef, Ref } from 'vue'
import { computed, nextTick, ref, watchEffect } from 'vue'

import { resolveTreeKeyboardIntent, type TreeKeyboardIntent } from './keys'
import { flattenVisibleTree, getFirstVisibleNodeId } from './projection'
import type { TreeContext } from './context'
import type { BrowserTreeNode, BrowserTreeNodeId, BrowserTreeVisibleItem } from './types'

export type UseTreeControllerOptions = {
  readonly nodes: ComputedRef<readonly BrowserTreeNode[]>
  readonly selectedNodeId: ComputedRef<BrowserTreeNodeId | undefined>
  readonly expandedNodeIds: ComputedRef<ReadonlySet<BrowserTreeNodeId>>
  readonly selectNode: (nodeId: BrowserTreeNodeId) => void
  readonly toggleNode: (nodeId: BrowserTreeNodeId) => void
  readonly activateAction: (nodeId: BrowserTreeNodeId) => void
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

  function registerItemElement(nodeId: BrowserTreeNodeId, element: HTMLElement | null): void {
    if (element !== null) {
      itemElements.set(nodeId, element)
      return
    }

    itemElements.delete(nodeId)
  }

  function focusNode(nodeId: BrowserTreeNodeId): void {
    setActiveNode(nodeId)

    void nextTick(() => {
      itemElements.get(nodeId)?.focus()
    })
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

    if (item?.canRevealChildren !== true) {
      return
    }

    options.toggleNode(nodeId)
  }

  function activateAction(nodeId: BrowserTreeNodeId): void {
    const item = visibleItems.value.find((visibleItem) => visibleItem.id === nodeId)

    if (item?.canActivateAction !== true) {
      return
    }

    options.activateAction(nodeId)
  }

  return {
    visibleItems,
    activeNodeId,
    getItemTabIndex,
    registerItemElement,
    setActiveNode,
    focusNode,
    selectNode: options.selectNode,
    toggleNode,
    activateAction,
    resolveKeyboardIntent
  }
}
