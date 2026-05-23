import { strict as assert } from 'node:assert'

import {
  deriveOperationFeedback,
  type LibraryOperationFeedback,
  type LibraryOperationFeedbackKind,
  type OperationFeedbackInputs
} from '../src/renderer/libraryBrowser/projection/operationFeedback'
import type { LibraryBoundaryHostStatus } from '../src/shared/libraryBoundary/status'
import type { NavigationRow } from '../src/shared/libraryNavigation/readRows'

void main()

function main(): void {
  validatesHostNotReady()
  validatesNoRoot()
  validatesActiveOperationsBeatIdle()
  validatesErrorStates()
  validatesSuccessAndTerminal()
  validatesNoFakeProgress()
  validatesErrorDetailPassthrough()
  validatesPersistedNavigationWithoutSessionRoot()
}

function validatesHostNotReady(): void {
  const base = noRootInputs()

  assertKindAndTone(
    deriveOperationFeedback({ ...base, hostStatus: undefined }),
    'checkingHost',
    'loading'
  )

  assertKindAndTone(
    deriveOperationFeedback({ ...base, hostStatus: hostWithState('starting') }),
    'checkingHost',
    'loading'
  )

  assertKindAndTone(
    deriveOperationFeedback({ ...base, hostStatus: hostWithState('idle') }),
    'hostUnavailable',
    'warning'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, hostStatus: hostWithState('stopping') }),
    'hostUnavailable',
    'warning'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, hostStatus: hostWithState('stopped') }),
    'hostUnavailable',
    'warning'
  )

  const failed = deriveOperationFeedback({ ...base, hostStatus: failedHost('host failure reason') })
  assertKindAndTone(failed, 'hostUnavailable', 'error')
  assert.match(failed.detail ?? '', /host failure reason/)

  const failedNavIgnored = deriveOperationFeedback({
    ...base,
    hostStatus: failedHost('host failure reason'),
    navigationReadRequestError: 'ignored error'
  })
  assert.equal(failedNavIgnored.kind, 'hostUnavailable')
}

function validatesNoRoot(): void {
  const base = startedInputs({ registeredRootPath: undefined })

  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'idle' }),
    'chooseRoot',
    'idle'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' }),
    'choosingRoot',
    'loading'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' }),
    'rootChoiceFailed',
    'error'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'canceled' }),
    'rootChoiceCanceled',
    'warning'
  )
}

function validatesActiveOperationsBeatIdle(): void {
  const base = startedInputs({})

  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' }),
    'choosingRoot',
    'loading'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, scanStatus: 'scanning' }),
    'scanningRoot',
    'loading'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, refreshStatus: 'refreshing' }),
    'refreshingView',
    'loading'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, navigationReadIsLoading: true }),
    'navigationLoading',
    'loading'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, hierarchyReadIsLoading: true }),
    'hierarchyLoading',
    'loading'
  )

  assert.equal(
    deriveOperationFeedback({
      ...base,
      scanStatus: 'scanning',
      navigationReadIsLoading: true,
      hierarchyReadIsLoading: true
    }).kind,
    'scanningRoot'
  )

  assert.equal(
    deriveOperationFeedback({
      ...base,
      refreshStatus: 'refreshing',
      navigationReadIsLoading: true
    }).kind,
    'refreshingView'
  )
}

function validatesErrorStates(): void {
  const base = startedInputs({})

  assertKindAndTone(
    deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' }),
    'rootChoiceFailed',
    'error'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, scanStatus: 'failed' }),
    'scanFailed',
    'error'
  )
  assertKindAndTone(
    deriveOperationFeedback({ ...base, refreshStatus: 'failed' }),
    'refreshFailed',
    'warning'
  )

  assertKindAndTone(
    deriveOperationFeedback({ ...base, navigationReadRequestError: 'nav error detail' }),
    'navigationFailed',
    'error'
  )

  assertKindAndTone(
    deriveOperationFeedback({ ...base, hierarchyReadRequestError: 'hierarchy error detail' }),
    'hierarchyFailed',
    'error'
  )

  const navPri = deriveOperationFeedback({
    ...base,
    navigationReadRequestError: 'nav wins',
    hierarchyReadRequestError: 'hierarchy loses'
  })
  assertKindAndTone(navPri, 'navigationFailed', 'error')
}

function validatesSuccessAndTerminal(): void {
  const base = startedInputs({})

  const scanned = deriveOperationFeedback({ ...base, scanStatus: 'scanned' })
  assertKindAndTone(scanned, 'scanComplete', 'success')

  const readyNav = deriveOperationFeedback({
    ...base,
    navigationReadResult: readyNavigation([makeNavRow()])
  })
  assertKindAndTone(readyNav, 'ready', 'success')

  const noSources = deriveOperationFeedback({
    ...base,
    navigationReadResult: readyNavigation([])
  })
  assertKindAndTone(noSources, 'noSources', 'warning')

  assert.equal(
    deriveOperationFeedback({
      ...base,
      scanStatus: 'scanned',
      navigationReadResult: readyNavigation([makeNavRow()])
    }).kind,
    'ready'
  )

  assert.equal(
    deriveOperationFeedback({
      ...base,
      navigationReadResult: readyNavigation([])
    }).kind,
    'noSources'
  )
}

