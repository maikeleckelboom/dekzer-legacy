import { describe, expect, it } from 'vitest'

import {
  deriveOperationFeedback,
  type OperationFeedbackInputs
} from '../../../../src/renderer/library/operationFeedback'
import type { LibraryBoundaryHostStatus } from '../../../../src/shared/libraryBoundary/status'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/libraryNavigation/readRows'

describe('deriveOperationFeedback', () => {
  it('reports host startup and unavailable states before library state', () => {
    const base = noRootInputs()

    expect(deriveOperationFeedback({ ...base, hostStatus: undefined })).toMatchObject({
      kind: 'checkingHost',
      tone: 'loading'
    })
    expect(
      deriveOperationFeedback({ ...base, hostStatus: hostWithState('starting') })
    ).toMatchObject({
      kind: 'checkingHost',
      tone: 'loading'
    })
    expect(deriveOperationFeedback({ ...base, hostStatus: hostWithState('idle') })).toMatchObject({
      kind: 'hostUnavailable',
      tone: 'warning'
    })

    const failed = deriveOperationFeedback({
      ...base,
      hostStatus: failedHost('host failure reason'),
      navigationReadRequestError: 'ignored error'
    })
    expect(failed).toMatchObject({ kind: 'hostUnavailable', tone: 'error' })
    expect(failed.detail).toMatch(/host failure reason/)
  })

  it('derives no-root states from the root choice state', () => {
    const base = startedInputs({ registeredRootPath: undefined })

    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'idle' })).toMatchObject({
      kind: 'chooseRoot',
      tone: 'idle'
    })
    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' })).toMatchObject({
      kind: 'choosingRoot',
      tone: 'loading'
    })
    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' })).toMatchObject({
      kind: 'rootChoiceFailed',
      tone: 'error'
    })
    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'canceled' })).toMatchObject({
      kind: 'rootChoiceCanceled',
      tone: 'warning'
    })
  })

  it('prioritizes active operations over idle and read states', () => {
    const base = startedInputs({})

    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' }).kind).toBe(
      'choosingRoot'
    )
    expect(deriveOperationFeedback({ ...base, scanStatus: 'scanning' }).kind).toBe('scanningRoot')
    expect(deriveOperationFeedback({ ...base, removeSourceStatus: 'removing' }).kind).toBe(
      'removingSource'
    )
    expect(deriveOperationFeedback({ ...base, refreshStatus: 'refreshing' }).kind).toBe(
      'refreshingView'
    )
    expect(deriveOperationFeedback({ ...base, navigationReadIsLoading: true }).kind).toBe(
      'navigationLoading'
    )
    expect(deriveOperationFeedback({ ...base, hierarchyReadIsLoading: true }).kind).toBe(
      'hierarchyLoading'
    )
    expect(
      deriveOperationFeedback({
        ...base,
        scanStatus: 'scanning',
        navigationReadIsLoading: true,
        hierarchyReadIsLoading: true
      }).kind
    ).toBe('scanningRoot')
    expect(
      deriveOperationFeedback({
        ...base,
        refreshStatus: 'refreshing',
        navigationReadIsLoading: true
      }).kind
    ).toBe('refreshingView')
  })

  it('derives failure feedback with safe details', () => {
    const base = startedInputs({})

    expect(deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' })).toMatchObject({
      kind: 'rootChoiceFailed',
      tone: 'error'
    })
    expect(deriveOperationFeedback({ ...base, scanStatus: 'failed' })).toMatchObject({
      kind: 'scanFailed',
      tone: 'error'
    })
    expect(
      deriveOperationFeedback({
        ...base,
        scanStatus: 'failed',
        scanFailureMessage: 'Unable to run local library root scan.',
        scanFailureDetail: 'fixture scan failure'
      })
    ).toMatchObject({
      kind: 'scanFailed',
      tone: 'error',
      detail: 'Unable to run local library root scan. fixture scan failure'
    })
    expect(
      deriveOperationFeedback({
        ...base,
        scanStatus: 'failed',
        scanFailureMessage: 'Unable to run local library root scan.'
      })
    ).toMatchObject({
      kind: 'scanFailed',
      tone: 'error',
      detail: 'Unable to run local library root scan.'
    })
    expect(
      deriveOperationFeedback({
        ...base,
        scanStatus: 'failed',
        scanFailureDetail: 'database is locked'
      })
    ).toMatchObject({
      kind: 'scanFailed',
      tone: 'error'
    })
    expect(deriveOperationFeedback({ ...base, refreshStatus: 'failed' })).toMatchObject({
      kind: 'refreshFailed',
      tone: 'warning'
    })
    expect(
      deriveOperationFeedback({
        ...base,
        removeSourceStatus: 'failed',
        removeSourceFailureMessage: 'The source is still visible after refresh.'
      })
    ).toMatchObject({
      kind: 'removeFailed',
      tone: 'error',
      detail: 'The source is still visible after refresh.'
    })
    expect(
      deriveOperationFeedback({
        ...base,
        navigationReadRequestError: 'nav wins',
        hierarchyReadRequestError: 'hierarchy loses'
      })
    ).toMatchObject({ kind: 'navigationFailed', tone: 'error', detail: 'nav wins' })
    expect(
      deriveOperationFeedback({
        ...base,
        hierarchyReadRequestError: 'Safe hierarchy error text.'
      })
    ).toMatchObject({
      kind: 'hierarchyFailed',
      tone: 'error',
      detail: 'Safe hierarchy error text.'
    })
  })

  it('derives success and terminal states without fake progress', () => {
    const base = startedInputs({})

    const scanning = deriveOperationFeedback({ ...base, scanStatus: 'scanning' })
    expect(scanning.kind).toBe('scanningRoot')
    expect(scanning.title).not.toMatch(/%|file\s+\d+|\d+\s+files?/i)
    expect(scanning.detail).not.toMatch(/%|file\s+\d+|\d+\s+files?/i)

    const scanned = deriveOperationFeedback({ ...base, scanStatus: 'scanned' })
    expect(scanned).toMatchObject({ kind: 'scanComplete', tone: 'success' })
    expect(scanned.title).not.toMatch(/%|\d+\s+files?/i)
    expect(scanned.detail).not.toMatch(/%/)

    const scanWithSummary = deriveOperationFeedback(
      startedInputs({
        scanStatus: 'scanned',
        scanSummary: {
          rootId: 'r1',
          scanRunId: 's1',
          discoveredFileCount: 42,
          queuedSourceWorkItems: 3
        }
      })
    )
    expect(scanWithSummary.detail).toMatch(/42 files/)
    expect(scanWithSummary.detail).toMatch(/3 items/)

    expect(
      deriveOperationFeedback({
        ...base,
        navigationReadResult: readyNavigation([makeNavRow()])
      })
    ).toMatchObject({ kind: 'ready', tone: 'success' })
    expect(
      deriveOperationFeedback({
        ...base,
        navigationReadResult: readyNavigation([])
      })
    ).toMatchObject({ kind: 'noSources', tone: 'warning' })
    expect(
      deriveOperationFeedback({
        ...base,
        scanStatus: 'scanned',
        navigationReadResult: readyNavigation([makeNavRow()])
      }).kind
    ).toBe('scanComplete')
  })

  it('allows persisted navigation to satisfy readiness without a session root', () => {
    const base = startedInputs({
      registeredRootPath: undefined,
      navigationReadResult: readyNavigation([makeNavRow()])
    })

    expect(deriveOperationFeedback(base)).toMatchObject({ kind: 'ready', tone: 'success' })
    expect(
      deriveOperationFeedback(
        startedInputs({
          registeredRootPath: undefined,
          navigationReadResult: readyNavigation([])
        })
      )
    ).toMatchObject({ kind: 'noSources', tone: 'warning' })
    expect(
      deriveOperationFeedback(noRootInputs({ navigationReadResult: undefined }))
    ).toMatchObject({
      kind: 'chooseRoot',
      tone: 'idle'
    })
    expect(
      deriveOperationFeedback({ ...base, hostStatus: failedHost('host failure reason') })
    ).toMatchObject({
      kind: 'hostUnavailable',
      tone: 'error'
    })
  })
})

