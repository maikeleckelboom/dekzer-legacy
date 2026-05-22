import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { relative } from 'node:path'

import {
  createLocalRootActionsController,
  type LocalRootActionsController,
  type LibraryRootActionsApi
} from '../src/renderer/libraryBrowser/boundary/localRootActions'
import {
  createRootLifecycleController,
  type RootLifecycleController
} from '../src/renderer/libraryBrowser/runtime/rootLifecycle'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import { desktopRoot, listSourceFiles, normalizePath, rendererSourceRoot } from './support/files'
import { deferred } from './support/libraryBoundary'

const approvedRunScanRendererOwner = 'src/renderer/libraryBrowser/boundary/localRootActions.ts'

void main()

async function main(): Promise<void> {
  await validatesScanUnavailableBeforeRegistration()
  await validatesScanUsesRegisteredRootId()
  await validatesAddMusicFolderScansRegisteredRootAndRefreshesHierarchy()
  await validatesRescanRefreshesHierarchy()
  await validatesRefreshFailureDoesNotOverwriteScanSuccess()
  await validatesDuplicateScanRequestsArePrevented()
  await validatesScanFailureUsesSafeCopy()
  await validatesThrownScanErrorUsesSafeCopy()
  await validatesCanceledRegistrationDoesNotEnableScan()
  await validatesFailedRegistrationDoesNotEnableScan()
  await validatesCanceledSecondRegistrationPreservesRegisteredRootAndScanState()
  await validatesFailedSecondRegistrationPreservesRegisteredRootAndScanState()
  await validatesSuccessfulSecondRegistrationReplacesRootAndResetsScanState()
  await validatesComposedCanceledSecondChoicePreservesRegisteredRootAndScanState()
  await validatesComposedFailedSecondChoicePreservesRegisteredRootAndScanState()
  await validatesComposedSuccessfulSecondChoiceReplacesRootAndResetsScanStateBeforeScanning()
  validatesRendererScanBoundaryOwnership()
}

async function validatesScanUnavailableBeforeRegistration(): Promise<void> {
  let scanAttempts = 0
  const controller = createLocalRootActionsController(
    testRootApi({
      runScan: async () => {
        scanAttempts += 1
        return scannedRootResult()
      }
    })
  )

  assert.equal(controller.canRunRegisteredRootScan.value, false)
  assert.equal(await controller.runRegisteredRootScan(), false)
  assert.equal(scanAttempts, 0)
  assert.equal(controller.scanStatus.value, 'idle')
}

async function validatesScanUsesRegisteredRootId(): Promise<void> {
  let choiceAttempts = 0
  let receivedScanRequest: unknown = null
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => {
        choiceAttempts += 1
        return registeredChoice({
          rootId: 'root-from-main',
          canonicalPath: 'C:/Music/root-from-main-is-not-parsed'
        })
      },
      runScan: async (request) => {
        receivedScanRequest = request
        return scannedRootResult({
          rootId: 'root-from-main',
          scanRunId: 'scan-42',
          discoveredFileCount: 42,
          queuedSourceWorkItems: 8
        })
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(controller.registeredRootPath.value, 'C:/Music/root-from-main-is-not-parsed')
  assert.equal(controller.rootChoiceStatus.value, 'registered')
  assert.equal(controller.canRunRegisteredRootScan.value, true)

  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.deepEqual(receivedScanRequest, {
    rootId: 'root-from-main'
  })
  assert.deepEqual(controller.scanSummary.value, {
    rootId: 'root-from-main',
    scanRunId: 'scan-42',
    discoveredFileCount: 42,
    queuedSourceWorkItems: 8
  })
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.equal(choiceAttempts, 1)
}

async function validatesAddMusicFolderScansRegisteredRootAndRefreshesHierarchy(): Promise<void> {
  const events: string[] = []
  let receivedScanRequest: unknown = null
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => {
        events.push('choose')
        return registeredChoice({
          rootId: 'root-from-main',
          canonicalPath: 'C:/Music/root-from-main-is-not-parsed'
        })
      },
      runScan: async (request) => {
        events.push('scan')
        receivedScanRequest = request
        return scannedRootResult({
          rootId: 'root-from-main',
          scanRunId: 'scan-42',
          discoveredFileCount: 42,
          queuedSourceWorkItems: 8
        })
      }
    }),
    async () => {
      events.push('refresh')
      return true
    }
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.deepEqual(events, ['choose', 'scan', 'refresh'])
  assert.deepEqual(receivedScanRequest, {
    rootId: 'root-from-main'
  })
  assert.equal(rootActions.registeredRootPath.value, 'C:/Music/root-from-main-is-not-parsed')
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.equal(lifecycle.refreshStatus.value, 'refreshed')
  assert.equal(rootActions.scanButtonLabel.value, 'Rescan folder')
}

