<script setup lang="ts">
import { computed } from 'vue'

import type { WorkstationContextLabel, WorkstationInspectorProps } from './types'

defineOptions({
  name: 'LibraryWorkstationInspector'
})

const props = defineProps<WorkstationInspectorProps>()

const label = computed<WorkstationContextLabel>(() => {
  if (props.selection.kind === 'row') {
    return props.selection.subject.kind === 'playableMedia' ? 'TRACK' : 'NO SELECTION'
  }

  if (props.selection.kind === 'source') {
    switch (props.selection.context.kind) {
      case 'registeredSource':
      case 'registeredDirectory':
      case 'localBrowse':
        return 'SOURCE'
    }
  }

  return 'NO SELECTION'
})

const title = computed(() => {
  if (props.selection.kind === 'row') {
    return props.selection.subject.kind === 'sourceFile'
      ? 'Source file selected'
      : props.selection.subject.label
  }

  if (props.selection.kind === 'source') {
    if (props.status?.title !== undefined) {
      return props.status.title
    }

    return props.selection.context.title
  }

  return 'Nothing selected'
})

const detail = computed(() => {
  if (props.selection.kind === 'row') {
    const subject = props.selection.subject

    if (subject.kind === 'sourceFile') {
      return `${subject.label} is a source file. Track inspection is deferred until playable media identity is available.`
    }

    const observations = [subject.mimeType, subject.codec]
      .filter((value): value is string => value !== undefined && value.trim().length > 0)
      .join(' - ')
    return observations.length > 0
      ? observations
      : (subject.detail ?? 'Playable media identity is available.')
  }

  if (props.selection.kind === 'source') {
    if (props.status?.detail !== undefined) {
      return props.status.detail
    }

    if ('detail' in props.selection.context) {
      return props.selection.context.detail
    }

    return undefined
  }

  return undefined
})

const badge = computed(() => {
  if (props.selection.kind !== 'source') {
    return undefined
  }

  return props.status?.badge
})

const showNoContextCopy = computed(
  () => props.selection.kind === 'none' && detail.value === undefined
)
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

    <div v-if="badge !== undefined" class="mt-3">
      <span
        class="inline-flex min-h-7 items-center rounded-sm border border-(--color-border) px-2 py-1 text-xs font-bold text-(--color-text)"
      >
        {{ badge }}
      </span>
    </div>

    <p v-if="detail !== undefined" class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      {{ detail }}
    </p>
    <p v-else-if="showNoContextCopy" class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      No inspectable context selected.
    </p>
  </aside>
</template>