function startedInputs(overrides: Partial<OperationFeedbackInputs> = {}): OperationFeedbackInputs {
  const base: OperationFeedbackInputs = {
    hostStatus: hostWithState('started'),
    rootChoiceStatus: 'idle',
    registeredRootPath: '/Music',
    scanStatus: 'idle',
    scanSummary: undefined,
    scanProgressFromEvents: undefined,
    refreshStatus: 'idle',
    navigationReadIsLoading: false,
    hierarchyReadIsLoading: false,
    navigationReadRequestError: undefined,
    hierarchyReadRequestError: undefined,
    navigationReadResult: undefined,
    removeSourceStatus: 'idle'
  }
  return { ...base, ...overrides }
}

function noRootInputs(overrides: Partial<OperationFeedbackInputs> = {}): OperationFeedbackInputs {
  return {
    ...startedInputs(overrides),
    registeredRootPath: undefined
  }
}

function hostWithState(state: LibraryBoundaryHostStatus['state']): LibraryBoundaryHostStatus {
  return {
    state,
    environment: 'development',
    binaryPolicy: { kind: 'developmentBinary', source: 'environmentOverride' },
    lastError: null
  }
}

function failedHost(message: string): LibraryBoundaryHostStatus {
  return {
    ...hostWithState('failed'),
    lastError: { code: 'unknown', message }
  }
}

function makeNavRow(): NavigationRow {
  return {
    navigationRowId: '1',
    stableKey: 'source:1',
    parentNavigationRowId: null,
    family: 'sources',
    rowKind: 'source',
    displayName: 'Source',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '1',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function readyNavigation(rows: readonly NavigationRow[]): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows
  }
}
