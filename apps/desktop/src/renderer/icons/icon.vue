<script setup lang="ts">
import { computed } from 'vue'

import { iconSizeClass, iconToneClass } from './tokens'
import type { IconProps } from './types'

const props = withDefaults(defineProps<IconProps>(), {
  accessibility: 'hidden',
  frame: 'none'
})

const resolvedSize = computed(() => props.size ?? 'md')
const resolvedTone = computed(() => props.tone ?? 'inherit')
const isHidden = computed(() => props.accessibility === 'hidden')
const sizeClass = computed(() => iconSizeClass[resolvedSize.value])
const toneClass = computed(() => iconToneClass[resolvedTone.value])
const iconClass = computed(() => {
  const classes = ['app-icon', sizeClass.value, toneClass.value]
  if (props.frame === 'square') {
    classes.push('app-icon--framed')
  }
  return classes
})
</script>

<template>
  <component
    :is="icon"
    :class="iconClass"
    :aria-hidden="isHidden ? 'true' : undefined"
    :aria-label="isHidden ? undefined : label"
    :role="isHidden ? 'presentation' : 'img'"
  />
</template>
