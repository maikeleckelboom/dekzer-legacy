<script setup lang="ts">
import { computed } from 'vue'

import { provideTreeContext } from './context'
import { useTreeController } from './controller'
import TreeItem from './treeItem.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './types'

defineOptions({
  name: 'TreeRoot'
})

const props = defineProps<{
  nodes: readonly BrowserTreeNode[]
  selectedNodeId?: BrowserTreeNodeId
  expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  labelledBy: string
  emptyLabel?: string
}>()

const emit = defineEmits<{
  select: [nodeId: BrowserTreeNodeId]
  toggle: [nodeId: BrowserTreeNodeId]
  activateAction: [nodeId: BrowserTreeNodeId]
}>()

const controller = useTreeController({
  nodes: computed(() => props.nodes),
  selectedNodeId: computed(() => props.selectedNodeId),
  expandedNodeIds: computed(() => props.expandedNodeIds),
  selectNode: (nodeId) => emit('select', nodeId),
  toggleNode: (nodeId) => emit('toggle', nodeId),
  activateAction: (nodeId) => emit('activateAction', nodeId)
})

// The DOM is intentionally flattened: hierarchy is declared through aria-level,
// aria-posinset, and aria-setsize. Keyboard order, visual order, and projection
// order remain identical; hierarchy belongs to the projection, not nested DOM state.
const visibleItems = controller.visibleItems

provideTreeContext(controller)
</script>

<template>
  <p
    v-if="visibleItems.length === 0"
    class="rounded-sm border border-dashed border-(--color-border) px-4 py-5 text-sm text-(--color-text-muted)"
    role="status"
  >
    {{ emptyLabel ?? 'No tree rows to display.' }}
  </p>

  <div v-else class="space-y-1" role="tree" :aria-labelledby="labelledBy">
    <TreeItem v-for="item in visibleItems" :key="item.id" :item="item" />
  </div>
</template>
