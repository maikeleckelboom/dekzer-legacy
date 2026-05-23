import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { relative } from 'node:path'

import {
  createLocalRootActionsController,
  type LibraryRootActionsApi
} from '../src/renderer/libraryBrowser/boundary/localRootActions'
import {
  createRootLifecycleController,
  type RootLifecycleController
} from '../src/renderer/libraryBrowser/runtime/rootLifecycle'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import { desktopRoot, listSourceFiles, normalizePath, rendererSourceRoot } from './support/files'

const approvedUnregisterRendererOwners = new Set([
  'src/renderer/libraryBrowser/boundary/localRootActions.ts',
  'src/renderer/libraryBrowser/runtime/rootLifecycle.ts'
])

void main()

async function main(): Promise<void> {
  await validatesCanUnregisterFalseWithNoRoot()
  await validatesCanUnregisterTrueWithOneActiveRoot()
  await validatesUnregisterCallsRendererApiWithRootIdFromBackendState()
  await validatesUnregisterSuccessClearsRegisteredRootAndScanState()
  await validatesUnregisterFalseLeavesStateHonest()
  await validatesUnregisterFailureLeavesStateHonest()
  await validatesRootLifecycleRefreshesNavigationAfterSuccessfulUnregister()
  await validatesConfirmCallbackControlsRemoval()
  await validatesRefreshFailureAfterSuccessfulUnregisterIsHonest()
  validatesRendererRemoveSourceBoundaryOwnership()
  validatesNoDeleteFilesCopy()
}

async function validatesCanUnregisterFalseWithNoRoot(): Promise<void> {
  const controller = createLocalRootActionsController(testRootApi())

  assert.equal(controller.canUnregisterLocalRoot.value, false)
  assert.equal(await controller.unregisterLocalRoot(), false)
  assert.equal(controller.removeSourceStatus.value, 'idle')
}

async function validatesCanUnregisterTrueWithOneActiveRoot(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(controller.registeredRoot.value?.rootId, 'root-1')
  assert.equal(controller.canUnregisterLocalRoot.value, true)
}

async function validatesUnregisterCallsRendererApiWithRootIdFromBackendState(): Promise<void> {
  let receivedRequest: unknown = null
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-from-backend',
          canonicalPath: 'C:/Music'
        }),
      unregisterLocalRoot: async (request) => {
        receivedRequest = request
        return {
          state: 'unregistered',
          unregistered: true
        }
      }
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.unregisterLocalRoot(), true)
  assert.deepEqual(receivedRequest, {
    rootId: 'root-from-backend'
  })
}

async function validatesUnregisterSuccessClearsRegisteredRootAndScanState(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }),
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
  assert.equal(controller.scanSummary.value?.scanRunId, 'scan-1')
  assert.equal(controller.rootChoiceStatus.value, 'registered')

  assert.equal(await controller.unregisterLocalRoot(), true)
  assert.equal(controller.removeSourceStatus.value, 'removed')
  assert.equal(controller.registeredRoot.value, undefined)
  assert.equal(controller.registeredRootPath.value, undefined)
  assert.equal(controller.scanStatus.value, 'idle')
  assert.equal(controller.scanSummary.value, undefined)
  assert.equal(controller.rootChoiceStatus.value, 'idle')
  assert.equal(controller.localRootsReadState.value.kind, 'unread')
}

async function validatesUnregisterFalseLeavesStateHonest(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }),
      unregisterLocalRoot: async () => ({
        state: 'unregistered',
        unregistered: false
      })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(controller.registeredRootPath.value, 'C:/Music')

  assert.equal(await controller.unregisterLocalRoot(), false)
  assert.equal(controller.removeSourceStatus.value, 'failed')
  assert.equal(controller.registeredRoot.value?.rootId, 'root-1')
  assert.equal(controller.registeredRootPath.value, 'C:/Music')
}

async function validatesUnregisterFailureLeavesStateHonest(): Promise<void> {
  const controller = createLocalRootActionsController(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }),
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        }),
      unregisterLocalRoot: async () => ({
        state: 'hostUnavailable',
        error: {
          code: 'hostFailed',
          message: 'fixture backend details must not be displayed'
        }
      })
    })
  )

  assert.equal(await controller.chooseAndRegisterLocalRoot(), true)
  assert.equal(await controller.runRegisteredRootScan(), true)
  assert.equal(controller.scanStatus.value, 'scanned')

  assert.equal(await controller.unregisterLocalRoot(), false)
  assert.equal(controller.removeSourceStatus.value, 'failed')
  assert.equal(controller.registeredRoot.value?.rootId, 'root-1')
  assert.equal(controller.registeredRootPath.value, 'C:/Music')
  assert.equal(controller.scanStatus.value, 'scanned')
  assert.equal(controller.scanSummary.value?.scanRunId, 'scan-1')
}

