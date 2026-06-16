<script setup lang="ts">
defineOptions({
  name: 'LibraryWorkstationPreview'
})
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 items-center gap-4 border-t border-(--color-border) bg-(--color-background) px-3 py-2"
    aria-label="Preview"
  >
    <div
      class="grid h-12 w-28 shrink-0 place-items-center rounded-sm border border-(--color-border) text-xs font-bold text-(--color-text-muted)"
    >
      Slot 1
    </div>
    <div class="min-w-0">
      <h3 class="truncate text-sm font-bold leading-5 text-(--color-text)">Preview</h3>
      <p class="truncate text-xs leading-5 text-(--color-text-muted)">No preview item selected.</p>
    </div>
  </section>
</template>
