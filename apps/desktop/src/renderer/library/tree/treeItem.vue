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
const itemElement = ref<HTMLElement>()

watch(
  itemElement,
  (element) => {
    tree.registerItemElement(props.item.id, element instanceof HTMLElement ? element : undefined)
  },
  { immediate: true }
)

onBeforeUnmount(() => {
  tree.cancelPrepareNode(props.item.id)
  tree.registerItemElement(props.item.id)
})

function focusItem(): void {
  tree.focusNode(props.item.id)
}

function activatePrimaryAction(): void {
  focusItem()
  tree.activatePrimary(props.item.id)
}

function handleRowClick(): void {
  focusItem()

  if (props.item.isActionItem) {
    tree.activateAction(props.item.id)
    return
  }

  tree.selectNode(props.item.id)
}

function handleRevealNode(): void {
  tree.revealNode(props.item.id)
}

function handleFocus(): void {
  tree.setActiveNode(props.item.id)
  tree.prepareNode(props.item.id)
}

function handlePointerEnter(): void {
  tree.prepareNode(props.item.id)
}

function handlePointerLeave(): void {
  tree.cancelPrepareNode(props.item.id)
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === 'Enter') {
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

    case 'revealNode':
      tree.revealNode(intent.nodeId)
      return

    case 'activateAction':
      tree.activateAction(intent.nodeId)
      return

    case 'select':
    case 'none':
      return
  }
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
    @click="handleRowClick"
    @focus="handleFocus"
    @keydown="handleKeydown"
    @pointerenter="handlePointerEnter"
    @pointerleave="handlePointerLeave"
  >
    <TreeRow :item="item" @reveal-node="handleRevealNode" />
  </div>
</template>
