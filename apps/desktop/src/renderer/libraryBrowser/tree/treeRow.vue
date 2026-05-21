<script setup lang="ts">
import { computed } from 'vue'

import type { BrowserTreeActionState, BrowserTreeIcon, BrowserTreeVisibleItem } from './types'
import {
  DisclosureClosedIcon,
  DisclosureOpenIcon,
  FileIcon,
  FolderIcon,
  FolderOpenIcon,
  Icon,
  LoadingIcon,
  MoreIcon,
  MusicIcon,
  NavigationIcon,
  SourceIcon,
  StateIcon,
  WarningIcon
} from '../../icons'
import type { IconComponent } from '../../icons'

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

const rowIcon = computed<IconComponent | undefined>(() => resolveRowIcon(props.item))

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

function resolveRowIcon(item: BrowserTreeVisibleItem): IconComponent | undefined {
  const iconKind = resolveExpandedIconKind(item)

  switch (iconKind) {
    case 'source':
      return SourceIcon
    case 'navigation':
      return NavigationIcon
    case 'folder':
      return FolderIcon
    case 'folderOpen':
      return FolderOpenIcon
    case 'file':
      return FileIcon
    case 'music':
      return MusicIcon
    case 'more':
      return MoreIcon
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    case 'state':
      return StateIcon
    default:
      return undefined
  }
}

function resolveExpandedIconKind(item: BrowserTreeVisibleItem): BrowserTreeIcon | undefined {
  if (item.node.icon === 'folder' && item.isExpanded) {
    return 'folderOpen'
  }

  return item.node.icon
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
      v-if="item.node.badgeLabel"
      class="shrink-0 rounded-sm border border-(--color-border) px-2 py-0.5 text-[11px] font-semibold uppercase leading-4 text-(--color-text-muted)"
    >
      {{ item.node.badgeLabel }}
    </span>
  </div>
</template>
