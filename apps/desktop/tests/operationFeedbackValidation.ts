import { strict as assert } from 'node:assert'

import {
  deriveOperationFeedback,
  type LibraryOperationFeedback,
  type OperationFeedbackInputs
} from '../src/renderer/libraryBrowser/projection/operationFeedback'
import type { LibraryBoundaryHostStatus } from '../src/shared/libraryBoundary/status'

void main()

function main(): void {
  validatesHostStatusChecks()
  validatesNoRootRegistered()
  validatesActiveOperations()
  validatesErrorStates()
  validatesSuccessStates()
  validatesNoFakeProgress()
}

function validatesHostStatusChecks(): void {
  const base = noRootInputs()

  assertFeedback(deriveOperationFeedback({ ...base, hostStatus: undefined }), {
    tone: 'loading',
    title: 'Checking library host'
  })

  assertFeedback(deriveOperationFeedback({ ...base, hostStatus: idleHost() }), {
    tone: 'warning',
    title: 'Library host idle'
  })

  assertFeedback(deriveOperationFeedback({ ...base, hostStatus: hostWithState('starting') }), {
    tone: 'loading',
    title: 'Library host starting'
  })

  assertFeedback(deriveOperationFeedback({ ...base, hostStatus: hostWithState('stopping') }), {
    tone: 'warning',
    title: 'Library host stopping'
  })

  assertFeedback(deriveOperationFeedback({ ...base, hostStatus: hostWithState('stopped') }), {
    tone: 'warning',
    title: 'Library host stopped'
  })

  assertFeedback(
    deriveOperationFeedback({ ...base, hostStatus: failedHost('host failed detail') }),
    {
      tone: 'error',
      title: 'Library host failed'
    }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      hostStatus: failedHost('host failed detail'),
      navigationReadRequestError: 'ignored error'
    }),
    {
      tone: 'error',
      title: 'Library host failed'
    }
  )
}

function validatesNoRootRegistered(): void {
  const base = startedInputs({ registeredRootPath: undefined })

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'idle' }), {
    tone: 'idle',
    title: 'Choose a music folder'
  })

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' }), {
    tone: 'loading',
    title: 'Choosing music folder'
  })

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' }), {
    tone: 'error',
    title: 'Unable to add music folder'
  })

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'canceled' }), {
    tone: 'warning',
    title: 'Folder selection canceled'
  })
}

function validatesActiveOperations(): void {
  const base = startedInputs({})

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'choosing' }), {
    tone: 'loading',
    title: 'Choosing music folder'
  })

  assertFeedback(deriveOperationFeedback({ ...base, scanStatus: 'scanning' }), {
    tone: 'loading',
    title: 'Scanning local root'
  })

  assertFeedback(deriveOperationFeedback({ ...base, refreshStatus: 'refreshing' }), {
    tone: 'loading',
    title: 'Refreshing library view'
  })

  assertFeedback(deriveOperationFeedback({ ...base, navigationReadIsLoading: true }), {
    tone: 'loading',
    title: 'Loading navigation'
  })

  assertFeedback(deriveOperationFeedback({ ...base, hierarchyReadIsLoading: true }), {
    tone: 'loading',
    title: 'Loading hierarchy'
  })

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      scanStatus: 'scanning',
      navigationReadIsLoading: true,
      hierarchyReadIsLoading: true
    }),
    { tone: 'loading', title: 'Scanning local root' }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      refreshStatus: 'refreshing',
      navigationReadIsLoading: true
    }),
    { tone: 'loading', title: 'Refreshing library view' }
  )
}

function validatesErrorStates(): void {
  const base = startedInputs({})

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'failed' }), {
    tone: 'error',
    title: 'Unable to add music folder'
  })

  assertFeedback(deriveOperationFeedback({ ...base, scanStatus: 'failed' }), {
    tone: 'error',
    title: 'Scan failed'
  })

  assertFeedback(deriveOperationFeedback({ ...base, refreshStatus: 'failed' }), {
    tone: 'warning',
    title: 'Refresh incomplete'
  })

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      navigationReadRequestError: 'Safe navigation read error.'
    }),
    { tone: 'error', title: 'Navigation read failed' }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      hierarchyReadRequestError: 'Safe hierarchy read error.'
    }),
    { tone: 'error', title: 'Hierarchy read failed' }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      navigationReadRequestError: 'Safe detail must be shown.',
      hierarchyReadRequestError: 'Safe hierarchy detail.'
    }),
    { tone: 'error', title: 'Navigation read failed', detail: 'Safe detail must be shown.' }
  )
}