async function validatesRootLifecycleRefreshesNavigationAfterSuccessfulUnregister(): Promise<void> {
  const events: string[] = []
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () => {
        events.push('choose')
        return registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        })
      },
      runScan: async () =>
        scannedRootResult({
          rootId: 'root-1',
          scanRunId: 'scan-1',
          discoveredFileCount: 12,
          queuedSourceWorkItems: 8
        }),
      unregisterLocalRoot: async () => {
        events.push('unregister')
        return {
          state: 'unregistered',
          unregistered: true
        }
      }
    }),
    async () => {
      events.push('refresh')
      return true
    },
    () => true
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  events.length = 0

  assert.equal(await lifecycle.removeSource(), true)
  assert.deepEqual(events, ['unregister', 'refresh'])
  assert.equal(rootActions.registeredRoot.value, undefined)
  assert.equal(rootActions.removeSourceStatus.value, 'removed')
  assert.equal(lifecycle.refreshStatus.value, 'refreshed')
}

async function validatesConfirmCallbackControlsRemoval(): Promise<void> {
  let unregisterAttempts = 0
  const { lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }),
      runScan: async () => scannedRootResult(),
      unregisterLocalRoot: async () => {
        unregisterAttempts += 1
        return {
          state: 'unregistered',
          unregistered: true
        }
      }
    }),
    async () => true,
    () => false
  )

  assert.equal(await lifecycle.addMusicFolder(), true)
  assert.equal(await lifecycle.removeSource(), false)
  assert.equal(unregisterAttempts, 0)
}

async function validatesRefreshFailureAfterSuccessfulUnregisterIsHonest(): Promise<void> {
  const { rootActions, lifecycle } = testRootLifecycle(
    testRootApi({
      chooseAndRegisterLocal: async () =>
        registeredChoice({
          rootId: 'root-1',
          canonicalPath: 'C:/Music'
        }),
      runScan: async () => scannedRootResult()
    }),
    async () => false,
    () => true
  )

  assert.equal(await lifecycle.addMusicFolder(), false)
  assert.equal(rootActions.scanStatus.value, 'scanned')
  assert.equal(lifecycle.refreshStatus.value, 'failed')
  assert.equal(rootActions.registeredRoot.value?.rootId, 'root-1')

  assert.equal(await lifecycle.removeSource(), false)
  assert.equal(rootActions.registeredRoot.value, undefined)
  assert.equal(rootActions.removeSourceStatus.value, 'removed')
  assert.equal(lifecycle.refreshStatus.value, 'failed')
}

function validatesRendererRemoveSourceBoundaryOwnership(): void {
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
    /\bdeleteLocalRoot\b/,
    /\bdeleteRoot\b/
  ]

  for (const filePath of listSourceFiles(rendererSourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    for (const pattern of forbiddenPatterns) {
      if (pattern.test(contents)) {
        violations.push(`${relativePath}: ${String(pattern)}`)
      }
    }

    if (
      /\.unregisterLocalRoot\(/.test(contents) &&
      !approvedUnregisterRendererOwners.has(relativePath)
    ) {
      violations.push(
        `${relativePath}: .unregisterLocalRoot is only allowed in ${[...approvedUnregisterRendererOwners].join(', ')}`
      )
    }
  }

  assert.deepEqual(violations, [])
}

function validatesNoDeleteFilesCopy(): void {
  const violations: string[] = []

  for (const filePath of listSourceFiles(rendererSourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    if (
      /(?:remove|unregister|unreg).*source/i.test(contents) &&
      /\bdelete.*files?\b|\bfiles?\b.*\bdelete\b/i.test(contents)
    ) {
      violations.push(
        `${relativePath}: remove/unregister source copy should not mention deleting files`
      )
    }

    if (
      /\bremove\b.*\bsource\b/i.test(contents) &&
      /will not\s+delete|does not\s+delete|won't\s+delete|do not\s+delete|don't\s+delete|never\s+delete/i.test(
        contents
      )
    ) {
      continue
    }
  }

  assert.equal(violations.length, 0)
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
    readLocalRoots: async () => ({
      state: 'hostFailed',
      error: {
        code: 'hostFailed',
        message: 'Local root read should not be called by this validation.'
      }
    }),
    unregisterLocalRoot: async () => ({
      state: 'unregistered',
      unregistered: true
    }),
    ...overrides
  }
}

function testRootLifecycle(
  rootApi: LibraryRootActionsApi,
  refresh: () => Promise<boolean> = async () => true,
  confirmRemoveSource: () => boolean = () => true
): {
  readonly rootActions: ReturnType<typeof createLocalRootActionsController>
  readonly lifecycle: RootLifecycleController
} {
  const rootActions = createLocalRootActionsController(rootApi)
  const lifecycle = createRootLifecycleController({
    rootActions,
    hierarchyRead: {
      refresh
    },
    confirmRemoveSource
  })

  return {
    rootActions,
    lifecycle
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
