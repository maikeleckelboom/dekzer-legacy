import type { Locator, Page, TestInfo } from '@playwright/test'
import { existsSync } from 'node:fs'

import type { ElectronAppHarness } from '../../fixtures/electronApp.fixture'
import {
  classifyLibraryV0Failure,
  type LibraryFailureClassification
} from './LibraryFailureClassifier'
import {
  isProbeFailure,
  LibraryRuntimeProbe,
  normalizeLocalSourcePath,
  sourceSnapshotLooksUnavailable,
  type LibraryRuntimeProbeFailure,
  type LibraryRuntimeSourceSnapshot
} from './LibraryRuntimeProbe'
import { LibraryPanel } from '../screens/LibraryPanel'
import type { LibraryV0Scenario } from '../scenarios/libraryV0Scenarios'

export type LibraryDiagnosticSource = {
  readonly label: string
  readonly rootPath: string
  readonly sourceId?: string
  readonly expectedRows?: readonly string[]
}

export type LibraryDiagnosticFixturePaths = {
  readonly rootDir?: string
  readonly userDataPath?: string
  readonly mediaRootPath?: string
  readonly sourceA?: string
  readonly sourceB?: string
  readonly offlineSourceA?: string
}

export type LibraryDiagnosticCapture = {
  readonly scenario: {
    readonly id: string
    readonly title: string
    readonly invariant: string
    readonly acceptanceGate: boolean
  }
  readonly phase: string
  readonly error: string
  readonly userDataPath: string
  readonly fixturePaths?: LibraryDiagnosticFixturePaths
  readonly runtime: {
    readonly preloadApiAvailable: boolean
    readonly hostStatus?: unknown
    readonly roots?: unknown
    readonly viewState?: unknown
    readonly sources: readonly LibraryDiagnosticSourceSnapshot[]
  }
  readonly visible: LibraryVisibleDiagnosticSnapshot
  readonly contradictions: readonly LibraryDiagnosticContradiction[]
  readonly harnessEvidence: {
    readonly expectedElectronFixtureAttachments: readonly string[]
  }
}

export type LibraryDiagnosticSourceSnapshot = {
  readonly label: string
  readonly rootPath: string
  readonly sourceId?: string
  readonly expectedRows?: readonly string[]
  readonly fixturePathExists: boolean
  readonly pathKnownToRuntime: boolean
  readonly rootAvailability?: string
  readonly snapshot?: LibraryRuntimeSourceSnapshot | LibraryRuntimeProbeFailure
}

export type LibraryDiagnosticContradiction = {
  readonly code: 'ui-runtime-contradiction'
  readonly detail: string
}

export type LibraryVisibleDiagnosticSnapshot = {
  readonly pageAvailable: boolean
  readonly panelVisible?: boolean
  readonly panelTitle?: string
  readonly contents?: {
    readonly title?: string
    readonly rows: readonly string[]
    readonly rowCount: number
  }
  readonly sourceStatus?: {
    readonly visible: boolean
    readonly text?: string
    readonly actions: readonly {
      readonly label: string
      readonly enabled: boolean
    }[]
  }
  readonly search?: {
    readonly visible: boolean
    readonly query?: string
  }
}

export type LibraryAcceptanceRunOptions = {
  readonly scenario: LibraryV0Scenario
  readonly testInfo: TestInfo
  readonly electronApp: ElectronAppHarness
  readonly page: () => Page | undefined
  readonly fixturePaths?: LibraryDiagnosticFixturePaths
  readonly admittedSources?: readonly LibraryDiagnosticSource[]
  readonly attachOnSuccess?: boolean
}

export class LibraryV0AcceptanceRun {
  private phase = 'not-started'
  private readonly admittedSources: LibraryDiagnosticSource[]

  constructor(private readonly options: LibraryAcceptanceRunOptions) {
    this.admittedSources = [...(options.admittedSources ?? [])]
  }

  addAdmittedSource(source: LibraryDiagnosticSource): void {
    const normalized = normalizeLocalSourcePath(source.rootPath)
    const existingIndex = this.admittedSources.findIndex(
      (candidate) => normalizeLocalSourcePath(candidate.rootPath) === normalized
    )

    if (existingIndex === -1) {
      this.admittedSources.push(source)
      return
    }

    this.admittedSources.splice(existingIndex, 1, source)
  }

  async step<T>(phase: string, callback: () => Promise<T>): Promise<T> {
    this.phase = phase
    return callback()
  }

  async run<T>(callback: (run: LibraryV0AcceptanceRun) => Promise<T>): Promise<T> {
    try {
      const result = await callback(this)

      if (this.options.attachOnSuccess === true) {
        await this.attachDiagnostics(undefined)
      }

      return result
    } catch (error) {
      await this.attachDiagnostics(error)
      throw error
    }
  }

