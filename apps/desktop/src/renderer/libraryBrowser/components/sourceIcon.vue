<script setup lang="ts">
import { computed } from 'vue'

import type { LocationSourceIcon, LocationSourceIconBadge } from '../projection/sourcePresentation'
import { Icon } from '../../icons'
import { resolveSourceIcon, resolveSourceIconBadge } from './sourceIconResolver'

defineOptions({
  name: 'SourceIcon'
})

const props = defineProps<{
  icon: LocationSourceIcon
  badge?: LocationSourceIconBadge
  label?: string
}>()

const baseComponent = computed(() => resolveSourceIcon(props.icon))
const badgeComponent = computed(() => resolveSourceIconBadge(props.badge))
</script>

<template>
  <span
    class="relative inline-grid h-7 w-7 shrink-0 place-items-center"
    :aria-hidden="label === undefined"
  >
    <Icon :icon="baseComponent" size="sm" />
    <Icon
      v-if="badgeComponent"
      :icon="badgeComponent"
      size="xs"
      tone="muted"
      class="absolute bottom-0 right-0"
    />
    <span v-if="label" class="sr-only">{{ label }}</span>
  </span>
</template>
