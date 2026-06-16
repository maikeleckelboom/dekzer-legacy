<script setup lang="ts">
defineOptions({
  name: 'LibraryWorkstationPreview'
})
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 items-center gap-3 border-t border-(--color-border) bg-(--color-background) px-3 py-1.5"
    aria-label="Preview"
  >
    <div class="min-w-0">
      <h3 class="truncate text-xs font-bold leading-4 text-(--color-text)">Preview</h3>
      <p class="truncate text-xs leading-4 text-(--color-text-muted)">No preview item selected.</p>
    </div>
  </section>
</template>
