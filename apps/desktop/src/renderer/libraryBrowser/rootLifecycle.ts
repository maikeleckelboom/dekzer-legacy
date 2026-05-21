import { computed, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryHierarchyReadController } from './hierarchyRead'
import type { LocalRootActionsController } from './localRootActions'

export type RootLifecycleRefreshStatus = 'idle' | 'refreshing' | 'refreshed' | 'failed'

export type RootLifecycleController = {
  readonly refreshStatus: Ref<RootLifecycleRefreshStatus>
  readonly refreshFeedback: ComputedRef<string | undefined>
  readonly refreshFeedbackClass: ComputedRef<string>
  readonly canAddMusicFolder: ComputedRef<boolean>
  readonly canScanRoot: ComputedRef<boolean>
  readonly addMusicFolder: () => Promise<boolean>
  readonly scanRoot: () => Promise<boolean>
}

export type RootLifecycleDependencies = {
  readonly rootActions: LocalRootActionsController
  readonly hierarchyRead: Pick<LibraryHierarchyReadController, 'refresh'>
}

const safeRefreshFailure = 'Scan complete, but the library view could not refresh.'

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

  const refreshFeedback = computed(() => {
    switch (refreshStatus.value) {
      case 'idle':
        return undefined
      case 'refreshing':
        return 'Refreshing library view...'
      case 'refreshed':
        return 'Library view refreshed.'
      case 'failed':
        return safeRefreshFailure
    }

    return safeRefreshFailure
  })

  const refreshFeedbackClass = computed(() => {
    switch (refreshStatus.value) {
      case 'refreshed':
        return 'text-(--color-accent)'
      case 'failed':
        return 'text-(--color-danger)'
      default:
        return 'text-(--color-text-muted)'
    }
  })

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

    return refreshAfterScan()
  }

  async function refreshAfterScan(): Promise<boolean> {
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
    refreshFeedback,
    refreshFeedbackClass,
    canAddMusicFolder,
    canScanRoot,
    addMusicFolder,
    scanRoot
  }
}