  private async attachDiagnostics(error: unknown): Promise<void> {
    const diagnostics = new LibraryDiagnostics({
      electronApp: this.options.electronApp,
      page: this.options.page
    })
    const capture = await diagnostics.capture({
      scenario: this.options.scenario,
      phase: this.phase,
      error,
      admittedSources: this.admittedSources,
      ...(this.options.fixturePaths === undefined
        ? {}
        : { fixturePaths: this.options.fixturePaths })
    })
    const classification = classifyLibraryV0Failure({
      scenario: this.options.scenario,
      phase: this.phase,
      error,
      diagnostics: capture
    })

    await attachJson(this.options.testInfo, 'library-v0-diagnostics.json', {
      classification,
      diagnostics: capture
    })
    await this.options.testInfo.attach('library-v0-failure-classification.md', {
      body: renderClassificationMarkdown(classification, capture),
      contentType: 'text/markdown'
    })
  }
}

export class LibraryDiagnostics {
  constructor(
    private readonly options: {
      readonly electronApp: ElectronAppHarness
      readonly page: () => Page | undefined
    }
  ) {}

  async capture(input: {
    readonly scenario: LibraryV0Scenario
    readonly phase: string
    readonly error: unknown
    readonly admittedSources: readonly LibraryDiagnosticSource[]
    readonly fixturePaths?: LibraryDiagnosticFixturePaths
  }): Promise<LibraryDiagnosticCapture> {
    const page = this.safePage()
    const probe = page === undefined || page.isClosed() ? undefined : new LibraryRuntimeProbe(page)
    const [preloadApiAvailable, hostStatus, roots, viewState, visible] = await Promise.all([
      probe?.hasPreloadApi() ?? Promise.resolve(false),
      probe?.readHostStatus() ?? Promise.resolve(undefined),
      probe?.readLocalRoots() ?? Promise.resolve(undefined),
      probe?.readViewState() ?? Promise.resolve(undefined),
      page === undefined || page.isClosed()
        ? Promise.resolve({ pageAvailable: false })
        : captureVisibleSnapshot(page)
    ])
    const sourceSnapshots =
      probe === undefined ? [] : await captureSourceSnapshots(probe, roots, input.admittedSources)
    const capture: LibraryDiagnosticCapture = {
      scenario: {
        id: input.scenario.id,
        title: input.scenario.title,
        invariant: input.scenario.invariant,
        acceptanceGate: input.scenario.acceptanceGate
      },
      phase: input.phase,
      error: formatUnknownError(input.error),
      userDataPath: this.options.electronApp.userDataPath,
      ...(input.fixturePaths === undefined ? {} : { fixturePaths: input.fixturePaths }),
      runtime: {
        preloadApiAvailable,
        ...(hostStatus === undefined ? {} : { hostStatus }),
        ...(roots === undefined ? {} : { roots }),
        ...(viewState === undefined ? {} : { viewState }),
        sources: sourceSnapshots
      },
      visible,
      contradictions: findContradictions(visible, sourceSnapshots),
      harnessEvidence: {
        expectedElectronFixtureAttachments: [
          'electron-stdout.log',
          'electron-stderr.log',
          'electron-main-console.log',
          'renderer-console.log',
          'renderer-page-errors.log',
          'electron-window-placement.log',
          'electron-shutdown.log',
          'renderer-screenshot',
          'playwright-trace'
        ]
      }
    }

    return capture
  }

  private safePage(): Page | undefined {
    try {
      return this.options.page()
    } catch {
      return undefined
    }
  }
}

export function createLibraryV0AcceptanceRun(
  options: LibraryAcceptanceRunOptions
): LibraryV0AcceptanceRun {
  return new LibraryV0AcceptanceRun(options)
}

export function libraryV0FixtureDiagnosticPaths(input: {
  readonly rootDir: string
  readonly userDataPath: string
  readonly mediaRootPath: string
  readonly sourceA: { readonly rootPath: string }
  readonly sourceB: { readonly rootPath: string }
  readonly offlineSourceA?: string
}): LibraryDiagnosticFixturePaths {
  return {
    rootDir: input.rootDir,
    userDataPath: input.userDataPath,
    mediaRootPath: input.mediaRootPath,
    sourceA: input.sourceA.rootPath,
    sourceB: input.sourceB.rootPath,
    ...(input.offlineSourceA === undefined ? {} : { offlineSourceA: input.offlineSourceA })
  }
}

