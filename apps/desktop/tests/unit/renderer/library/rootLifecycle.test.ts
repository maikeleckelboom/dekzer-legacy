import { describe, expect, it, vi } from 'vitest'

import {
  createLocalRootActionsController,
  type LibraryRootActionsApi,
  type LocalRootActionsController
} from '../../../../src/renderer/library/boundary/localRootActions'
import {
  createRootLifecycleController,
  type RootLifecycleController
} from '../../../../src/renderer/library/runtime/rootLifecycle'
import type { LocalRootChoiceResult } from '../../../../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../../../../src/shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../../../../src/shared/libraryRoots/runScan'

describe('local root scan lifecycle', () => {
  it('keeps scan unavailable before registration', async () => {
    const runScan = vi.fn(async () => startedRootResult())
    const controller = createLocalRootActionsController(testRootApi({ runScan }))

    expect(controller.canRunRegisteredRootScan.value).toBe(false)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(false)
    expect(runScan).not.toHaveBeenCalled()
    expect(controller.scanStatus.value).toBe('idle')
  })

  it('scans the root id returned by registration', async () => {
    const runScan = vi.fn(async () =>
      startedRootResult('scan-42')
    )
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({
            rootId: 'root-from-main',
            canonicalPath: 'C:/Music/root-from-main-is-not-parsed'
          }),
        runScan
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    expect(controller.registeredRootPath.value).toBe('C:/Music/root-from-main-is-not-parsed')

    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(runScan).toHaveBeenCalledWith({ rootId: 'root-from-main' })
    expect(controller.scanStatus.value).toBe('scanning')
  })

  it('refreshes after registration before scanning', async () => {
    const events: string[] = []
    const runScan = vi.fn(async (request) => {
      events.push('scan')
      expect(request).toEqual({ rootId: 'root-from-main' })
      return startedRootResult('scan-42')
    })
    const { rootActions, lifecycle } = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () => {
          events.push('choose')
          return registeredChoice({
            rootId: 'root-from-main',
            canonicalPath: 'C:/Music/root-from-main-is-not-parsed'
          })
        },
        runScan
      }),
      async () => {
        events.push('refresh')
        return true
      }
    )

    await expect(lifecycle.addMusicFolder()).resolves.toBe(true)
    expect(events).toEqual(['choose', 'refresh', 'scan'])
    expect(rootActions.scanStatus.value).toBe('scanning')
    expect(lifecycle.refreshStatus.value).toBe('idle')
    expect(rootActions.scanButtonLabel.value).toBe('Scanning folder')
  })

  it('does not let refresh failure overwrite scan success', async () => {
    const { rootActions, lifecycle } = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () => registeredChoice(),
        runScan: async () =>
          startedRootResult('scan-1')
      }),
      async () => false
    )

    await expect(lifecycle.addMusicFolder()).resolves.toBe(false)
    expect(rootActions.rootChoiceStatus.value).toBe('registered')
    expect(rootActions.scanStatus.value).toBe('scanning')
    expect(lifecycle.refreshStatus.value).toBe('idle')
  })

  it('prevents duplicate scan requests with explicit pending work', async () => {
    const pendingScan = deferred<LocalRootScanResult>()
    const runScan = vi.fn(async () => pendingScan.promise)
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () => registeredChoice(),
        runScan
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)

    const firstScan = controller.runRegisteredRootScan()
    expect(controller.scanStatus.value).toBe('scanning')
    expect(controller.canRunRegisteredRootScan.value).toBe(false)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(false)
    expect(runScan).toHaveBeenCalledTimes(1)

    pendingScan.resolve(startedRootResult())
    await expect(firstScan).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('scanning')
  })

  it('ignores stale scan completion after hydrated roots clear the active root', async () => {
    const pendingScan = deferred<LocalRootScanResult>()
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () => registeredChoice(),
        runScan: async () => pendingScan.promise,
        readLocalRoots: async (): Promise<ReadLocalRootsOutcome> => ({
          state: 'read',
          roots: []
        })
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    const scan = controller.runRegisteredRootScan()
    expect(controller.scanStatus.value).toBe('scanning')

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    expect(controller.registeredRoot.value).toBeUndefined()
    expect(controller.scanStatus.value).toBe('idle')

    pendingScan.resolve(startedRootResult())
    await expect(scan).resolves.toBe(false)
    expect(controller.scanStatus.value).toBe('idle')
    expect(controller.scanSummary.value).toBeUndefined()
  })

  it('preserves existing scan state when a replacement registration is canceled or fails', async () => {
    const secondChoice = deferred<LocalRootChoiceResult>()
    const choices: Array<LocalRootChoiceResult | Promise<LocalRootChoiceResult>> = [
      registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music/One' }),
      secondChoice.promise
    ]
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () => nextChoice(choices),
        runScan: async () =>
          startedRootResult('scan-1')
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('scanning')

    const pendingChoice = controller.chooseAndRegisterLocalRoot()
    expect(controller.rootChoiceStatus.value).toBe('registered')
    expect(controller.registeredRootPath.value).toBe('C:/Music/One')
    expect(controller.scanStatus.value).toBe('scanning')

    secondChoice.resolve({ state: 'canceled' })
    await expect(pendingChoice).resolves.toBe(false)
    expect(controller.rootChoiceStatus.value).toBe('registered')
    expect(controller.registeredRootPath.value).toBe('C:/Music/One')

    choices.push(failedChoice('registrationFailed'))
    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(false)
    expect(controller.rootChoiceStatus.value).toBe('registered')
    expect(controller.registeredRootPath.value).toBe('C:/Music/One')
    expect(controller.scanStatus.value).toBe('scanning')
  })

  it('replaces the root and resets scan state before scanning a second successful registration', async () => {
    const secondScan = deferred<LocalRootScanResult>()
    const secondScanStarted = deferred<void>()
    const choices: LocalRootChoiceResult[] = [
      registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music/One' }),
      registeredChoice({ rootId: 'root-2', canonicalPath: 'C:/Music/Two' })
    ]
    const scanRequests: unknown[] = []
    const { rootActions, lifecycle } = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () => nextChoice(choices),
        runScan: async (request) => {
          scanRequests.push(request)

          if (scanRequests.length === 2) {
            secondScanStarted.resolve()
            return secondScan.promise
          }

          return startedRootResult('scan-1')
        }
      }),
      async () => true
    )

    await expect(lifecycle.addMusicFolder()).resolves.toBe(true)

    rootActions.scanStatus.value = 'scanned'

    const pendingSecondAdd = lifecycle.addMusicFolder()
    await secondScanStarted.promise

    expect(rootActions.registeredRootPath.value).toBe('C:/Music/Two')
    expect(rootActions.scanSummary.value).toBeUndefined()
    expect(scanRequests[1]).toEqual({ rootId: 'root-2' })

    secondScan.resolve(
      startedRootResult('scan-2')
    )
    await expect(pendingSecondAdd).resolves.toBe(true)
  })

  it('hydrates local roots without silently picking multiple roots', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => ({
          state: 'read',
          roots: [
            { rootId: 'root-1', canonicalPath: 'C:/Music/One', availability: 'available' },
            { rootId: 'root-2', canonicalPath: 'C:/Music/Two', availability: 'available' }
          ]
        })
      })
    )

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    expect(controller.localRootsReadState.value).toMatchObject({ kind: 'ready' })
    expect(controller.registeredRoot.value).toBeUndefined()
    expect(controller.canRunRegisteredRootScan.value).toBe(false)
  })

  it('uses hydrated root availability to enable or disable scan', async () => {
    const available = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => ({
          state: 'read',
          roots: [{ rootId: 'root-7', canonicalPath: 'C:/Music', availability: 'available' }]
        })
      })
    )

    await expect(available.hydrateLocalRoots()).resolves.toBe(true)
    expect(available.registeredRoot.value?.rootId).toBe('root-7')
    expect(available.canRunRegisteredRootScan.value).toBe(true)

    const unavailable = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => ({
          state: 'read',
          roots: [{ rootId: 'root-8', canonicalPath: 'Z:/Missing', availability: 'unavailable' }]
        })
      })
    )

    await expect(unavailable.hydrateLocalRoots()).resolves.toBe(true)
    expect(unavailable.registeredRoot.value?.rootId).toBe('root-8')
    expect(unavailable.canRunRegisteredRootScan.value).toBe(false)
  })
})

