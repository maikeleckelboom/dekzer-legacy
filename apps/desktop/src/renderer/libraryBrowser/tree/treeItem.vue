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

function focusItem(): void {
  tree.focusNode(props.item.id)
}

function activatePrimaryAction(): void {
  focusItem()
  tree.activatePrimary(props.item.id)
}

function handleClick(): void {
  activatePrimaryAction()
}

function handleFocus(): void {
  tree.setActiveNode(props.item.id)
}

function handleKeydown(event: KeyboardEvent): void {
  if (isPrimaryActivationKey(event.key)) {
    event.preventDefault()
    activatePrimaryAction()
    return
  }

  const intent = tree.resolveKeyboardIntent(props.item, event.key)

  if (intent.shouldPreventDefault) {
    event.preventDefault()
  }

  applyKeyboardIntent(intent)
}

function applyKeyboardIntent(intent: ReturnType<typeof tree.resolveKeyboardIntent>): void {
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
    class="group select-none outline-none"
    role="treeitem"
    :aria-expanded="getTreeItemAriaExpanded(item)"
    :aria-level="item.level"
    :aria-posinset="item.ariaPosInSet"
    :aria-selected="getTreeItemAriaSelected(item)"
    :aria-setsize="item.ariaSetSize"
    :data-active="item.isActive ? 'true' : undefined"
    :data-expanded="item.isBranch ? String(item.isExpanded) : undefined"
    :data-selected="item.isSelected ? 'true' : undefined"
    :tabindex="tree.getItemTabIndex(item.id)"
    @click="handleClick"
    @focus="handleFocus"
    @keydown="handleKeydown"
  >
    <TreeRow :item="item" />
  </div>
</template>
