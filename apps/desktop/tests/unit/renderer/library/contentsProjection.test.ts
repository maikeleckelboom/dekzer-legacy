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
import { projectAddSourceState } from '../../../../src/renderer/library/localBrowse/projection'
import {
  projectContents,
  type ContentProjection,
  type ContentRow
} from '../../../../src/renderer/library/contents/projection'
import type {
  ChildRow,
  NavigableChildScopeState,
  EntryPoint,
  HierarchyCoverage
} from '../../../../src/shared/library/hierarchy/read'
import type {
  ContentsFileRow,
  ContentsReadPolicy,
  ContentsResult
} from '../../../../src/shared/library/contents/read'
import type {
  NavigationReadRowsResult,
  NavigationRow
} from '../../../../src/shared/library/navigation/read'
import type { LocalBrowseEntryPoint } from '../../../../src/shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  LocalBrowseItemKind
} from '../../../../src/shared/library/localBrowse/items'
import {
  localBrowseWindowKey,
  type LoadedLocalBrowseItems,
  type LocalBrowseItemState
} from '../../../../src/renderer/library/localBrowse/types'
import type { LibraryBrowseProfile } from '../../../../src/renderer/library/libraryBrowseProfile/types'
import type { AddSourceView } from '../../../../src/renderer/library/addSource/view'

