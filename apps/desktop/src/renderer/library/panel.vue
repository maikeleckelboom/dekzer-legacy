<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import { Icon } from '../icons'
import { profileLabel, profileOptions, type ProfileKey } from './browseProfile/types'
import { createProfileController } from './browseProfile/controller'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useContentsRead } from './boundary/contentsRead'
import { useLocalRootActions } from './boundary/localRootActions'
import { useLocalBrowseController } from './localBrowse/controller'
import {
  createLocalPreviewModeController,
  localPreviewModeLabel,
  localPreviewModeOptions,
  type LocalPreviewMode
} from './localBrowse/previewMode'
import { useBoundaryEvents, type ScanProgressState } from './boundary/boundaryEvents'
import {
  sourceLifecycleIdsForBrowserContext,
  useSourceLifecycleRead
} from './boundary/sourceLifecycleRead'
import { useRead as useIntegrityRead } from './sourceIntegrity/read'
import { useRead as useMaintenanceRead } from './sourceMaintenance/read'
import ContentsTable from './contents/table.vue'
import { projectContents, type ContentRow } from './contents/projection'
import { projectSearchFilterContents } from './searchFilter/contentsProjection'
import {
  buildGapPlan,
  buildInvalidationPlan,
  buildScanPlan,
  executeRefreshPlan,
  type RefreshPlanDeps
} from './runtime/invalidationRefresh'
import { createDisclosureReconciler } from './runtime/disclosureReconciliation'
import { createLibrarySearchController } from './runtime/librarySearch'
import { useSearchFilterRead } from './runtime/searchFilterState'
import { useRootLifecycle } from './runtime/rootLifecycle'
import {
  hasVisibleSourceRootBinding,
  resolveVisibleSourceRegistration,
  sourceRegistrationIntent,
  type SourceRegistrationIntent
} from './runtime/sourceActions'
import { projectLibraryToolbar } from './runtime/toolbarProjection'
import { projectSourceReadinessByNodeId } from './runtime/sourceReadiness'
import { projectStatusContext } from './sourceStatus/context'
import { projectStatusView, type StatusAction } from './sourceStatus/projection'
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
const maintenanceRunningRefreshMs = 1500

const buttonBaseClass =
  'inline-flex min-h-9 items-center justify-center gap-2 rounded-sm px-3 py-2 text-sm font-bold transition focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60'

const primaryButtonClass = `${buttonBaseClass} min-w-38.5 border border-(--color-accent) bg-(--color-accent) text-(--color-background) hover:brightness-110`
const iconButtonClass = `${buttonBaseClass} h-9 w-9 min-w-0 border border-(--color-border) bg-(--color-background) p-0 text-(--color-text) hover:border-(--color-accent) hover:text-(--color-accent)`

const viewStateStore = createViewStateStore()
const libraryBrowseProfile = createProfileController()
const localPreviewMode = createLocalPreviewModeController()
const hierarchyRead = useLibraryHierarchyRead(undefined, {
  profile: libraryBrowseProfile.profile
})
const localBrowse = useLocalBrowseController(undefined, {
  localPreviewMode: localPreviewMode.mode
})
const contentsRead = useContentsRead(undefined, { profile: libraryBrowseProfile.profile })
const rootActions = useLocalRootActions()
const boundaryEvents = useBoundaryEvents()
const sourceLifecycleRead = useSourceLifecycleRead()
const integrityRead = useIntegrityRead()
const maintenanceRead = useMaintenanceRead()
const searchFilterRead = useSearchFilterRead()
const librarySearch = createLibrarySearchController({
  profile: libraryBrowseProfile.profile,
  searchFilterRead
})
const disclosureReconciler = createDisclosureReconciler({
  requestNodeChildren: (nodeId) => requestBrowserNodeChildren(nodeId)
})

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
const pendingSourceRegistration = ref<SourceRegistrationIntent>()
const sourceRevealRequest = ref<{
  readonly nodeId: BrowserTreeNodeId
  readonly sequence: number
}>()
const browseProfileMenuOpen = ref(false)
const browseProfileMenuRef = ref<HTMLElement>()
const localPreviewModeMenuOpen = ref(false)
const localPreviewModeMenuRef = ref<HTMLElement>()
const searchInputRef = ref<HTMLInputElement>()
let sourceRevealSequence = 0
let maintenanceRefreshTimer: ReturnType<typeof setTimeout> | undefined

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
  libraryBrowseProfile: libraryBrowseProfile.profile.value,
  localPreviewMode: localPreviewMode.mode.value,
  sourceReadinessByNodeId: sourceReadinessByNodeId.value,
  sourceReadStates: hierarchyRead.sourceReadStates.value,
  directoryReadStates: hierarchyRead.directoryReadStates.value,
  localBrowseEntryPointsState: localBrowse.entryPointsState.value,
  localBrowseItemStates: localBrowse.itemStates.value,
  ...(hierarchyRead.hostStatus.value === undefined
    ? {}
    : { hostStatus: hierarchyRead.hostStatus.value }),
  ...(hierarchyRead.navigationReadResult.value === undefined
    ? {}
    : { navigationReadResult: hierarchyRead.navigationReadResult.value })
}))

