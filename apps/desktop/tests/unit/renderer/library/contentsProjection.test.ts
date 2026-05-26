import { describe, expect, it } from 'vitest'

import {
  contentsPolicyForVisibility,
  type ContentsBoundaryState
} from '../../../../src/renderer/library/boundary/contentsRead'
import type {
  BrowserState,
  DirectoryState,
  LoadedChildren,
  SourceState
} from '../../../../src/renderer/library/state'
import {
  projectState,
  type BrowserProjection
} from '../../../../src/renderer/library/tree/projection'
import {
  projectContents,
  type ContentProjection,
  type ContentRow
} from '../../../../src/renderer/library/contents/projection'
import type {
  ChildRow,
  EntryPoint,
  HierarchyCoverage,
  SourceFileVisibility
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/libraryNavigation/readRows'
import type { ContentsFileRow, ContentsResult } from '../../../../src/shared/libraryContents/read'

describe('projectContents', () => {
  it('projects selected source contents from contents state', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          primaryMediaRow('asset-1', 'track.wav', 'audio'),
          primaryMediaRow('asset-2', 'clip.mp4', 'video')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows.map(rowSummary)).toEqual([
      {
        id: 'asset-1',
        kind: 'file',
        label: 'track.wav',
        state: null,
        mediaClass: 'audio',
        availabilityState: 'available'
      },
      {
        id: 'asset-2',
        kind: 'file',
        label: 'clip.mp4',
        state: null,
        mediaClass: 'video',
        availabilityState: 'available'
      }
    ])
  })

  it('projects source-file-origin rows with distinct stable IDs', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          primaryMediaRow('library-asset:1', 'Promoted Track', 'audio', 'libraryAsset'),
          primaryMediaRow('source-file:2000', 'scanned.wav', 'audio', 'sourceFile')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows).toHaveLength(2)
    const rowIds = contents.rows.map((row) => row.id)
    expect(rowIds).toContain('library-asset:1')
    expect(rowIds).toContain('source-file:2000')
  })

  it('uses the selected directory label and contents result', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Album')])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:12',
      readyContents({ rows: [primaryMediaRow('asset-3', 'inside.wav', 'audio')] })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Album')
    expect(contents.rows.map((row) => row.label)).toEqual(['inside.wav'])
  })

  it('keeps selected directory contents primary-media scoped for image-only folders', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([
          directoryNode('50', 'Covers', undefined, {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete'
          })
        ])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:50',
      readyContents({ rows: [], state: 'empty' })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Covers')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No primary media found'
    })
  })

  it('shows load-more row and continuation detail when nextCursor exists', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      {
        kind: 'ready',
        requestKey: 'source:7',
        nextCursor: 'c2Y6...',
        accumulatedRows: [
          primaryMediaRow('asset-1', 'track.wav', 'audio'),
          primaryMediaRow('asset-2', 'clip.mp4', 'audio')
        ],
        result: {
          state: 'ready',
          result: {
            state: 'ready',
            scope: { kind: 'source', sourceId: '7' },
            policy: contentsPolicyForVisibility('performance'),
            recursion: 'recursive',
            rows: [
              primaryMediaRow('asset-1', 'track.wav', 'audio'),
              primaryMediaRow('asset-2', 'clip.mp4', 'audio')
            ],
            coverage: { state: 'complete', recursiveScopeComplete: true, emptyResultAuthoritative: true },
            nextCursor: 'c2Y6...'
          }
        }
      }
    )

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('2 primary media items loaded. More available.')
    expect(contents.rows).toHaveLength(3)
    expect(contents.rows[2]).toMatchObject({
      kind: 'more',
      label: 'More primary media items available',
      detail: 'Load more',
      icon: 'more',
      action: {
        kind: 'loadContentsPage',
        nodeId: 'navigation-row:7',
        label: 'Load more primary media items',
        cursor: 'c2Y6...'
      }
    })
  })

  it('shows accumulated row count in detail string when nextCursor exists', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      {
        kind: 'ready',
        requestKey: 'source:7',
        nextCursor: 'c2Y6...',
        accumulatedRows: [
          primaryMediaRow('a', 'first.wav', 'audio'),
          primaryMediaRow('b', 'second.wav', 'audio'),
          primaryMediaRow('c', 'third.wav', 'audio')
        ],
        result: {
          state: 'ready',
          result: {
            state: 'ready',
            scope: { kind: 'source', sourceId: '7' },
            policy: contentsPolicyForVisibility('performance'),
            recursion: 'recursive',
            rows: [primaryMediaRow('c', 'third.wav', 'audio')],
            coverage: { state: 'complete', recursiveScopeComplete: true, emptyResultAuthoritative: true },
            nextCursor: 'c2Y6...'
          }
        }
      }
    )

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('3 primary media items loaded. More available.')
    expect(contents.rows).toHaveLength(4)
  })

  it('shows page-local row count in detail string without nextCursor', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          primaryMediaRow('a', 'first.wav', 'audio'),
          primaryMediaRow('b', 'second.wav', 'audio'),
          primaryMediaRow('c', 'third.wav', 'audio')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('3 primary media items loaded.')
    expect(contents.rows).toHaveLength(3)
  })

  it('projects loading, empty, partial, failed, and unsupported contents read states', () => {
    expect(projectForSelection(browserState({}), 'navigation-row:7').rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Loading contents'
    })

    const empty = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'empty' })
    )
    expect(empty.kind).toBe('ready')
    expect(empty.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No primary media found'
    })

    const partial = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'partial' })
    )
    expect(partial.kind).toBe('ready')
    expect(partial.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Still indexing'
    })

    const failed = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'failed', detail: 'Read failed.' })
    )
    expect(failed.kind).toBe('failed')
    expect(failed.rows[0]).toMatchObject({
      kind: 'state',
      state: 'failed',
      label: 'Contents failed'
    })

    const missing = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'sourceUnavailable' })
    )
    expect(missing.kind).toBe('unsupported')
    expect(missing.rows[0]).toMatchObject({
      kind: 'state',
      state: 'unsupported',
      label: 'Source unavailable'
    })
  })

  it('projects empty selection, unsupported selection, host state, and literal files', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([fileNode('11', 'track.wav', 101)])
      }
    })
    const projection = browserProjection(state)

    expect(projectContents({ state, bindingsById: projection.bindingsById })).toMatchObject({
      kind: 'emptySelection',
      title: 'Library contents'
    })
    expect(
      projectContents({
        state,
        selectedNodeId: 'navigation-row:missing',
        bindingsById: projection.bindingsById
      })
    ).toMatchObject({ kind: 'unsupported', title: 'Selection unavailable' })

    const fileContents = projectContents({
      state,
      selectedNodeId: 'source-file:11',
      bindingsById: projection.bindingsById
    })
    expect(fileContents).toMatchObject({ kind: 'ready', title: 'track.wav' })
    expect(fileContents.rows[0]).toMatchObject({
      kind: 'state',
      label: 'File selected',
      state: 'file',
      detail: 'File'
    })

    const failedHost = projectContents({
      state: {
        ...state,
        hostStatus: {
          state: 'failed',
          environment: 'development',
          binaryPolicy: { kind: 'developmentBinary', source: 'repoDebugTarget' },
          lastError: { code: 'stdioTransportStartupFailure', message: 'Host failed' }
        }
      },
      bindingsById: projection.bindingsById
    })
    expect(failedHost.kind).toBe('failed')
    expect(failedHost.rows[0]).toMatchObject({
      state: 'failed',
      label: 'Library engine failed to start'
    })
  })
})