async function validatesRescanRefreshesHierarchy(): Promise<void> {
  const refreshes: string[] = []
  const { lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => registeredChoice(),
      runScan: async () => scannedRootResult()
    }),
    async () => {
      refreshes.push('refresh')
      return true
    }
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.equal(await lifecycle.scanRoot(), true)
  assert.deepEqual(refreshes, ['refresh', 'refresh'])
}

async function validatesRefreshFailureDoesNotOverwriteScanSuccess(): Promise<void> {
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => registeredChoice(),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    }),
    async () => false
  )

  assert.equal(await lifecycle.addMusicFolder(), false)
  assert.equal(rootActions.rootChoiceStatus.value, 'registered')
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.deepEqual(rootActions.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })
  assert.equal(lifecycle.refreshStatus.value, 'failed')
}

async function validatesDuplicateScanRequestsArePrevented(): Promise<void> {
  const pendingScan = deferred<LocalRootScanResult>()
  const scanRequests: unknown[] = []
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => registeredChoice(),
      runScan: async (request) => {
        scanRequests.push(request)
        return pendingScan.promise
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)

  const firstScan = controller.runRegisteredRootScan()
  assert.equal(controller.scanStatus.value, 'scanning')
  assert.equal(controller.canRunRegisteredRootScan.value, false)
  assert.equal(await controller.runRegisteredRootScan(), false)
  assert.equal(scanRequests.length, 1)

  pendingScan.resolve(scannedRootResult())
  assert.equal(await firstScan, true)
  assert.equal(controller.scanStatus.value, 'scanned')
}

async function validatesScanFailureUsesSafeCopy(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => registeredChoice(),
      runScan: async () => ({
        state: 'scanFailed',
        error: {
          code: 'scanFailed',
          message: 'fixture backend details must not be displayed'
        }
      })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.equal(controller.scanStatus.value, 'failed')
}

async function validatesThrownScanErrorUsesSafeCopy(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => registeredChoice(),
      runScan: async () => {
        throw new Error('fixture preload failure details')
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.equal(controller.scanStatus.value, 'failed')
}

async function validatesCanceledRegistrationDoesNotEnableScan(): Promise<void> {
  let scanAttempts = 0
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => ({
        state: 'canceled'
      }),
      runScan: async () => {
        scanAttempts += 1
        return scannedRootResult()
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), false)
  assert.equal(controller.rootChoiceStatus.value, 'canceled')
  assert.equal(controller.registeredRoot.value, undefined)
  assert.equal(controller.canRunRegisteredRootScan.value, false)
  assert.equal(await controller.runRegisteredRootScan(), false)
  assert.equal(scanAttempts, 0)
}

async function validatesFailedRegistrationDoesNotEnableScan(): Promise<void> {
  let scanAttempts = 0
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => failedChoice('dialogFailed'),
      runScan: async () => {
        scanAttempts += 1
        return scannedRootResult()
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), false)
  assert.equal(controller.rootChoiceStatus.value, 'failed')
  assert.equal(controller.registeredRoot.value, undefined)
  assert.equal(controller.canRunRegisteredRootScan.value, false)
  assert.equal(controller.scanStatus.value, 'idle')
  assert.equal(await controller.runRegisteredRootScan(), false)
  assert.equal(scanAttempts, 0)
}

