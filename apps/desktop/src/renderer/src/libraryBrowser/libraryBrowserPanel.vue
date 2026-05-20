<template>
  <section
    class="border border-(--color-border) bg-(--color-surface)"
    aria-labelledby="library-hierarchy-title"
  >
    <header class="border-b border-(--color-border) px-5 py-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p class="text-xs font-bold uppercase tracking-normal text-(--color-accent)">
            {{ modeEyebrow }}
          </p>
          <h2
            id="library-hierarchy-title"
            class="mt-1 text-xl font-bold leading-7 text-(--color-text)"
          >
            Library hierarchy foundation
          </h2>
        </div>
        <span
          class="rounded-sm border px-2.5 py-1 text-xs font-semibold"
          :class="
            isLiveTree
              ? 'border-(--color-accent) text-(--color-accent)'
              : 'border-(--color-warning) text-(--color-warning)'
          "
        >
          {{ modeBadge }}
        </span>
      </div>
      <p class="mt-3 max-w-2xl text-sm leading-6 text-(--color-text-muted)">
        {{ panelDetail }}
        <span v-if="!isLiveTree" class="font-semibold text-(--color-text)">
          {{ libraryHierarchyFixtureTree.name }}
        </span>
      </p>
    </header>

    <div class="grid gap-4 p-5">
      <TreeRoot
        :expanded-node-ids="expandedNodeIds"
        labelled-by="library-hierarchy-title"
        :nodes="currentTreeNodes"
        :selected-node-id="selectedNodeId"
        @select="selectNode"
        @toggle="toggleNode"
      />

      <aside
        class="rounded-sm border border-(--color-border) bg-(--color-background) px-4 py-3"
        aria-live="polite"
      >
        <p class="text-xs font-bold uppercase tracking-normal text-(--color-text-muted)">
          {{ selectedSummaryLabel }}
        </p>
        <p class="mt-1 text-sm font-semibold text-(--color-text)">
          {{ selectedNode?.label ?? 'None selected' }}
        </p>
        <p class="mt-1 text-xs leading-5 text-(--color-text-muted)">
          {{ selectedNode?.detail ?? selectedSummaryDetail }}
        </p>
      </aside>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundaryStatus'
import type { LibraryHierarchyReadResult } from '../../../shared/libraryHierarchyRead'
import { libraryHierarchyFixtureTree } from './libraryHierarchyFixture'
import { projectLibraryHierarchyReadToBrowserTree } from './libraryHierarchyProjection'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'

defineOptions({
  name: 'LibraryBrowserPanel'
})

const defaultFixtureExpandedNodeIds = new Set<BrowserTreeNodeId>([
  'fixture-root',
  'fixture-tracks',
  'fixture-playlists'
])
const selectedNodeId = ref<BrowserTreeNodeId | null>(null)
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const hostStatus = ref<LibraryBoundaryHostStatus | null>(null)
const hierarchyReadResult = ref<LibraryHierarchyReadResult | null>(null)
const hierarchyReadBridgeError = ref<string | null>(null)
const hierarchyReadIsLoading = ref(false)
let hasRequestedHierarchyRead = false
let unsubscribeFromHostStatus: (() => void) | null = null

const hierarchyProjection = computed(() => {
  if (hierarchyReadResult.value === null) {
    return null
  }

  return projectLibraryHierarchyReadToBrowserTree(hierarchyReadResult.value)
})

const liveTreeNodes = computed(() => {
  if (hierarchyProjection.value?.kind !== 'tree') {
    return null
  }

  return hierarchyProjection.value.nodes
})

const currentTreeNodes = computed(() => liveTreeNodes.value ?? libraryHierarchyFixtureTree.nodes)
const isLiveTree = computed(() => liveTreeNodes.value !== null)

const modeEyebrow = computed(() => {
  if (isLiveTree.value) {
    return 'Read-only hierarchy'
  }

  if (hierarchyReadResult.value?.state === 'ready') {
    return 'Read-only hierarchy unavailable'
  }

  return 'Fixture fallback'
})