const selectedBrowseProfileLabel = computed(() => profileLabel(libraryBrowseProfile.profile.value))
const selectedLocalPreviewModeLabel = computed(() =>
  localPreviewModeLabel(localPreviewMode.mode.value)
)

const browserProjection = computed(() => projectState(browserState.value))

const toolbarModel = computed(() =>
  projectLibraryToolbar({
    projection: browserProjection.value,
    selectedNodeId: selectedNodeId.value,
    selectedBrowseProfileLabel: selectedBrowseProfileLabel.value,
    selectedLocalPreviewModeLabel: selectedLocalPreviewModeLabel.value,
    localPreviewMode: localPreviewMode.mode.value,
    addMusicFolderLabel: rootActions.rootChoiceButtonLabel.value,
    canAddMusicFolder: rootLifecycle.canAddMusicFolder.value
  })
)

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
  localBrowseRead: {
    refreshEntryPoints: localBrowse.refreshEntryPoints
  },
  confirmRemoveSource: () => window.confirm(removeSourceMessage),
  isSourceRootVisible: (rootId) => hasVisibleSourceRootBinding(browserProjection.value, rootId),
  onSourceRegistered: (root) => {
    pendingSourceRegistration.value = sourceRegistrationIntent(root.rootId, selectedNodeId.value)
  },
  onSourceRemoved: clearBrowserView
})

const liveTreeNodes = computed(() => browserProjection.value?.nodes ?? [])

const treeRootProps = computed(() => ({
  expandedNodeIds: expandedNodeIds.value,
  emptyLabel: emptyTreeLabel,
  labelledBy: 'library-hierarchy-title',
  nodes: liveTreeNodes.value,
  ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
  ...(sourceRevealRequest.value === undefined ? {} : { revealRequest: sourceRevealRequest.value })
}))

const selectedContentsProjection = computed(() => {
  const projection = browserProjection.value

  return projectContents({
    state: browserState.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    ...(projection === undefined ? {} : { bindingsById: projection.bindingsById }),
    contentsState: contentsRead.state.value
  })
})

const contentsProjection = computed(() => {
  if (toolbarModel.value.search.visible && librarySearch.searchActive.value) {
    return projectSearchFilterContents({
      state: searchFilterRead.state.value,
      activeQuery: librarySearch.activeQuery.value,
      profile: libraryBrowseProfile.profile.value
    })
  }

  return selectedContentsProjection.value
})

const sourceStatusContext = computed(() =>
  projectStatusContext({
    projection: browserProjection.value,
    ...(selectedNodeId.value === undefined ? {} : { selectedNodeId: selectedNodeId.value }),
    selectedTitle: selectedContentsProjection.value.title
  })
)

const selectedStatusSourceId = computed(() => {
  const context = sourceStatusContext.value
  return 'sourceId' in context ? context.sourceId : undefined
})

const sourceStatusSourceIds = computed(() => {
  const sourceIds = new Set(sourceLifecycleSourceIds.value)
  const selectedSourceId = selectedStatusSourceId.value

  if (selectedSourceId !== undefined) {
    sourceIds.add(selectedSourceId)
  }

  return sourceIds
})

