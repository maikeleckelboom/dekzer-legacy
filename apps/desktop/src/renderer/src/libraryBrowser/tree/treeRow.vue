<template>
  <div
    class="flex min-w-0 items-center gap-2 rounded-sm border-l-2 px-3 py-2 text-left transition group-focus-visible:ring-2 group-focus-visible:ring-(--color-accent) group-focus-visible:ring-offset-2 group-focus-visible:ring-offset-(--color-background)"
    :class="rowClass"
    :style="rowStyle"
  >
    <span
      class="grid h-7 w-7 shrink-0 place-items-center text-xs font-bold"
      :class="item.isBranch ? 'text-(--color-text-muted)' : 'text-(--color-border)'"
      :data-tree-affordance="item.canRevealChildren ? 'true' : undefined"
      aria-hidden="true"
    >
      {{ branchGlyph }}
    </span>

    <span class="min-w-0 flex-1">
      <span class="block truncate text-sm font-semibold leading-5">{{ item.node.label }}</span>
      <span
        v-if="item.node.detail"
        class="block truncate text-xs leading-5 text-(--color-text-muted)"
      >
        {{ item.node.detail }}
      </span>
      <span
        v-if="childrenStateDetail"
        class="block truncate text-xs leading-5 text-(--color-text-muted)"
      >
        {{ childrenStateDetail }}
      </span>
    </span>

    <span
      class="shrink-0 rounded-sm border border-(--color-border) px-2 py-0.5 text-[11px] font-semibold uppercase leading-4 text-(--color-text-muted)"
    >
      {{ kindLabel }}
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { BrowserTreeChildrenState, BrowserTreeNodeKind, BrowserTreeVisibleItem } from './types'

defineOptions({
  name: 'TreeRow'
})

const props = defineProps<{
  item: BrowserTreeVisibleItem
}>()

const rowClass = computed(() => {
  const baseClass = props.item.isSelected
    ? 'border-l-(--color-accent) bg-(--color-surface-strong) text-(--color-text)'
    : 'border-l-transparent text-(--color-text-muted) hover:bg-white/5 hover:text-(--color-text)'
  const branchClass = props.item.isBranch ? 'font-semibold' : ''

  return [baseClass, branchClass]
})

const rowStyle = computed(() => ({
  paddingLeft: `${0.75 + (props.item.level - 1) * 1.25}rem`
}))

const branchGlyph = computed(() => {
  if (!props.item.canRevealChildren) {
    return ''
  }

  return props.item.isExpanded ? '-' : '+'
})

const kindLabel = computed(() => formatNodeKind(props.item.node.kind))
const childrenStateDetail = computed(() => formatChildrenStateDetail(props.item.node.childrenState))

function formatNodeKind(kind: BrowserTreeNodeKind): string {
  switch (kind) {
    case 'fixtureRoot':
      return 'Fixture root'
    case 'source':
      return 'Source'
    case 'folder':
      return 'Folder'
    case 'file':
      return 'File'
    case 'playlistGroup':
      return 'Playlist group'
    case 'preparation':
      return 'Preparation'
    case 'history':
      return 'History'
    case 'trackGroup':
      return 'Track group'
  }
}

function formatChildrenStateDetail(state: BrowserTreeChildrenState): string | undefined {
  switch (state.kind) {
    case 'leaf':
    case 'loaded':
      return undefined
    case 'unloaded':
      return state.detail ?? 'Children not loaded yet.'
    case 'loading':
      return state.detail ?? 'Loading children.'
    case 'failed':
      return state.detail
  }
}
</script>
