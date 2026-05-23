<script setup lang="ts">
import LibraryBrowserPanel from './libraryBrowser/panel.vue'
</script>

<template>
  <main
    class="grid h-svh overflow-y-hidden gap-8 overflow-auto bg-(--color-background) p-8 text-(--color-text)"
    aria-labelledby="dekzer-title"
  >
    <LibraryBrowserPanel class="self-center" />
  </main>
</template>
