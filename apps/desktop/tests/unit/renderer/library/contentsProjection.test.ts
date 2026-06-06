import { describe, expect, it } from 'vitest'

import { type ContentsBoundaryState } from '../../../../src/renderer/library/boundary/contentsRead'
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
  ChildRowState,
  EntryPoint,
  HierarchyCoverage
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  ContentsFileRow,
  ContentsReadPolicy,
  ContentsResult
} from '../../../../src/shared/libraryContents/read'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/libraryNavigation/readRows'

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
        fileClass: 'audio',
        availabilityState: 'available'
      },
      {
        id: 'asset-2',
        kind: 'file',
        label: 'clip.mp4',
        state: null,
        fileClass: 'video',
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

  it('projects selected directory image inventory rows for image-only folders', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([
          directoryNode('50', 'Covers', undefined, {
            hasChildDirectories: false,
            directoryPrimaryMediaState: { kind: 'noPrimaryMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete',
            childRowState: 'noChildRows'
          })
        ])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:50',
      readyContents({
        profile: { kind: 'sourceFileInventory', fileClasses: ['image'] },
        rows: [sourceFileRow('cover-1', 'front.jpg', 'image')]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Covers')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'file',
      fileClass: 'image',
      label: 'front.jpg'
    })
  })

  it('projects cue sheet inventory rows as non-primary metadata files', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        profile: { kind: 'sourceFileInventory', fileClasses: ['unsupported'] },
        rows: [sourceFileRow('cue-1', 'album.cue', 'unsupported', 'cueSheet')]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows[0]).toMatchObject({
      kind: 'file',
      fileClass: 'unsupported',
      icon: 'cueSheet',
      label: 'album.cue'
    })
  })

  it('projects audio-browse file-row payloads without filtering or sorting', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        profile: { kind: 'audioBrowse' },
        rows: [sourceFileRow('b', 'B.wav', 'audio'), sourceFileRow('a', 'A.jpg', 'image')]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows.map((row) => row.label)).toEqual(['B.wav', 'A.jpg'])
    expect(contents.rows.every((row) => row.kind === 'file')).toBe(true)
  })

  it('projects MP4 source rows as video under playable-media browse', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        profile: { kind: 'playableMediaBrowse' },
        rows: [sourceFileRow('video-1', 'clip.mp4', 'video')]
      })
    )

    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'file',
      label: 'clip.mp4',
      fileClass: 'video',
      icon: 'video'
    })
    expect(contents.rows[0]).not.toHaveProperty('state', 'empty')
  })

  it('shows load-more row and continuation detail when nextCursor exists', () => {
    const contents = projectForSelection(browserState({}), 'navigation-row:7', {
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
          policy: { kind: 'primaryMedia', mediaKinds: ['audio', 'video'] },
          recursion: 'recursive',
          rows: [
            primaryMediaRow('asset-1', 'track.wav', 'audio'),
            primaryMediaRow('asset-2', 'clip.mp4', 'audio')
          ],
          coverage: {
            state: 'complete',
            recursiveScopeComplete: true,
            emptyResultAuthoritative: true
          },
          hasRowsOmittedByPolicy: false,
          nextCursor: 'c2Y6...'
        }
      }
    })

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
    const contents = projectForSelection(browserState({}), 'navigation-row:7', {
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
          policy: { kind: 'primaryMedia', mediaKinds: ['audio', 'video'] },
          recursion: 'recursive',
          rows: [primaryMediaRow('c', 'third.wav', 'audio')],
          coverage: {
            state: 'complete',
            recursiveScopeComplete: true,
            emptyResultAuthoritative: true
          },
          hasRowsOmittedByPolicy: false,
          nextCursor: 'c2Y6...'
        }
      }
    })

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
    expect(contents.detail).toBe('3 playable media items loaded.')
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
      label: 'No playable media found'
    })

    const partial = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'ready',
      requestKey: 'source:7:audioBrowse:recursive',
      result: {
        state: 'ready',
        result: {
          state: 'partial',
          scope: { kind: 'source', sourceId: '7' },
          policy: { kind: 'audioBrowse' },
          recursion: 'recursive',
          rows: [],
          coverage: {
            state: 'scanning',
            recursiveScopeComplete: false,
            emptyResultAuthoritative: false
          },
          hasRowsOmittedByPolicy: false
        }
      }
    })
    expect(partial.kind).toBe('ready')
    expect(partial.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Still indexing'
    })
    expect(partial.detail).toContain('Still indexing')

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

  it('projects explicit idle contents without pending as not loaded', () => {
    const idle = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'idle',
      detail: 'No contents scope is active.'
    })

    expect(idle.kind).toBe('notLoaded')
    expect(idle.rows[0]).toMatchObject({
      kind: 'state',
      state: 'notLoaded',
      label: 'Contents not loaded',
      detail: 'No contents scope is active.'
    })
  })

  it('does not project pending contents without an accepted response as authoritative empty', () => {
    const pending = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'idle',
      pending: {
        requestKey: 'source:7',
        sequence: 1,
        detail: 'Loading contents.',
        presentation: 'visible'
      },
      detail: 'Contents request is pending.'
    })

    expect(pending.kind).toBe('notLoaded')
    expect(pending.rows[0]).toMatchObject({
      kind: 'state',
      state: 'notLoaded',
      label: 'Contents pending'
    })
  })

  it('does not project initial pending contents as loading before threshold promotion', () => {
    const pending = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'idle',
      pending: {
        requestKey: 'source:7',
        sequence: 1,
        detail: 'Loading contents.',
        presentation: 'visible'
      },
      detail: 'Contents request is pending.'
    })

    expect(pending.kind).toBe('notLoaded')
    expect(pending.rows[0]).toMatchObject({
      kind: 'state',
      state: 'notLoaded',
      label: 'Contents pending'
    })
  })

  it('projects threshold-promoted initial loading as loading', () => {
    const loading = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'loading',
      requestKey: 'source:7',
      sequence: 1,
      detail: 'Loading contents.'
    })

    expect(loading.kind).toBe('loading')
    expect(loading.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Loading contents'
    })
  })

  it('keeps accepted rows visible for same-scope pending reads', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        pendingRequestKey: 'source:7'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(contents.rows[0]).not.toMatchObject({ state: 'loading' })
  })

  it('keeps the previous source projection during deferred cross-scope pending reads', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'New Album')])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        requestKey: 'source:7',
        pendingRequestKey: 'directory:7:12:audioBrowse:audio:recursive',
        pendingPresentation: 'deferred'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(contents.rows.map((row) => row.label)).not.toContain('Contents pending')
  })

  it('keeps the previous directory projection during deferred folder-to-folder pending reads', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([
          directoryNode('11', 'Old Album'),
          directoryNode('12', 'New Album')
        ])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        requestKey: 'directory:7:11:audioBrowse:audio:recursive',
        pendingRequestKey: 'directory:7:12:audioBrowse:audio:recursive',
        pendingPresentation: 'deferred'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Old Album')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(contents.title).not.toBe('New Album')
  })

  it('swaps directly to new accepted rows for fast cross-scope responses', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'New Album')])
      }
    })
    const deferred = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        requestKey: 'source:7',
        pendingRequestKey: 'directory:7:12:audioBrowse:audio:recursive',
        pendingPresentation: 'deferred'
      })
    )
    const accepted = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('new', 'new.wav', 'audio')],
        requestKey: 'directory:7:12:audioBrowse:audio:recursive'
      })
    )

    expect(deferred.title).toBe('Source Fixture')
    expect(deferred.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(accepted.title).toBe('New Album')
    expect(accepted.rows.map((row) => row.label)).toEqual(['new.wav'])
  })

  it('retains accepted rows for threshold-visible cross-scope pending reads', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'New Album')])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        requestKey: 'source:7',
        pendingRequestKey: 'directory:7:12:audioBrowse:audio:recursive',
        pendingPresentation: 'visible'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.detail).toBe('Updating selected contents. 1 playable media item loaded.')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
  })

  it('does not let disclosure-only branch loading replace selected contents', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'New Album')])
      },
      directoryStates: new Map([
        [
          '12',
          {
            kind: 'loading',
            requestKey: 'source:7/directory:12',
            sequence: 1,
            detail: 'Loading children.'
          }
        ]
      ])
    })
    const contents = projectForSelection(
      state,
      'navigation-row:7',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(contents.rows[0]).not.toMatchObject({ state: 'loading' })
  })

  it('retains accepted rows after a failed refresh without projecting blocking failure', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [sourceFileRow('old', 'old.wav', 'audio')],
        refreshError: 'Unable to request library contents.'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows.map((row) => row.label)).toEqual(['old.wav'])
    expect(contents.rows[0]).not.toMatchObject({ state: 'failed' })
  })

  it('distinguishes incomplete zero rows from complete authoritative empty', () => {
    const pendingCoverage = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'partial', emptyAuthoritative: false })
    )

    expect(pendingCoverage.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Still indexing'
    })

    const authoritative = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({ rows: [], state: 'empty', emptyAuthoritative: true })
    )

    expect(authoritative.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No playable media found'
    })
  })

  it('projects policy-empty copy from service-owned omission metadata', () => {
    const audioBrowse = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: { kind: 'audioBrowse' },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )
    expect(audioBrowse.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No audio tracks in this view',
      detail: 'No audio tracks in this view'
    })

    const playableMediaBrowse = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: { kind: 'playableMediaBrowse' },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )
    expect(playableMediaBrowse.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No playable media in this view'
    })
  })

  it('keeps incomplete zero-row omission results in indexing presentation', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'partial',
        profile: { kind: 'audioBrowse' },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Still indexing'
    })
  })

  it('projects empty selection, unsupported selection, and host state', () => {
    const state = browserState({})
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

  it('still projects explicit non-tree file bindings for inventory/detail paths', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([fileNode('11', 'track.wav', 101)])
      }
    })
    const bindingsById: BrowserProjection['bindingsById'] = new Map([
      [
        'source-file:11',
        {
          kind: 'file',
          sourceId: '7',
          fileId: '11',
          entryPoint: sourceEntryPoint()
        }
      ]
    ])

    const fileContents = projectContents({
      state,
      selectedNodeId: 'source-file:11',
      bindingsById
    })

    expect(fileContents).toMatchObject({ kind: 'ready', title: 'track.wav' })
    expect(fileContents.rows[0]).toMatchObject({
      kind: 'state',
      label: 'File selected',
      state: 'file',
      detail: 'File'
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
    directoryReadStates: options.directoryStates ?? new Map()
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
    readonly childRowState?: ChildRowState
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
    childRowState: options.childRowState ?? 'hasChildRows',
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
    fileClass: 'audio',
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
  readonly profile?: ContentsReadPolicy
  readonly emptyAuthoritative?: boolean
  readonly omittedRows?: boolean
  readonly requestKey?: string
  readonly pendingRequestKey?: string
  readonly pendingPresentation?: 'deferred' | 'visible'
  readonly refreshError?: string
}): ContentsBoundaryState {
  return {
    kind: 'ready',
    requestKey: options.requestKey ?? 'source:7',
    result: {
      state: 'ready',
      result: contentsResult(options)
    },
    ...(options.pendingRequestKey === undefined
      ? {}
      : {
          pending: {
            requestKey: options.pendingRequestKey,
            sequence: 2,
            detail: 'Loading contents.',
            presentation: options.pendingPresentation ?? 'visible'
          }
        }),
    ...(options.refreshError === undefined ? {} : { refreshError: options.refreshError })
  }
}

