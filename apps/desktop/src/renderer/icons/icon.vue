<script setup lang="ts">
import { computed } from 'vue'

import { iconSizeClass, iconToneClass } from './tokens'
import type { IconProps } from './types'

const props = defineProps<IconProps>()

const resolvedSize = computed(() => props.size ?? 'md')
const resolvedTone = computed(() => props.tone ?? 'inherit')
const isDecorative = computed(() => props.decorative ?? true)
const sizeClass = computed(() => iconSizeClass[resolvedSize.value])
const toneClass = computed(() => iconToneClass[resolvedTone.value])
const iconClass = computed(() => ['app-icon', sizeClass.value, toneClass.value])
</script>

<template>
  <component
    :is="icon"
    :class="iconClass"
    :aria-hidden="isDecorative ? 'true' : undefined"
    :aria-label="isDecorative ? undefined : label"
    :role="isDecorative ? 'presentation' : 'img'"
  />
</template>
