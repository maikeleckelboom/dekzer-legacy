import { libraryV0AcceptanceTag, libraryV0ScenarioTag, libraryV0Tag } from './acceptanceTags'

export type LibraryV0ScenarioId =
  | 'launch'
  | 'golden-smoke'
  | 'scoped-search'
  | 'restart-persistence'
  | 'missing-source-health'
  | 'remove-readd-freshness'

export type LibraryV0Scenario = {
  readonly id: LibraryV0ScenarioId
  readonly title: string
  readonly invariant: string
  readonly fixtureRequirements: readonly string[]
  readonly failureEvidence: readonly string[]
  readonly acceptanceGate: boolean
  readonly tags: readonly string[]
}

const commonFailureEvidence = [
  'runtime local roots',
  'visible Library panel state',
  'visible contents rows',
  'visible source status',
  'user-data path',
  'fixture paths',
  'harness logs attached by the Electron fixture'
] as const

export const libraryV0Scenarios = {
  launch: scenario({
    id: 'launch',
    title: 'Library V0 launch',
    invariant:
      'The built Electron app launches with isolated user data and reaches Library intake.',
    fixtureRequirements: ['isolated user data path', 'generated source fixtures'],
    failureEvidence: commonFailureEvidence,
    acceptanceGate: true
  }),
  'golden-smoke': scenario({
    id: 'golden-smoke',
    title: 'Library V0 golden smoke',
    invariant:
      'A generated music source can be admitted, scanned, browsed, filtered, removed, and stays removed after relaunch.',
    fixtureRequirements: ['Source A generated fixture'],
    failureEvidence: [
      ...commonFailureEvidence,
      'source lifecycle snapshot',
      'source integrity snapshot',
      'source activity snapshot'
    ],
    acceptanceGate: true
  }),
  'scoped-search': scenario({
    id: 'scoped-search',
    title: 'Library V0 scoped search',
    invariant:
      'Search results respect the active source or folder scope and do not leak rows from other admitted sources.',
    fixtureRequirements: ['Source A generated fixture', 'Source B generated fixture'],
    failureEvidence: [...commonFailureEvidence, 'current search query and visible result state'],
    acceptanceGate: true
  }),
  'restart-persistence': scenario({
    id: 'restart-persistence',
    title: 'Library V0 restart persistence',
    invariant:
      'An admitted source persists across graceful restart with honest restored or reselectable browse state.',
    fixtureRequirements: ['Source A generated fixture', 'stable user data path across relaunch'],
    failureEvidence: [
      ...commonFailureEvidence,
      'persisted view-state snapshot',
      'source lifecycle snapshot',
      'source integrity snapshot',
      'source activity snapshot'
    ],
    acceptanceGate: true
  }),
  'missing-source-health': scenario({
    id: 'missing-source-health',
    title: 'Library V0 missing source health',
    invariant:
      'A known source that disappears remains admitted but is not presented as healthy or complete until recovered.',
    fixtureRequirements: ['Source A generated fixture', 'offline move/restore helper'],
    failureEvidence: [
      ...commonFailureEvidence,
      'source availability',
      'source lifecycle snapshot',
      'source integrity snapshot',
      'source activity snapshot'
    ],
    acceptanceGate: true
  }),
  'remove-readd-freshness': scenario({
    id: 'remove-readd-freshness',
    title: 'Library V0 remove/re-add freshness',
    invariant:
      'Removing and re-adding the exact same path does not reuse removed-source state as fresh truth or duplicate the source.',
    fixtureRequirements: ['Source A generated fixture', 'stable user data path across relaunch'],
    failureEvidence: [
      ...commonFailureEvidence,
      'root count for the admitted path',
      'source lifecycle snapshot',
      'source integrity snapshot',
      'source activity snapshot'
    ],
    acceptanceGate: true
  })
} as const satisfies Record<LibraryV0ScenarioId, LibraryV0Scenario>

export function libraryV0ScenarioTitle(
  scenario: LibraryV0Scenario,
  title = scenario.title
): string {
  return `${title} ${scenario.tags.join(' ')}`
}

function scenario(input: Omit<LibraryV0Scenario, 'tags'>): LibraryV0Scenario {
  return {
    ...input,
    tags: [libraryV0Tag, libraryV0AcceptanceTag, libraryV0ScenarioTag(input.id)]
  }
}
