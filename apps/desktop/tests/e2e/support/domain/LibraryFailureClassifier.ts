import type { LibraryV0Scenario } from '../scenarios/libraryV0Scenarios'

export type LibraryFailureClassificationKind =
  | 'product-blocker'
  | 'harness/setup-issue'
  | 'undefined-product-contract'
  | 'likely-flake/timing-issue'

export type LibraryFailureClassification = {
  readonly kind: LibraryFailureClassificationKind
  readonly confidence: 'low' | 'medium' | 'high'
  readonly summary: string
  readonly signals: readonly string[]
  readonly recommendedNextStep: string
}

export type LibraryFailureClassifierInput = {
  readonly scenario: LibraryV0Scenario
  readonly phase: string
  readonly error: unknown
  readonly diagnostics: unknown
}

export function classifyLibraryV0Failure(
  input: LibraryFailureClassifierInput
): LibraryFailureClassification {
  const signals = collectSignals(input)
  const errorText = formatUnknownError(input.error).toLowerCase()
  const diagnosticsText = JSON.stringify(input.diagnostics).toLowerCase()
  const harnessSignals = collectHarnessUnavailableSignals(input, diagnosticsText, errorText)

  if (hasUiRuntimeContradiction(input.diagnostics, diagnosticsText)) {
    return classification(
      'product-blocker',
      'high',
      'Visible UI contradicted the runtime source state captured through public APIs.',
      signals,
      'Keep the reproducer and fix the product projection/runtime contract before relaxing the spec.'
    )
  }

  if (harnessSignals.length > 0) {
    return classification(
      'harness/setup-issue',
      'high',
      'The app or preload harness was not available enough to evaluate the product contract.',
      [...signals, ...harnessSignals],
      'Rebuild the desktop app and inspect Electron stdout/stderr plus renderer page errors.'
    )
  }

  if (hasUndefinedContractEvidence(input.phase, diagnosticsText, errorText)) {
    return classification(
      'undefined-product-contract',
      'medium',
      'The scenario reached behavior that needs an explicit Library V0 product contract decision.',
      signals,
      'Decide the product contract, then encode the chosen behavior as an assertion.'
    )
  }

  if (isProductActionOrAssertionPhase(input.phase)) {
    return classification(
      'product-blocker',
      'medium',
      'The failure happened during a Library V0 product action or assertion.',
      signals,
      'Treat the failing assertion as the reproducer unless diagnostics show harness setup was unavailable.'
    )
  }

  if (
    errorText.includes('timeout') ||
    diagnosticsText.includes('still indexing') ||
    diagnosticsText.includes('scanning source') ||
    diagnosticsText.includes('updating selected contents')
  ) {
    return classification(
      'likely-flake/timing-issue',
      'medium',
      'The failure shape looks timing-sensitive, but the scenario still needs review.',
      signals,
      'Inspect trace timing and runtime snapshots; add product readiness evidence if the wait is ambiguous.'
    )
  }

  return classification(
    'product-blocker',
    'medium',
    'The failure happened during a Library V0 product action or assertion.',
    signals,
    'Treat the failing assertion as the reproducer unless diagnostics show harness setup was unavailable.'
  )
}

function collectSignals(input: LibraryFailureClassifierInput): readonly string[] {
  const signals = [
    `scenario=${input.scenario.id}`,
    `acceptanceGate=${String(input.scenario.acceptanceGate)}`,
    `phase=${input.phase}`
  ]
  const error = formatUnknownError(input.error)

  if (error.length > 0) {
    signals.push(`error=${firstLine(error)}`)
  }

  const diagnosticsText = JSON.stringify(input.diagnostics)
  if (diagnosticsText.includes('probeFailed')) {
    signals.push('runtimeProbe=failed')
  }
  if (hasUiRuntimeContradiction(input.diagnostics, diagnosticsText.toLowerCase())) {
    signals.push('uiRuntimeContradiction=true')
  }
  const preloadApiAvailable = preloadApiAvailability(input.diagnostics)
  if (preloadApiAvailable !== undefined) {
    signals.push(`preloadApiAvailable=${String(preloadApiAvailable)}`)
  }
  const pageAvailable = pageAvailability(input.diagnostics)
  if (pageAvailable !== undefined) {
    signals.push(`pageAvailable=${String(pageAvailable)}`)
  }

  return signals
}

