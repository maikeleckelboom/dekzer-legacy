import { describe, expect, it, vi } from 'vitest'

import {
  createLocalRootActionsController,
  type LibraryRootActionsApi
} from '../../../../src/renderer/library/boundary/localRootActions'
import type { ReadLocalRootsOutcome } from '../../../../src/shared/library/roots/read'
import type { LocalRootScanResult } from '../../../../src/shared/library/roots/scan'

describe('local root action controller', () => {
  it('keeps scan status source-scoped while another root is scanning', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => readyRoots(['root-1', 'root-2']),
        runScan: async () => startedRootResult('scan-1')
      })
    )

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    await expect(controller.runRootScan('root-1')).resolves.toBe(true)

    expect(controller.scanStatusForRoot('root-1')).toBe('scanning')
    expect(controller.scanStatusForRoot('root-2')).toBe('idle')
  })

  it('keeps root action applicability about source eligibility, not scan execution availability', async () => {
    const controller = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => readyRoots(['root-1', 'root-2']),
        runScan: async () => startedRootResult('scan-1')
      })
    )

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    await expect(controller.runRootScan('root-1')).resolves.toBe(true)

    expect(controller.canRunLocalRootScan('root-2')).toBe(true)
    expect(controller.canUnregisterLocalRootId('root-2')).toBe(true)
  })

  it('keeps scan execution single-flight under the V0 global scan lock', async () => {
    const runScan = vi.fn(async () => startedRootResult('scan-1'))
    const controller = createLocalRootActionsController(
      testRootApi({
        readLocalRoots: async () => readyRoots(['root-1', 'root-2']),
        runScan
      })
    )

    await expect(controller.hydrateLocalRoots()).resolves.toBe(false)
    await expect(controller.runRootScan('root-1')).resolves.toBe(true)
    await expect(controller.runRootScan('root-2')).resolves.toBe(false)

    expect(runScan).toHaveBeenCalledTimes(1)
    expect(controller.activeScanRootId.value).toBe('root-1')
  })
})

function testRootApi(overrides: Partial<LibraryRootActionsApi> = {}): LibraryRootActionsApi {
  return {
    chooseAndRegisterLocal: async () => ({ state: 'canceled' }),
    registerLocalPath: async () => ({
      state: 'registrationFailed',
      error: {
        code: 'registrationFailed',
        message: 'Local root registration should not be called by this test.'
      }
    }),
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
    cancelScan: async () => ({
      state: 'cancelFailed',
      error: {
        code: 'cancelFailed',
        message: 'Local root cancel should not be called by this test.'
      }
    }),
    unregisterLocalRoot: async () => ({
      state: 'unregistered',
      unregistered: true
    }),
    ...overrides
  }
}

function readyRoots(rootIds: readonly string[]): ReadLocalRootsOutcome {
  return {
    state: 'read',
    roots: rootIds.map((rootId) => ({
      rootId,
      admittedRootPath: `C:/Music/${rootId}`,
      availability: 'available'
    }))
  }
}

function startedRootResult(scanRunId: string): LocalRootScanResult {
  return {
    state: 'started',
    scanRunId
  }
}
