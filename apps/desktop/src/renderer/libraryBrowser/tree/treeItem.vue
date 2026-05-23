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

function handleClick(): void {
  tree.focusNode(props.item.id)
  tree.activatePrimary(props.item.id)
}

function handleKeydown(event: KeyboardEvent): void {
  if (isPrimaryActivationKey(event.key)) {
    event.preventDefault()
    tree.focusNode(props.item.id)
    tree.activatePrimary(props.item.id)
    return
  }

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
    case 'activateAction':
      tree.activateAction(intent.nodeId)
      return
    case 'select':
    case 'none':
      return
  }
}

function isPrimaryActivationKey(key: string): boolean {
  return key === 'Enter' || key === ' ' || key === 'Space' || key === 'Spacebar'
}
</script>

<template>
  <div
    ref="itemElement"
    class="group outline-none select-none"
    role="treeitem"
    :aria-expanded="getTreeItemAriaExpanded(props.item)"
    :aria-level="props.item.level"
    :aria-posinset="props.item.ariaPosInSet"
    :aria-selected="getTreeItemAriaSelected(props.item)"
    :aria-setsize="props.item.ariaSetSize"
    :data-active="props.item.isActive ? 'true' : undefined"
    :data-expanded="props.item.isBranch ? String(props.item.isExpanded) : undefined"
    :data-selected="props.item.isSelected ? 'true' : undefined"
    :tabindex="tree.getItemTabIndex(props.item.id)"
    @click="handleClick"
    @focus="tree.setActiveNode(props.item.id)"
    @keydown="handleKeydown"
  >
    <TreeRow :item="props.item" />
  </div>
</template>
