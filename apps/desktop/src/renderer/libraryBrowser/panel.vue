<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundary/status'
import { FolderPlusIcon, Icon, ScanIcon } from '../icons'
import { useLibraryHierarchyRead } from './hierarchyRead'
import { useLocalRootActions } from './localRootActions'
import { useRootLifecycle } from './rootLifecycle'
import { getLoadedBrowserTreeChildren } from './tree/projection'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'
import type { RowBinding } from './hierarchyState'

defineOptions({
  name: 'LibraryBrowserPanel'
})

const hierarchyRead = useLibraryHierarchyRead()
const rootActions = useLocalRootActions()
const rootLifecycle = useRootLifecycle({
  rootActions,
  hierarchyRead: { refresh: hierarchyRead.refresh }
})
const {
  hostStatus,
  navigationReadResult,
  navigationReadRequestError,
  hierarchyReadRequestError,
  navigationReadIsLoading,
  hierarchyReadIsLoading,
  browserProjection,
  requestNodeChildren
} = hierarchyRead
const {
  registeredRootPath,
  rootChoiceButtonLabel,
  rootChoiceFeedback,
  rootChoiceFeedbackClass,
  scanFeedback,
  scanFeedbackClass,
  scanButtonLabel
} = rootActions
const {
  refreshFeedback,
  refreshFeedbackClass,
  canAddMusicFolder,
  canScanRoot,
  addMusicFolder,
  scanRoot
} = rootLifecycle
const selectedNodeId = ref<BrowserTreeNodeId>()
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())

const liveTreeNodes = computed(() => {
  return browserProjection.value?.nodes
})
const preferredLiveNodeId = computed(() => {
  const projection = browserProjection.value

  if (projection === undefined) {
    return undefined
  }

  for (const [nodeId, binding] of projection.bindingsById) {
    if (binding.kind === 'source') {
      return nodeId
    }
  }

  return projection.nodes[0]?.id
})

const currentTreeNodes = computed(() => liveTreeNodes.value ?? [])
const treeRootProps = computed(() => ({
  expandedNodeIds: expandedNodeIds.value,
  emptyLabel: 'No persisted library navigation rows to display.',
  labelledBy: 'library-hierarchy-title',
  nodes: currentTreeNodes.value,
  ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
}))

const modeEyebrow = computed(() => {
  if (navigationReadResult.value?.state === 'ready') {
    return 'Persisted navigation'
  }

  if (navigationReadIsLoading.value) {
    return 'Loading navigation'
  }

  return 'Navigation unavailable'
})

const modeBadge = computed(() =>
  navigationReadResult.value?.state === 'ready' ? 'Local store' : 'Read state'
)

const panelDetail = computed(() => {
  if (navigationReadResult.value?.state === 'ready') {
    return 'Showing maintained navigation rows from the local library store.'
  }

  if (navigationReadRequestError.value !== undefined) {
    return navigationReadRequestError.value
  }

  if (hierarchyReadRequestError.value !== undefined) {
    return hierarchyReadRequestError.value
  }

  if (navigationReadIsLoading.value) {
    return 'Loading maintained navigation rows.'
  }

  if (hierarchyReadIsLoading.value) {
    return 'Loading literal hierarchy rows.'
  }

  if (hostStatus.value === undefined) {
    return 'Checking library boundary host status.'
  }

  if (hostStatus.value.state !== 'started') {
    return `Library boundary host is ${formatHostState(hostStatus.value.state).toLowerCase()}.`
  }

  return 'No maintained navigation read has completed yet.'
})

const selectedSummaryLabel = computed(() =>
  selectedProjectionRow.value === undefined
    ? 'Selected row'
    : formatSelectedRowKind(selectedProjectionRow.value.kind)
)

const selectedSummaryDetail = computed(() => {
  const row = selectedProjectionRow.value

  if (row === undefined) {
    return 'Select a persisted navigation or hierarchy row.'
  }

  switch (row.kind) {
    case 'navigation':
      return `${formatNavigationKind(row.navigationRow.rowKind)} navigation row.`
    case 'source':
      return 'Source entry point backed by a maintained navigation row.'
    case 'directory':
      return 'Literal directory backed by source_directory_id.'
    case 'file':
      return 'Literal file backed by source_file_id.'
    case 'readState':
      return row.detail
    case 'more':
      return row.detail
  }

  return 'Selected library browser row.'
})