async function validatesCanceledSecondRegistrationPreservesRegisteredRootAndScanState(): Promise<void> {
  const secondChoice = deferred<LocalRootChoiceResult>()
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    secondChoice.promise
  ]
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.deepEqual(controller.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })

  const pendingChoice = controller.chooseAndRegisterLocalRoot()
  assert.equal(controller.rootChoiceStatus.value, 'choosing')
  assert.equal(controller.registeredRootPath.value, 'C:/Music/One')
  assert.equal(controller.canRunRegisteredRootScan.value, true)
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.equal(controller.scanSummary.value?.scanRunId, 'scan-1')

  secondChoice.resolve({
    state: 'canceled'
  })
  assert.equal(await pendingChoice, false)
  assert.equal(controller.rootChoiceStatus.value, 'canceled')
  assert.equal(controller.registeredRootPath.value, 'C:/Music/One')
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.deepEqual(controller.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })
}

async function validatesFailedSecondRegistrationPreservesRegisteredRootAndScanState(): Promise<void> {
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    failedChoice('registrationFailed')
  ]
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)

  assert.equal(await controller.chooseAndRegisterLocalRoot(), false)
  assert.equal(controller.rootChoiceStatus.value, 'failed')
  assert.equal(controller.registeredRootPath.value, 'C:/Music/One')
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.deepEqual(controller.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })
}

async function validatesSuccessfulSecondRegistrationReplacesRootAndResetsScanState(): Promise<void> {
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    registeredChoice({
      rootId: 'root-2',
      canonicalPath: 'C:/Music/Two'
    })
  ]
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.equal(controller.scanSummary.value?.rootId, 'root-1')

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(controller.registeredRootPath.value, 'C:/Music/Two')
  assert.equal(controller.scanStatus.value, 'idle')
  assert.equal(controller.scanSummary.value, undefined)
}

async function validatesComposedCanceledSecondChoicePreservesRegisteredRootAndScanState(): Promise<void> {
  let refreshAttempts = 0
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    {
      state: 'canceled'
    } satisfies LocalRootChoiceResult
  ]
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    }),
    async () => {
      refreshAttempts += 1
      return true
    }
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.equal(rootActions.scanSummary.value?.scanRunId, 'scan-1')
  assert.equal(lifecycle.refreshStatus.value, 'refreshed')

  assert.equal(await lifecycle.addMusicFolder(), false)
  assert.equal(rootActions.rootChoiceStatus.value, 'canceled')
  assert.equal(rootActions.registeredRootPath.value, 'C:/Music/One')
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.deepEqual(rootActions.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })
  assert.equal(lifecycle.refreshStatus.value, 'refreshed')
  assert.equal(refreshAttempts, 1)
}

async function validatesComposedFailedSecondChoicePreservesRegisteredRootAndScanState(): Promise<void> {
  let refreshAttempts = 0
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    failedChoice('registrationFailed')
  ]
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
    }),
    async () => {
      refreshAttempts += 1
      return true
    }
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.equal(await lifecycle.addMusicFolder(), false)
  assert.equal(rootActions.rootChoiceStatus.value, 'failed')
  assert.equal(rootActions.registeredRootPath.value, 'C:/Music/One')
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.deepEqual(rootActions.scanSummary.value, {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  })
  assert.equal(refreshAttempts, 1)
}