function contentsResult(options: {
  readonly rows: readonly ContentsFileRow[]
  readonly state?: ContentsResult['state']
  readonly detail?: string
  readonly profile?: ContentsReadPolicy
  readonly emptyAuthoritative?: boolean
  readonly omittedRows?: boolean
}): ContentsResult {
  const state = options.state ?? 'ready'
  const policy = options.profile ?? ({ kind: 'playableMediaBrowse' } satisfies ContentsReadPolicy)
  return {
    state,
    scope: { kind: 'source', sourceId: '7' },
    policy,
    recursion: 'recursive',
    rows: options.rows,
    coverage: {
      state:
        state === 'failed'
          ? 'failed'
          : state === 'sourceUnavailable'
            ? 'sourceUnavailable'
            : state === 'partial'
              ? 'scanning'
              : 'complete',
      recursiveScopeComplete: state !== 'partial',
      emptyResultAuthoritative: options.emptyAuthoritative ?? state !== 'partial'
    },
    hasRowsOmittedByPolicy: options.omittedRows ?? false,
    ...(options.detail === undefined ? {} : { detail: options.detail })
  }
}

function primaryMediaRow(
  stableId: string,
  label: string,
  fileClass: Exclude<ContentsFileRow['fileClass'], 'image' | 'unsupported'>,
  origin: NonNullable<ContentsFileRow['primaryMedia']>['origin'] = 'libraryAsset'
): ContentsFileRow {
  return {
    id: stableId,
    sourceId: '7',
    sourceFileId: `file-${stableId}`,
    label,
    relativePath: label,
    fileName: label,
    fileClass,
    fileKind: fileClass,
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
  stableId: string,
  label: string,
  fileClass: ContentsFileRow['fileClass'],
  fileKind: ContentsFileRow['fileKind'] = fileClass === 'unsupported' ? 'cueSheet' : fileClass
): ContentsFileRow {
  return {
    id: `source-file:${stableId}`,
    sourceId: '7',
    sourceFileId: `file-${stableId}`,
    label,
    relativePath: label,
    fileName: label,
    fileClass,
    fileKind,
    presence: 'present',
    updatedAtMs: 100
  }
}

function rowSummary(row: ContentRow): {
  readonly id: string
  readonly kind: ContentRow['kind']
  readonly label: string
  readonly state: ContentRow['state'] | null
  readonly fileClass: ContentRow['fileClass'] | null
  readonly availabilityState: ContentRow['availabilityState'] | null
} {
  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    state: row.state ?? null,
    fileClass: row.fileClass ?? null,
    availabilityState: row.availabilityState ?? null
  }
}
