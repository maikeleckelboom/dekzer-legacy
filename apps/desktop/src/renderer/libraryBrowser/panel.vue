<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { CircleXIcon, Icon, ScanIcon } from '../icons'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useLocalRootActions } from './boundary/localRootActions'
import ContentsTable from './components/contentsTable.vue'
import { projectContents, type ContentRow } from './projection/contents'
import { useRootLifecycle } from './runtime/rootLifecycle'
import type { BrowserState, RowBinding } from './runtime/state'
import { createViewStateStore } from './runtime/viewState'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNodeId } from './tree/types'

defineOptions({
  name: 'LibraryBrowserPanel'
})

const emptyTreeLabel = 'Add a music folder to start building your library.'
const removeSourceMessage = 'Remove this source from Dekzer? Your files stay on disk.'
const maxRestoreAttempts = 10

const buttonBaseClass =
  'inline-flex min-h-9 items-center justify-center gap-2 rounded-sm px-3 py-2 text-sm font-bold transition focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60'

const primaryButtonClass = `${buttonBaseClass} min-w-38.5 border border-(--color-accent) bg-(--color-accent) text-(--color-background) hover:brightness-110`
const secondaryButtonClass = `${buttonBaseClass} min-w-31.5 border border-(--color-border) bg-(--color-background) text-(--color-text) hover:border-(--color-accent) hover:text-(--color-accent)`
const dangerButtonClass = `${buttonBaseClass} border border-(--color-accent) bg-(--color-background) text-(--color-accent) hover:brightness-110`

const viewStateStore = createViewStateStore()
const hierarchyRead = useLibraryHierarchyRead()
const rootActions = useLocalRootActions()

const rootLifecycle = useRootLifecycle({
  rootActions,
  hierarchyRead: {
    refresh: hierarchyRead.refresh
  },
  confirmRemoveSource: () => window.confirm(removeSourceMessage)
})

const selectedNodeId = ref<BrowserTreeNodeId>()
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const pendingRestoreIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())

const restoreState = {
  hydrationAttempted: false,
  readStarted: false,
  readCompleted: false,
  projectionAttempts: 0,
  userInteracted: false,
  initialNodeApplied: false
}

const liveTreeNodes = computed(() => hierarchyRead.browserProjection.value?.nodes ?? [])