describe('projectContents', () => {
  it('projects selected source contents from contents state', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          playableMediaRow('asset-1', 'track.wav', 'audio'),
          playableMediaRow('asset-2', 'clip.mp4', 'video')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.surfaceLabel).toBe('Contents')
    expect(contents.surfaceKind).toBe('indexedContents')
    expect(contents.title).toBe('Source Fixture')
    expect(contents.rows.map(rowSummary)).toEqual([
      {
        id: 'asset-1',
        kind: 'file',
        label: 'track.wav',
        state: null,
        fileClass: 'audio'
      },
      {
        id: 'asset-2',
        kind: 'file',
        label: 'clip.mp4',
        state: null,
        fileClass: 'video'
      }
    ])
  })

  it('projects playable-media rows with distinct stable IDs', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          playableMediaRow('playable-media:1', 'Promoted Track', 'audio'),
          playableMediaRow('source-file:2000', 'scanned.wav', 'audio')
        ]
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.rows).toHaveLength(2)
    const rowIds = contents.rows.map((row) => row.id)
    expect(rowIds).toContain('playable-media:1')
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
      readyContents({ rows: [playableMediaRow('asset-3', 'inside.wav', 'audio')] })
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
            directoryPlayableMediaState: { kind: 'noPlayableMediaDescendants' },
            directoryImageMediaState: { kind: 'hasImageMediaDescendants' },
            directoryScanState: 'complete',
            navigableChildScopeState: 'noNavigableChildScopes'
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

  it('projects cue sheet inventory rows as non-playable metadata files', () => {
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

  it('projects selected loaded local browse folders from local browse item windows', () => {
    const rootWindow = localBrowseWindow({
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items: [
        localBrowseItem('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums'),
        localBrowseItem('mediaFile', 'loose.flac', 'C:\\Users\\Maikel\\Music\\loose.flac', {
          fileKind: 'audio',
          mediaRelevance: 'mediaRelevant'
        })
      ]
    })
    const albumWindow = localBrowseWindow({
      label: 'Albums',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music\\Albums',
      items: [
        localBrowseItem('mediaFile', 'track.flac', 'C:\\Users\\Maikel\\Music\\Albums\\track.flac', {
          fileKind: 'audio',
          mediaRelevance: 'mediaRelevant'
        })
      ]
    })
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(rootWindow), { kind: 'loaded', window: rootWindow }],
        [localBrowseWindowStateKey(albumWindow), { kind: 'loaded', window: albumWindow }]
      ])
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const rootContents = projectContents({
      state,
      selectedNodeId: entryNode.id,
      bindingsById: projection.bindingsById
    })

    expect(rootContents.kind).toBe('ready')
    expect(rootContents.surfaceLabel).toBe('Source Preview')
    expect(rootContents.surfaceKind).toBe('sourcePreview')
    expect(rootContents.title).toBe('Music')
    expect(rootContents.rows.map((row) => row.label)).toEqual(['Albums', 'loose.flac'])
    expect(
      rootContents.rows.every((row) => row.action?.kind !== 'requestLocalBrowseAdmission')
    ).toBe(true)
    expect(rootContents.rows.some((row) => row.label === 'Contents unavailable')).toBe(false)

    const albumNode = entryNode.children.kind === 'loaded' ? entryNode.children.nodes[0] : undefined
    if (albumNode === undefined) {
      throw new Error('Expected loaded album node.')
    }
    const albumContents = projectContents({
      state,
      selectedNodeId: albumNode.id,
      bindingsById: projection.bindingsById
    })

    expect(albumContents.kind).toBe('ready')
    expect(albumContents.surfaceLabel).toBe('Source Preview')
    expect(albumContents.title).toBe('Albums')
    expect(albumContents.rows.map((row) => row.label)).toEqual(['track.flac'])
    expect(
      albumContents.rows.every((row) => row.action?.kind !== 'requestLocalBrowseAdmission')
    ).toBe(true)
  })

  it('projects selected unloaded local browse folders as loadable instead of unsupported', () => {
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()]
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const contents = projectContents({
      state,
      selectedNodeId: entryNode.id,
      bindingsById: projection.bindingsById
    })

    expect(contents.kind).toBe('notLoaded')
    expect(contents.surfaceLabel).toBe('Source Preview')
    expect(contents.title).toBe('Music')
    expect(contents.detail).toContain('Preview local folders and media signals')
    expect(contents.rows.map((row) => row.label)).toEqual(['Folder preview not loaded'])
    expect(contents.rows[0]).toMatchObject({
      state: 'notLoaded',
      action: {
        kind: 'loadLocalBrowseChildren',
        nodeId: entryNode.id,
        label: 'Load preview'
      }
    })
  })

  it('projects Add Source root as admission guidance without library-empty copy', () => {
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()]
    })
    const projection = browserProjection(state)
    const contents = projectContents({
      state,
      selectedNodeId: 'add-source:section',
      bindingsById: projection.bindingsById
    })

    expect(contents.kind).toBe('ready')
    expect(contents.surfaceLabel).toBe('Add Source')
    expect(contents.surfaceKind).toBe('addSource')
    expect(contents.title).toBe('Add Source')
    expect(contents.detail).toContain('managed source')
    expect(contents.rows).toEqual([
      expect.objectContaining({
        kind: 'state',
        label: 'Add a music folder',
        action: {
          kind: 'chooseMusicFolder',
          label: 'Add music folder'
        }
      })
    ])
    expect(`${contents.title} ${contents.detail}`).not.toMatch(
      /Unknown|Unsupported|No library contents/
    )
  })

  it('demotes system-drive root preview to plausible music-source candidates', () => {
    const systemRoot = 'C:\\'
    const rootWindow = localBrowseWindow({
      label: 'System Drive',
      entryPointKind: 'systemDriveRoot',
      resolvedRootPath: systemRoot,
      resolvedParentPath: systemRoot,
      items: [
        localBrowseItem('rejectedRoot', 'Windows', 'C:\\Windows', {
          identity: {
            entryPointKind: 'systemDriveRoot',
            resolvedRootPath: systemRoot,
            resolvedItemPath: 'C:\\Windows'
          },
          status: 'rejected',
          failure: {
            code: 'rejectedRoot',
            detail: 'system-owned directory is not directly admissible as a source root'
          }
        }),
        localBrowseItem('directory', 'Music', 'C:\\Music', {
          identity: {
            entryPointKind: 'systemDriveRoot',
            resolvedRootPath: systemRoot,
            resolvedItemPath: 'C:\\Music'
          },
          availableOperations: [
            { kind: 'browseChildren' },
            { kind: 'chooseDescendant' },
            {
              kind: 'requestSourceAdmission',
              requestKind: 'selectedDirectory',
              resolvedPath: 'C:\\Music'
            }
          ]
        })
      ]
    })
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [
        localBrowseEntryPoint({
          identity: { entryPointKind: 'systemDriveRoot', resolvedPath: systemRoot },
          displayName: 'System Drive',
          availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
        })
      ],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(rootWindow), { kind: 'loaded', window: rootWindow }]
      ])
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const contents = projectContents({
      state,
      selectedNodeId: entryNode.id,
      bindingsById: projection.bindingsById
    })

    expect(contents.surfaceLabel).toBe('Source Preview')
    expect(contents.title).toBe('System Drive')
    expect(contents.detail).toContain('plausible music-source candidate')
    expect(contents.rows.map((row) => row.label)).toEqual(['Music'])
    expect(contents.rows.some((row) => row.label === 'Windows')).toBe(false)
  })

  it('keeps raw local inventory out of normal admission but available in Inventory', () => {
    const items = [
      localBrowseItem('directory', 'Albums', 'C:\\Users\\Maikel\\Music\\Albums'),
      localBrowseItem('unsupportedFile', 'notes.txt', 'C:\\Users\\Maikel\\Music\\notes.txt', {
        fileKind: 'textDoc',
        mediaRelevance: 'unsupported'
      }),
      localBrowseItem('unknown', 'mystery', 'C:\\Users\\Maikel\\Music\\mystery', {
        status: 'unknown',
        fileKind: 'unknown',
        mediaRelevance: 'unknown'
      })
    ]
    const audioWindow = localBrowseWindow({
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items
    })
    const inventoryWindow = localBrowseWindow({
      addSourceView: 'inventory',
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items
    })

    const audioState = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(audioWindow), { kind: 'loaded', window: audioWindow }]
      ])
    })
    const audioProjection = browserProjection(audioState)
    const audioEntryNode = firstLocalBrowseEntryNode(audioProjection)
    const audioContents = projectContents({
      state: audioState,
      selectedNodeId: audioEntryNode.id,
      bindingsById: audioProjection.bindingsById
    })

    expect(audioContents.rows.map((row) => row.label)).toEqual(['Albums'])
    expect(audioContents.surfaceLabel).toBe('Source Preview')

    const inventoryState = browserState({
      addSourceView: 'inventory',
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(inventoryWindow), { kind: 'loaded', window: inventoryWindow }]
      ])
    })
    const inventoryProjection = browserProjection(inventoryState)
    const inventoryEntryNode = firstLocalBrowseEntryNode(inventoryProjection)
    const inventoryContents = projectContents({
      state: inventoryState,
      selectedNodeId: inventoryEntryNode.id,
      bindingsById: inventoryProjection.bindingsById
    })

    expect(inventoryContents.surfaceLabel).toBe('Source Inventory')
    expect(inventoryContents.surfaceKind).toBe('sourceInventory')
    expect(inventoryContents.detail).toContain('local inventory items shown')
    expect(inventoryContents.rows.map((row) => row.label)).toEqual([
      'Albums',
      'notes.txt',
      'mystery'
    ])
  })

  it('marks duplicate local browse occurrence as already added without add action', () => {
    const rootWindow = localBrowseWindow({
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items: [
        localBrowseItem('directory', 'Admitted', 'C:\\Users\\Maikel\\Music\\Admitted', {
          status: 'duplicateOfAdmittedSource',
          availableOperations: [{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }]
        })
      ]
    })
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(rootWindow), { kind: 'loaded', window: rootWindow }]
      ])
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const contents = projectContents({
      state,
      selectedNodeId: entryNode.id,
      bindingsById: projection.bindingsById
    })

    expect(contents.rows).toEqual([
      expect.objectContaining({
        label: 'Admitted',
        detail: 'Already added as a library source.'
      })
    ])
    expect(contents.rows[0]?.action).toBeUndefined()
  })

  it('projects protected reparse local browse items as terminal state without echoing the item', () => {
    const link = localBrowseItem(
      'inaccessible',
      'OutsideLink',
      'C:\\Users\\Maikel\\Music\\OutsideLink',
      {
        status: 'rejected',
        failure: {
          code: 'reparsePointSkipped',
          detail: 'Item is a reparse point and was not followed.'
        }
      }
    )
    const rootWindow = localBrowseWindow({
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items: [link]
    })
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(rootWindow), { kind: 'loaded', window: rootWindow }]
      ])
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const linkNode = entryNode.children.kind === 'loaded' ? entryNode.children.nodes[0] : undefined
    if (linkNode === undefined) {
      throw new Error('Expected protected local browse item node.')
    }

    const contents = projectContents({
      state,
      selectedNodeId: linkNode.id,
      bindingsById: projection.bindingsById
    })

    expect(contents.kind).toBe('unsupported')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      label: 'Protected location',
      detail: 'Item is a reparse point and was not followed.'
    })
    expect(contents.rows[0]?.label).not.toBe('OutsideLink')
  })

  it('projects local browse load-more from contents without switching to durable rows', () => {
    const rootWindow = localBrowseWindow({
      label: 'Music',
      resolvedParentPath: 'C:\\Users\\Maikel\\Music',
      items: [localBrowseItem('directory', 'A', 'C:\\Users\\Maikel\\Music\\A')],
      totalItems: 3,
      nextOffset: 1
    })
    const state = browserState({
      navigationReadResult: emptyNavigation(),
      entries: [localBrowseEntryPoint()],
      localBrowseItemStates: new Map([
        [localBrowseWindowStateKey(rootWindow), { kind: 'loaded', window: rootWindow }]
      ])
    })
    const projection = browserProjection(state)
    const entryNode = firstLocalBrowseEntryNode(projection)
    const contents = projectContents({
      state,
      selectedNodeId: entryNode.id,
      bindingsById: projection.bindingsById
    })

    expect(contents.kind).toBe('ready')
    expect(contents.rows.map((row) => row.kind)).toEqual(['directory', 'more'])
    expect(contents.rows[1]).toMatchObject({
      label: 'Load more',
      action: {
        kind: 'loadLocalBrowseMore',
        nodeId: entryNode.id
      }
    })
    expect(contents.rows.some((row) => row.id.startsWith('source-'))).toBe(false)
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
        playableMediaRow('asset-1', 'track.wav', 'audio'),
        playableMediaRow('asset-2', 'clip.mp4', 'audio')
      ],
      result: {
        state: 'ready',
        result: {
          state: 'ready',
          scope: { kind: 'source', sourceId: '7' },
          policy: { kind: 'playableMedia', mediaKinds: ['audio', 'video'] },
          scopeDepth: 'recursive',
          rows: [
            playableMediaRow('asset-1', 'track.wav', 'audio'),
            playableMediaRow('asset-2', 'clip.mp4', 'audio')
          ],
          scopeCoverage: {
            state: 'complete',
            subtreeCoverageComplete: true,
            emptyResultAuthoritative: true
          },
          hasPolicyOmittedRows: false,
          nextCursor: 'c2Y6...'
        }
      }
    })

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('2 playable media items loaded. More available.')
    expect(contents.rows).toHaveLength(3)
    expect(contents.rows[2]).toMatchObject({
      kind: 'more',
      label: 'More playable media items available',
      detail: 'Load more',
      icon: 'more',
      action: {
        kind: 'loadContentsPage',
        nodeId: 'navigation-row:7',
        label: 'Load more playable media items',
        cursor: 'c2Y6...'
      }
    })
  })

  it('projects zero rows plus nextCursor as continuation instead of empty', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        nextCursor: 'c2Y6...'
      })
    )

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('0 playable media items loaded. More available.')
    expect(contents.rows).toHaveLength(1)
    expect(contents.rows[0]).toMatchObject({
      kind: 'more',
      label: 'More playable media items available',
      action: {
        kind: 'loadContentsPage',
        cursor: 'c2Y6...'
      }
    })
    expect(contents.rows[0]).not.toMatchObject({ state: 'empty' })
  })

  it('does not render verified-empty policy labels while nextCursor exists', () => {
    const verifiedEmptyLabels = [
      'No audio tracks',
      'No video items in this scope.',
      'No companion files in this scope.',
      'No files'
    ]

    for (const profile of [
      { kind: 'audioBrowse' },
      { kind: 'playableMedia', mediaKinds: ['video'] },
      { kind: 'sourceFileInventory', fileClasses: ['unsupported'] },
      {
        kind: 'sourceFileInventory',
        fileClasses: ['audio', 'video', 'image', 'unsupported']
      }
    ] satisfies readonly ContentsReadPolicy[]) {
      const contents = projectForSelection(
        browserState({}),
        'navigation-row:7',
        readyContents({
          rows: [],
          state: 'empty',
          profile,
          emptyAuthoritative: false,
          omittedRows: true,
          nextCursor: 'c2Y6...'
        })
      )
      const projectedText = [
        contents.detail,
        ...contents.rows.flatMap((row) => [row.label, row.detail])
      ]
        .filter((value): value is string => value !== undefined)
        .join('\n')

      expect(contents.rows[0]).toMatchObject({ kind: 'more' })
      for (const label of verifiedEmptyLabels) {
        expect(projectedText).not.toContain(label)
      }
    }
  })

  it('shows accumulated row count in detail string when nextCursor exists', () => {
    const contents = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'ready',
      requestKey: 'source:7',
      nextCursor: 'c2Y6...',
      accumulatedRows: [
        playableMediaRow('a', 'first.wav', 'audio'),
        playableMediaRow('b', 'second.wav', 'audio'),
        playableMediaRow('c', 'third.wav', 'audio')
      ],
      result: {
        state: 'ready',
        result: {
          state: 'ready',
          scope: { kind: 'source', sourceId: '7' },
          policy: { kind: 'playableMedia', mediaKinds: ['audio', 'video'] },
          scopeDepth: 'recursive',
          rows: [playableMediaRow('c', 'third.wav', 'audio')],
          scopeCoverage: {
            state: 'complete',
            subtreeCoverageComplete: true,
            emptyResultAuthoritative: true
          },
          hasPolicyOmittedRows: false,
          nextCursor: 'c2Y6...'
        }
      }
    })

    expect(contents.kind).toBe('ready')
    expect(contents.detail).toBe('3 playable media items loaded. More available.')
    expect(contents.rows).toHaveLength(4)
  })

  it('shows page-local row count in detail string without nextCursor', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [
          playableMediaRow('a', 'first.wav', 'audio'),
          playableMediaRow('b', 'second.wav', 'audio'),
          playableMediaRow('c', 'third.wav', 'audio')
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
      label: 'No playable media'
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
          scopeDepth: 'recursive',
          rows: [],
          scopeCoverage: {
            state: 'scanning',
            subtreeCoverageComplete: false,
            emptyResultAuthoritative: false
          },
          hasPolicyOmittedRows: false
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
        pendingRequestKey: 'directory:7:12:audioBrowse:recursive',
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
        requestKey: 'directory:7:11:audioBrowse:recursive',
        pendingRequestKey: 'directory:7:12:audioBrowse:recursive',
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
        pendingRequestKey: 'directory:7:12:audioBrowse:recursive',
        pendingPresentation: 'deferred'
      })
    )
    const accepted = projectForSelection(
      state,
      'source-directory:12',
      readyContents({
        rows: [sourceFileRow('new', 'new.wav', 'audio')],
        requestKey: 'directory:7:12:audioBrowse:recursive'
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
        pendingRequestKey: 'directory:7:12:audioBrowse:recursive',
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
      label: 'No playable media'
    })
  })

  it('renders verified-empty policy copy when the cursor is exhausted', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: { kind: 'audioBrowse' },
        emptyAuthoritative: true
      })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No audio tracks',
      detail: 'No audio tracks'
    })
  })

  it('does not verify empty when only the raw contents result carries nextCursor', () => {
    const contents = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'ready',
      requestKey: 'source:7',
      result: {
        state: 'ready',
        result: contentsResult({
          rows: [],
          state: 'empty',
          profile: { kind: 'audioBrowse' },
          emptyAuthoritative: true,
          nextCursor: 'c2Y6...'
        })
      }
    })

    expect(contents.rows[0]).toMatchObject({
      kind: 'more',
      label: 'More audio tracks available'
    })
    expect(contents.rows[0]).not.toMatchObject({ state: 'empty' })
  })

  it('does not verify empty from complete zero rows without explicit empty evidence', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        coverageState: 'complete',
        subtreeCoverageComplete: true,
        emptyAuthoritative: false,
        omittedRows: false
      })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Contents coverage unverified'
    })
  })

  it('does not verify empty without complete subtree coverage', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        coverageState: 'complete',
        subtreeCoverageComplete: false,
        emptyAuthoritative: true
      })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Contents coverage unverified'
    })
  })

  it('does not verify empty from pending zero-row coverage', () => {
    const contents = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        coverageState: 'pending',
        subtreeCoverageComplete: false,
        emptyAuthoritative: true
      })
    )

    expect(contents.rows[0]).toMatchObject({
      kind: 'state',
      state: 'loading',
      label: 'Still indexing'
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
      label: 'No audio tracks',
      detail: 'No audio tracks'
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
      label: 'No playable media'
    })

    const video = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: { kind: 'playableMedia', mediaKinds: ['video'] },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )
    expect(video.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No video items in this scope.'
    })

    const companionFiles = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: { kind: 'sourceFileInventory', fileClasses: ['unsupported'] },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )
    expect(companionFiles.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No companion files in this scope.'
    })

    const allFiles = projectForSelection(
      browserState({}),
      'navigation-row:7',
      readyContents({
        rows: [],
        state: 'empty',
        profile: {
          kind: 'sourceFileInventory',
          fileClasses: ['audio', 'video', 'image', 'unsupported']
        },
        emptyAuthoritative: false,
        omittedRows: true
      })
    )
    expect(allFiles.rows[0]).toMatchObject({
      kind: 'state',
      state: 'empty',
      label: 'No files'
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

  it('does not project unavailable or terminal zero-row states as empty', () => {
    for (const [state, coverageState, expectedState, expectedLabel] of [
      ['blocked', 'blocked', 'failed', 'Contents blocked'],
      ['failed', 'failed', 'failed', 'Contents failed'],
      ['locationMissing', 'locationMissing', 'unsupported', 'Folder missing'],
      ['sourceUnavailable', 'sourceUnavailable', 'unsupported', 'Source unavailable']
    ] as const) {
      const contents = projectForSelection(
        browserState({}),
        'navigation-row:7',
        readyContents({
          rows: [],
          state,
          coverageState,
          subtreeCoverageComplete: false,
          emptyAuthoritative: false
        })
      )

      expect(contents.rows[0], state).toMatchObject({
        kind: 'state',
        state: expectedState,
        label: expectedLabel
      })
      expect(contents.rows[0], state).not.toMatchObject({ state: 'empty' })
    }

    const cursorInvalid = projectForSelection(browserState({}), 'navigation-row:7', {
      kind: 'ready',
      requestKey: 'source:7:playableMediaBrowse:recursive',
      result: {
        state: 'cursorInvalid',
        error: {
          code: 'cursorInvalid',
          message: 'The contents cursor is invalid.'
        }
      }
    })

    expect(cursorInvalid.rows[0]).toMatchObject({
      kind: 'state',
      state: 'unsupported',
      label: 'Contents unavailable'
    })
    expect(cursorInvalid.rows[0]).not.toMatchObject({ state: 'empty' })
  })

  it('projects library start, unsupported selection, and host state', () => {
    const state = browserState({})
    const projection = browserProjection(state)

    expect(projectContents({ state, bindingsById: projection.bindingsById })).toMatchObject({
      kind: 'libraryStart',
      title: 'Start your library',
      rows: [
        expect.objectContaining({
          label: 'Add a music folder',
          action: {
            kind: 'chooseMusicFolder',
            label: 'Add music folder'
          }
        })
      ]
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
  const projection =
    state.localBrowseEntryPointsState === undefined
      ? projectState(state)
      : projectAddSourceState({
          addSourceView: state.addSourceView ?? 'preview',
          entryPointsState: state.localBrowseEntryPointsState,
          itemStates: state.localBrowseItemStates
        })

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
  readonly entries?: readonly LocalBrowseEntryPoint[]
  readonly localBrowseItemStates?: ReadonlyMap<string, LocalBrowseItemState>
  readonly libraryBrowseProfile?: LibraryBrowseProfile
  readonly addSourceView?: AddSourceView
}): BrowserState {
  const sourceStates = new Map<string, SourceState>()

  if (options.sourceState !== undefined) {
    sourceStates.set('navigation-row:7', options.sourceState)
  }

  return {
    ...(options.libraryBrowseProfile === undefined
      ? {}
      : { libraryBrowseProfile: options.libraryBrowseProfile }),
    ...(options.addSourceView === undefined ? {} : { addSourceView: options.addSourceView }),
    navigationReadResult: options.navigationReadResult ?? {
      state: 'ready',
      rows: [sourceNavigationRow()]
    },
    sourceReadStates: sourceStates,
    directoryReadStates: options.directoryStates ?? new Map(),
    ...(options.entries === undefined
      ? {}
      : {
          localBrowseEntryPointsState: {
            kind: 'ready',
            result: {
              state: 'read',
              status: 'complete',
              entries: options.entries,
              failure: null
            }
          } as const
        }),
    ...(options.localBrowseItemStates === undefined
      ? {}
      : { localBrowseItemStates: options.localBrowseItemStates })
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
      subtreeCoverageComplete: true,
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

function emptyNavigation(): NavigationReadRowsResult {
  return {
    state: 'ready',
    rows: []
  }
}

function directoryNode(
  directoryId: string,
  label: string,
  parentDirectoryId?: string,
  options: {
    readonly hasChildDirectories?: boolean
    readonly directoryPlayableMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryPlayableMediaState']
    readonly directoryImageMediaState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryImageMediaState']
    readonly directoryScanState?: Extract<
      ChildRow,
      { readonly kind: 'directory' }
    >['directoryScanState']
    readonly navigableChildScopeState?: NavigableChildScopeState
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
    directoryPlayableMediaState: options.directoryPlayableMediaState ?? {
      kind: 'hasPlayableMediaDescendants'
    },
    directoryImageMediaState: options.directoryImageMediaState ?? {
      kind: 'noImageMediaDescendants'
    },
    directoryScanState: options.directoryScanState ?? 'scanning',
    navigableChildScopeState: options.navigableChildScopeState ?? 'hasNavigableChildScopes',
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

function localBrowseEntryPoint(
  overrides: Partial<LocalBrowseEntryPoint> = {}
): LocalBrowseEntryPoint {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedPath: 'C:\\Users\\Maikel\\Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    availableOperations: [
      { kind: 'browseChildren' },
      { kind: 'chooseDescendant' },
      {
        kind: 'requestSourceAdmission',
        requestKind: 'defaultMusicFolder',
        resolvedPath: 'C:\\Users\\Maikel\\Music'
      }
    ],
    failure: null,
    ...overrides
  }
}

function localBrowseItem(
  itemKind: LocalBrowseItemKind,
  displayName: string,
  resolvedItemPath: string,
  overrides: Partial<LocalBrowseItem> = {}
): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:\\Users\\Maikel\\Music',
      resolvedItemPath
    },
    itemKind,
    displayName,
    status: 'available',
    platform: 'windows',
    fileKind: null,
    mediaRelevance: null,
    availableOperations:
      itemKind === 'directory'
        ? [
            { kind: 'browseChildren' },
            { kind: 'chooseDescendant' },
            {
              kind: 'requestSourceAdmission',
              requestKind: 'selectedDirectory',
              resolvedPath: resolvedItemPath
            }
          ]
        : [],
    failure: null,
    ...overrides
  }
}

function localBrowseWindow(options: {
  readonly addSourceView?: AddSourceView
  readonly entryPointKind?: LoadedLocalBrowseItems['identity']['entryPointKind']
  readonly resolvedRootPath?: string
  readonly label: string
  readonly resolvedParentPath: string
  readonly items: readonly LocalBrowseItem[]
  readonly totalItems?: number
  readonly nextOffset?: number
}): LoadedLocalBrowseItems {
  const resolvedRootPath = options.resolvedRootPath ?? 'C:\\Users\\Maikel\\Music'
  return {
    addSourceView: options.addSourceView ?? 'preview',
    identity: {
      entryPointKind: options.entryPointKind ?? 'music',
      resolvedRootPath,
      resolvedParentPath: options.resolvedParentPath
    },
    label: options.label,
    items: options.items,
    totalItems: options.totalItems ?? options.items.length,
    status: 'complete',
    failure: null,
    limit: 50,
    ...(options.nextOffset === undefined ? {} : { nextOffset: options.nextOffset })
  }
}

function localBrowseWindowStateKey(window: LoadedLocalBrowseItems): string {
  return localBrowseWindowKey({
    addSourceView: window.addSourceView,
    entryPointKind: window.identity.entryPointKind,
    resolvedRootPath: window.identity.resolvedRootPath,
    resolvedParentPath: window.identity.resolvedParentPath,
    label: window.label
  })
}

function firstLocalBrowseEntryNode(
  projection: BrowserProjection
): BrowserProjection['nodes'][number] {
  const section = projection.nodes.find((node) => node.id === 'add-source:section')
  const node = section?.children.kind === 'loaded' ? section.children.nodes[0] : undefined

  if (node === undefined) {
    throw new Error('Expected a projected local browse entry point.')
  }

  return node
}

function readyContents(options: {
  readonly rows: readonly ContentsFileRow[]
  readonly state?: ContentsResult['state']
  readonly detail?: string
  readonly profile?: ContentsReadPolicy
  readonly coverageState?: ContentsResult['scopeCoverage']['state']
  readonly subtreeCoverageComplete?: boolean
  readonly emptyAuthoritative?: boolean
  readonly omittedRows?: boolean
  readonly requestKey?: string
  readonly pendingRequestKey?: string
  readonly pendingPresentation?: 'deferred' | 'visible'
  readonly refreshError?: string
  readonly nextCursor?: string
}): ContentsBoundaryState {
  return {
    kind: 'ready',
    requestKey: options.requestKey ?? 'source:7',
    result: {
      state: 'ready',
      result: contentsResult(options)
    },
    ...(options.nextCursor === undefined ? {} : { nextCursor: options.nextCursor }),
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
  readonly coverageState?: ContentsResult['scopeCoverage']['state']
  readonly subtreeCoverageComplete?: boolean
  readonly emptyAuthoritative?: boolean
  readonly omittedRows?: boolean
  readonly nextCursor?: string
}): ContentsResult {
  const state = options.state ?? 'ready'
  const policy = options.profile ?? ({ kind: 'playableMediaBrowse' } satisfies ContentsReadPolicy)
  const coverageState =
    options.coverageState ??
    (state === 'failed'
      ? 'failed'
      : state === 'sourceUnavailable'
        ? 'sourceUnavailable'
        : state === 'partial'
          ? 'scanning'
          : 'complete')
  return {
    state,
    scope: { kind: 'source', sourceId: '7' },
    policy,
    scopeDepth: 'recursive',
    rows: options.rows,
    scopeCoverage: {
      state: coverageState,
      subtreeCoverageComplete: options.subtreeCoverageComplete ?? state !== 'partial',
      emptyResultAuthoritative: options.emptyAuthoritative ?? state !== 'partial'
    },
    hasPolicyOmittedRows: options.omittedRows ?? false,
    ...(options.nextCursor === undefined ? {} : { nextCursor: options.nextCursor }),
    ...(options.detail === undefined ? {} : { detail: options.detail })
  }
}

function playableMediaRow(
  stableId: string,
  label: string,
  fileClass: Exclude<ContentsFileRow['fileClass'], 'image' | 'unsupported'>
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
    playableMedia: {
      playableMediaId: stableId,
      attachmentId: `attachment-${stableId}`,
      contentHashAlgorithm: 'blake3',
      contentHashValue: `hash-${stableId}`,
      evidenceSourceFileId: `file-${stableId}`,
      mediaKind: fileClass,
      mimeType: fileClass === 'audio' ? 'audio/wav' : 'video/mp4'
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
} {
  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    state: row.state ?? null,
    fileClass: row.fileClass ?? null
  }
}
