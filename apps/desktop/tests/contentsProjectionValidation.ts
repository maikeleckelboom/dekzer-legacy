import { strict as assert } from 'node:assert'

import {
  projectContents,
  type ContentProjection,
  type ContentRow
} from '../src/renderer/library/contents/projection'
import {
  projectState,
  type BrowserProjection
} from '../src/renderer/library/tree/projection'
import type {
  BrowserState,
  DirectoryState,
  LoadedChildren,
  SourceState
} from '../src/renderer/library/state'
import type { ChildRow, EntryPoint } from '../src/shared/libraryHierarchy/readChildren'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../src/shared/libraryNavigation/readRows'

void main()

function main(): void {
  validatesLoadedSourceContents()
  validatesLoadedDirectoryContents()
  validatesPartialWindowsExposeMoreActions()
  validatesEmptyLoadedContents()
  validatesFailedContents()
  validatesHostStatusOverridesEmptySelectionContents()
  validatesUnloadedContents()
  validatesUnsupportedNavigationContents()
  validatesLiteralFileSelection()
}

function validatesLoadedSourceContents(): void {
  const state = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album'), fileNode('11', 'track.wav', 101)])
    }
  })
  const contents = projectForSelection(state, 'navigation-row:7')

  assert.equal(contents.kind, 'ready')
  assert.equal(contents.title, 'Source Fixture')
  assert.deepEqual(contents.rows.map(rowSummary), [
    {
      id: 'source-directory:12',
      kind: 'directory',
      label: 'Album',
      presence: 'present',
      state: null,
      actionKind: null,
      actionNodeId: null
    },
    {
      id: 'source-file:11',
      kind: 'file',
      label: 'track.wav',
      presence: 'present',
      state: null,
      actionKind: null,
      actionNodeId: null
    }
  ])
}

function validatesLoadedDirectoryContents(): void {
  const state = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    },
    directoryStates: new Map([
      [
        '12',
        {
          kind: 'loaded',
          children: loadedChildren(
            [fileNode('12-a', 'inside.wav', 102, '12'), directoryNode('99', 'Nested', '12')],
            { parentDirectoryId: '12' }
          )
        }
      ]
    ])
  })
  const contents = projectForSelection(state, 'source-directory:12')

  assert.equal(contents.kind, 'ready')
  assert.equal(contents.title, 'Album')
  assert.deepEqual(contents.rows.map(rowSummary), [
    {
      id: 'source-file:12-a',
      kind: 'file',
      label: 'inside.wav',
      presence: 'present',
      state: null,
      actionKind: null,
      actionNodeId: null
    },
    {
      id: 'source-directory:99',
      kind: 'directory',
      label: 'Nested',
      presence: 'present',
      state: null,
      actionKind: null,
      actionNodeId: null
    }
  ])
}

function validatesPartialWindowsExposeMoreActions(): void {
  const partialSourceState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album'), fileNode('11', 'track.wav')], {
        totalRows: 3
      })
    }
  })
  const partialSourceContents = projectForSelection(partialSourceState, 'navigation-row:7')

  assert.deepEqual(
    partialSourceContents.rows.map((row) => [row.id, row.kind, row.action?.kind ?? null]),
    [
      ['source-directory:12', 'directory', null],
      ['source-file:11', 'file', null],
      ['more:navigation-row:7:2', 'more', 'loadMore']
    ]
  )
  assert.equal(partialSourceContents.rows[2]?.action?.nodeId, 'more:navigation-row:7:2')

  const partialDirectoryState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    },
    directoryStates: new Map([
      [
        '12',
        {
          kind: 'loaded',
          children: loadedChildren([fileNode('12-a', 'inside.wav', 102, '12')], {
            parentDirectoryId: '12',
            totalRows: 2
          })
        }
      ]
    ])
  })
  const partialDirectoryContents = projectForSelection(partialDirectoryState, 'source-directory:12')

  assert.deepEqual(
    partialDirectoryContents.rows.map((row) => [row.id, row.kind, row.action?.kind ?? null]),
    [
      ['source-file:12-a', 'file', null],
      ['more:source-directory:12:1', 'more', 'loadMore']
    ]
  )
  assert.equal(partialDirectoryContents.rows[1]?.action?.nodeId, 'more:source-directory:12:1')
}

