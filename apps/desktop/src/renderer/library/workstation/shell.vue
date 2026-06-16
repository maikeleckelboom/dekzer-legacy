<script setup lang="ts">
import Bar from './bar.vue'
import Browse from './browse.vue'
import Contents from './contents.vue'
import Inspector from './inspector.vue'
import Preview from './preview.vue'
import type {
  WorkstationContentsProps,
  WorkstationInspectorProps,
  WorkstationTreeProps
} from './types'
import type { BrowserTreeNodeId } from '../tree/types'

defineOptions({
  name: 'LibraryWorkstationShell'
})

defineProps<{
  title: string
  tree: WorkstationTreeProps
  contents: WorkstationContentsProps
  inspector: WorkstationInspectorProps
}>()

defineEmits<{
  select: [nodeId: BrowserTreeNodeId]
  toggle: [nodeId: BrowserTreeNodeId]
  activateAction: [nodeId: BrowserTreeNodeId]
  prepare: [nodeId: BrowserTreeNodeId]
  cancelPrepare: [nodeId: BrowserTreeNodeId]
}>()
</script>

<template>
  <section
    class="flex h-[80svh] min-h-0 flex-col overflow-hidden border border-(--color-border) bg-(--color-surface)"
    role="region"
    aria-label="Library panel"
  >
    <Bar :title="title">
      <template #controls>
        <slot name="bar-controls" />
      </template>
    </Bar>

    <div
      class="grid min-h-0 flex-1 grid-cols-[minmax(13rem,16rem)_minmax(0,1fr)_minmax(14rem,18rem)] grid-rows-[minmax(0,1fr)_3.25rem] overflow-hidden"
    >
      <Browse
        :tree="tree"
        @select="$emit('select', $event)"
        @toggle="$emit('toggle', $event)"
        @activate-action="$emit('activateAction', $event)"
        @prepare="$emit('prepare', $event)"
        @cancel-prepare="$emit('cancelPrepare', $event)"
      />
      <Contents v-bind="contents" />
      <Inspector v-bind="inspector" />
      <Preview class="col-span-3" />
    </div>
  </section>
</template>