describe('projectContents performanceAndImages', () => {
  it('maps renderer visibility modes to contents read policies', () => {
    expect(contentsPolicyForVisibility('performance')).toEqual({
      mediaClasses: ['audio', 'video'],
      rowProfile: { kind: 'primaryMedia' }
    })
    expect(contentsPolicyForVisibility('performanceAndImages')).toEqual({
      mediaClasses: ['audio', 'video', 'image'],
      rowProfile: { kind: 'sourceFile' }
    })
  })

  it('shows image file rows for an image-only directory returned by source-file contents', () => {
    const directoryId = '50'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode(directoryId, 'Covers')])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(
      state,
      `source-directory:${directoryId}`,
      readyContents({
        rows: [sourceFileRow('img-1', 'Cover.jpg', 'image')],
        profile: 'sourceFile'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Covers')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'file',
      label: 'Cover.jpg',
      mediaClass: 'image',
      icon: 'image'
    })
    expect(contents.detail).toBe('1 visible file loaded.')
  })

  it('shows audio/video/image rows for mixed-media source-file contents', () => {
    const directoryId = '60'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode(directoryId, 'Mixed')])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(
      state,
      `source-directory:${directoryId}`,
      readyContents({
        rows: [
          sourceFileRow('a-1', 'track.flac', 'audio'),
          sourceFileRow('v-1', 'clip.mp4', 'video'),
          sourceFileRow('i-1', 'artwork.png', 'image')
        ],
        profile: 'sourceFile'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Mixed')
    expect(contents.rows).toHaveLength(3)
    expect(contents.rows.map((r) => r.mediaClass)).toEqual(['audio', 'video', 'image'])
    expect(contents.rows.map((r) => r.icon)).toEqual(['music', 'video', 'image'])
    expect(contents.detail).toBe('3 visible files loaded.')
  })

  it('waits for backend contents when image-inclusive directory contents are not loaded yet', () => {
    const directoryId = '70'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode(directoryId, 'Pending')])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(state, `source-directory:${directoryId}`)

    expect(contents.kind).toBe('loading')
    expect(contents.title).toBe('Pending')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Loading contents'
    })
  })

  it('uses visible-file wording for empty source-file contents', () => {
    const directoryId = '80'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode(directoryId, 'EmptyDir')])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(
      state,
      `source-directory:${directoryId}`,
      readyContents({
        rows: [],
        state: 'empty',
        profile: 'sourceFile'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('EmptyDir')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No visible files found'
    })
    expect(contents.detail).toBe('No visible files found')
  })

  it('shows visible files for a source in performanceAndImages mode', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(
      state,
      'navigation-row:7',
      readyContents({
        rows: [
          sourceFileRow('a-2', 'song.wav', 'audio'),
          sourceFileRow('i-2', 'folder.jpg', 'image')
        ],
        profile: 'sourceFile'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows).toHaveLength(2)
    expect(contents.rows[0]).toMatchObject({ label: 'song.wav', mediaClass: 'audio' })
    expect(contents.rows[1]).toMatchObject({ label: 'folder.jpg', mediaClass: 'image' })
    expect(contents.detail).toBe('2 visible files loaded.')
  })

  it('shows loading state while backend contents are loading', () => {
    const directoryId = '90'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode(directoryId, 'LoadingDir')])
      },
      sourceFileVisibility: 'performanceAndImages'
    })

    const contents = projectForSelection(state, `source-directory:${directoryId}`, {
      kind: 'loading',
      requestKey: 'directory:7:90:sourceFile:audio,video,image:recursive',
      sequence: 1,
      detail: 'Loading visible files.'
    })

    expect(contents.kind).toBe('loading')
    expect(contents.title).toBe('LoadingDir')
    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Loading contents'
    })
  })

  it('still reports no primary media for image-only folder in performance mode', () => {
    const directoryId = '95'
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([
          directoryNode(directoryId, 'Covers', undefined, {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete'
          })
        ])
      },
      sourceFileVisibility: 'performance'
    })

    const contents = projectForSelection(
      state,
      `source-directory:${directoryId}`,
      readyContents({ rows: [], state: 'empty' })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No primary media found'
    })
  })
})