function validatesSuccessStates(): void {
  const base = startedInputs({})

  assertFeedback(deriveOperationFeedback({ ...base, scanStatus: 'scanned' }), {
    tone: 'success',
    title: 'Scan complete'
  })

  assertFeedback(deriveOperationFeedback({ ...base, refreshStatus: 'refreshed' }), {
    tone: 'success',
    title: 'Library view refreshed'
  })

  assertFeedback(deriveOperationFeedback({ ...base, rootChoiceStatus: 'registered' }), {
    tone: 'success',
    title: 'Folder added'
  })

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      rootChoiceStatus: 'registered',
      refreshStatus: 'refreshed',
      navigationReadResult: readyNavigation([])
    }),
    { tone: 'success', title: 'Library view refreshed' }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      navigationReadResult: readyNavigation([
        {
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
      ])
    }),
    { tone: 'success', title: 'Library ready' }
  )

  assertFeedback(
    deriveOperationFeedback({
      ...base,
      navigationReadResult: readyNavigation([])
    }),
    { tone: 'warning', title: 'No library sources' }
  )
}

function validatesNoFakeProgress(): void {
  const feedback = deriveOperationFeedback(startedInputs({ scanStatus: 'scanning' }))

  assert.equal(feedback.tone, 'loading')
  assert.equal(feedback.title, 'Scanning local root')
  assert.doesNotMatch(feedback.title, /%/g)
  assert.doesNotMatch(feedback.detail ?? '', /%/g)
  assert.doesNotMatch(feedback.title, /file\s+\d+/i)
  assert.doesNotMatch(feedback.detail ?? '', /file\s+\d+/i)

  const scanComplete = deriveOperationFeedback(startedInputs({ scanStatus: 'scanned' }))
  assert.equal(scanComplete.tone, 'success')
  assert.doesNotMatch(scanComplete.title, /%/g)
  assert.doesNotMatch(scanComplete.title, /\d+\s+files?/i)
}

function assertFeedback(
  actual: LibraryOperationFeedback,
  expected: {
    readonly tone: LibraryOperationFeedback['tone']
    readonly title: string
    readonly detail?: string
  }
): void {
  assert.equal(actual.tone, expected.tone, `tone mismatch for "${expected.title}"`)
  assert.equal(actual.title, expected.title, `title mismatch`)
  if (expected.detail !== undefined) {
    assert.equal(actual.detail, expected.detail, `detail mismatch for "${expected.title}"`)
  }
}

function startedInputs(overrides: Partial<OperationFeedbackInputs> = {}): OperationFeedbackInputs {
  return {
    hostStatus: hostWithState('started'),
    rootChoiceStatus: 'idle',
    registeredRootPath: '/Music',
    scanStatus: 'idle',
    refreshStatus: 'idle',
    navigationReadIsLoading: false,
    hierarchyReadIsLoading: false,
    navigationReadRequestError: undefined,
    hierarchyReadRequestError: undefined,
    navigationReadResult: undefined,
    ...overrides
  }
}

function noRootInputs(): OperationFeedbackInputs {
  return {
    hostStatus: hostWithState('started'),
    rootChoiceStatus: 'idle',
    registeredRootPath: undefined,
    scanStatus: 'idle',
    refreshStatus: 'idle',
    navigationReadIsLoading: false,
    hierarchyReadIsLoading: false,
    navigationReadRequestError: undefined,
    hierarchyReadRequestError: undefined,
    navigationReadResult: undefined
  }
}

function idleHost(): LibraryBoundaryHostStatus {
  return hostWithState('idle')
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
