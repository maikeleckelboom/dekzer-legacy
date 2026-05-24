<script setup lang="ts">
import LibraryBrowserLibraryBrowserPanel from './libraryBrowser/LibraryBrowserPanel.vue'
import { AppearanceToggle } from './appearance'
</script>

<template>
  <main
    class="grid h-svh gap-8 overflow-hidden bg-(--color-background) p-8 text-(--color-text)"
    aria-labelledby="dekzer-title"
  >
    <header class="flex justify-end">
      <AppearanceToggle />
    </header>

    <LibraryBrowserLibraryBrowserPanel class="self-center" />
  </main>
</template>
