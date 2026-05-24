import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

import type { LocalRootChoiceResult } from '../../../shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootRegistrationRoot } from '../../../shared/libraryRoots/registerLocalRoot'
import type { LocalRoot, ReadLocalRootsOutcome } from '../../../shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../../../shared/libraryRoots/runScan'
import type { UnregisterLocalRootResult } from '../../../shared/libraryRoots/unregisterLocalRoot'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibraryRootActionsApi = RendererApi['library']['roots']

export type LocalRootChoiceStatus = 'idle' | 'choosing' | 'canceled' | 'registered' | 'failed'

export type LocalRootScanStatus = 'idle' | 'scanning' | 'scanned' | 'failed'

export type RemoveSourceStatus = 'idle' | 'removing' | 'removed' | 'failed'

export type LocalRootsReadState =
  | { readonly kind: 'unread' }
  | { readonly kind: 'ready'; readonly roots: readonly LocalRoot[] }
  | { readonly kind: 'failed'; readonly message: string }

export type LocalRootScanSummary = {
  readonly rootId: string
  readonly scanRunId: string
  readonly discoveredFileCount: number
  readonly queuedSourceWorkItems: number
}

export type LocalRootActionsController = {
  readonly rootChoiceStatus: Ref<LocalRootChoiceStatus>
  readonly registeredRoot: Ref<LocalRootRegistrationRoot | undefined>
  readonly registeredRootPath: ComputedRef<string | undefined>
  readonly rootChoiceButtonLabel: ComputedRef<string>
  readonly canChooseLocalRoot: ComputedRef<boolean>
  readonly scanStatus: Ref<LocalRootScanStatus>
  readonly scanSummary: Ref<LocalRootScanSummary | undefined>
  readonly scanButtonLabel: ComputedRef<string>
  readonly canRunRegisteredRootScan: ComputedRef<boolean>
  readonly localRootsReadState: Ref<LocalRootsReadState>
  readonly removeSourceStatus: Ref<RemoveSourceStatus>
  readonly removeSourceButtonLabel: ComputedRef<string>
  readonly canUnregisterLocalRoot: ComputedRef<boolean>
  readonly isKnownLocalRootId: (rootId: string | undefined) => rootId is string
  readonly canUnregisterLocalRootId: (rootId: string | undefined) => rootId is string
  readonly chooseAndRegisterLocalRoot: () => Promise<boolean>
  readonly runRegisteredRootScan: () => Promise<boolean>
  readonly hydrateLocalRoots: () => Promise<boolean>
  readonly unregisterLocalRoot: (rootId?: string) => Promise<boolean>
}

const safeRootChoiceFailure = 'Unable to add music folder.'
const safeRootScanFailure = 'Unable to scan folder.'
const safeRootRemoveFailure = 'Unable to remove source.'

export function useLocalRootActions(
  rootApi: LibraryRootActionsApi = getRendererApi().library.roots
): LocalRootActionsController {
  return createLocalRootActionsController(rootApi)
}

