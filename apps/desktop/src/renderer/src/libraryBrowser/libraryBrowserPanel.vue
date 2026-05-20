<template>
  <section
    class="border border-(--color-border) bg-(--color-surface)"
    aria-labelledby="library-hierarchy-title"
  >
    <header class="border-b border-(--color-border) px-5 py-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p class="text-xs font-bold uppercase tracking-normal text-(--color-accent)">
            Fixture data only
          </p>
          <h2
            id="library-hierarchy-title"
            class="mt-1 text-xl font-bold leading-7 text-(--color-text)"
          >
            Library hierarchy foundation
          </h2>
        </div>
        <span
          class="rounded-sm border border-(--color-warning) px-2.5 py-1 text-xs font-semibold text-(--color-warning)"
        >
          Demo input
        </span>
      </div>
      <p class="mt-3 max-w-2xl text-sm leading-6 text-(--color-text-muted)">
        Real library commands not wired yet. This panel uses
        <span class="font-semibold text-(--color-text)">
          {{ libraryHierarchyFixtureTree.name }}
        </span>
        for renderer interaction only.
      </p>
    </header>

    <div class="grid gap-4 p-5">
      <TreeRoot
        :expanded-node-ids="expandedNodeIds"
        labelled-by="library-hierarchy-title"
        :nodes="libraryHierarchyFixtureTree.nodes"
        :selected-node-id="selectedNodeId"
        @select="selectNode"
        @toggle="toggleNode"
      />

      <aside
        class="rounded-sm border border-(--color-border) bg-(--color-background) px-4 py-3"
        aria-live="polite"
      >
        <p class="text-xs font-bold uppercase tracking-normal text-(--color-text-muted)">
          Selected fixture row
        </p>
        <p class="mt-1 text-sm font-semibold text-(--color-text)">
          {{ selectedNode?.label ?? 'None selected' }}
        </p>
        <p class="mt-1 text-xs leading-5 text-(--color-text-muted)">
          {{ selectedNode?.detail ?? libraryHierarchyFixtureTree.detail }}
        </p>
      </aside>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

import { libraryHierarchyFixtureTree } from './libraryHierarchyFixture'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'

defineOptions({
  name: 'LibraryBrowserPanel'
})

const selectedNodeId = ref<BrowserTreeNodeId | null>('fixture-root')
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(
  new Set(['fixture-root', 'fixture-tracks', 'fixture-playlists'])
)

const selectedNode = computed(() => {
  if (selectedNodeId.value === null) {
    return null
  }

  return findNodeById(libraryHierarchyFixtureTree.nodes, selectedNodeId.value)
})

function selectNode(nodeId: BrowserTreeNodeId): void {
  selectedNodeId.value = nodeId
}

function toggleNode(nodeId: BrowserTreeNodeId): void {
  const nextExpandedNodeIds = new Set(expandedNodeIds.value)

  if (nextExpandedNodeIds.has(nodeId)) {
    nextExpandedNodeIds.delete(nodeId)
  } else {
    nextExpandedNodeIds.add(nodeId)
  }

  expandedNodeIds.value = nextExpandedNodeIds
}

function findNodeById(
  nodes: readonly BrowserTreeNode[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNode | null {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    const childMatch = findNodeById(node.children ?? [], nodeId)

    if (childMatch !== null) {
      return childMatch
    }
  }

  return null
}
</script>
