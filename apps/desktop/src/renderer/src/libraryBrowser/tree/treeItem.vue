<template>
  <div
    :ref="(element) => tree.registerItemElement(item.id, element)"
    class="group outline-none"
    role="treeitem"
    :aria-expanded="getTreeItemAriaExpanded(item)"
    :aria-level="item.level"
    :aria-posinset="item.ariaPosInSet"
    :aria-selected="getTreeItemAriaSelected(item)"
    :aria-setsize="item.ariaSetSize"
    :data-active="item.isActive ? 'true' : undefined"
    :data-expanded="item.hasChildren ? String(item.isExpanded) : undefined"
    :data-selected="item.isSelected ? 'true' : undefined"
    :tabindex="tree.getItemTabIndex(item.id)"
    @click="tree.handleItemClick(item, $event)"
    @focus="tree.setActiveNode(item.id)"
    @keydown="tree.handleItemKeydown(item, $event)"
  >
    <TreeRow :item="item" />
  </div>
</template>

<script setup lang="ts">
import { getTreeItemAriaExpanded, getTreeItemAriaSelected } from './aria'
import { useTreeContext } from './context'
import TreeRow from './treeRow.vue'
import type { TreeVisibleItem } from './types'

defineOptions({
  name: 'TreeItem'
})

defineProps<{
  item: TreeVisibleItem
}>()

const tree = useTreeContext()
</script>
