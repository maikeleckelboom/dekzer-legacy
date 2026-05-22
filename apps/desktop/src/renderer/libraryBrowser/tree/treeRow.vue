<script setup lang="ts">
import { computed } from 'vue'

import type {
  BrowserTreeActionState,
  BrowserTreeBadgeTone,
  BrowserTreeBadgeEmphasis,
  BrowserTreeVisibleItem
} from './types'
import { DisclosureClosedIcon, DisclosureOpenIcon, Icon } from '../../icons'
import type { IconComponent } from '../../icons'
import { resolveBrowserTreeRowIcon } from './presentation'

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

const hasAffordance = computed(
  () =>
    !props.item.isActionItem &&
    (props.item.canRevealChildren || props.item.canActivateAction || props.item.isActionLoading)
)

const actionStateDetail = computed(() => formatActionStateDetail(props.item.node.action?.state))

const rowIcon = computed<IconComponent | undefined>(() =>
  resolveBrowserTreeRowIcon(props.item.node, props.item.isExpanded)
)

const badgeClass = computed(() => {
  const badge = props.item.node.badge

  if (badge === undefined) {
    return undefined
  }

  return badgeCssClasses(badge.tone, badge.emphasis)
})

function formatActionStateDetail(state: BrowserTreeActionState | undefined): string | undefined {
  if (state === undefined) {
    return undefined
  }

  switch (state.kind) {
    case 'idle':
      return state.detail
    case 'loading':
      return state.detail ?? 'Loading.'
    case 'failed':
      return state.detail
  }
}

function badgeCssClasses(
  tone: BrowserTreeBadgeTone | undefined,
  emphasis: BrowserTreeBadgeEmphasis | undefined
): string {
  const toneClass = badgeToneCssClass(tone)
  const emphasisClass = badgeEmphasisCssClass(emphasis)

  return [
    'shrink-0 rounded-sm px-2 py-0.5 text-[11px] font-semibold uppercase leading-4',
    toneClass,
    emphasisClass
  ]
    .filter(Boolean)
    .join(' ')
}

function badgeToneCssClass(tone: BrowserTreeBadgeTone | undefined): string {
  switch (tone) {
    case 'neutral':
      return 'border border-(--color-border) text-(--color-text-muted)'
    case 'accent':
      return 'border border-(--color-accent) text-(--color-accent)'
    case 'success':
      return 'border border-(--color-status-success) text-(--color-status-success)'
    case 'warning':
      return 'border border-(--color-status-warning) text-(--color-status-warning)'
    case 'danger':
      return 'border border-(--color-status-danger) text-(--color-status-danger)'
    case 'muted':
    default:
      return 'border border-(--color-border) text-(--color-text-muted)'
  }
}

function badgeEmphasisCssClass(emphasis: BrowserTreeBadgeEmphasis | undefined): string {
  switch (emphasis) {
    case 'soft':
      return 'border-transparent bg-(--color-surface-strong)'
    case 'solid':
      return 'border-transparent bg-(--color-accent) text-(--color-text-on-accent)'
    case 'outline':
    default:
      return 'border border-(--color-border) text-(--color-text-muted)'
  }
}
</script>

<template>
  <div
    class="flex min-w-0 items-center gap-2 rounded-sm border-l-2 px-3 py-2 text-left transition group-focus-visible:ring-2 group-focus-visible:ring-(--color-accent) group-focus-visible:ring-offset-2 group-focus-visible:ring-offset-(--color-background)"
    :class="rowClass"
    :style="rowStyle"
  >
    <span
      class="grid h-7 w-7 shrink-0 place-items-center"
      :data-tree-affordance="hasAffordance ? 'true' : undefined"
      aria-hidden="true"
    >
      <Icon
        v-if="hasAffordance"
        :icon="item.isExpanded ? DisclosureOpenIcon : DisclosureClosedIcon"
        size="sm"
        :decorative="true"
      />
    </span>

    <span class="grid h-7 w-7 shrink-0 place-items-center" aria-hidden="true">
      <Icon v-if="rowIcon" :icon="rowIcon" size="sm" :decorative="true" />
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
        v-if="actionStateDetail"
        class="block truncate text-xs leading-5 text-(--color-text-muted)"
      >
        {{ actionStateDetail }}
      </span>
    </span>

    <span
      v-if="item.node.badge"
      :class="badgeClass"
      :title="item.node.badge.title"
      :aria-label="item.node.badge.ariaLabel"
    >
      {{ item.node.badge.value }}
    </span>
  </div>
</template>