export function libraryV0DiagnosticSource(input: {
  readonly label: string
  readonly rootPath: string
  readonly sourceId?: string
  readonly expectedRows?: readonly string[]
}): LibraryDiagnosticSource {
  return {
    label: input.label,
    rootPath: input.rootPath,
    ...(input.sourceId === undefined ? {} : { sourceId: input.sourceId }),
    ...(input.expectedRows === undefined ? {} : { expectedRows: input.expectedRows })
  }
}

export function currentElectronPage(electronApp: ElectronAppHarness): Page | undefined {
  try {
    return electronApp.window()
  } catch {
    return undefined
  }
}

async function captureSourceSnapshots(
  probe: LibraryRuntimeProbe,
  roots: unknown,
  admittedSources: readonly LibraryDiagnosticSource[]
): Promise<readonly LibraryDiagnosticSourceSnapshot[]> {
  const runtimeRoots = rootsFromSnapshot(roots)

  return Promise.all(
    admittedSources.map(async (source) => {
      const sourceId = source.sourceId ?? (await probe.sourceIdForAdmittedPath(source.rootPath))
      const root = runtimeRoots.find(
        (candidate) =>
          normalizeLocalSourcePath(candidate.admittedRootPath) ===
          normalizeLocalSourcePath(source.rootPath)
      )
      const snapshot = sourceId === undefined ? undefined : await probe.readSourceSnapshot(sourceId)

      return {
        label: source.label,
        rootPath: source.rootPath,
        ...(sourceId === undefined ? {} : { sourceId }),
        ...(source.expectedRows === undefined ? {} : { expectedRows: source.expectedRows }),
        fixturePathExists: existsSync(source.rootPath),
        pathKnownToRuntime: sourceId !== undefined,
        ...(root?.availability === undefined ? {} : { rootAvailability: root.availability }),
        ...(snapshot === undefined ? {} : { snapshot })
      }
    })
  )
}

async function captureVisibleSnapshot(page: Page): Promise<LibraryVisibleDiagnosticSnapshot> {
  const panel = new LibraryPanel(page)
  const [panelVisible, panelTitle, contentsTitle, contentsRows, sourceStatus, search] =
    await Promise.all([
      safeIsVisible(panel.root),
      safeText(panel.title),
      safeText(panel.contents.title),
      safeRows(panel.contents.rows()),
      captureSourceStatus(panel.sourceStatus.root),
      captureSearch(panel.search.input)
    ])

  return {
    pageAvailable: true,
    panelVisible,
    ...(panelTitle === undefined ? {} : { panelTitle }),
    contents: {
      ...(contentsTitle === undefined ? {} : { title: contentsTitle }),
      rows: contentsRows.rows,
      rowCount: contentsRows.rowCount
    },
    sourceStatus,
    search
  }
}

async function captureSourceStatus(
  root: Locator
): Promise<NonNullable<LibraryVisibleDiagnosticSnapshot['sourceStatus']>> {
  const visible = await safeIsVisible(root)
  const text = visible ? await safeText(root) : undefined
  const actions = visible ? await safeButtons(root) : []

  return {
    visible,
    ...(text === undefined ? {} : { text }),
    actions
  }
}

async function captureSearch(
  input: Locator
): Promise<NonNullable<LibraryVisibleDiagnosticSnapshot['search']>> {
  const visible = await safeIsVisible(input)
  const query = visible ? await input.inputValue().catch(() => undefined) : undefined

  return {
    visible,
    ...(query === undefined ? {} : { query })
  }
}

async function safeIsVisible(locator: Locator): Promise<boolean> {
  return locator.isVisible({ timeout: 500 }).catch(() => false)
}

async function safeText(locator: Locator): Promise<string | undefined> {
  if (!(await safeIsVisible(locator))) {
    return undefined
  }

  return locator
    .textContent({ timeout: 500 })
    .then((value) => normalizeText(value ?? ''))
    .catch(() => undefined)
}

async function safeRows(
  locator: Locator
): Promise<{ readonly rows: readonly string[]; readonly rowCount: number }> {
  const rowCount = await locator.count().catch(() => 0)
  const rows = await Promise.all(
    Array.from({ length: Math.min(rowCount, 20) }, async (_value, index) =>
      locator
        .nth(index)
        .textContent({ timeout: 500 })
        .then((value) => normalizeText(value ?? ''))
        .catch(() => '')
    )
  )

  return {
    rows: rows.filter((row) => row.length > 0),
    rowCount
  }
}

