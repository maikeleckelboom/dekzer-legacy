<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'

import { getTreeItemAriaExpanded, getTreeItemAriaSelected } from './aria'
import { useTreeContext } from './context'
import TreeRow from './treeRow.vue'
import type { BrowserTreeVisibleItem } from './types'

defineOptions({
  name: 'TreeItem'
})

const props = defineProps<{
  item: BrowserTreeVisibleItem
}>()

const tree = useTreeContext()
const itemElement = ref<HTMLElement | null>(null)

watch(
  itemElement,
  (element) => {
    tree.registerItemElement(props.item.id, element)
  },
  { immediate: true }
)

onBeforeUnmount(() => {
  tree.registerItemElement(props.item.id, null)
})

function handleClick(event: MouseEvent): void {
  tree.focusNode(props.item.id)

  if (isBranchAffordanceEvent(event)) {
    if (props.item.canRevealChildren) {
      tree.toggleNode(props.item.id)
      return
    }

    if (props.item.canRequestChildren) {
      tree.requestChildren(props.item.id)
      return
    }

    return
  }

  tree.selectNode(props.item.id)
}

function handleKeydown(event: KeyboardEvent): void {
  const intent = tree.resolveKeyboardIntent(props.item, event.key)

  if (intent.shouldPreventDefault) {
    event.preventDefault()
  }

  switch (intent.kind) {
    case 'focus':
      tree.focusNode(intent.nodeId)
      return
    case 'expand':
    case 'collapse':
      tree.toggleNode(intent.nodeId)
      return
    case 'requestChildren':
      tree.requestChildren(intent.nodeId)
      return
    case 'select':
      tree.selectNode(intent.nodeId)
      return
    case 'none':
      return
  }
}

function isBranchAffordanceEvent(event: MouseEvent): boolean {
  return (
    event.target instanceof HTMLElement &&
    event.target.closest('[data-tree-affordance="true"]') !== null
  )
}
</script>

<template>
  <div
    ref="itemElement"
    class="group outline-none"
    role="treeitem"
    :aria-expanded="getTreeItemAriaExpanded(props.item)"
    :aria-level="props.item.level"
    :aria-posinset="props.item.ariaPosInSet"
    :aria-selected="getTreeItemAriaSelected(props.item)"
    :aria-setsize="props.item.ariaSetSize"
    :data-active="props.item.isActive ? 'true' : undefined"
    :data-expanded="props.item.canRevealChildren ? String(props.item.isExpanded) : undefined"
    :data-selected="props.item.isSelected ? 'true' : undefined"
    :tabindex="tree.getItemTabIndex(props.item.id)"
    @click="handleClick"
    @focus="tree.setActiveNode(props.item.id)"
    @keydown="handleKeydown"
  >
    <TreeRow :item="props.item" />
  </div>
</template>