function collectHarnessUnavailableSignals(
  input: LibraryFailureClassifierInput,
  diagnosticsText: string,
  errorText: string
): readonly string[] {
  const signals: string[] = []
  const phase = input.phase.toLowerCase()

  if (hasLaunchEvidence(errorText, diagnosticsText)) {
    signals.push('harnessEvidence=launch-unavailable')
  }

  if (
    preloadApiAvailability(input.diagnostics) === false &&
    (phase.includes('launch') || hasPreloadEvidence(errorText, diagnosticsText))
  ) {
    signals.push('harnessEvidence=preload-unavailable')
  }

  if (
    pageAvailability(input.diagnostics) === false &&
    (phase.includes('launch') || hasPageEvidence(errorText, diagnosticsText))
  ) {
    signals.push('harnessEvidence=page-unavailable')
  }

  if (phase.includes('fixture') && hasFixtureEvidence(errorText)) {
    signals.push('harnessEvidence=fixture-unavailable')
  }

  if (phase.includes('teardown') && hasTeardownEvidence(errorText)) {
    signals.push('harnessEvidence=teardown-unavailable')
  }

  return signals
}

function hasLaunchEvidence(errorText: string, diagnosticsText: string): boolean {
  return (
    errorText.includes('built electron main entry was not found') ||
    errorText.includes('electron package did not resolve to an executable path') ||
    diagnosticsText.includes('packagedbinaryunavailable')
  )
}

function hasPreloadEvidence(errorText: string, diagnosticsText: string): boolean {
  return (
    errorText.includes('preload') ||
    errorText.includes('window.dekzer') ||
    errorText.includes('waitforfunction') ||
    diagnosticsText.includes('preload api unavailable')
  )
}

function hasPageEvidence(errorText: string, diagnosticsText: string): boolean {
  return (
    errorText.includes('electron main window is not available') ||
    errorText.includes('electron app is not running') ||
    errorText.includes('target page') ||
    errorText.includes('browser has been closed') ||
    diagnosticsText.includes('electron main window is not available')
  )
}

function hasFixtureEvidence(errorText: string): boolean {
  return /\b(enoent|eacces|eperm|rename|mkdir|rmdir|unlink|fixture|filesystem)\b/.test(errorText)
}

function hasTeardownEvidence(errorText: string): boolean {
  return /close|graceful|timed out|timedout|killed|shutdown|process/.test(errorText)
}

function hasUiRuntimeContradiction(diagnostics: unknown, diagnosticsText: string): boolean {
  const record = asRecord(diagnostics)
  const contradictions = asReadonlyArray(record?.contradictions)

  return (
    contradictions.some(
      (contradiction) => asRecord(contradiction)?.code === 'ui-runtime-contradiction'
    ) || diagnosticsText.includes('ui-runtime-contradiction')
  )
}

function hasUndefinedContractEvidence(
  phase: string,
  diagnosticsText: string,
  errorText: string
): boolean {
  return (
    phase.toLowerCase().includes('contract') &&
    (errorText.includes('contract') || diagnosticsText.includes('product contract decision'))
  )
}

function isProductActionOrAssertionPhase(phase: string): boolean {
  const normalized = phase.toLowerCase()
  return normalized.startsWith('product-action:') || normalized.startsWith('assertion:')
}

function preloadApiAvailability(diagnostics: unknown): boolean | undefined {
  const runtime = asRecord(asRecord(diagnostics)?.runtime)
  const value = runtime?.preloadApiAvailable

  return typeof value === 'boolean' ? value : undefined
}

function pageAvailability(diagnostics: unknown): boolean | undefined {
  const visible = asRecord(asRecord(diagnostics)?.visible)
  const value = visible?.pageAvailable

  return typeof value === 'boolean' ? value : undefined
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined
}

function asReadonlyArray(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : []
}

function classification(
  kind: LibraryFailureClassificationKind,
  confidence: LibraryFailureClassification['confidence'],
  summary: string,
  signals: readonly string[],
  recommendedNextStep: string
): LibraryFailureClassification {
  return {
    kind,
    confidence,
    summary,
    signals,
    recommendedNextStep
  }
}

function firstLine(value: string): string {
  return value.split(/\r?\n/, 1)[0] ?? value
}

function formatUnknownError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? error.message
  }

  return String(error)
}