function projectForSelection(
  state: BrowserState,
  selectedNodeId: string,
  contentsState?: ContentsBoundaryState
): ContentProjection {
  const projection = browserProjection(state)

  return projectContents({
    state,
    selectedNodeId,
    ...(contentsState === undefined ? {} : { contentsState }),
    bindingsById: projection.bindingsById
  })
}

function browserProjection(state: BrowserState): BrowserProjection {
  const projection = projectState(state)

  expect(projection?.kind).toBe('tree')
  if (projection?.kind !== 'tree') {
    throw new Error('Expected browser tree projection.')
  }

  return projection
}

function browserState(options: {
  readonly navigationReadResult?: NavigationReadRowsResult
  readonly sourceState?: SourceState
  readonly directoryStates?: ReadonlyMap<string, DirectoryState>
  readonly sourceFileVisibility?: SourceFileVisibility
}): BrowserState {
  const sourceStates = new Map<string, SourceState>()

  if (options.sourceState !== undefined) {
    sourceStates.set('navigation-row:7', options.sourceState)
  }

  return {
    navigationReadResult: options.navigationReadResult ?? {
      state: 'ready',
      rows: [sourceNavigationRow()]
    },
    sourceReadStates: sourceStates,
    directoryReadStates: options.directoryStates ?? new Map(),
    sourceFileVisibility: options.sourceFileVisibility ?? 'performance'
  }
}

function loadedChildren(
  rows: readonly ChildRow[],
  options: {
    readonly parentDirectoryId?: string
    readonly totalRows?: number
  } = {}
): LoadedChildren {
  const totalRows = options.totalRows ?? rows.length
  const nextOffset = rows.length < totalRows ? rows.length : undefined

  return {
    entryPoint: sourceEntryPoint(),
    label: 'Source Fixture',
    ...(options.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: options.parentDirectoryId }),
    sourceFileVisibility: 'performance',
    rows,
    totalRows,
    coverage: {
      state: 'complete',
      recursiveScopeComplete: true,
      emptyResultAuthoritative: rows.length === 0
    } satisfies HierarchyCoverage,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: 50
  }
}