async function safeButtons(
  root: Locator
): Promise<readonly { readonly label: string; readonly enabled: boolean }[]> {
  const buttons = root.getByRole('button')
  const count = await buttons.count().catch(() => 0)

  return Promise.all(
    Array.from({ length: Math.min(count, 20) }, async (_value, index) => {
      const button = buttons.nth(index)
      const [label, enabled] = await Promise.all([
        safeText(button).then((value) => value ?? '(unlabeled button)'),
        button.isEnabled({ timeout: 500 }).catch(() => false)
      ])

      return { label, enabled }
    })
  )
}

function findContradictions(
  visible: LibraryVisibleDiagnosticSnapshot,
  sources: readonly LibraryDiagnosticSourceSnapshot[]
): readonly LibraryDiagnosticContradiction[] {
  const contradictions: LibraryDiagnosticContradiction[] = []
  const statusText = visible.sourceStatus?.text ?? ''

  for (const source of sources) {
    const unavailable =
      !source.fixturePathExists ||
      source.rootAvailability === 'unavailable' ||
      (source.snapshot !== undefined &&
        !isProbeFailure(source.snapshot) &&
        sourceSnapshotLooksUnavailable(source.snapshot))

    if (!unavailable) {
      continue
    }

    if (
      /\bReady\b/.test(statusText) &&
      !/Missing|Offline\/unavailable|Blocked|Partial/.test(statusText)
    ) {
      contradictions.push({
        code: 'ui-runtime-contradiction',
        detail: `${source.label} is unavailable in runtime snapshots while the visible source status presents Ready.`
      })
    }

    const visibleRows = visible.contents?.rows ?? []
    const expectedRows = source.expectedRows ?? []
    const activeVisibleExpectedRows = expectedRows.filter((row) =>
      visibleRows.some((visibleRow) => visibleRow.includes(row) && !rowLooksUnavailable(visibleRow))
    )

    if (activeVisibleExpectedRows.length > 0) {
      contradictions.push({
        code: 'ui-runtime-contradiction',
        detail: `${source.label} is unavailable but visible contents include active-looking expected media rows: ${activeVisibleExpectedRows.join(', ')}.`
      })
    }
  }

  return contradictions
}

function rootsFromSnapshot(
  roots: unknown
): readonly { readonly admittedRootPath: string; readonly availability?: string }[] {
  if (!isRecord(roots) || roots.state !== 'read' || !Array.isArray(roots.roots)) {
    return []
  }

  return roots.roots
    .filter((root): root is Record<string, unknown> => isRecord(root))
    .map((root) => ({
      admittedRootPath:
        typeof root.admittedRootPath === 'string' ? root.admittedRootPath : '(unknown)',
      ...(typeof root.availability === 'string' ? { availability: root.availability } : {})
    }))
}

async function attachJson(testInfo: TestInfo, name: string, value: unknown): Promise<void> {
  await testInfo.attach(name, {
    body: JSON.stringify(value, null, 2),
    contentType: 'application/json'
  })
}

function renderClassificationMarkdown(
  classification: LibraryFailureClassification,
  capture: LibraryDiagnosticCapture
): string {
  const sourceLines = capture.runtime.sources.map(
    (source) =>
      `- ${source.label}: exists=${String(source.fixturePathExists)}, known=${String(source.pathKnownToRuntime)}, sourceId=${source.sourceId ?? '(unknown)'}, availability=${source.rootAvailability ?? '(unknown)'}`
  )
  const contradictionLines = capture.contradictions.map(
    (contradiction) => `- ${contradiction.detail}`
  )

  return [
    `# Library V0 Failure Classification`,
    ``,
    `- Scenario: ${capture.scenario.id}`,
    `- Phase: ${capture.phase}`,
    `- Classification: ${classification.kind}`,
    `- Confidence: ${classification.confidence}`,
    `- Summary: ${classification.summary}`,
    `- Recommended next step: ${classification.recommendedNextStep}`,
    ``,
    `## Signals`,
    ...classification.signals.map((signal) => `- ${signal}`),
    ``,
    `## Runtime Sources`,
    ...(sourceLines.length === 0
      ? ['- No admitted test sources were registered with diagnostics.']
      : sourceLines),
    ``,
    `## UI/Runtime Contradictions`,
    ...(contradictionLines.length === 0
      ? ['- None detected by the rule-based classifier.']
      : contradictionLines),
    ``,
    `## Expected Electron Fixture Attachments`,
    ...capture.harnessEvidence.expectedElectronFixtureAttachments.map((name) => `- ${name}`)
  ].join('\n')
}

function normalizeText(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

function rowLooksUnavailable(row: string): boolean {
  return /\b(Missing|Removed|Unavailable|File missing|File removed)\b/i.test(row)
}

function formatUnknownError(error: unknown): string {
  if (error === undefined) {
    return '(no error)'
  }

  if (error instanceof Error) {
    return error.stack ?? error.message
  }

  return String(error)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