describe('local root scan failure detail preservation', () => {
  it('preserves scan failure message and detail from the result error', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => ({
          state: 'scanFailed',
          error: {
            code: 'scanFailed',
            message: 'Unable to run local library root scan.',
            detail: 'durableStoreFailure: database is locked'
          }
        })
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('failed')
    expect(controller.scanFailureMessage.value).toBe('Unable to run local library root scan.')
    expect(controller.scanFailureDetail.value).toBe('durableStoreFailure: database is locked')
  })

  it('preserves scan failure message without detail', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => ({
          state: 'scanFailed',
          error: {
            code: 'scanFailed',
            message: 'Library root scan failed due to a transport error.'
          }
        })
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('failed')
    expect(controller.scanFailureMessage.value).toBe(
      'Library root scan failed due to a transport error.'
    )
    expect(controller.scanFailureDetail.value).toBeUndefined()
  })

  it('falls back to a safe message when the scan API throws', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => {
          throw new Error('unexpected renderer failure')
        }
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('failed')
    expect(controller.scanFailureMessage.value).toBe('Unable to scan folder.')
    expect(controller.scanFailureDetail.value).toBeUndefined()
  })

  it('uses a safe message for hostUnavailable and invalidRequest scan results', async () => {
    const hostUnavailableController = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => ({
          state: 'hostUnavailable',
          error: { code: 'hostFailed', message: 'Library boundary host is unavailable.' }
        })
      })
    )

    await expect(hostUnavailableController.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(hostUnavailableController.runRegisteredRootScan()).resolves.toBe(true)
    expect(hostUnavailableController.scanStatus.value).toBe('failed')
    expect(hostUnavailableController.scanFailureMessage.value).toBe(
      'Library service is not ready. Try again when it has started.'
    )
    expect(hostUnavailableController.scanFailureDetail.value).toBeUndefined()
  })

  it('resets scan failure message and detail when scan succeeds after failure', async () => {
    let scanCallCount = 0
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => {
          scanCallCount++
          if (scanCallCount === 1) {
            return {
              state: 'scanFailed',
              error: {
                code: 'scanFailed',
                message: 'Unable to run local library root scan.',
                detail: 'transport error'
              }
            }
          }
          return startedRootResult()
        }
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('failed')
    expect(controller.scanFailureMessage.value).toBe('Unable to run local library root scan.')
    expect(controller.scanFailureDetail.value).toBe('transport error')

    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('scanning')
    expect(controller.scanFailureMessage.value).toBeUndefined()
    expect(controller.scanFailureDetail.value).toBeUndefined()
  })
})