function validatesNoFakeProgress(): void {
  const scanning = deriveOperationFeedback(startedInputs({ scanStatus: 'scanning' }))

  assert.equal(scanning.kind, 'scanningRoot')
  assert.doesNotMatch(scanning.title, /%/g)
  assert.doesNotMatch(scanning.detail ?? '', /%/g)
  assert.doesNotMatch(scanning.title, /file\s+\d+/i)
  assert.doesNotMatch(scanning.detail ?? '', /file\s+\d+/i)
  assert.doesNotMatch(scanning.title, /\d+\s+files?/i)

  const scanComplete = deriveOperationFeedback(startedInputs({ scanStatus: 'scanned' }))
  assert.equal(scanComplete.kind, 'scanComplete')
  assert.doesNotMatch(scanComplete.title, /%/g)
  assert.doesNotMatch(scanComplete.title, /\d+\s+files?/i)
  assert.doesNotMatch(scanComplete.detail ?? '', /%/g)

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
  assert.match(scanWithSummary.detail ?? '', /42 files/)
  assert.match(scanWithSummary.detail ?? '', /3 items/)

  const refreshFeedback = deriveOperationFeedback(startedInputs({ refreshStatus: 'refreshing' }))
  assert.doesNotMatch(refreshFeedback.title, /%/g)
  assert.doesNotMatch(refreshFeedback.title, /file\s+\d+/i)
}

function validatesErrorDetailPassthrough(): void {
  const navErr = deriveOperationFeedback(
    startedInputs({ navigationReadRequestError: 'Safe nav error text.' })
  )
  assert.equal(navErr.kind, 'navigationFailed')
  assert.equal(navErr.tone, 'error')
  assert.match(navErr.detail ?? '', /Safe nav error text/)

  const hierarchyErr = deriveOperationFeedback(
    startedInputs({ hierarchyReadRequestError: 'Safe hierarchy error text.' })
  )
  assert.equal(hierarchyErr.kind, 'hierarchyFailed')
  assert.match(hierarchyErr.detail ?? '', /Safe hierarchy error text/)
}

function validatesPersistedNavigationWithoutSessionRoot(): void {
  const base = startedInputs({
    registeredRootPath: undefined,
    navigationReadResult: readyNavigation([makeNavRow()])
  })

  const readyFromPersistence = deriveOperationFeedback(base)
  assertKindAndTone(readyFromPersistence, 'ready', 'success')

  const noSourcesFromPersistence = deriveOperationFeedback(
    startedInputs({
      registeredRootPath: undefined,
      navigationReadResult: readyNavigation([])
    })
  )
  assertKindAndTone(noSourcesFromPersistence, 'noSources', 'warning')

  const stillChooseRoot = deriveOperationFeedback(noRootInputs({ navigationReadResult: undefined }))
  assertKindAndTone(stillChooseRoot, 'chooseRoot', 'idle')

  const hostStillWins = deriveOperationFeedback({
    ...base,
    hostStatus: failedHost('host failure reason')
  })
  assert.equal(hostStillWins.kind, 'hostUnavailable')
  assert.equal(hostStillWins.tone, 'error')
}

function assertKindAndTone(
  actual: LibraryOperationFeedback,
  expectedKind: LibraryOperationFeedbackKind,
  expectedTone: LibraryOperationFeedback['tone']
): void {
  assert.equal(
    actual.kind,
    expectedKind,
    `kind mismatch: expected "${expectedKind}", got "${actual.kind}"`
  )
  assert.equal(actual.tone, expectedTone, `tone mismatch for "${expectedKind}"`)
}

function startedInputs(overrides: Partial<OperationFeedbackInputs> = {}): OperationFeedbackInputs {
  return {
    hostStatus: hostWithState('started'),
    rootChoiceStatus: 'idle',
    registeredRootPath: '/Music',
    scanStatus: 'idle',
    scanSummary: undefined,
    refreshStatus: 'idle',
    navigationReadIsLoading: false,
    hierarchyReadIsLoading: false,
    navigationReadRequestError: undefined,
    hierarchyReadRequestError: undefined,
    navigationReadResult: undefined,
    ...overrides
  }
}

function noRootInputs(overrides: Partial<OperationFeedbackInputs> = {}): OperationFeedbackInputs {
  return {
    hostStatus: hostWithState('started'),
    rootChoiceStatus: 'idle',
    registeredRootPath: undefined,
    scanStatus: 'idle',
    scanSummary: undefined,
    refreshStatus: 'idle',
    navigationReadIsLoading: false,
    hierarchyReadIsLoading: false,
    navigationReadRequestError: undefined,
    hierarchyReadRequestError: undefined,
    navigationReadResult: undefined,
    ...overrides
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
    state: 'failed',
    environment: 'development',
    binaryPolicy: { kind: 'developmentBinary', source: 'environmentOverride' },
    lastError: { code: 'unknown', message }
  }
}

function makeNavRow(): NavigationRow {
  return {
    navigationRowId: '1',
    stableKey: 'source:1',
    parentNavigationRowId: null,
    family: 'sources' as const,
    rowKind: 'source' as const,
    displayName: 'Source',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '1',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function readyNavigation(
  rows: OperationFeedbackInputs['navigationReadResult'] extends {
    state: 'ready'
    rows: infer R
  }
    ? R
    : never
): OperationFeedbackInputs['navigationReadResult'] {
  return {
    state: 'ready',
    rows
  }
}
