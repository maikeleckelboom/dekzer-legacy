<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import { Icon } from '../icons'
import {
  libraryBrowseProfileLabel,
  libraryBrowseProfileOptions,
  type LibraryBrowseProfile
} from './libraryBrowseProfile/types'
import { createLibraryBrowseProfileController } from './libraryBrowseProfile/controller'
import { useLibraryHierarchyRead } from './boundary/hierarchyRead'
import { useContentsRead } from './boundary/contentsRead'
import { useLocalRootActions } from './boundary/localRootActions'
import {
  useLocalBrowseController,
  type LocalBrowseBranchWarmupTrace
} from './localBrowse/controller'
import { addSourceSectionNodeId, projectAddSourceState } from './localBrowse/projection'
import {
  createAddSourceViewController,
  addSourceViewLabel,
  addSourceViewOptions,
  type AddSourceView
} from './addSource/view'
import {
  createViewModeController,
  viewModeLabel,
  viewModeOptions,
  type ViewMode
} from './viewMode/model'
import { useBoundaryEvents } from './boundary/boundaryEvents'
import {
  sourceLifecycleIdsForBrowserContext,
  useSourceLifecycleRead
} from './boundary/sourceLifecycleRead'
import { useRead as useIntegrityRead } from './sourceIntegrity/read'
import { useRead as useMaintenanceRead } from './sourceMaintenance/read'
import { useRead as useActivityRead } from './sourceActivity/read'
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
import {
  createLibrarySearchController,
  librarySearchScopeForBinding
} from './runtime/librarySearch'
import { useSearchFilterRead } from './runtime/searchFilterState'
import { useRootLifecycle } from './runtime/rootLifecycle'
import {
  hasVisibleSourceRootBinding,
  resolveVisibleSourceRegistration,
  sourceRegistrationIntent,
  type SourceRegistrationIntent
} from './runtime/sourceActions'
import {
  projectSourceAdmissionHandoff,
  sourceAdmissionHandoffFromRoot,
  type SourceAdmissionHandoffAction,
  type SourceAdmissionHandoffState
} from './runtime/sourceAdmissionHandoff'
import { projectLibraryToolbar } from './runtime/toolbarProjection'
import { projectSourceActivityBySourceId } from './runtime/sourceActivity'
import { projectSourceReadinessByNodeId } from './runtime/sourceReadiness'
import { invalidateSourceStatus as invalidateSourceStatusReaders } from './runtime/sourceStatusInvalidation'
import { projectStatusContext } from './sourceStatus/context'
import {
  projectStatusView,
  sourceStatusDiagnosticTrace,
  type ActiveSourceOperation,
  type StatusAction
} from './sourceStatus/projection'
import {
  clearSelection,
  isValidSelection,
  rowSubject,
  sourceSubject,
  type PrimarySelection
} from './selection/model'
import type { BrowserState, RowBinding } from './state'
import { createViewStateStore } from './runtime/viewState'
import { projectState } from './tree/projection'
import WorkstationShell from './workstation/shell.vue'
import type { BrowserTreeNodeId } from './tree/types'
import type { LibraryPanelSurface } from '../../shared/library/viewState/persistence'

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
const toolbarControlClass =
  'border border-(--color-text-muted) bg-(--color-background) text-(--color-text) shadow-[inset_0_0_0_1px_var(--color-border)] hover:border-(--color-accent) hover:bg-(--color-surface) hover:text-(--color-accent) focus-visible:border-(--color-accent)'
const iconButtonClass = `${buttonBaseClass} h-9 w-9 min-w-0 ${toolbarControlClass} p-0`

const viewStateStore = createViewStateStore()
const libraryBrowseProfile = createLibraryBrowseProfileController()
const addSourceView = createAddSourceViewController()
const viewMode = createViewModeController()
const activeSurface = ref<LibraryPanelSurface>('libraryBrowse')
const selectedLibraryNodeId = ref<BrowserTreeNodeId>()
const selectedAddSourceNodeId = ref<BrowserTreeNodeId>()
const expandedLibraryNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const expandedAddSourceNodeIds = ref<ReadonlySet<BrowserTreeNodeId>>(
  new Set([addSourceSectionNodeId])
)
const pendingLibraryRestoreIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const pendingAddSourceRestoreIds = ref<ReadonlySet<BrowserTreeNodeId>>(new Set())
const pendingSourceRegistration = ref<SourceRegistrationIntent>()
const sourceAdmissionHandoff = ref<SourceAdmissionHandoffState>()
const primarySelection = ref<PrimarySelection>(clearSelection())
const sourceRevealRequest = ref<{
  readonly nodeId: BrowserTreeNodeId
  readonly sequence: number
}>()
const hierarchyRead = useLibraryHierarchyRead(undefined, {
  profile: libraryBrowseProfile.profile
})
const localBrowse = useLocalBrowseController(undefined, {
  addSourceView: addSourceView.view,
  warmup: {
    shouldContinue: (nodeId) =>
      activeSurface.value === 'addSource' &&
      (expandedAddSourceNodeIds.value.has(nodeId) || selectedAddSourceNodeId.value === nodeId),
    trace: traceLocalBrowseBranchWarmup
  }
})
const contentsRead = useContentsRead(undefined, { profile: libraryBrowseProfile.profile })
const rootActions = useLocalRootActions()
const boundaryEvents = useBoundaryEvents()
const sourceLifecycleRead = useSourceLifecycleRead()
const integrityRead = useIntegrityRead()
const maintenanceRead = useMaintenanceRead()
const activityRead = useActivityRead()
const searchFilterRead = useSearchFilterRead()
const disclosureReconciler = createDisclosureReconciler({
  requestNodeChildren: (nodeId) => requestLibraryNodeChildren(nodeId)
})
const addSourceDisclosureReconciler = createDisclosureReconciler({
  requestNodeChildren: (nodeId) => requestAddSourceNodeChildren(nodeId)
})

const scanProgressForActiveScan = computed(() => {
  const rootId = rootActions.activeScanRootId.value
  if (rootId === undefined) {
    return undefined
  }
  return boundaryEvents.scanProgress.value.get(rootId)
})