describe('local root remove lifecycle', () => {
  it('calls unregister with the selected backend root id', async () => {
    const unregisterLocalRoot = vi.fn(async () => ({
      state: 'unregistered' as const,
      unregistered: true
    }))
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-from-backend', canonicalPath: 'C:/Music' }),
        unregisterLocalRoot
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.unregisterLocalRoot()).resolves.toBe(true)
    expect(unregisterLocalRoot).toHaveBeenCalledWith({ rootId: 'root-from-backend' })
  })

  it('can unregister a selected hydrated root without picking the first root', async () => {
    const unregisterLocalRoot = vi.fn(async () => ({
      state: 'unregistered' as const,
      unregistered: true
    }))
    const controller = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => ({
          state: 'read',
          roots: [
            { rootId: 'root-1', canonicalPath: 'C:/Music/One', availability: 'available' },
            { rootId: 'root-2', canonicalPath: 'C:/Music/Two', availability: 'available' }
          ]
        }),
        unregisterLocalRoot
      })
    )

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    expect(controller.registeredRoot.value).toBeUndefined()
    expect(controller.canUnregisterLocalRootId('root-2')).toBe(true)

    await expect(controller.unregisterLocalRoot('root-2')).resolves.toBe(true)
    expect(unregisterLocalRoot).toHaveBeenCalledWith({ rootId: 'root-2' })
    expect(controller.removeSourceStatus.value).toBe('removing')

    controller.completeRemoveSource('root-2')
    expect(controller.removeSourceStatus.value).toBe('removed')
    const readState = controller.localRootsReadState.value
    expect(readState.kind).toBe('ready')
    if (readState.kind === 'ready') {
      expect(readState.roots.map((root) => root.rootId)).toEqual(['root-1'])
    }
  })

  it('clears registered root and scan state only after removal completes', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () =>
          startedRootResult('scan-1')
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('scanning')

    controller.scanStatus.value = 'scanned'

    await expect(controller.unregisterLocalRoot()).resolves.toBe(true)
    expect(controller.removeSourceStatus.value).toBe('removing')
    expect(controller.registeredRoot.value?.rootId).toBe('root-1')

    controller.completeRemoveSource('root-1')
    expect(controller.removeSourceStatus.value).toBe('removed')
    expect(controller.registeredRoot.value).toBeUndefined()
    expect(controller.scanStatus.value).toBe('idle')
    expect(controller.scanSummary.value).toBeUndefined()
    expect(controller.rootChoiceStatus.value).toBe('idle')
  })

  it('leaves state honest when unregister fails or reports already removed', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => startedRootResult(),
        unregisterLocalRoot: async () => ({
          state: 'hostUnavailable',
          error: { code: 'hostFailed', message: 'backend detail' }
        })
      })
    )

    await expect(controller.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(controller.runRegisteredRootScan()).resolves.toBe(true)
    expect(controller.scanStatus.value).toBe('scanning')

    controller.scanStatus.value = 'scanned'

    await expect(controller.unregisterLocalRoot()).resolves.toBe(false)
    expect(controller.removeSourceStatus.value).toBe('failed')
    expect(controller.registeredRootPath.value).toBe('C:/Music')
    expect(controller.scanStatus.value).toBe('scanned')

    const alreadyRemoved = createLocalRootActionsController(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        unregisterLocalRoot: async () => ({
          state: 'unregistered',
          unregistered: false
        })
      })
    )

    await expect(alreadyRemoved.chooseAndRegisterLocalRoot()).resolves.toBe(true)
    await expect(alreadyRemoved.unregisterLocalRoot()).resolves.toBe(false)
    expect(alreadyRemoved.removeSourceStatus.value).toBe('failed')
    expect(alreadyRemoved.registeredRootPath.value).toBe('C:/Music')
  })

  it('refreshes navigation after successful unregister and completes removal postcondition', async () => {
    const events: string[] = []
    const { rootActions, lifecycle } = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () => {
          events.push('choose')
          return registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' })
        },
        runScan: async () => startedRootResult(),
        unregisterLocalRoot: async () => {
          events.push('unregister')
          return { state: 'unregistered', unregistered: true }
        }
      }),
      async () => {
        events.push('refresh')
        return true
      }
    )

    await expect(lifecycle.addMusicFolder()).resolves.toBe(true)
    events.length = 0

    rootActions.scanStatus.value = 'scanned'

    await expect(lifecycle.removeSource()).resolves.toBe(true)
    expect(events).toEqual(['unregister', 'refresh'])
    expect(rootActions.registeredRoot.value).toBeUndefined()
    expect(rootActions.removeSourceStatus.value).toBe('removed')
    expect(lifecycle.refreshStatus.value).toBe('refreshed')
  })

  it('lets confirmation and refresh postconditions control removal', async () => {
    const unregisterLocalRoot = vi.fn(async () => ({
      state: 'unregistered' as const,
      unregistered: true
    }))
    const blocked = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-1', canonicalPath: 'C:/Music' }),
        runScan: async () => startedRootResult(),
        unregisterLocalRoot
      }),
      async () => true,
      () => false
    )

    await expect(blocked.lifecycle.addMusicFolder()).resolves.toBe(true)
    blocked.rootActions.scanStatus.value = 'scanned'
    await expect(blocked.lifecycle.removeSource()).resolves.toBe(false)
    expect(unregisterLocalRoot).not.toHaveBeenCalled()

    const visibleAfterRefresh = testRootLifecycle(
      testRootApi({
        chooseAndRegisterLocal: async () =>
          registeredChoice({ rootId: 'root-2', canonicalPath: 'C:/Music/Two' }),
        runScan: async () => startedRootResult(),
        unregisterLocalRoot: async () => ({ state: 'unregistered', unregistered: true })
      }),
      async () => true,
      () => true,
      () => true
    )

    await expect(visibleAfterRefresh.lifecycle.addMusicFolder()).resolves.toBe(true)
    visibleAfterRefresh.rootActions.scanStatus.value = 'scanned'
    await expect(visibleAfterRefresh.lifecycle.removeSource()).resolves.toBe(false)
    expect(visibleAfterRefresh.rootActions.registeredRoot.value?.rootId).toBe('root-2')
    expect(visibleAfterRefresh.rootActions.removeSourceStatus.value).toBe('failed')
    expect(visibleAfterRefresh.rootActions.removeSourceFailureMessage.value).toBe(
      'The source is still visible after refresh, so removal did not complete.'
    )
  })
})

