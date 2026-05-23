<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { CircleXIcon, FolderPlusIcon, Icon, ScanIcon } from '../icons'
import { projectContents, type ContentRow } from './projection/contents'
import ContentsTable from './components/contentsTable.vue'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useLocalRootActions } from './boundary/localRootActions'
import { useRootLifecycle } from './runtime/rootLifecycle'
import { deriveOperationFeedback } from './projection/operationFeedback'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNodeId } from './tree/types'
import type { BrowserState, RowBinding } from './runtime/state'
import { createViewStateStore } from './runtime/viewState'

defineOptions({
  name: 'LibraryBrowserPanel'
})

const viewStateStore = createViewStateStore()
const hierarchyRead = useLibraryHierarchyRead()
const rootActions = useLocalRootActions()
const rootLifecycle = useRootLifecycle({
  rootActions,
  hierarchyRead: { refresh: hierarchyRead.refresh },
  confirmRemoveSource: () =>
    window.confirm('Remove this source from Dekzer? Your files stay on disk.')
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
const {
  registeredRootPath,
  rootChoiceButtonLabel,
  scanSummary,
  scanButtonLabel,
  removeSourceButtonLabel,
  removeSourceStatus
} = rootActions
const {
  canAddMusicFolder,
  canScanRoot,
  canRemoveSource,
  addMusicFolder,
  scanRoot,
  removeSource,
  hydrateLocalRoots
} = rootLifecycle
const selectedNodeId = ref<BrowserTreeNodeId>()
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const hasUserInteractedWithTree = ref(false)
const hasAppliedInitialPreferredNode = ref(false)
const pendingExpandedRestoreIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
let hydrationAttempted = false
let restoreReadStarted = false
let restoreReadCompleted = false
let projectionRestorationAttempts = 0

const maxProjectionRestorationAttempts = 10

function saveCurrentViewState(): void {
  viewStateStore.save({
    version: 1,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    expandedNodeIds: [...expandedNodeIds.value]
  })
}

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
  emptyLabel: 'Add a music folder to start building your library.',
  labelledBy: 'library-hierarchy-title',
  nodes: currentTreeNodes.value,
  ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
}))

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
    navigationReadResult: navigationReadResult.value,
    removeSourceStatus: removeSourceStatus.value
  })
)

