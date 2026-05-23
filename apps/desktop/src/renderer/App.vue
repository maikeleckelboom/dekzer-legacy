<script setup lang="ts">
import LibraryBrowserPanel from './libraryBrowser/panel.vue'
</script>

<template>
  <main
    class="grid h-svh gap-8 overflow-hidden bg-(--color-background) p-8 text-(--color-text)"
    aria-labelledby="dekzer-title"
  >
    <LibraryBrowserPanel class="self-center" />
  </main>
</template>