function testRootApi(overrides: Partial<LibraryRootActionsApi> = {}): LibraryRootActionsApi {
  return {
    chooseAndRegisterLocal: async () => ({ state: 'canceled' }),
    runScan: async () => ({
      state: 'scanFailed',
      error: {
        code: 'scanFailed',
        message: 'Local root scan should not be called by this test.'
      }
    }),
    readLocalRoots: async () => ({
      state: 'hostFailed',
      error: {
        code: 'hostFailed',
        message: 'Local root read should not be called by this test.'
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
  confirmRemoveSource: () => boolean = () => true,
  isSourceRootVisible: (rootId: string) => boolean = () => false
): {
  readonly rootActions: LocalRootActionsController
  readonly lifecycle: RootLifecycleController
} {
  const rootActions = createLocalRootActionsController(rootApi)
  const lifecycle = createRootLifecycleController({
    rootActions,
    hierarchyRead: { refresh },
    confirmRemoveSource,
    isSourceRootVisible
  })

  return { rootActions, lifecycle }
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

function startedRootResult(
  scanRunId: string = 'scan-1'
): LocalRootScanResult {
  return {
    state: 'started',
    scanRunId
  }
}

function deferred<T>(): {
  readonly promise: Promise<T>
  readonly resolve: (value: T | PromiseLike<T>) => void
  readonly reject: (reason?: unknown) => void
} {
  let resolve: (value: T | PromiseLike<T>) => void = () => undefined
  let reject: (reason?: unknown) => void = () => undefined
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve
    reject = promiseReject
  })

  return { promise, resolve, reject }
}

function nextChoice(
  choices: Array<LocalRootChoiceResult | Promise<LocalRootChoiceResult>>
): LocalRootChoiceResult | Promise<LocalRootChoiceResult> {
  const choice = choices.shift()

  if (choice === undefined) {
    throw new Error('Unexpected local root choice.')
  }

  return choice
}
