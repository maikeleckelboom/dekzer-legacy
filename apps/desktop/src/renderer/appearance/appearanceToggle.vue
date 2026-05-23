<script setup lang="ts">
import { computed } from 'vue'
import { useAppearance } from './useAppearance'

const appearance = useAppearance()

const labelMap: Record<string, string> = {
  system: 'System',
  light: 'Light',
  dark: 'Dark'
}

const currentLabel = computed(() => labelMap[appearance.preference.value] || 'System')
const accessibleLabel = computed(
  () => `Switch appearance. Current appearance: ${currentLabel.value}.`
)
</script>

<template>
  <button
    :aria-label="accessibleLabel"
    :title="accessibleLabel"
    class="px-3 py-1.5 rounded text-(--color-text) bg-(--color-surface) hover:bg-(--color-surface-strong) border border-(--color-border) transition-colors"
    @click="appearance.cycle()"
  >
    Appearance: {{ currentLabel }}
  </button>
</template>