export function createLocalRootActionsController(
  rootApi: LibraryRootActionsApi
): LocalRootActionsController {
  const rootChoiceStatus = ref<LocalRootChoiceStatus>('idle')
  const rootChoiceFailureMessage = ref<string>()
  const registeredRoot = ref<LocalRootRegistrationRoot>()
  const scanStatus = ref<LocalRootScanStatus>('idle')
  const scanSummary = ref<LocalRootScanSummary>()
  const scanFailureMessage = ref<string>()
  const localRootsReadState = ref<LocalRootsReadState>({ kind: 'unread' })
  const removeSourceStatus = ref<RemoveSourceStatus>('idle')
  const removeFailureMessage = ref<string>()

  const registeredRootPath = computed(() => registeredRoot.value?.canonicalPath)
  const canChooseLocalRoot = computed(
    () => rootChoiceStatus.value !== 'choosing' && scanStatus.value !== 'scanning'
  )
  const rootChoiceButtonLabel = computed(() =>
    rootChoiceStatus.value === 'choosing' ? 'Adding folder' : 'Add music folder'
  )
  const selectedRootAvailability = computed<LocalRoot['availability'] | undefined>(() => {
    const root = registeredRoot.value
    if (root === undefined) return undefined
    const readState = localRootsReadState.value
    if (readState.kind !== 'ready') return undefined
    return readState.roots.find((r) => r.rootId === root.rootId)?.availability
  })
  const canRunRegisteredRootScan = computed(
    () =>
      registeredRoot.value !== undefined &&
      scanStatus.value !== 'scanning' &&
      selectedRootAvailability.value !== 'unavailable'
  )
  const scanButtonLabel = computed(() => {
    switch (scanStatus.value) {
      case 'scanning':
        return 'Scanning folder'
      case 'scanned':
        return 'Rescan folder'
      case 'failed':
        return 'Retry scan'
      case 'idle':
        return 'Scan folder'
    }

    return 'Scan folder'
  })

  const canUnregisterLocalRoot = computed(
    () => registeredRoot.value !== undefined && removeSourceStatus.value !== 'removing'
  )

  const removeSourceButtonLabel = computed(() => {
    if (removeSourceStatus.value === 'removing') return 'Removing source'
    return 'Remove source'
  })

  async function chooseAndRegisterLocalRoot(): Promise<boolean> {
    if (!canChooseLocalRoot.value) {
      return false
    }

    rootChoiceStatus.value = 'choosing'
    rootChoiceFailureMessage.value = undefined

    try {
      const result = await rootApi.chooseAndRegisterLocal()

      if (result.state === 'registered') {
        rootChoiceStatus.value = 'registered'
        registeredRoot.value = result.root
        localRootsReadState.value = { kind: 'unread' }
        resetScanState()
        return true
      }

      if (result.state === 'canceled') {
        rootChoiceStatus.value = 'canceled'
        return false
      }

      rootChoiceStatus.value = 'failed'
      rootChoiceFailureMessage.value = rootChoiceFailureFor(result.state)
      return false
    } catch {
      rootChoiceStatus.value = 'failed'
      rootChoiceFailureMessage.value = safeRootChoiceFailure
      return false
    }
  }

  async function runRegisteredRootScan(): Promise<boolean> {
    const root = registeredRoot.value

    if (root === undefined || scanStatus.value === 'scanning') {
      return false
    }

    scanStatus.value = 'scanning'
    scanSummary.value = undefined
    scanFailureMessage.value = undefined

    try {
      const result = await rootApi.runScan({
        rootId: root.rootId
      })

      if (result.state === 'scanned') {
        scanStatus.value = 'scanned'
        scanSummary.value = scanSummaryFromResult(result)
        return true
      }

      scanStatus.value = 'failed'
      scanFailureMessage.value = scanFailureFor(result.state)
      return true
    } catch {
      scanStatus.value = 'failed'
      scanFailureMessage.value = safeRootScanFailure
      return true
    }
  }

  function resetScanState(): void {
    scanStatus.value = 'idle'
    scanSummary.value = undefined
    scanFailureMessage.value = undefined
  }

  async function hydrateLocalRoots(): Promise<boolean> {
    if (rootChoiceStatus.value !== 'idle' || registeredRoot.value !== undefined) {
      return false
    }

    let result: ReadLocalRootsOutcome
    try {
      result = await rootApi.readLocalRoots()
    } catch {
      localRootsReadState.value = { kind: 'failed', message: 'Unable to read local roots.' }
      return false
    }

    if (result.state !== 'read') {
      localRootsReadState.value = { kind: 'failed', message: result.error.message }
      return false
    }

    localRootsReadState.value = { kind: 'ready', roots: result.roots }

    if (result.roots.length !== 1) {
      return false
    }

    const firstRoot = result.roots[0]
    if (firstRoot === undefined) {
      return false
    }

    registeredRoot.value = registeredRootFromRecord(firstRoot)
    rootChoiceStatus.value = 'registered'
    return true
  }

  function canUnregisterLocalRootId(rootId: string | undefined): rootId is string {
    return (
      rootId !== undefined &&
      removeSourceStatus.value !== 'removing' &&
      isKnownLocalRootId(rootId)
    )
  }

  async function unregisterLocalRoot(
    rootId: string | undefined = registeredRoot.value?.rootId
  ): Promise<boolean> {
    const rootIdToRemove = rootId

    if (!canUnregisterLocalRootId(rootIdToRemove)) {
      return false
    }

    removeSourceStatus.value = 'removing'
    removeFailureMessage.value = undefined

    try {
      const result = await rootApi.unregisterLocalRoot({ rootId: rootIdToRemove })

      if (result.state === 'unregistered') {
        removeSourceStatus.value = 'removed'
        const removedRegisteredRoot = clearRegisteredRoot(rootIdToRemove)
        if (removedRegisteredRoot || registeredRoot.value === undefined) {
          resetScanState()
        }
        removeReadLocalRoot(rootIdToRemove)
        rootChoiceStatus.value = registeredRoot.value === undefined ? 'idle' : 'registered'
        return true
      }

      removeSourceStatus.value = 'failed'
      removeFailureMessage.value = removeFailureFor(result)
      return false
    } catch {
      removeSourceStatus.value = 'failed'
      removeFailureMessage.value = safeRootRemoveFailure
      return false
    }
  }

  function clearRegisteredRoot(rootId: string): boolean {
    if (registeredRoot.value?.rootId === rootId) {
      registeredRoot.value = undefined
      return true
    }

    return false
  }

  function removeReadLocalRoot(rootId: string): void {
    const readState = localRootsReadState.value

    if (readState.kind !== 'ready') {
      localRootsReadState.value = { kind: 'unread' }
      return
    }

    localRootsReadState.value = {
      kind: 'ready',
      roots: readState.roots.filter((root) => root.rootId !== rootId)
    }
  }

  function isKnownLocalRootId(rootId: string | undefined): rootId is string {
    if (rootId === undefined) {
      return false
    }

    if (registeredRoot.value?.rootId === rootId) {
      return true
    }

    const readState = localRootsReadState.value

    return readState.kind === 'ready' && readState.roots.some((root) => root.rootId === rootId)
  }

  return {
    rootChoiceStatus,
    registeredRoot,
    registeredRootPath,
    rootChoiceButtonLabel,
    canChooseLocalRoot,
    scanStatus,
    scanSummary,
    scanButtonLabel,
    canRunRegisteredRootScan,
    localRootsReadState,
    removeSourceStatus,
    removeSourceButtonLabel,
    canUnregisterLocalRoot,
    isKnownLocalRootId,
    canUnregisterLocalRootId,
    chooseAndRegisterLocalRoot,
    runRegisteredRootScan,
    hydrateLocalRoots,
    unregisterLocalRoot
  }
}

