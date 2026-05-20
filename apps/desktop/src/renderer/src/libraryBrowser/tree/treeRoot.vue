<template>
  <p
    v-if="visibleItems.length === 0"
    class="rounded-sm border border-dashed border-(--color-border) px-4 py-5 text-sm text-(--color-text-muted)"
    role="status"
  >
    No fixture hierarchy nodes to display.
  </p>

  <div v-else class="space-y-1" role="tree" :aria-labelledby="labelledBy">
    <TreeGroup :items="visibleItems" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import { provideTreeContext } from './context'
import { useTreeController } from './controller'
import TreeGroup from './treeGroup.vue'
import type { TreeNode, TreeNodeId } from './types'

defineOptions({
  name: 'TreeRoot'
})

const props = defineProps<{
  nodes: readonly TreeNode[]
  selectedNodeId: TreeNodeId | null
  expandedNodeIds: ReadonlySet<TreeNodeId>
  labelledBy: string
}>()

const emit = defineEmits<{
  select: [nodeId: TreeNodeId]
  toggle: [nodeId: TreeNodeId]
}>()

const controller = useTreeController({
  nodes: computed(() => props.nodes),
  selectedNodeId: computed(() => props.selectedNodeId),
  expandedNodeIds: computed(() => props.expandedNodeIds),
  selectNode: (nodeId) => emit('select', nodeId),
  toggleNode: (nodeId) => emit('toggle', nodeId)
})

// The visible tree projection is flattened for deterministic keyboard and focus control.
// Each rendered treeitem carries explicit ARIA hierarchy metadata.
const visibleItems = controller.visibleItems

provideTreeContext(controller)
</script>
