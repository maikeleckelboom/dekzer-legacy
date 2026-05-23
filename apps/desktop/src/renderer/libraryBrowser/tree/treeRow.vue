<script setup lang="ts">
import { computed } from 'vue'

import type { BrowserTreeVisibleItem } from './types'
import type { IconComponent, IconTone } from '../../icons'
import { DisclosureClosedIcon, DisclosureOpenIcon, Icon } from '../../icons'
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

const iconTone = computed<IconTone>(() => {
  const icon = props.item.node.icon
  const role = props.item.node.role

  if (role === 'state' && icon === 'warning') return 'warning'
  if (role === 'action' && icon === 'warning') return 'warning'

  switch (icon) {
    case 'music':
    case 'video':
      return 'primary'
    case 'image':
    case 'cueSheet':
    case 'playlist':
    case 'metadata':
      return 'muted'
    case 'warning':
      return 'warning'
    default:
      return 'inherit'
  }
})

const labelClass = computed(() => {
  const icon = props.item.node.icon

  switch (icon) {
    case 'music':
    case 'video':
      return 'text-(--color-text)'
    case 'image':
    case 'cueSheet':
    case 'playlist':
    case 'metadata':
      return 'text-(--color-text-muted)'
    default:
      return ''
  }
})

const badgeToneClass = computed(() => {
  const tone = props.item.node.badge?.tone
  switch (tone) {
    case 'warning':
      return 'border-(--color-warning) text-(--color-warning)'
    case 'danger':
      return 'border-(--color-danger) text-(--color-danger)'
    case 'accent':
      return 'border-(--color-accent) text-(--color-accent)'
    default:
      return 'border-(--color-border) text-(--color-text-muted)'
  }
})
</script>

<template>
  <div
    class="flex min-w-0 items-center gap-2 rounded-sm border-l-2 px-3 py-2 text-left group-focus-visible:ring-2 group-focus-visible:ring-(--color-accent)"
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
      <Icon v-if="rowIcon" :icon="rowIcon" size="sm" :tone="iconTone" />
    </span>

    <span class="min-w-0 flex-1 truncate text-sm font-medium leading-5" :class="labelClass">
      {{ item.node.label }}
    </span>

    <span
      v-if="item.node.badge"
      class="shrink-0 rounded-sm border px-1.5 text-[11px] leading-5"
      :class="badgeToneClass"
      :title="item.node.badge.title"
      :aria-label="item.node.badge.ariaLabel"
    >
      {{ item.node.badge.value }}
    </span>
  </div>
</template>
