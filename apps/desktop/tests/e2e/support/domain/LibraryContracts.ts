export type LibraryBrowseProfileLabel = 'Audio' | 'Audio + Video' | 'All Files'

export type LibraryV0GoldenSource = {
  readonly rootPath: string
  readonly sourceName: RegExp
  readonly rootTrack: RegExp
  readonly nestedTrack: RegExp
  readonly descendantOnlyTrack: RegExp
  readonly coverImage: RegExp
  readonly readmeText: RegExp
  readonly audioRows: readonly RegExp[]
}

export type LibraryV0AdmittedSource = LibraryV0GoldenSource & {
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
