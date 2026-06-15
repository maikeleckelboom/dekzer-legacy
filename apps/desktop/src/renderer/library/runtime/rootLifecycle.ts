import { computed, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LibraryHierarchyReadController } from '../boundary/hierarchyRead'
import type { LocalRootActionsController } from '../boundary/localRootActions'
import type { LocalRootRegistrationRoot } from '../../../shared/library/roots/register'
import type { LocalBrowseController } from '../localBrowse/controller'

export type RootLifecycleRefreshStatus = 'idle' | 'refreshing' | 'refreshed' | 'failed'

export type RootLifecycleController = {
  readonly refreshStatus: Ref<RootLifecycleRefreshStatus>
  readonly canAddMusicFolder: ComputedRef<boolean>
  readonly canScanRoot: ComputedRef<boolean>
  readonly canRemoveSource: ComputedRef<boolean>
  readonly canScanSourceRoot: (rootId: string | undefined) => boolean
  readonly canRemoveSourceRoot: (rootId: string | undefined) => boolean
  readonly addMusicFolder: () => Promise<boolean>
  readonly addLocalPath: (requestedPath: string) => Promise<boolean>
  readonly scanRoot: (rootId?: string) => Promise<boolean>
  readonly removeSource: (rootId?: string) => Promise<boolean>
  readonly hydrateLocalRoots: () => Promise<boolean>
}

export type RootLifecycleDependencies = {
  readonly rootActions: LocalRootActionsController
  readonly hierarchyRead: Pick<LibraryHierarchyReadController, 'refresh'>
  readonly localBrowseRead?: Pick<LocalBrowseController, 'clearItemWindows' | 'refreshEntryPoints'>
  readonly confirmRemoveSource: () => boolean
  readonly isSourceRootVisible: (rootId: string) => boolean
  readonly invalidateSourceStatus?: (rootId: string) => void
  readonly onSourceRegistered?: (root: LocalRootRegistrationRoot) => void
  readonly onSourceRemoved?: (rootId: string) => void
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

  function canScanSourceRoot(rootId: string | undefined): boolean {
    return (
      dependencies.rootActions.canRunLocalRootScan(rootId) && refreshStatus.value !== 'refreshing'
    )
  }

  async function addMusicFolder(): Promise<boolean> {
    if (!canAddMusicFolder.value) {
      return false
    }

    const registered = await dependencies.rootActions.chooseAndRegisterLocalRoot()

    return finishRegistration(registered)
  }

  async function addLocalPath(requestedPath: string): Promise<boolean> {
    if (!canAddMusicFolder.value) {
      return false
    }

    const registered = await dependencies.rootActions.registerLocalPath(requestedPath)

    return finishRegistration(registered)
  }

  async function finishRegistration(registered: boolean): Promise<boolean> {
    if (!registered) {
      return false
    }

    const registeredRoot = dependencies.rootActions.registeredRoot.value
    if (registeredRoot !== undefined) {
      dependencies.invalidateSourceStatus?.(registeredRoot.rootId)
      dependencies.onSourceRegistered?.(registeredRoot)
    }

    const localRootsHydrated = dependencies.rootActions.hydrateLocalRoots().catch(() => false)
    const registrationRefreshed = await runRefresh()
    const scanned = await scanRoot()
    await localRootsHydrated

    return registrationRefreshed && scanned
  }

  async function scanRoot(rootId?: string): Promise<boolean> {
    const selectedRootId = rootId ?? dependencies.rootActions.registeredRoot.value?.rootId

    if (
      selectedRootId === undefined ||
      dependencies.rootActions.scanStatus.value === 'scanning' ||
      !canScanSourceRoot(selectedRootId)
    ) {
      return false
    }

    resetRefreshState()

    const scanWasRequested =
      rootId === undefined
        ? await dependencies.rootActions.runRegisteredRootScan()
        : await dependencies.rootActions.runRootScan(selectedRootId)

    if (!scanWasRequested) {
      return false
    }

    return true
  }

  async function removeSource(rootId?: string): Promise<boolean> {
    if (
      dependencies.rootActions.scanStatus.value === 'scanning' ||
      dependencies.rootActions.removeSourceStatus.value === 'removing' ||
      (rootId === undefined ? !canRemoveSource.value : !canRemoveSourceRoot(rootId))
    ) {
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

    dependencies.invalidateSourceStatus?.(rootIdToRemove)
    dependencies.localBrowseRead?.clearItemWindows()
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

    dependencies.onSourceRemoved?.(rootIdToRemove)
    dependencies.rootActions.completeRemoveSource(rootIdToRemove)
    return true
  }

  async function runRefresh(): Promise<boolean> {
    const sequence = ++refreshSequence
    refreshStatus.value = 'refreshing'

    let refreshed = false

    try {
      refreshed = await refreshAll()
    } catch {
      refreshed = false
    }

    if (sequence !== refreshSequence) {
      return false
    }

    refreshStatus.value = refreshed ? 'refreshed' : 'failed'
    return refreshed
  }

  async function refreshAll(): Promise<boolean> {
    const [hierarchyRefreshed, localBrowseRefreshed] = await Promise.all([
      runBooleanRefresh(dependencies.hierarchyRead.refresh),
      runBooleanRefresh(dependencies.localBrowseRead?.refreshEntryPoints)
    ])

    return hierarchyRefreshed && localBrowseRefreshed
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
    canScanSourceRoot,
    canRemoveSourceRoot,
    addMusicFolder,
    addLocalPath,
    scanRoot,
    removeSource,
    hydrateLocalRoots: dependencies.rootActions.hydrateLocalRoots
  }
}

async function runBooleanRefresh(refresh: (() => Promise<boolean>) | undefined): Promise<boolean> {
  if (refresh === undefined) {
    return true
  }

  try {
    return await refresh()
  } catch {
    return false
  }
}
