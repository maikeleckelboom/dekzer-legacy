<script setup lang="ts">
import { computed } from 'vue'

import type { BrowserTreeVisibleItem } from './types'
import { DisclosureClosedIcon, DisclosureOpenIcon, Icon } from '../../icons'
import type { IconComponent } from '../../icons'
import { resolveBrowserTreeRowIcon } from './presentation'

defineOptions({
  name: 'TreeRow'
})

const props = defineProps<{
  item: BrowserTreeVisibleItem
}>()

const rowClass = computed(() =>
  props.item.isSelected
    ? 'border-l-(--color-accent) bg-(--color-surface-strong) text-(--color-text)'
    : 'border-l-transparent text-(--color-text-muted) hover:bg-white/5 hover:text-(--color-text)'
)

const rowStyle = computed(() => ({
  paddingLeft: `${0.75 + (props.item.level - 1) * 1.25}rem`
}))

const hasAffordance = computed(() => props.item.canRevealChildren || props.item.isExpanded)

const rowIcon = computed<IconComponent | undefined>(() =>
  resolveBrowserTreeRowIcon(props.item.node, props.item.isExpanded)
)
</script>

<template>
  <div
    class="flex min-w-0 items-center gap-2 rounded-sm border-l-2 px-3 py-2 text-left transition group-focus-visible:ring-2 group-focus-visible:ring-(--color-accent)"
    :class="rowClass"
    :style="rowStyle"
  >
    <span class="grid size-6 shrink-0 place-items-center" aria-hidden="true">
      <Icon
        v-if="hasAffordance"
        :icon="item.isExpanded ? DisclosureOpenIcon : DisclosureClosedIcon"
        size="sm"
      />
    </span>

    <span class="grid size-6 shrink-0 place-items-center" aria-hidden="true">
      <Icon v-if="rowIcon" :icon="rowIcon" size="sm" />
    </span>

    <span class="min-w-0 flex-1 truncate text-sm font-medium leading-5">
      {{ item.node.label }}
    </span>

    <span
      v-if="item.node.badge"
      class="shrink-0 rounded-sm border border-(--color-border) px-1.5 text-[11px] leading-5 text-(--color-text-muted)"
      :title="item.node.badge.title"
      :aria-label="item.node.badge.ariaLabel"
    >
      {{ item.node.badge.value }}
    </span>
  </div>
</template>