function validatesEmptyLoadedContents(): void {
  const emptySourceState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([])
    }
  })
  const emptySourceContents = projectForSelection(emptySourceState, 'navigation-row:7')

  assert.equal(emptySourceContents.kind, 'ready')
  assert.deepEqual(emptySourceContents.rows.map(rowSummary), [
    {
      id: 'contents-state:navigation-row:7:empty',
      kind: 'state',
      label: 'Empty folder',
      presence: null,
      state: 'empty',
      actionKind: null,
      actionNodeId: null
    }
  ])

  const emptyDirectoryState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    },
    directoryStates: new Map([
      [
        '12',
        {
          kind: 'loaded',
          children: loadedChildren([], { parentDirectoryId: '12' })
        }
      ]
    ])
  })
  const emptyDirectoryContents = projectForSelection(emptyDirectoryState, 'source-directory:12')

  assert.equal(emptyDirectoryContents.kind, 'ready')
  assert.equal(emptyDirectoryContents.rows[0]?.state, 'empty')
}

function validatesFailedContents(): void {
  const failedSourceState = browserState({
    sourceState: {
      kind: 'failed',
      detail: 'Unable to read source contents.'
    }
  })
  const failedSourceContents = projectForSelection(failedSourceState, 'navigation-row:7')

  assert.equal(failedSourceContents.kind, 'failed')
  assert.equal(failedSourceContents.rows[0]?.state, 'failed')
  assert.deepEqual(failedSourceContents.rows[0]?.action, {
    kind: 'loadChildren',
    nodeId: 'navigation-row:7',
    label: 'Retry'
  })

  const failedDirectoryState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    },
    directoryStates: new Map([
      [
        '12',
        {
          kind: 'failed',
          detail: 'Unable to read folder contents.'
        }
      ]
    ])
  })
  const failedDirectoryContents = projectForSelection(failedDirectoryState, 'source-directory:12')

  assert.equal(failedDirectoryContents.kind, 'failed')
  assert.equal(failedDirectoryContents.rows[0]?.state, 'failed')
  assert.deepEqual(failedDirectoryContents.rows[0]?.action, {
    kind: 'loadChildren',
    nodeId: 'source-directory:12',
    label: 'Retry'
  })
}

function validatesHostStatusOverridesEmptySelectionContents(): void {
  const state = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    }
  })
  const projection = browserProjection(state)

  const failedContents = projectContents({
    state: {
      ...state,
      hostStatus: {
        state: 'failed',
        environment: 'development',
        binaryPolicy: {
          kind: 'developmentBinary',
          source: 'repoDebugTarget'
        },
        lastError: {
          code: 'stdioTransportStartupFailure',
          message: 'Host failed'
        }
      }
    },
    bindingsById: projection.bindingsById
  })

  assert.equal(failedContents.kind, 'failed')
  assert.equal(failedContents.rows[0]?.state, 'failed')
  assert.equal(failedContents.rows[0]?.label, 'Library engine failed to start')

  const stoppedContents = projectContents({
    state: {
      ...state,
      hostStatus: {
        state: 'stopped',
        environment: 'development',
        binaryPolicy: {
          kind: 'developmentBinary',
          source: 'repoDebugTarget'
        },
        lastError: null
      }
    },
    bindingsById: projection.bindingsById
  })

  assert.equal(stoppedContents.kind, 'unsupported')
  assert.equal(stoppedContents.rows[0]?.state, 'unsupported')
  assert.equal(stoppedContents.rows[0]?.label, 'Library engine unavailable')
}