const sourceStatusView = computed(() => {
  const context = sourceStatusContext.value
  const sourceId = selectedStatusSourceId.value
  const sourceLifecycle =
    sourceId === undefined
      ? undefined
      : sourceLifecycleRead.sourceLifecycleBySourceId.value.get(sourceId)
  const sourceIntegrity =
    sourceId === undefined ? undefined : integrityRead.snapshotBySourceId.value.get(sourceId)
  const sourceMaintenance =
    sourceId === undefined ? undefined : maintenanceRead.snapshotBySourceId.value.get(sourceId)
  const maintenanceRunState =
    sourceId === undefined ? undefined : maintenanceRead.runStateBySourceId.value.get(sourceId)
  const sourceReadiness =
    selectedNodeId.value === undefined
      ? undefined
      : sourceReadinessByNodeId.value.get(selectedNodeId.value)

  return projectStatusView({
    context,
    ...(sourceLifecycle === undefined ? {} : { sourceLifecycle }),
    ...(sourceIntegrity === undefined ? {} : { sourceIntegrity }),
    ...(sourceMaintenance === undefined ? {} : { sourceMaintenance }),
    ...(maintenanceRunState === undefined ? {} : { maintenanceRunState }),
    ...(sourceReadiness === undefined ? {} : { sourceReadiness }),
    canAddLocalPath: rootLifecycle.canAddMusicFolder.value,
    scanStatus: rootActions.scanStatus.value,
    removeSourceStatus: rootActions.removeSourceStatus.value,
    refreshStatus: rootLifecycle.refreshStatus.value,
    canScan: sourceId !== undefined && rootLifecycle.canScanSourceRoot(sourceId),
    canRemove: sourceId !== undefined && rootActions.canUnregisterLocalRootId(sourceId),
    canRunMaintenance: sourceId !== undefined
  })
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

watch(
  () => libraryBrowseProfile.profile.value,
  async () => {
    contentsRead.clear()
    await hierarchyRead.refreshBrowserWindows(expandedNodeIds.value)
    requestContentsForCurrentSelection({ force: true })
    saveViewState()
  }
)

watch(
  () => localPreviewMode.mode.value,
  async () => {
    await localBrowse.refreshBrowserWindows(expandedNodeIds.value, browserProjection.value)
    saveViewState()
  }
)

watch(browserProjection, (projection) => {
  applyPendingSourceRegistration(projection)

  const selectedId = selectedNodeId.value

  if (
    selectedId === undefined ||
    projection === undefined ||
    projection.bindingsById.has(selectedId)
  ) {
    return
  }

  selectedNodeId.value = undefined
  contentsRead.clear()
  saveViewState()
})

watch(
  [() => browserProjection.value, () => expandedNodeIds.value],
  ([projection, expandedIds]) => {
    disclosureReconciler.reconcile({
      projection,
      expandedNodeIds: expandedIds,
      sourceReadStates: hierarchyRead.sourceReadStates.value,
      directoryReadStates: hierarchyRead.directoryReadStates.value,
      localBrowseItemStates: localBrowse.itemStates.value
    })
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
    void localBrowse.refreshEntryPoints()
  },
  { immediate: true }
)

onMounted(() => {
  document.addEventListener('pointerdown', handleBrowseProfileOutsidePointerDown)
})

onUnmounted(() => {
  document.removeEventListener('pointerdown', handleBrowseProfileOutsidePointerDown)
  clearMaintenanceRefreshTimer()
  librarySearch.dispose()
})

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
  [sourceStatusSourceIds, () => hierarchyRead.hostStatus.value?.state],
  ([sourceIds, hostState]) => {
    if (hostState !== 'started') {
      return
    }

    void integrityRead.refresh(sourceIds)
    void maintenanceRead.refresh(sourceIds)
  },
  { immediate: true }
)

watch(
  [
    sourceStatusSourceIds,
    () => maintenanceRead.snapshotBySourceId.value,
    () => maintenanceRead.runStateBySourceId.value,
    () => hierarchyRead.hostStatus.value?.state
  ],
  () => {
    scheduleRunningMaintenanceRefresh()
  },
  { immediate: true }
)

watch(
  () => boundaryEvents.recoveryNeeded.value,
  async (needed) => {
    if (!needed) {
      return
    }

    const plan = buildGapPlan()
    await executeRefreshPlan(plan, refreshPlanExecutionDependencies())

    if (plan.acknowledgeGapAfterExecution) {
      boundaryEvents.acknowledgedGap()
    }
  }
)

watch(
  () => boundaryEvents.maintainedSnapshotInvalidationSignal.value,
  async () => {
    const invalidations = boundaryEvents.consumeMaintainedSnapshotInvalidations()
    const plan = buildInvalidationPlan({
      invalidations,
      sourceLifecycleSourceIds: sourceLifecycleSourceIds.value
    })

    await executeRefreshPlan(plan, refreshPlanExecutionDependencies())
    void refreshSelectedSourceStatus()
  }
)

watch(
  () => boundaryEvents.sourceScanSignal.value,
  () => {
    const events = boundaryEvents.consumeSourceScanEvents()

    if (events.some((event) => event.kind === 'sourceScanCompleted')) {
      disclosureReconciler.clearFailed()
    }

    const plan = buildScanPlan({
      events,
      sourceLifecycleSourceIds: sourceLifecycleSourceIds.value
    })

    void executeRefreshPlan(plan, refreshPlanExecutionDependencies())
    void refreshSelectedSourceStatus()
  }
)

function saveViewState(): void {
  viewStateStore.save({
    version: 1,
    expandedNodeIds: [...expandedNodeIds.value],
    libraryBrowseProfile: libraryBrowseProfile.profile.value,
    localPreviewMode: localPreviewMode.mode.value,
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
    libraryBrowseProfile.restoreProfile(result.viewState.libraryBrowseProfile)
    localPreviewMode.restoreMode(result.viewState.localPreviewMode)
  } catch {
    restoreState.readCompleted = true
  }
}

function toggleBrowseProfileMenu(): void {
  browseProfileMenuOpen.value = !browseProfileMenuOpen.value
}

function selectBrowseProfile(profile: ProfileKey): void {
  libraryBrowseProfile.setProfile(profile)
  browseProfileMenuOpen.value = false
}

function closeBrowseProfileMenu(): void {
  browseProfileMenuOpen.value = false
}

function toggleLocalPreviewModeMenu(): void {
  localPreviewModeMenuOpen.value = !localPreviewModeMenuOpen.value
}

function selectLocalPreviewMode(mode: LocalPreviewMode): void {
  localPreviewMode.setMode(mode)
  localPreviewModeMenuOpen.value = false
}

function closeLocalPreviewModeMenu(): void {
  localPreviewModeMenuOpen.value = false
}

function openSearch(): void {
  if (!toolbarModel.value.search.enabled) {
    return
  }

  librarySearch.openSearch()
  void nextTick(() => {
    searchInputRef.value?.focus()
  })
}

function handleSearchEscape(): void {
  librarySearch.handleEscape()
}

function handleBrowseProfileOutsidePointerDown(event: PointerEvent): void {
  const target = event.target

  if (
    browseProfileMenuOpen.value &&
    (!(target instanceof Node) || !browseProfileMenuRef.value?.contains(target))
  ) {
    browseProfileMenuOpen.value = false
  }

  if (
    localPreviewModeMenuOpen.value &&
    (!(target instanceof Node) || !localPreviewModeMenuRef.value?.contains(target))
  ) {
    localPreviewModeMenuOpen.value = false
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
}

function markUserInteraction(): void {
  restoreState.userInteracted = true
  pendingRestoreIds.value = new Set()
}

function applyPendingSourceRegistration(projection: ReturnType<typeof projectState>): void {
  const pendingRegistration = pendingSourceRegistration.value

  if (pendingRegistration === undefined) {
    return
  }

  const visibleRegistration = resolveVisibleSourceRegistration(pendingRegistration, projection)

  if (visibleRegistration === undefined) {
    return
  }

  sourceRevealRequest.value = {
    nodeId: visibleRegistration.nodeId,
    sequence: ++sourceRevealSequence
  }
  pendingSourceRegistration.value = undefined

  if (!visibleRegistration.activate) {
    return
  }

  selectedNodeId.value = visibleRegistration.nodeId
  restoreState.initialNodeApplied = true
  requestContentsForCurrentSelection()
  saveViewState()
}

function selectNode(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()
  selectedNodeId.value = nodeId
  void requestLocalBrowseNodeChildren(nodeId)
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

  disclosureReconciler.clearFailedForNode(nodeId)
  void requestBrowserNodeChildren(nodeId)
}

function prepareNodeContents(nodeId: BrowserTreeNodeId): void {
  contentsRead.preloadForBinding(browserProjection.value?.bindingsById.get(nodeId))
}

function cancelPrepareNodeContents(nodeId: BrowserTreeNodeId): void {
  contentsRead.cancelPreloadForBinding(browserProjection.value?.bindingsById.get(nodeId))
}

function refreshPlanExecutionDependencies(): RefreshPlanDeps {
  return {
    hierarchyRead,
    refreshLocalBrowseEntryPoints: () => localBrowse.refreshEntryPoints(),
    sourceLifecycleRead,
    expandedNodeIds: expandedNodeIds.value,
    clearContentsWarmSnapshots: () => contentsRead.clearWarmSnapshots(),
    refreshContentsForCurrentSelection,
    refreshActiveSearchFilter: () => searchFilterRead.invalidationSignal()
  }
}

function refreshContentsForCurrentSelection(): Promise<boolean> {
  const selectedId = selectedNodeId.value
  const projection = browserProjection.value

  if (selectedId === undefined || projection === undefined) {
    contentsRead.clear()
    return Promise.resolve(false)
  }

  return contentsRead.readForBinding(projection.bindingsById.get(selectedId), { force: true })
}

async function handleStatusAction(action: StatusAction): Promise<void> {
  if (!action.enabled) {
    return
  }

  switch (action.kind) {
    case 'addLocalPath':
      await rootLifecycle.addLocalPath(action.resolvedPath).then(async (registered) => {
        if (registered) {
          await localBrowse.refreshBrowserWindows(expandedNodeIds.value, browserProjection.value)
        }
      })
      break
    case 'showSource':
      showAdmittedSource(action.sourceId)
      break
    case 'scanSource':
      await rootLifecycle.scanRoot(action.sourceId)
      break
    case 'runMaintenance':
      await maintenanceRead.run(action.sourceId)
      await refreshSourceStatus(action.sourceId)
      await refreshContentsForCurrentSelection()
      await searchFilterRead.invalidationSignal()
      break
    case 'removeSource':
      await rootLifecycle.removeSource(action.sourceId)
      break
    case 'refreshStatus':
      await refreshSourceStatus(action.sourceId)
      break
  }
}

function showAdmittedSource(sourceId: string): void {
  markUserInteraction()
  const projection = browserProjection.value
  const nodeId = sourceNodeIdForSourceId(projection, sourceId)

  if (nodeId === undefined) {
    pendingSourceRegistration.value = sourceRegistrationIntent(sourceId, undefined)
    return
  }

  sourceRevealRequest.value = {
    nodeId,
    sequence: ++sourceRevealSequence
  }
  selectedNodeId.value = nodeId
  restoreState.initialNodeApplied = true
  requestContentsForCurrentSelection()
  saveViewState()
}

function sourceNodeIdForSourceId(
  projection: ReturnType<typeof projectState>,
  sourceId: string
): BrowserTreeNodeId | undefined {
  if (projection?.kind !== 'tree') {
    return undefined
  }

  for (const [nodeId, binding] of projection.bindingsById) {
    if (
      binding.kind === 'source' &&
      binding.target.entryPoint.kind === 'source' &&
      binding.target.entryPoint.sourceId === sourceId
    ) {
      return nodeId
    }
  }

  return undefined
}

async function refreshSelectedSourceStatus(): Promise<boolean> {
  const sourceId = selectedStatusSourceId.value
  if (sourceId === undefined) {
    return false
  }

  return refreshSourceStatus(sourceId)
}

async function refreshSourceStatus(sourceId: string): Promise<boolean> {
  const [lifecycle, integrity, maintenance] = await Promise.all([
    sourceLifecycleRead.readSourceLifecycle(sourceId),
    integrityRead.read(sourceId),
    maintenanceRead.read(sourceId)
  ])

  return lifecycle && integrity && maintenance
}

function scheduleRunningMaintenanceRefresh(): void {
  clearMaintenanceRefreshTimer()

  if (hierarchyRead.hostStatus.value?.state !== 'started') {
    return
  }

  const runningSourceIds = [...sourceStatusSourceIds.value].filter(
    (sourceId) =>
      maintenanceRead.runStateBySourceId.value.get(sourceId) === 'running' ||
      maintenanceRead.snapshotBySourceId.value.get(sourceId)?.status === 'running'
  )

  if (runningSourceIds.length === 0) {
    return
  }

  maintenanceRefreshTimer = setTimeout(() => {
    maintenanceRefreshTimer = undefined
    void Promise.all(runningSourceIds.map((sourceId) => refreshSourceStatus(sourceId))).finally(
      scheduleRunningMaintenanceRefresh
    )
  }, maintenanceRunningRefreshMs)
}

function clearMaintenanceRefreshTimer(): void {
  if (maintenanceRefreshTimer === undefined) {
    return
  }

  clearTimeout(maintenanceRefreshTimer)
  maintenanceRefreshTimer = undefined
}

function requestBrowserNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
  const projection = browserProjection.value
  const binding = projection?.bindingsById.get(nodeId)

  if (
    binding?.kind === 'localBrowseEntryPoint' ||
    binding?.kind === 'localBrowseItem' ||
    binding?.kind === 'localBrowseMore'
  ) {
    return localBrowse.requestNodeChildren(nodeId, projection)
  }

  return hierarchyRead.requestNodeChildren(nodeId)
}

function requestLocalBrowseNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
  const projection = browserProjection.value
  const binding = projection?.bindingsById.get(nodeId)

  if (binding?.kind !== 'localBrowseEntryPoint' && binding?.kind !== 'localBrowseItem') {
    return Promise.resolve(false)
  }

  return localBrowse.requestNodeChildren(nodeId, projection)
}

function clearBrowserView(): void {
  selectedNodeId.value = undefined
  contentsRead.clear()
  expandedNodeIds.value = new Set()
  pendingRestoreIds.value = new Set()
  pendingSourceRegistration.value = undefined
  sourceRevealRequest.value = undefined

  restoreState.userInteracted = false
  restoreState.initialNodeApplied = false

  viewStateStore.save({
    version: 1,
    libraryBrowseProfile: libraryBrowseProfile.profile.value,
    localPreviewMode: localPreviewMode.mode.value,
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
    disclosureReconciler.clearFailedForNode(action.nodeId)
    void hierarchyRead.requestNodeChildren(action.nodeId)
  } else if (action.kind === 'loadLocalBrowseChildren') {
    expandedNodeIds.value = new Set([...expandedNodeIds.value, action.nodeId])
    saveViewState()
    disclosureReconciler.clearFailedForNode(action.nodeId)
    void localBrowse.requestNodeChildren(action.nodeId, browserProjection.value)
  } else if (action.kind === 'loadLocalBrowseMore') {
    void localBrowse.requestNodeMore(action.nodeId, browserProjection.value)
  } else if (action.kind === 'requestLocalBrowseAdmission') {
    void rootLifecycle.addLocalPath(action.resolvedPath).then(async (registered) => {
      if (registered) {
        await localBrowse.refreshBrowserWindows(expandedNodeIds.value, browserProjection.value)
      }
    })
  } else if (action.kind === 'chooseMusicFolder') {
    void rootLifecycle.addMusicFolder()
  } else if (action.kind === 'loadContentsPage') {
    void contentsRead.readForBinding(browserProjection.value?.bindingsById.get(action.nodeId), {
      cursor: action.cursor
    })
  } else if (action.kind === 'loadSearchPage') {
    void searchFilterRead.loadNext()
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
    class="flex h-[80svh] min-h-0 flex-col overflow-hidden border border-(--color-border) bg-(--color-surface)"
    aria-labelledby="library-hierarchy-title"
  >
    <header class="flex shrink-0 items-center justify-between gap-4">
      <h2 id="library-hierarchy-title" class="text-xl font-bold leading-none text-(--color-text)">
        Library
      </h2>

      <div class="flex flex-wrap items-center justify-end gap-2">
        <button
          v-if="toolbarModel.search.visible && !librarySearch.searchOpen.value"
          type="button"
          :class="iconButtonClass"
          :aria-label="toolbarModel.search.label"
          :title="toolbarModel.search.title"
          :disabled="!toolbarModel.search.enabled"
          @click="openSearch"
        >
          <Icon role="action.search" size="md" />
        </button>

        <div v-else-if="toolbarModel.search.visible" class="inline-flex items-center gap-1">
          <input
            ref="searchInputRef"
            v-model="librarySearch.searchText.value"
            type="search"
            class="h-9 w-44 rounded-sm border border-(--color-border) bg-(--color-background) px-3 py-2 text-sm font-semibold text-(--color-text) outline-none transition placeholder:text-(--color-text-muted) hover:border-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
            :aria-label="toolbarModel.search.label"
            :placeholder="toolbarModel.search.placeholder"
            @keydown.escape.stop.prevent="handleSearchEscape"
          />
          <button
            v-if="librarySearch.searchText.value.length > 0"
            type="button"
            :class="iconButtonClass"
            aria-label="Clear search"
            title="Clear search"
            @click="librarySearch.clearSearch()"
          >
            <Icon role="action.clear" size="md" />
          </button>
        </div>

        <div
          v-if="toolbarModel.browseProfile.visible"
          ref="browseProfileMenuRef"
          class="relative inline-flex"
        >
          <button
            type="button"
            :class="iconButtonClass"
            :aria-label="toolbarModel.browseProfile.label"
            :aria-expanded="browseProfileMenuOpen"
            aria-haspopup="listbox"
            :title="toolbarModel.browseProfile.title"
            :disabled="!toolbarModel.browseProfile.enabled"
            @click="toggleBrowseProfileMenu"
            @keydown.escape.stop.prevent="closeBrowseProfileMenu"
          >
            <Icon role="action.browseView" size="md" />
          </button>

          <div
            v-if="browseProfileMenuOpen"
            class="absolute right-0 top-full z-20 mt-1 min-w-40 border border-(--color-border) bg-(--color-background) py-1 shadow-lg"
            role="listbox"
            :aria-label="toolbarModel.browseProfile.label"
            tabindex="-1"
            @keydown.escape.stop.prevent="closeBrowseProfileMenu"
          >
            <button
              v-for="option in profileOptions"
              :key="option.key"
              type="button"
              class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-(--color-text) hover:bg-(--color-surface) focus-visible:bg-(--color-surface) focus-visible:outline-none"
              :class="
                libraryBrowseProfile.profile.value === option.key ? 'font-bold' : 'font-normal'
              "
              role="option"
              :aria-selected="libraryBrowseProfile.profile.value === option.key"
              @click="selectBrowseProfile(option.key)"
            >
              <span>{{ option.label }}</span>
            </button>
          </div>
        </div>

        <div
          v-if="toolbarModel.localPreviewMode.visible"
          ref="localPreviewModeMenuRef"
          class="relative inline-flex"
        >
          <button
            type="button"
            :class="iconButtonClass"
            :aria-label="toolbarModel.localPreviewMode.label"
            :aria-expanded="localPreviewModeMenuOpen"
            aria-haspopup="listbox"
            :title="toolbarModel.localPreviewMode.title"
            :disabled="!toolbarModel.localPreviewMode.enabled"
            @click="toggleLocalPreviewModeMenu"
            @keydown.escape.stop.prevent="closeLocalPreviewModeMenu"
          >
            <Icon role="action.browseView" size="md" />
          </button>

          <div
            v-if="localPreviewModeMenuOpen"
            class="absolute right-0 top-full z-20 mt-1 min-w-48 border border-(--color-border) bg-(--color-background) py-1 shadow-lg"
            role="listbox"
            :aria-label="toolbarModel.localPreviewMode.label"
            tabindex="-1"
            @keydown.escape.stop.prevent="closeLocalPreviewModeMenu"
          >
            <button
              v-for="option in localPreviewModeOptions"
              :key="option.key"
              type="button"
              class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-(--color-text) hover:bg-(--color-surface) focus-visible:bg-(--color-surface) focus-visible:outline-none"
              :class="localPreviewMode.mode.value === option.key ? 'font-bold' : 'font-normal'"
              role="option"
              :aria-selected="localPreviewMode.mode.value === option.key"
              @click="selectLocalPreviewMode(option.key)"
            >
              <span>{{ option.label }}</span>
            </button>
          </div>
        </div>

        <button
          v-if="toolbarModel.addMusicFolder.visible"
          type="button"
          :class="primaryButtonClass"
          :disabled="!toolbarModel.addMusicFolder.enabled"
          :title="toolbarModel.addMusicFolder.reason"
          @click="rootLifecycle.addMusicFolder"
        >
          {{ toolbarModel.addMusicFolder.label }}
        </button>
      </div>
    </header>

    <div class="grid min-h-0 flex-1 grid-cols-[minmax(18rem,24rem)_minmax(0,1fr)] overflow-hidden">
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
        :status-view="sourceStatusView"
        :activate-row-action="activateContentRowAction"
        :activate-status-action="handleStatusAction"
      />
    </div>
  </section>
</template>
