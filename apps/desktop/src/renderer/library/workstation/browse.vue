<script setup lang="ts">
import TreeRoot from '../tree/treeRoot.vue'
import type { WorkstationBrowseEvents, WorkstationTreeProps } from './types'

defineOptions({
  name: 'LibraryWorkstationBrowse'
})

defineProps<{
  tree: WorkstationTreeProps
}>()

defineEmits<WorkstationBrowseEvents>()
</script>

<template>
  <aside
    class="min-h-0 min-w-0 overflow-y-auto border-r border-(--color-border) p-1 scrollbar-gutter-stable scrollbar-track-transparent scrollbar-thumb-gray-200"
    aria-label="Browse"
  >
    <TreeRoot
      v-bind="tree"
      @select="$emit('select', $event)"
      @toggle="$emit('toggle', $event)"
      @activate-action="$emit('activateAction', $event)"
      @prepare="$emit('prepare', $event)"
      @cancel-prepare="$emit('cancelPrepare', $event)"
    />
  </aside>
</template>