const selectedNode = computed(() => {
  if (selectedNodeId.value === undefined) {
    return undefined
  }

  return findNodeById(currentTreeNodes.value, selectedNodeId.value)
})

const selectedProjectionRow = computed(() => {
  if (selectedNodeId.value === undefined) {
    return undefined
  }

  return browserProjection.value?.bindingsById.get(selectedNodeId.value)
})

watch(
  preferredLiveNodeId,
  (preferredNodeId) => {
    if (preferredNodeId !== undefined) {
      selectedNodeId.value = preferredNodeId
      expandedNodeIds.value = new Set([preferredNodeId])
      return
    }

    selectedNodeId.value = undefined
    expandedNodeIds.value = new Set()
  },
  { immediate: true }
)

function selectNode(nodeId: BrowserTreeNodeId): void {
  selectedNodeId.value = nodeId
  void requestNodeChildren(nodeId)
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

function activateNodeAction(nodeId: BrowserTreeNodeId): void {
  expandedNodeIds.value = new Set([...expandedNodeIds.value, nodeId])
  void requestNodeChildren(nodeId)
}

function findNodeById(
  nodes: readonly BrowserTreeNode[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    const childMatch = findNodeById(getLoadedBrowserTreeChildren(node), nodeId)

    if (childMatch !== undefined) {
      return childMatch
    }
  }

  return undefined
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

function formatSelectedRowKind(kind: RowBinding['kind']): string {
  switch (kind) {
    case 'navigation':
      return 'Navigation row'
    case 'source':
      return 'Source entry row'
    case 'directory':
      return 'Literal directory row'
    case 'file':
      return 'Literal file row'
    case 'readState':
      return 'Read state row'
    case 'more':
      return 'More row'
  }
}

function formatNavigationKind(kind: string): string {
  switch (kind) {
    case 'collectionGroup':
      return 'collection group'
    case 'prepPolicyGroup':
      return 'preparation group'
    case 'prepPolicyScope':
      return 'preparation scope'
    case 'locationGroup':
      return 'location group'
    default:
      return kind
  }
}
</script>

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
        <div class="flex flex-wrap items-center justify-end gap-2">
          <button
            type="button"
            class="inline-flex min-h-9 min-w-38.5 items-center justify-center gap-2 rounded-sm border border-(--color-accent) bg-(--color-accent) px-3 py-2 text-sm font-bold text-(--color-background) transition hover:brightness-110 focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canAddMusicFolder"
            @click="addMusicFolder"
          >
            <Icon :icon="FolderPlusIcon" size="md" :decorative="true" />
            <span>{{ rootChoiceButtonLabel }}</span>
          </button>
          <button
            v-if="registeredRootPath !== undefined"
            type="button"
            class="inline-flex min-h-9 min-w-31.5 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-background) px-3 py-2 text-sm font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canScanRoot"
            @click="scanRoot"
          >
            <Icon :icon="ScanIcon" size="md" :decorative="true" />
            <span>{{ scanButtonLabel }}</span>
          </button>
          <span
            class="rounded-sm border px-2.5 py-1 text-xs font-semibold"
            :class="
              navigationReadResult?.state === 'ready'
                ? 'border-(--color-accent) text-(--color-accent)'
                : 'border-(--color-warning) text-(--color-warning)'
            "
          >
            {{ modeBadge }}
          </span>
        </div>
      </div>
      <p class="mt-3 max-w-2xl text-sm leading-6 text-(--color-text-muted)">
        {{ panelDetail }}
      </p>
      <div
        class="mt-3 rounded-sm border border-(--color-border) bg-(--color-background) px-3 py-2"
        aria-live="polite"
      >
        <p class="text-sm font-semibold" :class="rootChoiceFeedbackClass">
          {{ rootChoiceFeedback }}
        </p>
        <p v-if="scanFeedback !== undefined" class="mt-1 text-sm" :class="scanFeedbackClass">
          {{ scanFeedback }}
        </p>
        <p v-if="refreshFeedback !== undefined" class="mt-1 text-sm" :class="refreshFeedbackClass">
          {{ refreshFeedback }}
        </p>
        <p
          v-if="registeredRootPath !== undefined"
          class="mt-1 wrap-anywhere font-mono text-xs text-(--color-text-muted)"
        >
          {{ registeredRootPath }}
        </p>
      </div>
    </header>

    <div class="grid gap-4 p-5">
      <TreeRoot
        v-bind="treeRootProps"
        @select="selectNode"
        @toggle="toggleNode"
        @activate-action="activateNodeAction"
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