const libraryBrowseProfileMenuOpen = ref(false)
const libraryBrowseProfileMenuRef = ref<HTMLElement>()
const viewModeMenuOpen = ref(false)
const viewModeMenuRef = ref<HTMLElement>()
const addSourceViewMenuOpen = ref(false)
const addSourceViewMenuRef = ref<HTMLElement>()
const searchOpenButtonRef = ref<HTMLButtonElement>()
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
    ...(rootActions.activeScanRootId.value === undefined
      ? {}
      : { currentScanRootId: rootActions.activeScanRootId.value }),
    currentScanStatus: rootActions.scanStatus.value
  })
)

const sourceActivityBySourceId = computed(() =>
  projectSourceActivityBySourceId({
    activityBySourceId: activityRead.snapshotBySourceId.value,
    scanProgressByRootId: boundaryEvents.scanProgress.value,
    maintenanceRunStateBySourceId: maintenanceRead.runStateBySourceId.value
  })
)

const browserState = computed<BrowserState>(() => ({
  libraryBrowseProfile: libraryBrowseProfile.profile.value,
  addSourceView: addSourceView.view.value,
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

const selectedLibraryBrowseProfileLabel = computed(() =>
  libraryBrowseProfileLabel(libraryBrowseProfile.profile.value)
)
const selectedViewModeLabel = computed(() => viewModeLabel(viewMode.view.value))
const selectedAddSourceViewLabel = computed(() => addSourceViewLabel(addSourceView.view.value))
const activeSurfaceTitle = computed(() =>
  activeSurface.value === 'addSource' ? 'Add Source' : 'Library Browse'
)
const contentsView = computed(() =>
  activeSurface.value === 'libraryBrowse' ? viewMode.view.value : 'list'
)

const libraryBrowseProjection = computed(() => projectState(browserState.value))
const selectedLibrarySearchScope = computed(() =>
  librarySearchScopeForBinding(
    selectedLibraryNodeId.value === undefined
      ? undefined
      : libraryBrowseProjection.value?.bindingsById.get(selectedLibraryNodeId.value)
  )
)
const librarySearch = createLibrarySearchController({
  profile: libraryBrowseProfile.profile,
  scope: selectedLibrarySearchScope,
  searchFilterRead
})
const addSourceProjection = computed(() =>
  projectAddSourceState({
    addSourceView: addSourceView.view.value,
    entryPointsState: localBrowse.entryPointsState.value,
    itemStates: localBrowse.itemStates.value
  })
)
const activeProjection = computed(() =>
  activeSurface.value === 'addSource' ? addSourceProjection.value : libraryBrowseProjection.value
)
const activeSelectedNodeId = computed(() =>
  activeSurface.value === 'addSource' ? selectedAddSourceNodeId.value : selectedLibraryNodeId.value
)
const activeExpandedNodeIds = computed(() =>
  activeSurface.value === 'addSource'
    ? expandedAddSourceNodeIds.value
    : expandedLibraryNodeIds.value
)
const hasAdmittedLibraryRowsVisible = computed(() =>
  hasAdmittedLibraryRows(libraryBrowseProjection.value)
)

const toolbarModel = computed(() =>
  projectLibraryToolbar({
    activeSurface: activeSurface.value,
    projection: activeProjection.value,
    selectedNodeId: activeSelectedNodeId.value,
    selectedViewModeLabel: selectedViewModeLabel.value,
    selectedLibraryBrowseProfileLabel: selectedLibraryBrowseProfileLabel.value,
    selectedAddSourceViewLabel: selectedAddSourceViewLabel.value,
    addSourceView: addSourceView.view.value,
    addMusicFolderLabel: rootActions.rootChoiceButtonLabel.value,
    canAddMusicFolder: rootLifecycle.canAddMusicFolder.value
  })
)

const sourceLifecycleSourceIds = computed(() =>
  sourceLifecycleIdsForBrowserContext({
    projection: libraryBrowseProjection.value,
    ...(selectedLibraryNodeId.value === undefined
      ? {}
      : { selectedNodeId: selectedLibraryNodeId.value }),
    expandedNodeIds: expandedLibraryNodeIds.value
  })
)

const rootLifecycle = useRootLifecycle({
  rootActions,
  hierarchyRead: {
    refresh: hierarchyRead.refresh
  },
  localBrowseRead: {
    clearItemWindows: localBrowse.clearItemWindows,
    refreshEntryPoints: localBrowse.refreshEntryPoints
  },
  confirmRemoveSource: () => window.confirm(removeSourceMessage),
  isSourceRootVisible: (rootId) =>
    hasVisibleSourceRootBinding(libraryBrowseProjection.value, rootId),
  invalidateSourceStatus,
  onSourceRegistered: (root) => {
    pendingSourceRegistration.value = sourceRegistrationIntent(root.rootId)
    sourceAdmissionHandoff.value = sourceAdmissionHandoffFromRoot(root)
  },
  onSourceRemoved: clearBrowserView
})

const liveTreeNodes = computed(() => activeProjection.value?.nodes ?? [])

const treeRootProps = computed(() => ({
  expandedNodeIds: activeExpandedNodeIds.value,
  emptyLabel:
    activeSurface.value === 'addSource'
      ? 'Choose a music folder to add as a source.'
      : emptyTreeLabel,
  labelledBy: 'library-hierarchy-title',
  nodes: liveTreeNodes.value,
  ...(activeSelectedNodeId.value === undefined
    ? {}
    : { selectedNodeId: activeSelectedNodeId.value }),
  ...(activeSurface.value !== 'libraryBrowse' || sourceRevealRequest.value === undefined
    ? {}
    : { revealRequest: sourceRevealRequest.value })
}))

const selectedLibraryContentsProjection = computed(() => {
  const projection = libraryBrowseProjection.value

  return projectContents({
    surface: 'libraryBrowse',
    state: browserState.value,
    ...(selectedLibraryNodeId.value === undefined
      ? {}
      : { selectedNodeId: selectedLibraryNodeId.value }),
    ...(projection === undefined ? {} : { bindingsById: projection.bindingsById }),
    contentsState: contentsRead.state.value,
    sourceIntegrityBySourceId: integrityRead.snapshotBySourceId.value,
    sourceMaintenanceBySourceId: maintenanceRead.snapshotBySourceId.value,
    sourceActivityBySourceId: sourceActivityBySourceId.value
  })
})

const selectedAddSourceContentsProjection = computed(() => {
  const projection = addSourceProjection.value

  return projectContents({
    surface: 'addSource',
    state: browserState.value,
    ...(selectedAddSourceNodeId.value === undefined
      ? {}
      : { selectedNodeId: selectedAddSourceNodeId.value }),
    ...(projection === undefined ? {} : { bindingsById: projection.bindingsById })
  })
})

const contentsProjection = computed(() => {
  if (
    activeSurface.value === 'libraryBrowse' &&
    toolbarModel.value.search.visible &&
    librarySearch.searchActive.value
  ) {
    return projectSearchFilterContents({
      state: searchFilterRead.state.value,
      activeQuery: librarySearch.activeQuery.value,
      profile: libraryBrowseProfile.profile.value
    })
  }

  return activeSurface.value === 'addSource'
    ? selectedAddSourceContentsProjection.value
    : selectedLibraryContentsProjection.value
})

const selectedContentRowId = computed(() =>
  primarySelection.value.kind === 'row' ? primarySelection.value.rowId : undefined
)

const sourceStatusContext = computed(() =>
  projectStatusContext({
    projection: activeProjection.value,
    ...(activeSelectedNodeId.value === undefined
      ? {}
      : { selectedNodeId: activeSelectedNodeId.value }),
    selectedTitle:
      activeSurface.value === 'addSource'
        ? selectedAddSourceContentsProjection.value.title
        : selectedLibraryContentsProjection.value.title
  })
)

const selectedStatusSourceId = computed(() => {
  const context = sourceStatusContext.value
  return 'sourceId' in context ? context.sourceId : undefined
})

const selectedStatusSourcePath = computed(() => {
  const context = sourceStatusContext.value
  if (context.kind === 'localBrowse') {
    return context.detail
  }

  const sourceId = selectedStatusSourceId.value
  const rootsState = rootActions.localRootsReadState.value
  if (sourceId === undefined || rootsState.kind !== 'ready') {
    return undefined
  }

  return rootsState.roots.find((root) => root.rootId === sourceId)?.admittedRootPath
})

const sourceStatusSourceIds = computed(() => {
  const sourceIds = new Set(sourceLifecycleSourceIds.value)
  const selectedSourceId = selectedStatusSourceId.value

  if (selectedSourceId !== undefined) {
    sourceIds.add(selectedSourceId)
  }

  return sourceIds
})

const activeSourceOperation = computed<ActiveSourceOperation | undefined>(() => {
  if (rootActions.scanStatus.value === 'scanning') {
    const rootId = rootActions.activeScanRootId.value

    return rootId === undefined
      ? { kind: 'scan', scope: 'global' }
      : { kind: 'scan', scope: 'source', sourceId: rootId }
  }

  if (rootActions.removeSourceStatus.value === 'removing') {
    return { kind: 'remove', scope: 'global' }
  }

  return undefined
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
  const sourceActivity =
    sourceId === undefined ? undefined : sourceActivityBySourceId.value.get(sourceId)
  const maintenanceRunState =
    sourceId === undefined ? undefined : maintenanceRead.runStateBySourceId.value.get(sourceId)
  const sourceReadiness =
    activeSurface.value !== 'libraryBrowse' || selectedLibraryNodeId.value === undefined
      ? undefined
      : sourceReadinessByNodeId.value.get(selectedLibraryNodeId.value)

  return projectStatusView({
    context,
    ...(activeSurface.value === 'libraryBrowse'
      ? { libraryBrowseProfile: libraryBrowseProfile.profile.value }
      : {}),
    ...(sourceLifecycle === undefined ? {} : { sourceLifecycle }),
    ...(sourceIntegrity === undefined ? {} : { sourceIntegrity }),
    ...(sourceMaintenance === undefined ? {} : { sourceMaintenance }),
    ...(sourceActivity === undefined ? {} : { sourceActivity }),
    ...(maintenanceRunState === undefined ? {} : { maintenanceRunState }),
    ...(sourceReadiness === undefined ? {} : { sourceReadiness }),
    ...(activeSourceOperation.value === undefined
      ? {}
      : { activeSourceOperation: activeSourceOperation.value }),
    ...(selectedStatusSourcePath.value === undefined
      ? {}
      : { sourcePath: selectedStatusSourcePath.value }),
    canAddLocalPath: rootLifecycle.canAddMusicFolder.value,
    scanStatus: rootActions.scanStatusForRoot(sourceId),
    removeSourceStatus: rootActions.removeSourceStatus.value,
    refreshStatus: rootLifecycle.refreshStatus.value,
    canScan: sourceId !== undefined && rootLifecycle.canScanSourceRoot(sourceId),
    canRemove: sourceId !== undefined && rootActions.canUnregisterLocalRootId(sourceId),
    canRunMaintenance: sourceId !== undefined
  })
})

const inspectorStatusView = computed(() =>
  primarySelection.value.kind === 'source' ? sourceStatusView.value : undefined
)

watch(
  [
    sourceStatusContext,
    selectedStatusSourcePath,
    () => integrityRead.snapshotBySourceId.value,
    () => maintenanceRead.snapshotBySourceId.value,
    () => activityRead.snapshotBySourceId.value,
    () => boundaryEvents.scanProgress.value,
    () => maintenanceRead.runStateBySourceId.value
  ],
  () => {
    if (!import.meta.env.DEV) {
      return
    }

    const context = sourceStatusContext.value
    const sourceId = selectedStatusSourceId.value
    const sourceIntegrity =
      sourceId === undefined ? undefined : integrityRead.snapshotBySourceId.value.get(sourceId)
    const sourceMaintenance =
      sourceId === undefined ? undefined : maintenanceRead.snapshotBySourceId.value.get(sourceId)
    const sourceActivity =
      sourceId === undefined ? undefined : sourceActivityBySourceId.value.get(sourceId)
    const maintenanceRunState =
      sourceId === undefined ? undefined : maintenanceRead.runStateBySourceId.value.get(sourceId)

    console.debug(
      '[dekzer:library:source-status]',
      sourceStatusDiagnosticTrace({
        context,
        ...(sourceIntegrity === undefined ? {} : { sourceIntegrity }),
        ...(sourceMaintenance === undefined ? {} : { sourceMaintenance }),
        ...(sourceActivity === undefined ? {} : { sourceActivity }),
        ...(maintenanceRunState === undefined ? {} : { maintenanceRunState }),
        ...(activeSourceOperation.value === undefined
          ? {}
          : { activeSourceOperation: activeSourceOperation.value }),
        ...(selectedStatusSourcePath.value === undefined
          ? {}
          : { sourcePath: selectedStatusSourcePath.value }),
        canAddLocalPath: rootLifecycle.canAddMusicFolder.value,
        scanStatus: rootActions.scanStatusForRoot(sourceId),
        removeSourceStatus: rootActions.removeSourceStatus.value,
        refreshStatus: rootLifecycle.refreshStatus.value,
        canScan: sourceId !== undefined && rootLifecycle.canScanSourceRoot(sourceId),
        canRemove: sourceId !== undefined && rootActions.canUnregisterLocalRootId(sourceId),
        canRunMaintenance: sourceId !== undefined
      })
    )
  }
)

watch(sourceStatusContext, (context) => {
  if (primarySelection.value.kind !== 'source') {
    return
  }

  primarySelection.value = sourceSubject(context)
})

watch(
  [contentsProjection, () => contentSelectionRowsAreCurrent()],
  ([projection, rowsAreCurrent]) => {
    if (primarySelection.value.kind !== 'row') {
      return
    }

    if (!rowsAreCurrent || !isValidSelection(primarySelection.value, projection)) {
      primarySelection.value = clearSelection()
    }
  },
  { immediate: true }
)

function traceLocalBrowseBranchWarmup(trace: LocalBrowseBranchWarmupTrace): void {
  if (!import.meta.env.DEV) {
    return
  }

  console.debug('[dekzer:library:local-browse-warmup]', trace)
}

const sourceAdmissionHandoffView = computed(() =>
  projectSourceAdmissionHandoff({
    ...(sourceAdmissionHandoff.value === undefined
      ? {}
      : { handoff: sourceAdmissionHandoff.value }),
    ...(libraryBrowseProjection.value === undefined
      ? {}
      : { projection: libraryBrowseProjection.value }),
    sourceReadinessByNodeId: sourceReadinessByNodeId.value,
    sourceActivityBySourceId: sourceActivityBySourceId.value,
    canScanSource:
      sourceAdmissionHandoff.value === undefined
        ? false
        : rootLifecycle.canScanSourceRoot(sourceAdmissionHandoff.value.sourceId)
  })
)

watch([scanProgressForActiveScan, () => rootActions.activeScanRunId.value], ([progress]) => {
  if (progress !== undefined) {
    rootActions.applyScanProgress(progress)
  }
})

watch(
  [
    selectedLibraryNodeId,
    () => libraryBrowseProjection.value,
    () => hierarchyRead.hostStatus.value?.state
  ],
  () => {
    requestContentsForCurrentSelection()
  },
  { immediate: true }
)

watch(
  () => libraryBrowseProfile.profile.value,
  async () => {
    requestContentsForCurrentSelection({ force: true })
    await hierarchyRead.refreshBrowserWindows(expandedLibraryNodeIds.value)
    requestContentsForCurrentSelection()
    saveViewState()
  }
)

watch(
  () => addSourceView.view.value,
  async () => {
    await localBrowse.refreshBrowserWindows(
      expandedAddSourceNodeIds.value,
      addSourceProjection.value
    )
    saveViewState()
  }
)

watch(libraryBrowseProjection, (projection) => {
  applyPendingSourceRegistration(projection)
  applyEmptyLibraryDefaultSurface(projection)

  const selectedId = selectedLibraryNodeId.value

  if (
    selectedId === undefined ||
    projection === undefined ||
    projection.bindingsById.has(selectedId)
  ) {
    return
  }

  selectedLibraryNodeId.value = undefined
  contentsRead.clear()
  saveViewState()
})

watch(addSourceProjection, (projection) => {
  const selectedId = selectedAddSourceNodeId.value

  if (
    selectedId === undefined ||
    projection === undefined ||
    projection.bindingsById.has(selectedId)
  ) {
    return
  }

  selectedAddSourceNodeId.value = undefined
  saveViewState()
})

watch(
  [() => libraryBrowseProjection.value, () => expandedLibraryNodeIds.value],
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

watch(
  [() => addSourceProjection.value, () => expandedAddSourceNodeIds.value],
  ([projection, expandedIds]) => {
    addSourceDisclosureReconciler.reconcile({
      projection,
      expandedNodeIds: expandedIds,
      sourceReadStates: hierarchyRead.sourceReadStates.value,
      directoryReadStates: hierarchyRead.directoryReadStates.value,
      localBrowseItemStates: localBrowse.itemStates.value
    })
  },
  { immediate: true }
)

watch([() => libraryBrowseProjection.value?.nodes, () => addSourceProjection.value?.nodes], () => {
  if (!restoreState.readStarted) {
    restoreState.readStarted = true
    void restoreViewState()
    return
  }

  if (
    restoreState.readCompleted &&
    (pendingLibraryRestoreIds.value.size > 0 || pendingAddSourceRestoreIds.value.size > 0) &&
    !restoreState.userInteracted
  ) {
    applyPendingRestoreIds('libraryBrowse')
    applyPendingRestoreIds('addSource')
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
  document.addEventListener('pointerdown', handleLibraryBrowseProfileOutsidePointerDown)
})

onUnmounted(() => {
  document.removeEventListener('pointerdown', handleLibraryBrowseProfileOutsidePointerDown)
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
    void activityRead.refresh(sourceIds)
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

    const currentContentsSourceId = selectedContentsSourceId()
    const plan = buildScanPlan({
      events,
      sourceLifecycleSourceIds: sourceLifecycleSourceIds.value,
      ...(currentContentsSourceId === undefined ? {} : { currentContentsSourceId })
    })

    void executeRefreshPlan(plan, refreshPlanExecutionDependencies())
    void refreshSelectedSourceStatus()
  }
)

function saveViewState(): void {
  viewStateStore.save({
    version: 3,
    activeSurface: activeSurface.value,
    expandedLibraryNodeIds: [...expandedLibraryNodeIds.value],
    expandedAddSourceNodeIds: [...expandedAddSourceNodeIds.value],
    libraryBrowseProfile: libraryBrowseProfile.profile.value,
    addSourceView: addSourceView.view.value,
    ...(selectedLibraryNodeId.value === undefined
      ? {}
      : { selectedLibraryNodeId: selectedLibraryNodeId.value }),
    ...(selectedAddSourceNodeId.value === undefined
      ? {}
      : { selectedAddSourceNodeId: selectedAddSourceNodeId.value })
  })
}

async function restoreViewState(): Promise<void> {
  const libraryProjection = libraryBrowseProjection.value
  const addSource = addSourceProjection.value

  if (libraryProjection?.kind !== 'tree' && addSource?.kind !== 'tree') {
    restoreState.readStarted = false
    return
  }

  try {
    const result = await viewStateStore.load()

    restoreState.readCompleted = true

    if (result.state !== 'ready' || restoreState.userInteracted) {
      return
    }

    activeSurface.value = result.viewState.activeSurface
    applyRestoredSelection(
      'libraryBrowse',
      result.viewState.selectedLibraryNodeId,
      libraryProjection?.bindingsById
    )
    applyRestoredSelection(
      'addSource',
      result.viewState.selectedAddSourceNodeId,
      addSource?.bindingsById
    )
    applyRestoredExpansion(
      'libraryBrowse',
      result.viewState.expandedLibraryNodeIds,
      libraryProjection?.bindingsById
    )
    applyRestoredExpansion(
      'addSource',
      result.viewState.expandedAddSourceNodeIds,
      addSource?.bindingsById
    )
    libraryBrowseProfile.restoreProfile(result.viewState.libraryBrowseProfile)
    addSourceView.restoreView(result.viewState.addSourceView)
    primarySelection.value = sourceSubject(sourceStatusContext.value)
  } catch {
    restoreState.readCompleted = true
  }
}

function toggleLibraryBrowseProfileMenu(): void {
  libraryBrowseProfileMenuOpen.value = !libraryBrowseProfileMenuOpen.value
}

function selectLibraryBrowseProfile(profile: LibraryBrowseProfile): void {
  libraryBrowseProfile.setProfile(profile)
  clearContentSelection()
  libraryBrowseProfileMenuOpen.value = false
}

function closeLibraryBrowseProfileMenu(): void {
  libraryBrowseProfileMenuOpen.value = false
}

function toggleViewModeMenu(): void {
  viewModeMenuOpen.value = !viewModeMenuOpen.value
}

function selectView(nextView: ViewMode): void {
  viewMode.setView(nextView)
  viewModeMenuOpen.value = false
}

function closeViewModeMenu(): void {
  viewModeMenuOpen.value = false
}

function toggleAddSourceViewMenu(): void {
  addSourceViewMenuOpen.value = !addSourceViewMenuOpen.value
}

function selectAddSourceView(mode: AddSourceView): void {
  addSourceView.setView(mode)
  clearContentSelection()
  addSourceViewMenuOpen.value = false
}

function closeAddSourceViewMenu(): void {
  addSourceViewMenuOpen.value = false
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
  if (!librarySearch.searchOpen.value) {
    void nextTick(() => {
      searchOpenButtonRef.value?.focus()
    })
  }
}

function clearSearchAndReturnToScope(): void {
  librarySearch.clearSearch()
  clearContentSelection()
  requestContentsForCurrentSelection()
  void nextTick(() => {
    searchOpenButtonRef.value?.focus()
  })
}

function openLibraryBrowseSurface(): void {
  markUserInteraction()
  activeSurface.value = 'libraryBrowse'
  primarySelection.value = sourceSubject(sourceStatusContext.value)
  requestContentsForCurrentSelection()
  saveViewState()
}

function openAddSourceIntake(): void {
  markUserInteraction()
  sourceAdmissionHandoff.value = undefined
  activeSurface.value = 'addSource'
  selectedAddSourceNodeId.value = undefined
  primarySelection.value = clearSelection()
  saveViewState()
}

function activateToolbarAddMusicFolder(): void {
  if (!toolbarModel.value.addMusicFolder.enabled) {
    return
  }

  if (toolbarModel.value.addMusicFolder.kind === 'openAddSource') {
    openAddSourceIntake()
    return
  }

  void rootLifecycle.addMusicFolder()
}

function handleLibraryBrowseProfileOutsidePointerDown(event: PointerEvent): void {
  const target = event.target

  if (
    libraryBrowseProfileMenuOpen.value &&
    (!(target instanceof Node) || !libraryBrowseProfileMenuRef.value?.contains(target))
  ) {
    libraryBrowseProfileMenuOpen.value = false
  }

  if (
    viewModeMenuOpen.value &&
    (!(target instanceof Node) || !viewModeMenuRef.value?.contains(target))
  ) {
    viewModeMenuOpen.value = false
  }

  if (
    addSourceViewMenuOpen.value &&
    (!(target instanceof Node) || !addSourceViewMenuRef.value?.contains(target))
  ) {
    addSourceViewMenuOpen.value = false
  }
}

function applyRestoredSelection(
  surface: LibraryPanelSurface,
  nodeId: BrowserTreeNodeId | undefined,
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding> | undefined
): void {
  if (nodeId === undefined || bindingsById === undefined || !bindingsById.has(nodeId)) {
    return
  }

  if (surface === 'addSource') {
    selectedAddSourceNodeId.value = nodeId
  } else {
    selectedLibraryNodeId.value = nodeId
  }

  restoreState.initialNodeApplied = true
}

function applyRestoredExpansion(
  surface: LibraryPanelSurface,
  nodeIds: readonly BrowserTreeNodeId[],
  bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding> | undefined
): void {
  const visibleIds: BrowserTreeNodeId[] = []
  const pendingIds: BrowserTreeNodeId[] = []

  for (const nodeId of nodeIds) {
    if (bindingsById?.has(nodeId)) {
      visibleIds.push(nodeId)
    } else {
      pendingIds.push(nodeId)
    }
  }

  if (visibleIds.length > 0) {
    if (surface === 'addSource') {
      expandedAddSourceNodeIds.value = new Set(visibleIds)
    } else {
      expandedLibraryNodeIds.value = new Set(visibleIds)
    }
    restoreState.initialNodeApplied = true
  }

  if (pendingIds.length > 0) {
    if (surface === 'addSource') {
      pendingAddSourceRestoreIds.value = new Set(pendingIds)
    } else {
      pendingLibraryRestoreIds.value = new Set(pendingIds)
    }
  }
}

function applyPendingRestoreIds(surface: LibraryPanelSurface): void {
  const projection =
    surface === 'addSource' ? addSourceProjection.value : libraryBrowseProjection.value
  const pendingRestoreIds =
    surface === 'addSource' ? pendingAddSourceRestoreIds : pendingLibraryRestoreIds

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
  if (surface === 'addSource') {
    expandedAddSourceNodeIds.value = new Set([...expandedAddSourceNodeIds.value, ...appliedIds])
  } else {
    expandedLibraryNodeIds.value = new Set([...expandedLibraryNodeIds.value, ...appliedIds])
  }
  restoreState.initialNodeApplied = true
}

function markUserInteraction(): void {
  restoreState.userInteracted = true
  pendingLibraryRestoreIds.value = new Set()
  pendingAddSourceRestoreIds.value = new Set()
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

  activeSurface.value = 'libraryBrowse'
  selectedLibraryNodeId.value = visibleRegistration.nodeId
  primarySelection.value = sourceSubject(sourceStatusContext.value)
  restoreState.initialNodeApplied = true
  requestContentsForCurrentSelection()
  saveViewState()
}

function applyEmptyLibraryDefaultSurface(projection: ReturnType<typeof projectState>): void {
  if (restoreState.userInteracted || activeSurface.value === 'addSource') {
    return
  }

  if (hasAdmittedLibraryRows(projection)) {
    return
  }

  activeSurface.value = 'addSource'
  saveViewState()
}

function hasAdmittedLibraryRows(projection: ReturnType<typeof projectState>): boolean {
  if (projection?.kind !== 'tree') {
    return false
  }

  for (const binding of projection.bindingsById.values()) {
    if (binding.kind === 'source' || binding.kind === 'directory' || binding.kind === 'file') {
      return true
    }
  }

  return false
}

function selectNode(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()

  if (activeSurface.value === 'addSource') {
    selectedAddSourceNodeId.value = nodeId
    void requestAddSourceNodeChildren(nodeId)
  } else {
    selectedLibraryNodeId.value = nodeId
    requestContentsForCurrentSelection()
  }

  primarySelection.value = sourceSubject(sourceStatusContext.value)
  saveViewState()
}

function toggleNode(nodeId: BrowserTreeNodeId): void {
  markUserInteraction()

  const expandedNodeIds =
    activeSurface.value === 'addSource' ? expandedAddSourceNodeIds : expandedLibraryNodeIds
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

  if (activeSurface.value === 'addSource') {
    expandedAddSourceNodeIds.value = new Set([...expandedAddSourceNodeIds.value, nodeId])
    saveViewState()
    addSourceDisclosureReconciler.clearFailedForNode(nodeId)
    void requestAddSourceNodeChildren(nodeId)
    return
  }

  expandedLibraryNodeIds.value = new Set([...expandedLibraryNodeIds.value, nodeId])
  saveViewState()

  disclosureReconciler.clearFailedForNode(nodeId)
  void requestLibraryNodeChildren(nodeId)
}

function prepareNodeContents(nodeId: BrowserTreeNodeId): void {
  if (activeSurface.value !== 'libraryBrowse') {
    return
  }

  contentsRead.preloadForBinding(libraryBrowseProjection.value?.bindingsById.get(nodeId))
}

function cancelPrepareNodeContents(nodeId: BrowserTreeNodeId): void {
  if (activeSurface.value !== 'libraryBrowse') {
    return
  }

  contentsRead.cancelPreloadForBinding(libraryBrowseProjection.value?.bindingsById.get(nodeId))
}

function refreshPlanExecutionDependencies(): RefreshPlanDeps {
  return {
    hierarchyRead,
    refreshLocalBrowseEntryPoints: () => localBrowse.refreshEntryPoints(),
    sourceLifecycleRead,
    expandedNodeIds: expandedLibraryNodeIds.value,
    clearContentsWarmSnapshots: () => contentsRead.clearWarmSnapshots(),
    refreshContentsForCurrentSelection,
    refreshActiveSearchFilter: () => searchFilterRead.invalidationSignal()
  }
}

function refreshContentsForCurrentSelection(): Promise<boolean> {
  const selectedId = selectedLibraryNodeId.value
  const projection = libraryBrowseProjection.value

  if (selectedId === undefined || projection === undefined) {
    contentsRead.clear()
    return Promise.resolve(false)
  }

  return contentsRead.readForBinding(projection.bindingsById.get(selectedId), {
    force: true,
    retainAccumulatedRows: true
  })
}

async function handleStatusAction(action: StatusAction): Promise<void> {
  if (!action.enabled) {
    return
  }

  switch (action.kind) {
    case 'addLocalPath':
      await rootLifecycle.addLocalPath(action.resolvedPath).then(async (registered) => {
        if (registered) {
          await localBrowse.refreshBrowserWindows(
            expandedAddSourceNodeIds.value,
            addSourceProjection.value
          )
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
  const projection = libraryBrowseProjection.value
  const nodeId = sourceNodeIdForSourceId(projection, sourceId)

  if (nodeId === undefined) {
    pendingSourceRegistration.value = sourceRegistrationIntent(sourceId)
    return
  }

  sourceRevealRequest.value = {
    nodeId,
    sequence: ++sourceRevealSequence
  }
  activeSurface.value = 'libraryBrowse'
  selectedLibraryNodeId.value = nodeId
  primarySelection.value = sourceSubject(sourceStatusContext.value)
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

function selectedContentsSourceId(): string | undefined {
  const selectedId = selectedLibraryNodeId.value
  const binding =
    selectedId === undefined
      ? undefined
      : libraryBrowseProjection.value?.bindingsById.get(selectedId)

  if (binding?.kind === 'source' && binding.target.entryPoint.kind === 'source') {
    return binding.target.entryPoint.sourceId
  }

  if (binding?.kind === 'directory') {
    return binding.sourceId
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
  const [localRoots, lifecycle, integrity, maintenance, activity] = await Promise.all([
    refreshLocalRootAvailabilityForSourceStatus(sourceId),
    sourceLifecycleRead.readSourceLifecycle(sourceId, { force: true }),
    integrityRead.read(sourceId, { force: true }),
    maintenanceRead.read(sourceId, { force: true }),
    activityRead.read(sourceId, { force: true })
  ])

  const statusRefreshed = localRoots && lifecycle && integrity && maintenance && activity
  if (statusRefreshed && selectedStatusSourceId.value === sourceId) {
    await refreshContentsForCurrentSelection()
  }

  return statusRefreshed
}

async function refreshLocalRootAvailabilityForSourceStatus(sourceId: string): Promise<boolean> {
  const matchedCurrentRoot = await rootLifecycle.hydrateLocalRoots().catch(() => false)
  if (matchedCurrentRoot) {
    return true
  }

  const localRootsReadState = rootActions.localRootsReadState.value
  return (
    localRootsReadState.kind === 'ready' &&
    localRootsReadState.roots.some((root) => root.rootId === sourceId)
  )
}

function invalidateSourceStatus(sourceId: string): void {
  invalidateSourceStatusReaders(
    {
      sourceLifecycleRead,
      integrityRead,
      maintenanceRead,
      activityRead
    },
    sourceId
  )
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

function requestLibraryNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
  return hierarchyRead.requestNodeChildren(nodeId)
}

function requestAddSourceNodeChildren(nodeId: BrowserTreeNodeId): Promise<boolean> {
  const projection = addSourceProjection.value
  const binding = projection?.bindingsById.get(nodeId)

  if (
    binding?.kind !== 'localBrowseEntryPoint' &&
    binding?.kind !== 'localBrowseItem' &&
    binding?.kind !== 'localBrowseMore'
  ) {
    return Promise.resolve(false)
  }

  return localBrowse.requestNodeChildren(nodeId, projection)
}

function clearBrowserView(): void {
  selectedLibraryNodeId.value = undefined
  primarySelection.value = clearSelection()
  contentsRead.clear()
  localBrowse.clearItemWindows()
  expandedLibraryNodeIds.value = new Set()
  pendingLibraryRestoreIds.value = new Set()
  pendingSourceRegistration.value = undefined
  sourceAdmissionHandoff.value = undefined
  sourceRevealRequest.value = undefined
  activeSurface.value = 'addSource'

  restoreState.userInteracted = false
  restoreState.initialNodeApplied = false

  viewStateStore.save({
    version: 3,
    activeSurface: activeSurface.value,
    libraryBrowseProfile: libraryBrowseProfile.profile.value,
    addSourceView: addSourceView.view.value,
    expandedLibraryNodeIds: [],
    expandedAddSourceNodeIds: [...expandedAddSourceNodeIds.value],
    ...(selectedAddSourceNodeId.value === undefined
      ? {}
      : { selectedAddSourceNodeId: selectedAddSourceNodeId.value })
  })
}

function selectContentRow(row: ContentRow): void {
  markUserInteraction()
  const selection = rowSubject(row)

  if (!isValidSelection(selection, contentsProjection.value)) {
    return
  }

  primarySelection.value = selection
}

function clearContentSelection(): void {
  if (primarySelection.value.kind === 'row') {
    primarySelection.value = clearSelection()
  }
}

function contentSelectionRowsAreCurrent(): boolean {
  if (activeSurface.value === 'libraryBrowse' && librarySearch.searchActive.value) {
    return searchFilterRead.state.value.kind !== 'Pending'
  }

  const state = contentsRead.state.value
  return (
    state?.kind !== 'ready' ||
    state.pending === undefined ||
    state.pending.requestKey === state.requestKey
  )
}

function activateContentRowAction(row: ContentRow): void {
  markUserInteraction()

  const action = row.action

  if (action === undefined) {
    return
  }

  if (action.kind === 'loadChildren') {
    expandedLibraryNodeIds.value = new Set([...expandedLibraryNodeIds.value, action.nodeId])
    saveViewState()
    disclosureReconciler.clearFailedForNode(action.nodeId)
    void hierarchyRead.requestNodeChildren(action.nodeId)
  } else if (action.kind === 'loadLocalBrowseChildren') {
    expandedAddSourceNodeIds.value = new Set([...expandedAddSourceNodeIds.value, action.nodeId])
    saveViewState()
    addSourceDisclosureReconciler.clearFailedForNode(action.nodeId)
    void localBrowse.requestNodeChildren(action.nodeId, addSourceProjection.value)
  } else if (action.kind === 'loadLocalBrowseMore') {
    void localBrowse.requestNodeMore(action.nodeId, addSourceProjection.value)
  } else if (action.kind === 'requestLocalBrowseAdmission') {
    void rootLifecycle.addLocalPath(action.resolvedPath).then(async (registered) => {
      if (registered) {
        await localBrowse.refreshBrowserWindows(
          expandedAddSourceNodeIds.value,
          addSourceProjection.value
        )
      }
    })
  } else if (action.kind === 'chooseMusicFolder') {
    void rootLifecycle.addMusicFolder()
  } else if (action.kind === 'loadContentsPage') {
    void contentsRead.readForBinding(
      libraryBrowseProjection.value?.bindingsById.get(action.nodeId),
      {
        cursor: action.cursor
      }
    )
  } else if (action.kind === 'loadSearchPage') {
    void searchFilterRead.loadNext()
  }
}

function activateSourceAdmissionHandoffAction(action: SourceAdmissionHandoffAction): void {
  if (!action.enabled) {
    return
  }

  if (action.kind === 'viewSource') {
    showAdmittedSource(action.sourceId)
    sourceAdmissionHandoff.value = undefined
    return
  }

  if (action.kind === 'scanSource') {
    void rootLifecycle.scanRoot(action.sourceId)
    return
  }

  if (action.kind === 'keepBrowsing') {
    sourceAdmissionHandoff.value = undefined
    return
  }

  openAddSourceIntake()
}

function requestContentsForCurrentSelection(options: { readonly force?: boolean } = {}): void {
  const selectedId = selectedLibraryNodeId.value
  const projection = libraryBrowseProjection.value

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
  <WorkstationShell
    :title="activeSurfaceTitle"
    :tree="treeRootProps"
    :contents="{
      view: contentsView,
      projection: contentsProjection,
      columnProjection: activeProjection,
      selectedNodeId: activeSelectedNodeId,
      statusView: sourceStatusView,
      sourceAdmissionHandoff: sourceAdmissionHandoffView,
      selectedRowId: selectedContentRowId,
      selectNode: selectNode,
      selectRow: selectContentRow,
      activateRowAction: activateContentRowAction,
      activateStatusAction: handleStatusAction,
      activateSourceAdmissionHandoffAction: activateSourceAdmissionHandoffAction
    }"
    :inspector="{
      selection: primarySelection,
      status: inspectorStatusView
    }"
    @select="selectNode"
    @toggle="toggleNode"
    @activate-action="activateNodeAction"
    @prepare="prepareNodeContents"
    @cancel-prepare="cancelPrepareNodeContents"
  >
    <template #bar-controls>
      <button
        v-if="activeSurface === 'addSource' && hasAdmittedLibraryRowsVisible"
        type="button"
        :class="iconButtonClass"
        aria-label="Library Browse"
        title="Library Browse"
        @click="openLibraryBrowseSurface"
      >
        <Icon role="folder.plain" size="md" />
      </button>

      <button
        v-if="toolbarModel.search.visible && !librarySearch.searchOpen.value"
        ref="searchOpenButtonRef"
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
          :class="`${toolbarControlClass} h-9 w-44 rounded-sm px-3 py-2 text-sm font-semibold outline-none transition placeholder:text-(--color-text-muted) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)`"
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
          @click="clearSearchAndReturnToScope"
        >
          <Icon role="action.clear" size="md" />
        </button>
      </div>

      <div v-if="toolbarModel.viewMode.visible" ref="viewModeMenuRef" class="relative inline-flex">
        <button
          type="button"
          :class="iconButtonClass"
          :aria-label="toolbarModel.viewMode.label"
          :aria-expanded="viewModeMenuOpen"
          aria-haspopup="listbox"
          :title="toolbarModel.viewMode.title"
          :disabled="!toolbarModel.viewMode.enabled"
          @click="toggleViewModeMenu"
          @keydown.escape.stop.prevent="closeViewModeMenu"
        >
          <Icon role="action.browseView" size="md" />
        </button>

        <div
          v-if="viewModeMenuOpen"
          class="absolute right-0 top-full z-20 mt-1 min-w-40 border border-(--color-border) bg-(--color-background) py-1 shadow-lg"
          role="listbox"
          :aria-label="toolbarModel.viewMode.label"
          tabindex="-1"
          @keydown.escape.stop.prevent="closeViewModeMenu"
        >
          <button
            v-for="option in viewModeOptions"
            :key="option.key"
            type="button"
            class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-(--color-text) hover:bg-(--color-surface) focus-visible:bg-(--color-surface) focus-visible:outline-none"
            :class="viewMode.view.value === option.key ? 'font-bold' : 'font-normal'"
            role="option"
            :aria-selected="viewMode.view.value === option.key"
            @click="selectView(option.key)"
          >
            <span>{{ option.label }}</span>
          </button>
        </div>
      </div>

      <div
        v-if="toolbarModel.libraryBrowseProfile.visible"
        ref="libraryBrowseProfileMenuRef"
        class="relative inline-flex"
      >
        <button
          type="button"
          :class="iconButtonClass"
          :aria-label="toolbarModel.libraryBrowseProfile.label"
          :aria-expanded="libraryBrowseProfileMenuOpen"
          aria-haspopup="listbox"
          :title="toolbarModel.libraryBrowseProfile.title"
          :disabled="!toolbarModel.libraryBrowseProfile.enabled"
          @click="toggleLibraryBrowseProfileMenu"
          @keydown.escape.stop.prevent="closeLibraryBrowseProfileMenu"
        >
          <Icon role="action.browseView" size="md" />
        </button>

        <div
          v-if="libraryBrowseProfileMenuOpen"
          class="absolute right-0 top-full z-20 mt-1 min-w-40 border border-(--color-border) bg-(--color-background) py-1 shadow-lg"
          role="listbox"
          :aria-label="toolbarModel.libraryBrowseProfile.label"
          tabindex="-1"
          @keydown.escape.stop.prevent="closeLibraryBrowseProfileMenu"
        >
          <button
            v-for="option in libraryBrowseProfileOptions"
            :key="option.key"
            type="button"
            class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-(--color-text) hover:bg-(--color-surface) focus-visible:bg-(--color-surface) focus-visible:outline-none"
            :class="libraryBrowseProfile.profile.value === option.key ? 'font-bold' : 'font-normal'"
            role="option"
            :aria-selected="libraryBrowseProfile.profile.value === option.key"
            @click="selectLibraryBrowseProfile(option.key)"
          >
            <span>{{ option.label }}</span>
          </button>
        </div>
      </div>

      <div
        v-if="toolbarModel.addSourceView.visible"
        ref="addSourceViewMenuRef"
        class="relative inline-flex"
      >
        <button
          type="button"
          :class="iconButtonClass"
          :aria-label="toolbarModel.addSourceView.label"
          :aria-expanded="addSourceViewMenuOpen"
          aria-haspopup="listbox"
          :title="toolbarModel.addSourceView.title"
          :disabled="!toolbarModel.addSourceView.enabled"
          @click="toggleAddSourceViewMenu"
          @keydown.escape.stop.prevent="closeAddSourceViewMenu"
        >
          <Icon role="action.browseView" size="md" />
        </button>

        <div
          v-if="addSourceViewMenuOpen"
          class="absolute right-0 top-full z-20 mt-1 min-w-48 border border-(--color-border) bg-(--color-background) py-1 shadow-lg"
          role="listbox"
          :aria-label="toolbarModel.addSourceView.label"
          tabindex="-1"
          @keydown.escape.stop.prevent="closeAddSourceViewMenu"
        >
          <button
            v-for="option in addSourceViewOptions"
            :key="option.key"
            type="button"
            class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm text-(--color-text) hover:bg-(--color-surface) focus-visible:bg-(--color-surface) focus-visible:outline-none"
            :class="addSourceView.view.value === option.key ? 'font-bold' : 'font-normal'"
            role="option"
            :aria-selected="addSourceView.view.value === option.key"
            @click="selectAddSourceView(option.key)"
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
        @click="activateToolbarAddMusicFolder"
      >
        {{ toolbarModel.addMusicFolder.label }}
      </button>
    </template>
  </WorkstationShell>
</template>
