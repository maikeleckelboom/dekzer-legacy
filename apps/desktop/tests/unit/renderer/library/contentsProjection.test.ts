import { describe, expect, it } from 'vitest'

import type { SelectedContentsBoundaryState } from '../../../../src/renderer/library/boundary/selectedContentsRead'
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
  HierarchyCoverage
} from '../../../../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/libraryNavigation/readRows'
import type {
  SelectedContentsResult,
  SelectedContentsRow
} from '../../../../src/shared/librarySelectedContents/read'

describe('projectContents', () => {
  it('projects selected source contents from selected contents state', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readySelectedContents({
        rows: [
          selectedRow('asset-1', 'track.wav', 'audio'),
          selectedRow('asset-2', 'clip.mp4', 'video')
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
      readySelectedContents({
        rows: [
          selectedRow('library-asset:1', 'Promoted Track', 'audio', 'libraryAsset'),
          selectedRow('source-file:2000', 'scanned.wav', 'audio', 'sourceFile')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows).toHaveLength(2)
    const rowIds = contents.rows.map((row) => row.id)
    expect(rowIds).toContain('library-asset:1')
    expect(rowIds).toContain('source-file:2000')
  })

  it('uses the selected directory label and selected contents result', () => {
    const state = browserState({
      sourceState: {
        kind: 'loaded',
        children: loadedChildren([directoryNode('12', 'Album')])
      }
    })
    const contents = projectForSelection(
      state,
      'source-directory:12',
      readySelectedContents({ rows: [selectedRow('asset-3', 'inside.wav', 'audio')] })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.title).toBe('Album')
    expect(contents.rows.map((row) => row.label)).toEqual(['inside.wav'])
  })

  it('projects loading, empty, partial, failed, and unsupported selected content states', () => {
    expect(projectForSelection(browserState({}), 'navigation-row:7').rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Loading selected contents'
    })

    const empty = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readySelectedContents({ rows: [], state: 'empty' })
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
      readySelectedContents({ rows: [], state: 'partial' })
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
      readySelectedContents({ rows: [], state: 'failed', detail: 'Read failed.' })
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
      readySelectedContents({ rows: [], state: 'sourceUnavailable' })
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

function projectForSelection(
  state: BrowserState,
  selectedNodeId: string,
  selectedContentsState?: SelectedContentsBoundaryState
): ContentProjection {
  const projection = browserProjection(state)

  return projectContents({
    state,
    selectedNodeId,
    ...(selectedContentsState === undefined ? {} : { selectedContentsState }),
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
    directoryReadStates: options.directoryStates ?? new Map(),
    sourceFileVisibility: 'performance'
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
  parentDirectoryId?: string
): Extract<ChildRow, { readonly kind: 'directory' }> {
  return {
    id: `source-directory:${directoryId}`,
    kind: 'directory',
    label,
    sourceId: '7',
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryPrimaryMediaState: { kind: 'hasPrimaryMediaDescendants' },
    directoryImageMediaState: { kind: 'noImageMediaDescendants' },
    directoryScanState: 'scanning',
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

function readySelectedContents(options: {
  readonly rows: readonly SelectedContentsRow[]
  readonly state?: SelectedContentsResult['state']
  readonly detail?: string
}): SelectedContentsBoundaryState {
  return {
    kind: 'ready',
    requestKey: 'source:7',
    result: {
      state: 'ready',
      result: selectedContentsResult(options)
    }
  }
}

function selectedContentsResult(options: {
  readonly rows: readonly SelectedContentsRow[]
  readonly state?: SelectedContentsResult['state']
  readonly detail?: string
}): SelectedContentsResult {
  const state = options.state ?? 'ready'
  return {
    state,
    scope: { kind: 'source', sourceId: '7' },
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

function selectedRow(
  stableId: string,
  label: string,
  mediaClass: SelectedContentsRow['mediaClass'],
  origin: SelectedContentsRow['origin'] = 'libraryAsset'
): SelectedContentsRow {
  return {
    stableId,
    label,
    origin,
    ...(origin === 'libraryAsset' ? { libraryAssetId: stableId } : {}),
    ...(origin === 'libraryAsset' ? { rowVersion: '1' } : {}),
    scopedSourceFileId: `file-${stableId}`,
    sourceId: '7',
    relativePath: label,
    fileName: label,
    mediaClass,
    availabilityState: 'available',
    ...(origin === 'libraryAsset' ? { artist: 'Artist' } : {}),
    ...(origin === 'libraryAsset' ? { album: 'Album' } : {}),
    prepReadinessSummary: origin === 'libraryAsset' ? 'notRequired' : 'underprepared',
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