function validatesUnloadedContents(): void {
  const unloadedSourceState = browserState({
    sourceState: {
      kind: 'unloaded',
      detail: 'Contents not loaded yet.'
    }
  })
  const unloadedSourceContents = projectForSelection(unloadedSourceState, 'navigation-row:7')

  assert.equal(unloadedSourceContents.kind, 'notLoaded')
  assert.equal(unloadedSourceContents.rows[0]?.state, 'notLoaded')
  assert.deepEqual(unloadedSourceContents.rows[0]?.action, {
    kind: 'loadChildren',
    nodeId: 'navigation-row:7',
    label: 'Load contents'
  })

  const unloadedDirectoryState = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([directoryNode('12', 'Album')])
    },
    directoryStates: new Map([
      [
        '12',
        {
          kind: 'unloaded',
          detail: 'Contents not loaded yet.'
        }
      ]
    ])
  })
  const unloadedDirectoryContents = projectForSelection(
    unloadedDirectoryState,
    'source-directory:12'
  )

  assert.equal(unloadedDirectoryContents.kind, 'notLoaded')
  assert.equal(unloadedDirectoryContents.rows[0]?.state, 'notLoaded')
  assert.deepEqual(unloadedDirectoryContents.rows[0]?.action, {
    kind: 'loadChildren',
    nodeId: 'source-directory:12',
    label: 'Load contents'
  })
}

function validatesUnsupportedNavigationContents(): void {
  const state = browserState({
    navigationReadResult: {
      state: 'ready',
      rows: [unsupportedNavigationRow()]
    }
  })
  const projection = browserProjection(state)
  const navigationLabels = projection.nodes
    .filter((node) => node.role !== 'state')
    .map((node) => node.label)

  assert.ok(
    !navigationLabels.includes('All audio'),
    'unsupported navigation row must not appear in visible tree'
  )
  assert.equal(
    navigationLabels.length,
    0,
    'only state placeholder should remain when no supported rows exist'
  )

  const contents = projectContents({
    state,
    selectedNodeId: 'navigation-row:8',
    bindingsById: projection.bindingsById
  })

  assert.equal(contents.kind, 'unsupported')
  assert.equal(contents.rows[0]?.state, 'unsupported')
}

function validatesLiteralFileSelection(): void {
  const state = browserState({
    sourceState: {
      kind: 'loaded',
      children: loadedChildren([fileNode('11', 'track.wav', 101)])
    }
  })
  const contents = projectForSelection(state, 'source-file:11')

  assert.equal(contents.kind, 'ready')
  assert.equal(contents.title, 'track.wav')
  assert.deepEqual(contents.rows.map(rowSummary), [
    {
      id: 'contents-state:source-file:11:file',
      kind: 'state',
      label: 'File selected',
      presence: null,
      state: 'file',
      actionKind: null,
      actionNodeId: null
    }
  ])
  assert.equal(contents.rows[0]?.detail, 'File')
}

function projectForSelection(state: BrowserState, selectedNodeId: string): ContentProjection {
  const projection = browserProjection(state)

  return projectContents({
    state,
    selectedNodeId,
    bindingsById: projection.bindingsById
  })
}

function browserProjection(state: BrowserState): BrowserProjection {
  const projection = projectState(state)

  assert.equal(projection?.kind, 'tree')
  if (projection?.kind !== 'tree') {
    assert.fail('expected browser tree projection')
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

function unsupportedNavigationRow(): NavigationRow {
  return {
    navigationRowId: '8',
    stableKey: 'view:all-audio',
    parentNavigationRowId: null,
    family: 'collections',
    rowKind: 'view',
    displayName: 'All audio',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'allAudio',
    selectorPayload: null,
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
    directoryId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
    presence: 'present',
    hasChildDirectories: true,
    directoryMediaState: { kind: 'hasMediaDescendants' },
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
    fileId,
    ...(parentDirectoryId === undefined ? {} : { parentDirectoryId }),
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

function rowSummary(row: ContentRow): {
  readonly id: string
  readonly kind: ContentRow['kind']
  readonly label: string
  readonly presence: ContentRow['presence'] | null
  readonly state: ContentRow['state'] | null
  readonly actionKind: NonNullable<ContentRow['action']>['kind'] | null
  readonly actionNodeId: NonNullable<ContentRow['action']>['nodeId'] | null
} {
  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    presence: row.presence ?? null,
    state: row.state ?? null,
    actionKind: row.action?.kind ?? null,
    actionNodeId: row.action?.nodeId ?? null
  }
}
