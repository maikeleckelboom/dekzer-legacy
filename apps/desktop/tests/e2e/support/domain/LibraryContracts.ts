export type LibraryBrowseProfileLabel = 'Audio' | 'Audio + Video' | 'All Files'

export type LibraryV0Source = {
  readonly rootPath: string
  readonly sourceName: RegExp
  readonly audioRows: readonly RegExp[]
}

export type LibraryV0GoldenSource = LibraryV0Source & {
  readonly rootTrack: RegExp
  readonly nestedTrack: RegExp
  readonly descendantOnlyTrack: RegExp
  readonly coverImage: RegExp
  readonly readmeText: RegExp
}

export type LibraryV0ScopedSearchSourceA = LibraryV0GoldenSource & {
  readonly nestedFolder: RegExp
}

export type LibraryV0ScopedSearchSourceB = LibraryV0Source & {
  readonly sourceBTrack: RegExp
}

export type LibraryV0AdmittedSource<Source extends LibraryV0Source = LibraryV0GoldenSource> =
  Source & {
    readonly sourceId: string
  }

export function libraryV0GoldenSource(rootPath: string): LibraryV0GoldenSource {
  const rootTrack = /Root Track A\.wav/
  const nestedTrack = /Nested Track A\.wav/
  const descendantOnlyTrack = /Descendant Only A\.wav/

  return {
    rootPath,
    sourceName: /source-a\b/i,
    rootTrack,
    nestedTrack,
    descendantOnlyTrack,
    coverImage: /cover\.jpg/,
    readmeText: /readme\.txt/,
    audioRows: [rootTrack, nestedTrack, descendantOnlyTrack]
  }
}

export function libraryV0ScopedSearchSourceA(rootPath: string): LibraryV0ScopedSearchSourceA {
  return {
    ...libraryV0GoldenSource(rootPath),
    nestedFolder: /nested\b/i
  }
}

export function libraryV0ScopedSearchSourceB(rootPath: string): LibraryV0ScopedSearchSourceB {
  const sourceBTrack = /Only Track B\.wav/

  return {
    rootPath,
    sourceName: /source-b\b/i,
    sourceBTrack,
    audioRows: [sourceBTrack]
  }
}
