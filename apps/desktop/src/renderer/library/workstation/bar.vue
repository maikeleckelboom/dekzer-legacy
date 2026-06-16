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
    class="flex shrink-0 items-center justify-between gap-3 border-b border-(--color-border) px-3 py-1.5"
  >
    <h2 id="library-hierarchy-title" class="text-base font-bold leading-none text-(--color-text)">
      {{ title }}
    </h2>

    <div class="flex flex-wrap items-center justify-end gap-1.5">
      <slot name="controls" />
    </div>
  </header>
</template>
