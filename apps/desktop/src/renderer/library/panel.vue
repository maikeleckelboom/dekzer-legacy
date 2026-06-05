<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import { CircleXIcon, Icon, ScanIcon } from '../icons'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useContentsRead } from './boundary/contentsRead'
import { useLocalRootActions } from './boundary/localRootActions'
import { useBoundaryEvents, type ScanProgressState } from './boundary/boundaryEvents'
import {
  sourceLifecycleIdsForBrowserContext,
  useSourceLifecycleRead
} from './boundary/sourceLifecycleRead'
import ContentsTable from './contents/table.vue'
import { projectContents, type ContentRow } from './contents/projection'
import { refreshHierarchyForMaintainedSnapshotInvalidation } from './runtime/invalidationRefresh'
import { useRootLifecycle } from './runtime/rootLifecycle'
import { deriveSourceActionModel, hasVisibleSourceRootBinding } from './runtime/sourceActions'
import { projectSourceReadinessByNodeId } from './runtime/sourceReadiness'
import type { BrowserState, RowBinding } from './state'
import { createViewStateStore } from './runtime/viewState'
import { projectState } from './tree/projection'
import TreeRoot from './tree/treeRoot.vue'
import type { BrowserTreeNodeId } from './tree/types'

defineOptions({
  name: 'LibraryPanel'
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
const contentsRead = useContentsRead()
const rootActions = useLocalRootActions()
const boundaryEvents = useBoundaryEvents()
const sourceLifecycleRead = useSourceLifecycleRead()

const scanProgressForRegisteredRoot = computed<ScanProgressState | undefined>(() => {
  const root = rootActions.registeredRoot.value
  if (root === undefined) {
    return undefined
  }
  return boundaryEvents.scanProgress.value.get(root.rootId)
})

const selectedNodeId = ref<BrowserTreeNodeId>()
const expandedNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const pendingRestoreIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())

const restoreState = {
  readStarted: false,
  readCompleted: false,
  projectionAttempts: 0,
  userInteracted: false,
  initialNodeApplied: false
}

const sourceReadinessByNodeId = computed(() =>
  projectSourceReadinessByNodeId({
    projection: hierarchyRead.browserProjection.value,
    localRootsReadState: rootActions.localRootsReadState.value,
    sourceReadStates: hierarchyRead.sourceReadStates.value,
    sourceLifecycleBySourceId: sourceLifecycleRead.sourceLifecycleBySourceId.value,
    scanProgressByRootId: boundaryEvents.scanProgress.value,
    ...(rootActions.registeredRoot.value === undefined
      ? {}
      : { currentScanRootId: rootActions.registeredRoot.value.rootId }),
    currentScanStatus: rootActions.scanStatus.value
  })
)

const browserState = computed<BrowserState>(() => ({
  sourceReadinessByNodeId: sourceReadinessByNodeId.value,
  sourceReadStates: hierarchyRead.sourceReadStates.value,
  directoryReadStates: hierarchyRead.directoryReadStates.value,
  ...(hierarchyRead.hostStatus.value === undefined
    ? {}
    : { hostStatus: hierarchyRead.hostStatus.value }),
  ...(hierarchyRead.navigationReadResult.value === undefined
    ? {}
    : { navigationReadResult: hierarchyRead.navigationReadResult.value })
}))

const browserProjection = computed(() => projectState(browserState.value))

const sourceLifecycleSourceIds = computed(() =>
  sourceLifecycleIdsForBrowserContext({
    projection: hierarchyRead.browserProjection.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    expandedNodeIds: expandedNodeIds.value
  })
)

const rootLifecycle = useRootLifecycle({
  rootActions,
  hierarchyRead: {
    refresh: hierarchyRead.refresh
  },
  confirmRemoveSource: () => window.confirm(removeSourceMessage),
  isSourceRootVisible: (rootId) => hasVisibleSourceRootBinding(browserProjection.value, rootId)
})

const liveTreeNodes = computed(() => browserProjection.value?.nodes ?? [])

const preferredNodeId = computed(() => {
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

const treeRootProps = computed(() => ({
  expandedNodeIds: expandedNodeIds.value,
  emptyLabel: emptyTreeLabel,
  labelledBy: 'library-hierarchy-title',
  nodes: liveTreeNodes.value,
  ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
}))

const contentsProjection = computed(() => {
  const projection = browserProjection.value

  return projectContents({
    state: browserState.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    ...(projection === undefined ? {} : { bindingsById: projection.bindingsById }),
    contentsState: contentsRead.state.value
  })
})

const sourceActionModel = computed(() =>
  deriveSourceActionModel({
    projection: browserProjection.value,
    selectedNodeId: selectedNodeId.value,
    localRootsReadState: rootActions.localRootsReadState.value,
    scanStatus: rootActions.scanStatus.value,
    removeSourceStatus: rootActions.removeSourceStatus.value,
    refreshStatus: rootLifecycle.refreshStatus.value
  })
)

const removeSourceRootId = computed(() => sourceActionModel.value.selectedRemovableSourceRootId)

watch(preferredNodeId, (nodeId) => {
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
})

watch(scanProgressForRegisteredRoot, (progress) => {
  if (progress === undefined) {
    return
  }

  switch (progress.kind) {
    case 'completed':
      rootActions.scanStatus.value = 'scanned'
      rootActions.scanSummary.value = {
        rootId: progress.rootId,
        scanRunId: progress.scanRunId,
        discoveredFileCount: progress.filesDiscovered,
        queuedSourceWorkItems: progress.queuedWorkItems
      }
      break
    case 'failed':
      rootActions.scanStatus.value = 'failed'
      rootActions.scanFailureMessage.value = progress.detail ?? 'Scan failed.'
      break
    case 'blocked':
      rootActions.scanStatus.value = 'blocked'
      rootActions.scanFailureMessage.value = progress.detail ?? 'Scan blocked.'
      break
    case 'cancelled':
      rootActions.scanStatus.value = 'canceled'
      rootActions.scanFailureMessage.value = progress.detail ?? 'Scan canceled.'
      break
  }
})

watch(
  [selectedNodeId, () => browserProjection.value, () => hierarchyRead.hostStatus.value?.state],
  () => {
    requestContentsForCurrentSelection()
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

watch(
  () => hierarchyRead.hostStatus.value?.state,
  (state) => {
    if (state !== 'started') {
      return
    }

    void rootLifecycle.hydrateLocalRoots()
  },
  { immediate: true }
)

watch(
  [sourceLifecycleSourceIds, () => hierarchyRead.hostStatus.value?.state],
  ([sourceIds, hostState]) => {
    if (hostState !== 'started') {
      return
    }

    void sourceLifecycleRead.refreshSourceLifecycles(sourceIds)
  },
  { immediate: true }
)

watch(
  () => boundaryEvents.recoveryNeeded.value,
  async (needed) => {
    if (!needed) {
      return
    }

    await hierarchyRead.refresh()
    contentsRead.clearWarmSnapshots()
    boundaryEvents.acknowledgedGap()
  }
)

watch(
  () => boundaryEvents.maintainedSnapshotInvalidationSignal.value,
  async () => {
    const invalidations = boundaryEvents.consumeMaintainedSnapshotInvalidations()

    for (const event of invalidations) {
      await refreshHierarchyForMaintainedSnapshotInvalidation(event, {
        hierarchyRead,
        sourceLifecycleRead,
        sourceLifecycleSourceIds: sourceLifecycleSourceIds.value,
        expandedNodeIds: expandedNodeIds.value,
        refreshContentsForCurrentSelection: () => {
          contentsRead.clearWarmSnapshots()
          requestContentsForCurrentSelection({ force: true })
        }
      })
    }
  }
)

watch(
  () => boundaryEvents.sourceScanSignal.value,
  () => {
    const sourceIds = sourceLifecycleSourceIds.value
    const refreshedSourceIds = new Set<string>()

    for (const event of boundaryEvents.consumeSourceScanEvents()) {
      if (sourceIds.has(event.rootId)) {
        refreshedSourceIds.add(event.rootId)
      }
    }

    void sourceLifecycleRead.refreshSourceLifecycles(refreshedSourceIds)
  }
)

function saveViewState(): void {
  viewStateStore.save({
    version: 1,
    expandedNodeIds: [...expandedNodeIds.value],
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value })
  })
}

async function restoreViewState(): Promise<void> {
  const projection = browserProjection.value

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
  const projection = browserProjection.value

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
  requestContentsForCurrentSelection()
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

function prepareNodeContents(nodeId: BrowserTreeNodeId): void {
  contentsRead.preloadForBinding(browserProjection.value?.bindingsById.get(nodeId))
}

function cancelPrepareNodeContents(nodeId: BrowserTreeNodeId): void {
  contentsRead.cancelPreloadForBinding(browserProjection.value?.bindingsById.get(nodeId))
}

async function handleRemoveSource(): Promise<void> {
  const rootId = removeSourceRootId.value

  if (rootId === undefined) {
    return
  }

  const removed = await rootLifecycle.removeSource(rootId)

  if (!removed) {
    return
  }

  selectedNodeId.value = undefined
  contentsRead.clear()
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
    void hierarchyRead.requestNodeChildren(action.nodeId)
  } else if (action.kind === 'loadContentsPage') {
    void contentsRead.readForBinding(browserProjection.value?.bindingsById.get(action.nodeId), {
      cursor: action.cursor
    })
  }
}

function requestContentsForCurrentSelection(options: { readonly force?: boolean } = {}): void {
  const selectedId = selectedNodeId.value
  const projection = browserProjection.value

  if (selectedId === undefined || projection === undefined) {
    contentsRead.clear()
    return
  }

  void contentsRead.readForBinding(projection.bindingsById.get(selectedId), {
    ...options
  })
}
</script>

<template>
  <section
    class="flex h-[80svh] flex-col border border-(--color-border) bg-(--color-surface)"
    aria-labelledby="library-hierarchy-title"
  >
    <header class="flex items-center justify-between gap-4">
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
          v-if="sourceActionModel.removeVisible"
          type="button"
          :class="dangerButtonClass"
          :disabled="!sourceActionModel.removeEnabled"
          :title="sourceActionModel.reasonUnavailable"
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
          @prepare="prepareNodeContents"
          @cancel-prepare="cancelPrepareNodeContents"
        />
      </aside>

      <ContentsTable
        :projection="contentsProjection"
        :activate-row-action="activateContentRowAction"
      />
    </div>
  </section>
</template>