const operationFeedbackToneClass = computed(() => {
  switch (operationFeedback.value.tone) {
    case 'success':
      return 'text-(--color-accent)'
    case 'error':
      return 'text-(--color-accent)'
    case 'warning':
      return 'text-(--color-text-muted)'
    default:
      return 'text-(--color-text)'
  }
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

watch(liveTreeNodes, (nodes) => {
  if (nodes === undefined) {
    return
  }

  if (!restoreReadStarted) {
    restoreReadStarted = true
    void restoreViewStateIfValid()
    return
  }

  if (
    restoreReadCompleted &&
    pendingExpandedRestoreIds.value.size > 0 &&
    !hasUserInteractedWithTree.value
  ) {
    applyPendingRestoredIds()
  }
})

async function restoreViewStateIfValid(): Promise<void> {
  const projection = browserProjection.value

  if (projection?.kind !== 'tree') {
    restoreReadStarted = false
    return
  }

  try {
    const result = await viewStateStore.load()

    restoreReadCompleted = true

    if (result.state !== 'ready') {
      return
    }

    if (hasUserInteractedWithTree.value) {
      return
    }

    const { viewState } = result
    const bindingsById = projection.bindingsById

    if (viewState.selectedNodeId !== undefined && bindingsById.has(viewState.selectedNodeId)) {
      selectedNodeId.value = viewState.selectedNodeId
      hasAppliedInitialPreferredNode.value = true
    }

    const projectableExpandedIds: BrowserTreeNodeId[] = []
    const unprojectableExpandedIds: BrowserTreeNodeId[] = []

    for (const id of viewState.expandedNodeIds) {
      if (bindingsById.has(id)) {
        projectableExpandedIds.push(id)
      } else {
        unprojectableExpandedIds.push(id)
      }
    }

    if (projectableExpandedIds.length > 0) {
      expandedNodeIds.value = new Set(projectableExpandedIds)
      hasAppliedInitialPreferredNode.value = true

      for (const id of projectableExpandedIds) {
        requestNodeChildrenIfExpandable(id, bindingsById)
      }
    }

    if (unprojectableExpandedIds.length > 0) {
      pendingExpandedRestoreIds.value = new Set(unprojectableExpandedIds)
    }
  } catch {
    restoreReadCompleted = true
  }
}

function applyPendingRestoredIds(): void {
  const projection = browserProjection.value

  if (projection?.kind !== 'tree') {
    return
  }

  projectionRestorationAttempts++

  if (projectionRestorationAttempts > maxProjectionRestorationAttempts) {
    pendingExpandedRestoreIds.value = new Set()
    return
  }

  const bindingsById = projection.bindingsById
  const appliedIds: BrowserTreeNodeId[] = []
  const nextPending = new Set<BrowserTreeNodeId>()

  for (const id of pendingExpandedRestoreIds.value) {
    if (bindingsById.has(id)) {
      appliedIds.push(id)
    } else {
      nextPending.add(id)
    }
  }

  if (appliedIds.length === 0) {
    return
  }

  pendingExpandedRestoreIds.value = nextPending
  const nextExpanded = new Set([...expandedNodeIds.value, ...appliedIds])
  expandedNodeIds.value = nextExpanded
  hasAppliedInitialPreferredNode.value = true

  for (const id of appliedIds) {
    requestNodeChildrenIfExpandable(id, bindingsById)
  }
}

function requestNodeChildrenIfExpandable(
  nodeId: BrowserTreeNodeId,
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
): void {
  const binding = bindingsById.get(nodeId)

  if (binding?.kind === 'source' || binding?.kind === 'directory') {
    void requestNodeChildren(nodeId)
  }
}

watch(hostStatus, (status) => {
  if (hydrationAttempted || status?.state !== 'started') {
    return
  }

  hydrationAttempted = true
  void hydrateLocalRoots()
})

function selectNode(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  pendingExpandedRestoreIds.value = new Set()
  selectedNodeId.value = nodeId
  saveCurrentViewState()
}

function toggleNode(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  pendingExpandedRestoreIds.value = new Set()
  const nextExpandedNodeIds = new Set(expandedNodeIds.value)

  if (nextExpandedNodeIds.has(nodeId)) {
    nextExpandedNodeIds.delete(nodeId)
  } else {
    nextExpandedNodeIds.add(nodeId)
  }

  expandedNodeIds.value = nextExpandedNodeIds
  saveCurrentViewState()
}

function activateNodeAction(nodeId: BrowserTreeNodeId): void {
  hasUserInteractedWithTree.value = true
  pendingExpandedRestoreIds.value = new Set()
  expandedNodeIds.value = new Set([...expandedNodeIds.value, nodeId])
  saveCurrentViewState()
  void requestNodeChildren(nodeId)
}

async function handleRemoveSource(): Promise<void> {
  const removed = await removeSource()
  if (removed) {
    selectedNodeId.value = undefined
    expandedNodeIds.value = new Set()
    hasUserInteractedWithTree.value = false
    pendingExpandedRestoreIds.value = new Set()
    viewStateStore.save({
      version: 1,
      expandedNodeIds: []
    })
  }
}

function activateContentRowAction(row: ContentRow): void {
  hasUserInteractedWithTree.value = true
  pendingExpandedRestoreIds.value = new Set()
  const action = row.action

  if (action === undefined) {
    return
  }

  if (action.kind === 'loadChildren') {
    expandedNodeIds.value = new Set([...expandedNodeIds.value, action.nodeId])
    saveCurrentViewState()
  }

  void requestNodeChildren(action.nodeId)
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
          <h2 id="library-hierarchy-title" class="text-xl font-bold leading-7 text-(--color-text)">
            Local Files
          </h2>
        </div>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <button
            type="button"
            class="inline-flex min-h-9 min-w-38.5 items-center justify-center gap-2 rounded-sm border border-(--color-accent) bg-(--color-accent) px-3 py-2 text-sm font-bold text-(--color-background) transition hover:brightness-110 focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canAddMusicFolder"
            @click="addMusicFolder"
          >
            <Icon :icon="FolderPlusIcon" size="md" />
            <span>{{ rootChoiceButtonLabel }}</span>
          </button>
          <button
            v-if="registeredRootPath !== undefined"
            type="button"
            class="inline-flex min-h-9 min-w-31.5 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-background) px-3 py-2 text-sm font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canScanRoot"
            @click="scanRoot"
          >
            <Icon :icon="ScanIcon" size="md" />
            <span>{{ scanButtonLabel }}</span>
          </button>
          <button
            v-if="registeredRootPath !== undefined"
            type="button"
            class="inline-flex min-h-9 items-center justify-center gap-2 rounded-sm border border-(--color-accent) bg-(--color-background) px-3 py-2 text-sm font-bold text-(--color-accent) transition hover:brightness-110 focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canRemoveSource"
            @click="handleRemoveSource"
          >
            <Icon :icon="CircleXIcon" size="md" />
            <span>{{ removeSourceButtonLabel }}</span>
          </button>
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

    <div
      class="grid gap-4 p-5 xl:grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)] grid-rows-1 h-[80svh]"
    >
      <div
        class="min-w-0 overflow-y-auto space-y-4 scrollbar-gutter-stable scrollbar-track-transparent scrollbar-thumb-gray-200"
      >
        <TreeRoot
          v-bind="treeRootProps"
          @select="selectNode"
          @toggle="toggleNode"
          @activate-action="activateNodeAction"
        />
      </div>

      <ContentsTable
        :projection="contentsProjection"
        :activate-row-action="activateContentRowAction"
      />
    </div>
  </section>
</template>
