<script setup lang="ts">
import { computed } from 'vue'

import type { WorkstationContextLabel, WorkstationInspectorProps } from './types'

defineOptions({
  name: 'LibraryWorkstationInspector'
})

const props = defineProps<WorkstationInspectorProps>()

const label = computed<WorkstationContextLabel>(() => {
  switch (props.selection.kind) {
    case 'registeredFile':
      return 'TRACK'
    case 'registeredSource':
    case 'registeredDirectory':
    case 'localBrowse':
      return 'SOURCE'
    case 'none':
    case 'navigation':
    case 'readState':
      return 'NO SELECTION'
  }

  return 'NO SELECTION'
})

const title = computed(() => {
  if (props.status?.title !== undefined) {
    return props.status.title
  }

  if ('title' in props.selection) {
    return props.selection.title
  }

  return 'Nothing selected'
})

const detail = computed(() => {
  if (props.status?.detail !== undefined) {
    return props.status.detail
  }

  if ('detail' in props.selection) {
    return props.selection.detail
  }

  return undefined
})
</script>

<template>
  <aside
    class="min-h-0 min-w-0 border-l border-(--color-border) bg-(--color-surface) p-3"
    aria-label="Inspector"
  >
    <p class="text-xs font-bold tracking-normal text-(--color-text-muted)">
      {{ label }}
    </p>
    <h3 class="mt-2 truncate text-sm font-bold leading-5 text-(--color-text)" :title="title">
      {{ title }}
    </h3>

    <div v-if="status?.badge !== undefined" class="mt-3">
      <span
        class="inline-flex min-h-7 items-center rounded-sm border border-(--color-border) px-2 py-1 text-xs font-bold text-(--color-text)"
      >
        {{ status.badge }}
      </span>
    </div>

    <p v-if="detail !== undefined" class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      {{ detail }}
    </p>
    <p v-else class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      No inspectable context selected.
    </p>
  </aside>
</template>
