import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'

import {
  createLocalRootActionsController,
  type LibraryRootActionsApi
} from '../src/renderer/libraryBrowser/localRootActions'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import { deferred } from './support/libraryBoundary'

void main()

async function main(): Promise<void> {
  await validatesScanUnavailableBeforeRegistration()
  await validatesScanUsesRegisteredRootId()
  await validatesDuplicateScanRequestsArePrevented()
  await validatesScanFailureUsesSafeCopy()
  await validatesThrownScanErrorUsesSafeCopy()
  await validatesCanceledRegistrationDoesNotEnableScan()
  await validatesNewRegistrationResetsScanState()
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
  assert.equal(controller.scanFeedback.value, undefined)
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
  assert.match(controller.rootChoiceFeedback.value, /Scan is not started yet/)
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
  assert.match(controller.scanFeedback.value ?? '', /Scan complete\. Refresh is not wired yet\./)
  assert.match(controller.scanFeedback.value ?? '', /42 files discovered/)
  assert.match(controller.scanFeedback.value ?? '', /8 source work items queued/)
  assert.equal(choiceAttempts, 1)
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
  assert.equal(controller.scanFeedback.value, 'Unable to scan folder.')
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
  assert.equal(controller.scanFeedback.value, 'Unable to scan folder.')
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

async function validatesNewRegistrationResetsScanState(): Promise<void> {
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
  assert.match(controller.rootChoiceFeedback.value, /Scan is not started yet/)
}

function validatesRendererScanBoundaryOwnership(): void {
  const actionSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/localRootActions.ts', import.meta.url),
    'utf8'
  )
  const panelSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/panel.vue', import.meta.url),
    'utf8'
  )

  assert.match(actionSource, /rootApi\.runScan\(\{\s*rootId: root\.rootId\s*\}\)/s)
  assert.doesNotMatch(actionSource, /\bBrowserTreeNodeId\b/)
  assert.doesNotMatch(actionSource, /source-directory:/)
  assert.doesNotMatch(actionSource, /source:/)
  assert.doesNotMatch(actionSource, /fixture-/)
  assert.doesNotMatch(actionSource, /\.split\(/)
  assert.doesNotMatch(actionSource, /\bdocument\./)
  assert.doesNotMatch(actionSource, /\bdataset\b/)
  assert.doesNotMatch(actionSource, /\bquerySelector\b/)
  assert.doesNotMatch(actionSource, /\breadChildren\b/)
  assert.doesNotMatch(actionSource, /\bhierarchy\b/)
  assert.doesNotMatch(actionSource, /\bregisterLocal\b/)
  assert.doesNotMatch(panelSource, /\.runScan\(/)
  assert.doesNotMatch(panelSource, /\brootId\b/)
  assert.match(panelSource, /v-if="registeredRootPath !== undefined"/)
  assert.match(panelSource, /@click="runRegisteredRootScan"/)
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
