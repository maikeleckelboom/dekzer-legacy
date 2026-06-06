<script setup lang="ts">
import { computed, watch } from 'vue'

import { provideTreeContext } from './context'
import { useTreeController } from './controller'
import TreeItem from './treeItem.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './types'

defineOptions({
  name: 'TreeRoot'
})

const { nodes, expandedNodeIds, selectedNodeId, revealRequest } = defineProps<{
  nodes: readonly BrowserTreeNode[]
  selectedNodeId?: BrowserTreeNodeId
  revealRequest?: {
    readonly nodeId: BrowserTreeNodeId
    readonly sequence: number
  }
  expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  labelledBy: string
  emptyLabel?: string
}>()

const emit = defineEmits<{
  select: [nodeId: BrowserTreeNodeId]
  toggle: [nodeId: BrowserTreeNodeId]
  activateAction: [nodeId: BrowserTreeNodeId]
  prepare: [nodeId: BrowserTreeNodeId]
  cancelPrepare: [nodeId: BrowserTreeNodeId]
}>()

const controller = useTreeController({
  nodes: computed(() => nodes),
  selectedNodeId: computed(() => selectedNodeId),
  expandedNodeIds: computed(() => expandedNodeIds),
  selectNode: (id) => emit('select', id),
  toggleNode: (id) => emit('toggle', id),
  activateAction: (id) => emit('activateAction', id),
  prepareNode: (id) => emit('prepare', id),
  cancelPrepareNode: (id) => emit('cancelPrepare', id)
})

// The DOM is intentionally flattened: hierarchy is declared through aria-level,
// aria-posinset, and aria-setsize. Keyboard order, visual order, and projection
// order remain identical; hierarchy belongs to the projection, not the nested DOM state.
const visibleItems = controller.visibleItems

watch(
  () => revealRequest,
  (request) => {
    if (request !== undefined) {
      controller.scrollNodeIntoView(request.nodeId)
    }
  },
  { immediate: true }
)

provideTreeContext(controller)
</script>

<template>
  <p
    v-if="visibleItems.length === 0"
    class="rounded-sm border border-dashed select-none border-(--color-border) text-sm text-(--color-text-muted)"
    role="status"
  >
    {{ emptyLabel ?? 'Nothing to display' }}
  </p>

  <div v-else role="tree" :aria-labelledby="labelledBy">
    <TreeItem v-for="item in visibleItems" :key="item.id" :item="item" />
  </div>
</template>