const preferredNodeId = computed(() => {
  const projection = hierarchyRead.browserProjection.value

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

const treeRootProps = computed(() => ({
  expandedNodeIds: expandedNodeIds.value,
  emptyLabel: emptyTreeLabel,
  labelledBy: 'library-hierarchy-title',
  nodes: liveTreeNodes.value,
  ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
}))

const browserState = computed<BrowserState>(() => ({
  sourceReadStates: hierarchyRead.sourceReadStates.value,
  directoryReadStates: hierarchyRead.directoryReadStates.value,
  ...(hierarchyRead.navigationReadResult.value === undefined
    ? {}
    : { navigationReadResult: hierarchyRead.navigationReadResult.value })
}))

const contentsProjection = computed(() => {
  const projection = hierarchyRead.browserProjection.value

  return projectContents({
    state: browserState.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    ...(projection === undefined ? {} : { bindingsById: projection.bindingsById })
  })
})

watch(
  preferredNodeId,
  (nodeId) => {
    if (restoreState.userInteracted || restoreState.initialNodeApplied) {
      return
    }

    if (nodeId === undefined) {
      selectedNodeId.value = undefined
      expandedNodeIds.value = new Set()
      return
    }

    selectedNodeId.value = nodeId
    expandedNodeIds.value = new Set([nodeId])
    restoreState.initialNodeApplied = true
  },
  { immediate: true }
)

watch(liveTreeNodes, () => {
  if (!restoreState.readStarted) {
    restoreState.readStarted = true
    void restoreViewState()
    return
  }

  if (
    restoreState.readCompleted &&
    pendingRestoreIds.value.size > 0 &&
    !restoreState.userInteracted
  ) {
    applyPendingRestoreIds()
  }
})

watch(hierarchyRead.hostStatus, (status) => {
  if (restoreState.hydrationAttempted || status?.state !== 'started') {
    return
  }

  restoreState.hydrationAttempted = true
  void rootLifecycle.hydrateLocalRoots()
})

function saveViewState(): void {
  viewStateStore.save({
    version: 1,
    expandedNodeIds: [...expandedNodeIds.value],
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
  })
}

async function restoreViewState(): Promise<void> {
  const projection = hierarchyRead.browserProjection.value

  if (projection?.kind !== 'tree') {
    restoreState.readStarted = false
    return
  }

  try {
    const result = await viewStateStore.load()

    restoreState.readCompleted = true

    if (result.state !== 'ready' || restoreState.userInteracted) {
      return
    }

    applyRestoredSelection(result.viewState.selectedNodeId, projection.bindingsById)
    applyRestoredExpansion(result.viewState.expandedNodeIds, projection.bindingsById)
  } catch {
    restoreState.readCompleted = true
  }
}

function applyRestoredSelection(
  nodeId: BrowserTreeNodeId | undefined,
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
): void {
  if (nodeId === undefined || !bindingsById.has(nodeId)) {
    return
  }

  selectedNodeId.value = nodeId
  restoreState.initialNodeApplied = true
}

function applyRestoredExpansion(
  nodeIds: readonly BrowserTreeNodeId[],
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
): void {
  const visibleIds: BrowserTreeNodeId[] = []
  const pendingIds: BrowserTreeNodeId[] = []

  for (const nodeId of nodeIds) {
    if (bindingsById.has(nodeId)) {
      visibleIds.push(nodeId)
    } else {
      pendingIds.push(nodeId)
    }
  }

  if (visibleIds.length > 0) {
    expandedNodeIds.value = new Set(visibleIds)
    restoreState.initialNodeApplied = true

    for (const nodeId of visibleIds) {
      requestNodeChildrenIfExpandable(nodeId, bindingsById)
    }
  }

  if (pendingIds.length > 0) {
    pendingRestoreIds.value = new Set(pendingIds)
  }
}

function applyPendingRestoreIds(): void {
  const projection = hierarchyRead.browserProjection.value

  if (projection?.kind !== 'tree') {
    return
  }

  restoreState.projectionAttempts++

  if (restoreState.projectionAttempts > maxRestoreAttempts) {
    pendingRestoreIds.value = new Set()
    return
  }

  const bindingsById = projection.bindingsById
  const appliedIds: BrowserTreeNodeId[] = []
  const remainingIds = new Set<BrowserTreeNodeId>()

  for (const nodeId of pendingRestoreIds.value) {
    if (bindingsById.has(nodeId)) {
      appliedIds.push(nodeId)
    } else {
      remainingIds.add(nodeId)
    }
  }

  if (appliedIds.length === 0) {
    return
  }

  pendingRestoreIds.value = remainingIds
  expandedNodeIds.value = new Set([...expandedNodeIds.value, ...appliedIds])
  restoreState.initialNodeApplied = true

  for (const nodeId of appliedIds) {
    requestNodeChildrenIfExpandable(nodeId, bindingsById)
  }
}

function requestNodeChildrenIfExpandable(
  nodeId: BrowserTreeNodeId,
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
): void {
  const binding = bindingsById.get(nodeId)

  if (binding?.kind !== 'source' && binding?.kind !== 'directory') {
    return
  }

  void hierarchyRead.requestNodeChildren(nodeId)
}

function markUserInteraction(): void {
  restoreState.userInteracted = true
  pendingRestoreIds.value = new Set()
}

function selectNode(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()
  selectedNodeId.value = nodeId
  saveViewState()
}

function toggleNode(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()

  const nextExpandedIds = new Set(expandedNodeIds.value)

  if (nextExpandedIds.has(nodeId)) {
    nextExpandedIds.delete(nodeId)
  } else {
    nextExpandedIds.add(nodeId)
  }

  expandedNodeIds.value = nextExpandedIds
  saveViewState()
}

function activateNodeAction(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()

  expandedNodeIds.value = new Set([...expandedNodeIds.value, nodeId])
  saveViewState()

  void hierarchyRead.requestNodeChildren(nodeId)
}

async function handleRemoveSource(): Promise<void> {
  const removed = await rootLifecycle.removeSource()

  if (!removed) {
    return
  }

  selectedNodeId.value = undefined
  expandedNodeIds.value = new Set()
  pendingRestoreIds.value = new Set()

  restoreState.userInteracted = false
  restoreState.initialNodeApplied = false

  viewStateStore.save({
    version: 1,
    expandedNodeIds: []
  })
}

function activateContentRowAction(row: ContentRow): void {
  markUserInteraction()

  const action = row.action

  if (action === undefined) {
    return
  }

  if (action.kind === 'loadChildren') {
    expandedNodeIds.value = new Set([...expandedNodeIds.value, action.nodeId])
    saveViewState()
  }

  void hierarchyRead.requestNodeChildren(action.nodeId)
}
</script>

<template>
  <section
    class="flex h-[80svh] flex-col border border-(--color-border) bg-(--color-surface)"
    aria-labelledby="library-hierarchy-title"
  >
    <header class="flex items-center justify-between gap-4 p-2">
      <h2 id="library-hierarchy-title" class="text-xl font-bold leading-none text-(--color-text)">
        Library
      </h2>

      <div class="flex flex-wrap items-center justify-end gap-2">
        <button
          type="button"
          :class="primaryButtonClass"
          :disabled="!rootLifecycle.canAddMusicFolder.value"
          @click="rootLifecycle.addMusicFolder"
        >
          {{ rootActions.rootChoiceButtonLabel.value }}
        </button>

        <button
          v-if="rootActions.registeredRootPath.value !== undefined"
          type="button"
          :class="secondaryButtonClass"
          :disabled="!rootLifecycle.canScanRoot.value"
          @click="rootLifecycle.scanRoot"
        >
          <Icon :icon="ScanIcon" size="md" />
          <span>{{ rootActions.scanButtonLabel.value }}</span>
        </button>

        <button
          v-if="rootActions.registeredRootPath.value !== undefined"
          type="button"
          :class="dangerButtonClass"
          :disabled="!rootLifecycle.canRemoveSource.value"
          @click="handleRemoveSource"
        >
          <Icon :icon="CircleXIcon" size="md" />
          <span>{{ rootActions.removeSourceButtonLabel.value }}</span>
        </button>
      </div>
    </header>

    <div class="grid min-h-0 flex-1 grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)]">
      <aside
        class="min-h-0 min-w-0 overflow-y-auto border-r border-(--color-border) p-1 scrollbar-gutter-stable scrollbar-track-transparent scrollbar-thumb-gray-200"
      >
        <TreeRoot
          v-bind="treeRootProps"
          @select="selectNode"
          @toggle="toggleNode"
          @activate-action="activateNodeAction"
        />
      </aside>

      <ContentsTable
        :projection="contentsProjection"
        :activate-row-action="activateContentRowAction"
      />
    </div>
  </section>
</template>