function registeredRootFromRecord(record: LocalRoot): LocalRootRegistrationRoot {
  return {
    rootId: record.rootId,
    canonicalPath: record.canonicalPath
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

function rootChoiceFailureFor(
  state: Exclude<LocalRootChoiceResult['state'], 'canceled' | 'registered'>
): string {
  switch (state) {
    case 'hostUnavailable':
      return 'Library service is not ready. Try again when it has started.'
    case 'registrationFailed':
      return safeRootChoiceFailure
    case 'dialogFailed':
      return 'Unable to open folder picker.'
  }
}

function scanFailureFor(state: Exclude<LocalRootScanResult['state'], 'scanned'>): string {
  switch (state) {
    case 'hostUnavailable':
      return 'Library service is not ready. Try again when it has started.'
    case 'invalidRequest':
    case 'scanFailed':
      return safeRootScanFailure
  }
}

function scanSummaryFromResult(
  result: Extract<LocalRootScanResult, { state: 'scanned' }>
): LocalRootScanSummary {
  return {
    rootId: result.rootId,
    scanRunId: result.scanRunId,
    discoveredFileCount: result.discoveredFileCount,
    queuedSourceWorkItems: result.queuedSourceWorkItems
  }
}

function removeFailureFor(result: UnregisterLocalRootResult): string {
  if (result.state === 'unregistered') {
    return 'The source was not found or has already been removed.'
  }

  switch (result.state) {
    case 'hostUnavailable':
      return 'Library service is not ready. Try again when it has started.'
    case 'invalidRequest':
      return safeRootRemoveFailure
  }
}
