<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { FolderPlusIcon, Icon, ScanIcon } from '../icons'
import { projectContents, type ContentRow } from './projection/contents'
import ContentsTable from './components/contentsTable.vue'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useLocalRootActions } from './boundary/localRootActions'
import { useRootLifecycle } from './runtime/rootLifecycle'
import { deriveOperationFeedback } from './projection/operationFeedback'
import { getLoadedBrowserTreeChildren } from './tree/projection'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'
import type { BrowserState, RowBinding } from './runtime/state'

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
  sourceReadStates,
  directoryReadStates,
  browserProjection,
  requestNodeChildren
} = hierarchyRead
const { registeredRootPath, rootChoiceButtonLabel, scanSummary, scanButtonLabel } = rootActions
const { canAddMusicFolder, canScanRoot, addMusicFolder, scanRoot } = rootLifecycle
const selectedNodeId = ref<BrowserTreeNodeId>()
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const hasUserInteractedWithTree = ref(false)
const hasAppliedInitialPreferredNode = ref(false)

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
  navigationReadResult.value?.state === 'ready' ? 'Active' : 'Unavailable'
)

const operationFeedback = computed(() =>
  deriveOperationFeedback({
    hostStatus: hostStatus.value,
    rootChoiceStatus: rootActions.rootChoiceStatus.value,
    registeredRootPath: registeredRootPath.value,
    scanStatus: rootActions.scanStatus.value,
    scanSummary: scanSummary.value,
    refreshStatus: rootLifecycle.refreshStatus.value,
    navigationReadIsLoading: navigationReadIsLoading.value,
    hierarchyReadIsLoading: hierarchyReadIsLoading.value,
    navigationReadRequestError: navigationReadRequestError.value,
    hierarchyReadRequestError: hierarchyReadRequestError.value,
    navigationReadResult: navigationReadResult.value
  })
)

const operationFeedbackToneClass = computed(() => {
  switch (operationFeedback.value.tone) {
    case 'success':
      return 'text-(--color-accent)'
    case 'error':
      return 'text-(--color-danger)'
    case 'warning':
      return 'text-(--color-warning)'
    default:
      return 'text-(--color-text)'
  }
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
      return 'Persisted library navigation row.'
    case 'source':
      return 'Library source entry point.'
    case 'directory':
      return 'Directory entry from the library hierarchy.'
    case 'file':
      return 'File entry from the library hierarchy.'
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

const browserState = computed<BrowserState>(() => ({
  ...(navigationReadResult.value === undefined
    ? {}
    : { navigationReadResult: navigationReadResult.value }),
  sourceReadStates: sourceReadStates.value,
  directoryReadStates: directoryReadStates.value
}))

const contentsProjection = computed(() =>
  projectContents({
    state: browserState.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    ...(browserProjection.value === undefined
      ? {}
      : { bindingsById: browserProjection.value.bindingsById })
  })
)

watch(
  preferredLiveNodeId,
  (preferredNodeId) => {
    if (hasUserInteractedWithTree.value || hasAppliedInitialPreferredNode.value) {
      return
    }

    if (preferredNodeId !== undefined) {
      selectedNodeId.value = preferredNodeId
      expandedNodeIds.value = new Set([preferredNodeId])
      hasAppliedInitialPreferredNode.value = true
      return
    }

    selectedNodeId.value = undefined
    expandedNodeIds.value = new Set()
  },
  { immediate: true }
)

function selectNode(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  selectedNodeId.value = nodeId
  expandedNodeIds.value = new Set([...expandedNodeIds.value, nodeId])
  void requestNodeChildren(nodeId)
}

function toggleNode(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  const nextExpandedNodeIds = new Set(expandedNodeIds.value)

  if (nextExpandedNodeIds.has(nodeId)) {
    nextExpandedNodeIds.delete(nodeId)
  } else {
    nextExpandedNodeIds.add(nodeId)
  }

  expandedNodeIds.value = nextExpandedNodeIds
}

function activateNodeAction(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  expandedNodeIds.value = new Set([...expandedNodeIds.value, nodeId])
  void requestNodeChildren(nodeId)
}

function activateContentRowAction(row: ContentRow): void {
  hasUserInteractedWithTree.value = true
  const action = row.action

  if (action === undefined) {
    return
  }

  if (action.kind === 'loadChildren') {
    expandedNodeIds.value = new Set([...expandedNodeIds.value, action.nodeId])
  }

  void requestNodeChildren(action.nodeId)
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
      <div class="mt-3" aria-live="polite">
        <p class="text-sm font-semibold leading-6" :class="operationFeedbackToneClass">
          {{ operationFeedback.title }}
        </p>
        <p
          v-if="operationFeedback.detail !== undefined"
          class="mt-1 max-w-2xl text-xs leading-5 text-(--color-text-muted)"
        >
          {{ operationFeedback.detail }}
        </p>
      </div>
    </header>

    <div class="grid gap-4 p-5 xl:grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)]">
      <div class="min-w-0 space-y-4">
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

      <ContentsTable
        :projection="contentsProjection"
        :activate-row-action="activateContentRowAction"
      />
    </div>
  </section>
</template>