async function validatesComposedSuccessfulSecondChoiceReplacesRootAndResetsScanStateBeforeScanning(): Promise<void> {
  const secondScan = deferred<LocalRootScanResult>()
  const choices = [
    registeredChoice({
      rootId: 'root-1',
      canonicalPath: 'C:/Music/One'
    }),
    registeredChoice({
      rootId: 'root-2',
      canonicalPath: 'C:/Music/Two'
    })
  ]
  const scanRequests: unknown[] = []
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => choices.shift() ?? assert.fail('unexpected choice'),
      runScan: async (request) => {
        scanRequests.push(request)

        if (scanRequests.length === 2) {
          return secondScan.promise
        }

        return scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        })
      }
    }),
    async () => true
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.equal(rootActions.scanSummary.value?.rootId, 'root-1')

  const pendingSecondAdd = lifecycle.addMusicFolder()
  await flushPromises()

  assert.equal(rootActions.registeredRootPath.value, 'C:/Music/Two')
  assert.equal(rootActions.scanStatus.value, 'scanning')
  assert.equal(rootActions.scanSummary.value, undefined)
  assert.deepEqual(scanRequests[1], {
    rootId: 'root-2'
  })

  secondScan.resolve(
    scannedRootResult({
      rootId: 'root-2',
      scanRunId: 'scan-2',
      discoveredFileCount: 4,
      queuedSourceWorkItems: 2
    })
  )
  assert.equal(await pendingSecondAdd, true)
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.deepEqual(rootActions.scanSummary.value, {
    rootId: 'root-2',
    scanRunId: 'scan-2',
    discoveredFileCount: 4,
    queuedSourceWorkItems: 2
  })
}

function validatesRendererScanBoundaryOwnership(): void {
  const violations: string[] = []
  const forbiddenPatterns = [
    /@dekzer\/library-boundary-client/,
    /@dekzer\/library-boundary-stdio-transport/,
    /\bLibraryBoundaryClient\b/,
    /\bipcRenderer\b/,
    /from ['"]electron['"]/,
    /from ['"]node:fs['"]/,
    /from ['"]fs['"]/,
    /from ['"]node:path['"]/,
    /from ['"]path['"]/,
    /from ['"].*\/main\//,
    /\bshowOpenDialog\b/,
    /\brunRootScan\b/
  ]
  for (const filePath of listSourceFiles(rendererSourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    for (const pattern of forbiddenPatterns) {
      if (pattern.test(contents)) {
        violations.push(`${relativePath}: ${String(pattern)}`)
      }
    }

    if (/\.runScan\(/.test(contents) && relativePath !== approvedRunScanRendererOwner) {
      violations.push(
        `${relativePath}: .runScan is only allowed in ${approvedRunScanRendererOwner}`
      )
    }
  }

  assert.deepEqual(violations, [])
}

function testRootApi(overrides: Partial<LibraryRootActionsApi> = {}): LibraryRootActionsApi {
  return {
    chooseAndRegisterLocal: async () => ({
      state: 'canceled'
    }),
    runScan: async () => ({
      state: 'scanFailed',
      error: {
        code: 'scanFailed',
        message: 'Local root scan should not be called by this validation.'
      }
    }),
    ...overrides
  }
}

function testRootLifecycle(
  rootApi: LibraryRootActionsApi,
  refresh: () => Promise<boolean> = async () => true
): {
  readonly rootActions: LocalRootActionsController
  readonly lifecycle: RootLifecycleController
} {
  const rootActions = createLocalRootActionsController(rootApi)
  const lifecycle = createRootLifecycleController({
    rootActions,
    hierarchyRead: {
      refresh
    }
  })

  return {
    rootActions,
    lifecycle
  }
}

async function flushPromises(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

function registeredChoice(
  root: {
    readonly rootId: string
    readonly canonicalPath: string
  } = {
    rootId: 'root-1',
    canonicalPath: 'C:/Music'
  }
): LocalRootChoiceResult {
  return {
    state: 'registered',
    root
  }
}

function failedChoice(
  state: Exclude<LocalRootChoiceResult['state'], 'canceled' | 'registered'>
): LocalRootChoiceResult {
  return {
    state,
    error: {
      code: state === 'dialogFailed' ? 'dialogFailed' : 'registrationFailed',
      message: 'fixture backend details must not be displayed'
    }
  }
}

function scannedRootResult(
  result: {
    readonly rootId: string
    readonly scanRunId: string
    readonly discoveredFileCount: number
    readonly queuedSourceWorkItems: number
  } = {
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  }
): LocalRootScanResult {
  return {
    state: 'scanned',
    ...result
  }
}
