export type LibraryV0FixtureId = 'source-a' | 'source-b'

export type LibraryV0FixtureCatalogEntry = {
  readonly id: LibraryV0FixtureId
  readonly summary: string
  readonly generatedFiles: readonly string[]
  readonly generatedDirectories: readonly string[]
}

export const libraryV0FixtureCatalog = {
  sourceA: {
    id: 'source-a',
    summary:
      'Primary generated source with root audio, nested audio, descendant-only audio, empty folder, and mixed companion files.',
    generatedFiles: [
      'Root Track A.wav',
      'nested/Nested Track A.wav',
      'descendants-only/deeper/Descendant Only A.wav',
      'mixed/cover.jpg',
      'mixed/readme.txt'
    ],
    generatedDirectories: ['nested', 'descendants-only/deeper', 'empty', 'mixed']
  },
  sourceB: {
    id: 'source-b',
    summary: 'Secondary generated source for cross-source search isolation.',
    generatedFiles: ['Only Track B.wav'],
    generatedDirectories: []
  }
} as const satisfies Record<string, LibraryV0FixtureCatalogEntry>

export const libraryV0FixtureRules = [
  'Generated fixtures must be deterministic and isolated under the Playwright test output path.',
  'Fixture media stays tiny and generated; do not add real media binaries.',
  'Offline/missing-source simulations move generated fixture directories only.',
  'Cleanup helpers must be safe to call after partial failures.'
] as const
