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
  const text = JSON.stringify(input.diagnostics).toLowerCase()
  const errorText = formatUnknownError(input.error).toLowerCase()
  const phase = input.phase.toLowerCase()

  if (phase.includes('fixture') || phase.includes('launch') || phase.includes('teardown')) {
    return classification(
      'harness/setup-issue',
      'high',
      'The failure happened while preparing, launching, or tearing down the test harness.',
      signals,
      'Check fixture paths, built app availability, Electron launch logs, and teardown logs.'
    )
  }

  if (
    text.includes('preload api unavailable') ||
    text.includes('electron main window is not available') ||
    text.includes('built electron main entry was not found')
  ) {
    return classification(
      'harness/setup-issue',
      'high',
      'The app or preload harness was not available enough to evaluate the product contract.',
      signals,
      'Rebuild the desktop app and inspect Electron stdout/stderr plus renderer page errors.'
    )
  }

  if (text.includes('ui-runtime-contradiction')) {
    return classification(
      'product-blocker',
      'high',
      'Visible UI contradicted the runtime source state captured through public APIs.',
      signals,
      'Keep the reproducer and fix the product projection/runtime contract before relaxing the spec.'
    )
  }

  if (phase.includes('contract')) {
    return classification(
      'undefined-product-contract',
      'medium',
      'The scenario reached behavior that needs an explicit Library V0 product contract decision.',
      signals,
      'Decide the product contract, then encode the chosen behavior as an assertion.'
    )
  }

  if (
    errorText.includes('timeout') ||
    text.includes('still indexing') ||
    text.includes('scanning source') ||
    text.includes('updating selected contents')
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
  if (diagnosticsText.includes('ui-runtime-contradiction')) {
    signals.push('uiRuntimeContradiction=true')
  }

  return signals
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
