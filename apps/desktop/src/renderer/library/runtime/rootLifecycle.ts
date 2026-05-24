import { computed, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { LocalRootActionsController } from '../boundary/localRootActions'

export type RootLifecycleRefreshStatus = 'idle' | 'refreshing' | 'refreshed' | 'failed'

export type RootLifecycleController = {
  readonly refreshStatus: Ref<RootLifecycleRefreshStatus>
  readonly canAddMusicFolder: ComputedRef<boolean>
  readonly canScanRoot: ComputedRef<boolean>
  readonly canRemoveSource: ComputedRef<boolean>
  readonly canRemoveSourceRoot: (rootId: string | undefined) => boolean
  readonly addMusicFolder: () => Promise<boolean>
  readonly scanRoot: () => Promise<boolean>
  readonly removeSource: (rootId?: string) => Promise<boolean>
  readonly hydrateLocalRoots: () => Promise<boolean>
}

export type RootLifecycleDependencies = {
  readonly rootActions: LocalRootActionsController
  readonly hierarchyRead: Pick<LibraryHierarchyReadController, 'refresh'>
  readonly confirmRemoveSource: () => boolean
  readonly isSourceRootVisible: (rootId: string) => boolean
}

export function useRootLifecycle(dependencies: RootLifecycleDependencies): RootLifecycleController {
  return createRootLifecycleController(dependencies)
}

export function createRootLifecycleController(
  dependencies: RootLifecycleDependencies
): RootLifecycleController {
  const refreshStatus = ref<RootLifecycleRefreshStatus>('idle')
  let refreshSequence = 0

  const canAddMusicFolder = computed(
    () => dependencies.rootActions.canChooseLocalRoot.value && refreshStatus.value !== 'refreshing'
  )
  const canScanRoot = computed(
    () =>
      dependencies.rootActions.canRunRegisteredRootScan.value &&
      refreshStatus.value !== 'refreshing'
  )
  const canRemoveSource = computed(
    () =>
      dependencies.rootActions.canUnregisterLocalRoot.value && refreshStatus.value !== 'refreshing'
  )

  function canRemoveSourceRoot(rootId: string | undefined): boolean {
    return (
      dependencies.rootActions.canUnregisterLocalRootId(rootId) &&
      refreshStatus.value !== 'refreshing'
    )
  }

  async function addMusicFolder(): Promise<boolean> {
    if (!canAddMusicFolder.value) {
      return false
    }

    const registered = await dependencies.rootActions.chooseAndRegisterLocalRoot()

    if (!registered) {
      return false
    }

    void dependencies.rootActions.hydrateLocalRoots()
    return scanRoot()
  }

  async function scanRoot(): Promise<boolean> {
    if (!canScanRoot.value) {
      return false
    }

    resetRefreshState()

    const scanWasRequested = await dependencies.rootActions.runRegisteredRootScan()

    if (!scanWasRequested || dependencies.rootActions.scanStatus.value !== 'scanned') {
      return false
    }

    return runRefresh()
  }

  async function removeSource(rootId?: string): Promise<boolean> {
    if (rootId === undefined ? !canRemoveSource.value : !canRemoveSourceRoot(rootId)) {
      return false
    }

    const confirmed = dependencies.confirmRemoveSource()

    if (!confirmed) {
      return false
    }

    const rootIdToRemove = rootId ?? dependencies.rootActions.registeredRoot.value?.rootId

    if (rootIdToRemove === undefined) {
      return false
    }

    const unregistered = await dependencies.rootActions.unregisterLocalRoot(rootIdToRemove)

    if (!unregistered) {
      return false
    }

    resetRefreshState()
    const refreshed = await runRefresh()

    if (!refreshed) {
      dependencies.rootActions.failRemoveSource(
        'The source removal was requested, but the library view did not refresh.'
      )
      return false
    }

    if (dependencies.isSourceRootVisible(rootIdToRemove)) {
      dependencies.rootActions.failRemoveSource(
        'The source is still visible after refresh, so removal did not complete.'
      )
      return false
    }

    dependencies.rootActions.completeRemoveSource(rootIdToRemove)
    return true
  }

  async function runRefresh(): Promise<boolean> {
    const sequence = ++refreshSequence
    refreshStatus.value = 'refreshing'

    let refreshed = false

    try {
      refreshed = await dependencies.hierarchyRead.refresh()
    } catch {
      refreshed = false
    }

    if (sequence !== refreshSequence) {
      return false
    }

    refreshStatus.value = refreshed ? 'refreshed' : 'failed'
    return refreshed
  }

  function resetRefreshState(): void {
    refreshSequence += 1
    refreshStatus.value = 'idle'
  }

  return {
    refreshStatus,
    canAddMusicFolder,
    canScanRoot,
    canRemoveSource,
    canRemoveSourceRoot,
    addMusicFolder,
    scanRoot,
    removeSource,
    hydrateLocalRoots: dependencies.rootActions.hydrateLocalRoots
  }
}
