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
  paddingLeft: `${0.75 + (props.item.level - 1) * 0.75}em`
}))

const hasAffordance = computed(() => props.item.canRevealChildren)

const rowIcon = computed<IconComponent | undefined>(() =>
  resolveBrowserTreeRowIcon(props.item.node, props.item.isExpanded)
)

const iconTone = computed<IconTone>(() => {
  const icon = props.item.node.icon
  const role = props.item.node.role

  if (role === 'state' && icon === 'warning') {
    return 'warning'
  }
  if (role === 'action' && icon === 'warning') {
    return 'warning'
  }

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
</script>

<template>
  <div
    class="flex min-w-0 items-center gap-2 rounded-sm border-l-2 py-1.5 text-left group-focus-visible:ring-2 group-focus-visible:ring-(--color-accent)"
    :class="rowClass"
    :style="rowStyle"
    :title="item.node.detail"
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
  </div>
</template>
