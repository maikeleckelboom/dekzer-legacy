<script setup lang="ts">
import { computed } from 'vue'

import type { BrowserTreeVisibleItem } from './types'
import { Icon, type IconRole, type IconTone } from '../../icons'
import { resolveBrowserTreeRowIcon, resolveBrowserTreeStateIcon } from './presentation'

defineOptions({
  name: 'TreeRow'
})

const props = defineProps<{
  item: BrowserTreeVisibleItem
}>()

const emit = defineEmits<{
  revealNode: []
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

const rowIcon = computed<IconRole | undefined>(() => resolveBrowserTreeRowIcon(props.item.node))

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
    case 'metadata':
      return 'muted'
    case 'warning':
      return 'warning'
    default:
      return 'inherit'
  }
})

const childReadinessIcon = computed<IconRole | undefined>(() =>
  props.item.childReadinessNode === undefined
    ? undefined
    : resolveBrowserTreeStateIcon(props.item.childReadinessNode.icon)
)

const childReadinessTone = computed<IconTone>(() => {
  switch (props.item.childReadinessNode?.icon) {
    case 'warning':
      return 'warning'
    case 'loading':
      return 'muted'
    default:
      return 'muted'
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
    <button
      v-if="hasAffordance"
      class="grid size-6 shrink-0 place-items-center rounded-sm text-inherit hover:bg-white/10 focus:outline-none"
      type="button"
      tabindex="-1"
      :aria-label="item.isExpanded ? 'Collapse' : 'Expand'"
      @click.stop="emit('revealNode')"
      @mousedown.prevent
    >
      <Icon :role="item.isExpanded ? 'disclosure.open' : 'disclosure.closed'" size="sm" />
    </button>
    <span v-else class="grid size-6 shrink-0 place-items-center" aria-hidden="true"> </span>

    <span class="grid size-6 shrink-0 place-items-center" aria-hidden="true">
      <Icon v-if="rowIcon" :role="rowIcon" size="sm" :tone="iconTone" />
    </span>

    <span class="min-w-0 flex-1 truncate text-sm font-medium leading-5" :class="labelClass">
      {{ item.node.label }}
    </span>

    <span
      v-if="item.childReadinessNode"
      class="flex min-w-0 max-w-[45%] shrink items-center gap-1 text-xs leading-4 text-(--color-text-muted)"
      :title="item.childReadinessNode.detail"
    >
      <Icon
        v-if="childReadinessIcon"
        :role="childReadinessIcon"
        size="xs"
        :tone="childReadinessTone"
      />
      <span class="min-w-0 truncate">{{ item.childReadinessNode.label }}</span>
    </span>
  </div>
</template>