const modeBadge = computed(() => (isLiveTree.value ? 'Live read-only' : 'Demo input'))

const panelDetail = computed(() => {
  if (isLiveTree.value) {
    return 'Showing a real library source through the desktop-owned read path.'
  }

  if (hierarchyReadBridgeError.value !== null) {
    return `${hierarchyReadBridgeError.value} Showing `
  }

  if (hierarchyReadIsLoading.value) {
    return 'Checking for a real library hierarchy. Showing '
  }

  if (hierarchyProjection.value?.kind === 'unsupported') {
    return `${hierarchyProjection.value.message} Showing `
  }

  if (hierarchyProjection.value?.kind === 'unavailable') {
    return `${hierarchyProjection.value.message} Showing `
  }

  if (hostStatus.value === null) {
    return 'Checking library boundary host status. Showing '
  }

  if (hostStatus.value.state !== 'started') {
    return `Library boundary host is ${formatHostState(hostStatus.value.state).toLowerCase()}. Showing `
  }

  return 'No real library hierarchy read has completed yet. Showing '
})

const selectedSummaryLabel = computed(() =>
  isLiveTree.value ? 'Selected live row' : 'Selected fixture row'
)

const selectedSummaryDetail = computed(() =>
  isLiveTree.value
    ? 'Desktop-owned read-only library hierarchy row.'
    : libraryHierarchyFixtureTree.detail
)

const selectedNode = computed(() => {
  if (selectedNodeId.value === null) {
    return null
  }

  return findNodeById(currentTreeNodes.value, selectedNodeId.value)
})

watch(
  liveTreeNodes,
  (nodes) => {
    const liveRootId = nodes?.[0]?.id ?? null

    if (liveRootId !== null) {
      selectedNodeId.value = liveRootId
      expandedNodeIds.value = new Set([liveRootId])
      return
    }

    selectedNodeId.value = 'fixture-root'
    expandedNodeIds.value = new Set(defaultFixtureExpandedNodeIds)
  },
  { immediate: true }
)

onMounted(() => {
  void window.dekzer.libraryBoundary
    .getStatus()
    .then((status) => {
      hostStatus.value = status
      hierarchyReadBridgeError.value = null
      requestHierarchyReadIfStarted(status)
    })
    .catch(() => {
      hierarchyReadBridgeError.value = 'Unable to read library boundary host status.'
    })

  unsubscribeFromHostStatus = window.dekzer.libraryBoundary.onStatusChanged((status) => {
    hostStatus.value = status
    hierarchyReadBridgeError.value = null
    requestHierarchyReadIfStarted(status)
  })
})

onUnmounted(() => {
  unsubscribeFromHostStatus?.()
  unsubscribeFromHostStatus = null
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

function requestHierarchyReadIfStarted(status: LibraryBoundaryHostStatus): void {
  if (status.state !== 'started' || hasRequestedHierarchyRead) {
    return
  }

  hasRequestedHierarchyRead = true
  void readFirstAvailableSourceHierarchy()
}

async function readFirstAvailableSourceHierarchy(): Promise<void> {
  hierarchyReadIsLoading.value = true
  hierarchyReadBridgeError.value = null

  try {
    hierarchyReadResult.value = await window.dekzer.libraryBoundary.readLiteralHierarchyChildren({
      target: {
        kind: 'firstAvailableSource'
      },
      parentSourceDirectoryId: null,
      offset: 0,
      limit: 50
    })
  } catch {
    hierarchyReadBridgeError.value = 'Unable to request library hierarchy children.'
  } finally {
    hierarchyReadIsLoading.value = false
  }
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

function formatHostState(state: LibraryBoundaryHostStatus['state']): string {
  switch (state) {
    case 'idle':
      return 'Idle'
    case 'starting':
      return 'Starting'
    case 'started':
      return 'Started'
    case 'stopping':
      return 'Stopping'
    case 'stopped':
      return 'Stopped'
    case 'failed':
      return 'Failed'
  }
}
</script>
