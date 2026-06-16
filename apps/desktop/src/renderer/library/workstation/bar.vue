<script setup lang="ts">
defineOptions({
  name: 'LibraryWorkstationBar'
})

defineProps<{
  title: string
}>()
</script>

<template>
  <header
    class="flex shrink-0 items-center justify-between gap-4 border-b border-(--color-border) px-3 py-2"
  >
    <h2 id="library-hierarchy-title" class="text-xl font-bold leading-none text-(--color-text)">
      {{ title }}
    </h2>

    <div class="flex flex-wrap items-center justify-end gap-2">
      <slot name="controls" />
    </div>
  </header>
</template>
