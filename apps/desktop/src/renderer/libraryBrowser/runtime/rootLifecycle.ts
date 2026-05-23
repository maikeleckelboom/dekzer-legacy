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
  readonly addMusicFolder: () => Promise<boolean>
  readonly scanRoot: () => Promise<boolean>
  readonly removeSource: () => Promise<boolean>
  readonly hydrateLocalRoots: () => Promise<boolean>
}

export type RootLifecycleDependencies = {
  readonly rootActions: LocalRootActionsController
  readonly hierarchyRead: Pick<LibraryHierarchyReadController, 'refresh'>
  readonly confirmRemoveSource: () => boolean
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

  async function addMusicFolder(): Promise<boolean> {
    if (!canAddMusicFolder.value) {
      return false
    }

    const registered = await dependencies.rootActions.chooseAndRegisterLocalRoot()

    if (!registered) {
      return false
    }

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

  async function removeSource(): Promise<boolean> {
    if (!canRemoveSource.value) {
      return false
    }

    const confirmed = dependencies.confirmRemoveSource()

    if (!confirmed) {
      return false
    }

    const unregistered = await dependencies.rootActions.unregisterLocalRoot()

    if (!unregistered) {
      return false
    }

    resetRefreshState()
    return runRefresh()
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
    addMusicFolder,
    scanRoot,
    removeSource,
    hydrateLocalRoots: dependencies.rootActions.hydrateLocalRoots
  }
}
