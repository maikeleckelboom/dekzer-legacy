import { computed, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'

import type { LocalRootChoiceResult } from '../../../shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootRegistrationRoot } from '../../../shared/libraryRoots/registerLocalRoot'
import type { ReadLocalRootsOutcome, LocalRoot } from '../../../shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../../../shared/libraryRoots/runScan'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibraryRootActionsApi = RendererApi['library']['roots']

export type LocalRootChoiceStatus = 'idle' | 'choosing' | 'canceled' | 'registered' | 'failed'

export type LocalRootScanStatus = 'idle' | 'scanning' | 'scanned' | 'failed'

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
  readonly chooseAndRegisterLocalRoot: () => Promise<boolean>
  readonly runRegisteredRootScan: () => Promise<boolean>
  readonly hydrateLocalRoots: () => Promise<boolean>
}

const safeRootChoiceFailure = 'Unable to add music folder.'
const safeRootScanFailure = 'Unable to scan folder.'

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

  const registeredRootPath = computed(() => registeredRoot.value?.canonicalPath)
  const canChooseLocalRoot = computed(
    () => rootChoiceStatus.value !== 'choosing' && scanStatus.value !== 'scanning'
  )
  const rootChoiceButtonLabel = computed(() =>
    rootChoiceStatus.value === 'choosing' ? 'Adding folder' : 'Add music folder'
  )
  const canRunRegisteredRootScan = computed(
    () => registeredRoot.value !== undefined && scanStatus.value !== 'scanning'
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
      return false
    }

    if (result.state !== 'read') {
      return false
    }

    if (result.roots.length === 0) {
      return false
    }

    if (result.roots.length > 1) {
      rootChoiceStatus.value = 'failed'
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
    chooseAndRegisterLocalRoot,
    runRegisteredRootScan,
    hydrateLocalRoots
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