function sourceNavigationRow(): NavigationRow {
  return {
    navigationRowId: '7',
    stableKey: 'source:7',
    parentNavigationRowId: null,
    family: 'sources',
    rowKind: 'source',
    displayName: 'Source Fixture',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '7',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string,
  options: {
    readonly hasChildDirectories?: boolean
    readonly directoryPrimaryMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryPrimaryMediaState']
    readonly directoryImageMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryImageMediaState']
    readonly directoryScanState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryScanState']
  } = {}
): Extract<ChildRow, { readonly kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: options.hasChildDirectories ?? true,
    directoryPrimaryMediaState: options.directoryPrimaryMediaState ?? {
      kind: 'hasPrimaryMediaDescendants'
    },
    directoryImageMediaState: options.directoryImageMediaState ?? {
      kind: 'noImageMediaDescendants'
    },
    directoryScanState: options.directoryScanState ?? 'scanning',
    updatedAtMs: 100
  }
}

function fileNode(
  fileId: string,
  label: string,
  updatedAtMs = 100,
  parentDirectoryId?: string
): Extract<ChildRow, { readonly kind: 'file' }> {
  return {
    id: `source-file:${fileId}`,
    kind: 'file',
    label,
    sourceId: '7',
    fileId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    mediaClass: 'audio',
    presence: 'present',
    updatedAtMs
  }
}

function sourceEntryPoint(): EntryPoint {
  return {
    kind: 'source',
    sourceId: '7'
  }
}

function readyContents(options: {
  readonly rows: readonly ContentsFileRow[]
  readonly state?: ContentsResult['state']
  readonly detail?: string
  readonly profile?: ContentsResult['policy']['rowProfile']['kind']
}): ContentsBoundaryState {
  return {
    kind: 'ready',
    requestKey: 'source:7',
    result: {
      state: 'ready',
      result: contentsResult(options)
    }
  }
}

function contentsResult(options: {
  readonly rows: readonly ContentsFileRow[]
  readonly state?: ContentsResult['state']
  readonly detail?: string
  readonly profile?: ContentsResult['policy']['rowProfile']['kind']
}): ContentsResult {
  const state = options.state ?? 'ready'
  const profile = options.profile ?? 'primaryMedia'
  return {
    state,
    scope: { kind: 'source', sourceId: '7' },
    policy:
      profile === 'sourceFile'
        ? contentsPolicyForVisibility('performanceAndImages')
        : contentsPolicyForVisibility('performance'),
    recursion: 'recursive',
    rows: options.rows,
    coverage: {
      state:
        state === 'failed'
          ? 'failed'
          : state === 'sourceUnavailable'
            ? 'sourceUnavailable'
            : 'complete',
      recursiveScopeComplete: state !== 'partial',
      emptyResultAuthoritative: state !== 'partial'
    },
    ...(options.detail === undefined ? {} : { detail: options.detail })
  }
}

function primaryMediaRow(
  stableId: string,
  label: string,
  mediaClass: Exclude<ContentsFileRow['mediaClass'], 'image'>,
  origin: NonNullable<ContentsFileRow['primaryMedia']>['origin'] = 'libraryAsset'
): ContentsFileRow {
  return {
    id: stableId,
    sourceId: '7',
    sourceFileId: `file-${stableId}`,
    label,
    relativePath: label,
    fileName: label,
    mediaClass,
    presence: 'present',
    availabilityState: 'available',
    primaryMedia: {
      origin,
      primarySourceFileId: `file-${stableId}`,
      ...(origin === 'libraryAsset' ? { libraryAssetId: stableId } : {}),
      ...(origin === 'libraryAsset' ? { rowVersion: '1' } : {}),
      ...(origin === 'libraryAsset' ? { artist: 'Artist' } : {}),
      ...(origin === 'libraryAsset' ? { album: 'Album' } : {}),
      prepReadinessSummary: origin === 'libraryAsset' ? 'notRequired' : 'underprepared'
    },
    updatedAtMs: 100
  }
}

function sourceFileRow(
  sourceFileId: string,
  label: string,
  mediaClass: ContentsFileRow['mediaClass']
): ContentsFileRow {
  return {
    id: `source-file:${sourceFileId}`,
    sourceId: '7',
    sourceFileId,
    label,
    relativePath: label,
    fileName: label,
    mediaClass,
    presence: 'present',
    updatedAtMs: 100
  }
}

function rowSummary(row: ContentRow): {
  readonly id: string
  readonly kind: ContentRow['kind']
  readonly label: string
  readonly state: ContentRow['state'] | null
  readonly mediaClass: ContentRow['mediaClass'] | null
  readonly availabilityState: ContentRow['availabilityState'] | null
} {
  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    state: row.state ?? null,
    mediaClass: row.mediaClass ?? null,
    availabilityState: row.availabilityState ?? null
  }
}
