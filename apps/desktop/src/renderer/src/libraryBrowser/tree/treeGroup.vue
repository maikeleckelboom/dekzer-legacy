<template>
  <TreeItem v-for="item in items" :key="item.id" :item="item" />
</template>

<script setup lang="ts">
import TreeItem from './treeItem.vue'
import type { TreeVisibleItem } from './types'

defineOptions({
  name: 'TreeGroup'
})

defineProps<{
  items: readonly TreeVisibleItem[]
}>()
</script>
