import type { ComponentPublicInstance, ComputedRef, Ref } from 'vue'
import { computed, nextTick, ref, watchEffect } from 'vue'

import { resolveTreeKeyboardIntent } from './keys'
import { flattenVisibleTree, getFirstVisibleNodeId } from './projection'
import type { TreeContext } from './context'
import type { TreeNode, TreeNodeId, TreeVisibleItem } from './types'

export type UseTreeControllerOptions = {
  readonly nodes: ComputedRef<readonly TreeNode[]>
  readonly selectedNodeId: ComputedRef<TreeNodeId | null>
  readonly expandedNodeIds: ComputedRef<ReadonlySet<TreeNodeId>>
  readonly selectNode: (nodeId: TreeNodeId) => void
  readonly toggleNode: (nodeId: TreeNodeId) => void
}

export function useTreeController(options: UseTreeControllerOptions): TreeContext {
  const activeNodeId: Ref<TreeNodeId | null> = ref(null)
  const itemElements = new Map<TreeNodeId, HTMLElement>()

  const visibleItems = computed(() =>
    flattenVisibleTree({
      nodes: options.nodes.value,
      expandedNodeIds: options.expandedNodeIds.value,
      selectedNodeId: options.selectedNodeId.value,
      activeNodeId: activeNodeId.value
    })
  )

  watchEffect(() => {
    const items = visibleItems.value

    if (items.length === 0) {
      activeNodeId.value = null
      return
    }

    if (activeNodeId.value !== null && items.some((item) => item.id === activeNodeId.value)) {
      return
    }

    if (
      options.selectedNodeId.value !== null &&
      items.some((item) => item.id === options.selectedNodeId.value)
    ) {
      activeNodeId.value = options.selectedNodeId.value
      return
    }

    activeNodeId.value = getFirstVisibleNodeId(items)
  })

  function getItemTabIndex(nodeId: TreeNodeId): 0 | -1 {
    return activeNodeId.value === nodeId ? 0 : -1
  }

  function setActiveNode(nodeId: TreeNodeId): void {
    if (visibleItems.value.some((item) => item.id === nodeId)) {
      activeNodeId.value = nodeId
    }
  }

  function registerItemElement(
    nodeId: TreeNodeId,
    element: Element | ComponentPublicInstance | null
  ): void {
    if (element instanceof HTMLElement) {
      itemElements.set(nodeId, element)
      return
    }

    itemElements.delete(nodeId)
  }

  function focusNode(nodeId: TreeNodeId): void {
    setActiveNode(nodeId)

    void nextTick(() => {
      itemElements.get(nodeId)?.focus()
    })
  }

  function handleItemClick(item: TreeVisibleItem, event: MouseEvent): void {
    focusNode(item.id)

    if (item.hasChildren && isBranchAffordanceEvent(event)) {
      options.toggleNode(item.id)
      return
    }

    options.selectNode(item.id)
  }

  function handleItemKeydown(item: TreeVisibleItem, event: KeyboardEvent): void {
    const intent = resolveTreeKeyboardIntent({
      key: event.key,
      activeNodeId: item.id,
      visibleItems: visibleItems.value
    })

    if (intent.shouldPreventDefault) {
      event.preventDefault()
    }

    switch (intent.kind) {
      case 'focus':
        focusNode(intent.nodeId)
        return
      case 'expand':
      case 'collapse':
        options.toggleNode(intent.nodeId)
        return
      case 'select':
        options.selectNode(intent.nodeId)
        return
      case 'none':
        return
    }
  }

  return {
    visibleItems,
    activeNodeId,
    getItemTabIndex,
    setActiveNode,
    registerItemElement,
    handleItemClick,
    handleItemKeydown
  }
}

function isBranchAffordanceEvent(event: MouseEvent): boolean {
  return (
    event.target instanceof HTMLElement &&
    event.target.closest('[data-tree-affordance="true"]') !== null
  )
}
