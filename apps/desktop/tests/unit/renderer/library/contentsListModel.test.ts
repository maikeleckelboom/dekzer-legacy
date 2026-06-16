import { describe, expect, it } from 'vitest'

import {
  projectList,
  rowSubject as listRowSubject,
  shouldSelectRowForKey,
  visibleCols
} from '../../../../src/renderer/library/contents/listModel'
import {
  isValidSelection,
  rowSubject as selectionRowSubject
} from '../../../../src/renderer/library/selection/model'
import type {
  ContentProjection,
  ContentRow
} from '../../../../src/renderer/library/contents/projection'

describe('contents list model', () => {
  it('does not select rows for Space but keeps Enter row selection', () => {
    expect(shouldSelectRowForKey('Enter')).toBe(true)
    expect(shouldSelectRowForKey(' ')).toBe(false)
    expect(shouldSelectRowForKey('Spacebar')).toBe(false)
  })

  it('projects playable media as inspectable track-compatible list rows', () => {
    const list = projectList(
      projection([
        playableRow({
          id: 'playable-1',
          label: 'Track One.wav',
          relativePath: 'Music/Track One.wav'
        })
      ])
    )
    const row = list.rows[0]

    expect(list.columns.map((col) => col.label)).toEqual([
      '#',
      'Title',
      'Artist',
      'Album',
      'BPM',
      'Key',
      'Time',
      'Rating',
      'Ready',
      'Source'
    ])
    expect(row).toMatchObject({
      family: 'playableMedia',
      subject: {
        kind: 'playableMedia',
        playableMediaId: 'playable-1'
      }
    })
    expect(row.cells.index.text).toBe('1')
    expect(row.cells.title.text).toBe('Track One.wav')
    expect(row.cells.ready.text).toBe('Ready')
    expect(row.cells.source.text).toBe('Music/Track One.wav')
  })

  it('hides deferred blank metadata columns from the visible table', () => {
    const list = projectList(
      projection([
        playableRow({
          id: 'playable-1',
          label: 'Track One.wav',
          relativePath: 'Music/Track One.wav'
        })
      ])
    )

    expect(visibleCols(list).map((col) => col.key)).toEqual(['index', 'title', 'ready', 'source'])
  })

  it('omits the index column when it has no useful values', () => {
    const list = projectList(
      projection([
        sourceFileRow({
          id: 'source-file:1',
          label: 'Loose Audio.wav',
          relativePath: 'Incoming/Loose Audio.wav'
        })
      ])
    )

    expect(visibleCols(list).map((col) => col.key)).toEqual(['title', 'ready', 'source'])
  })

  it('uses compact visible columns for constrained terminal panes', () => {
    const list = projectList(
      projection([
        playableRow({
          id: 'playable-1',
          label: 'Track One.wav',
          relativePath: 'Music/Track One.wav'
        })
      ])
    )

    expect(visibleCols(list, 'compact').map((col) => col.key)).toEqual(['title', 'ready', 'source'])
  })

  it('keeps source-file rows source-file-like instead of displaying them as tracks', () => {
    const list = projectList(
      projection([
        sourceFileRow({
          id: 'source-file:1',
          label: 'Loose Audio.wav',
          fileClass: 'audio',
          relativePath: 'Incoming/Loose Audio.wav'
        })
      ])
    )
    const row = list.rows[0]

    expect(row.family).toBe('sourceFile')
    expect(row.subject?.kind).toBe('sourceFile')
    expect(row.cells.index.text).toBe('')
    expect(row.cells.ready.text).toBe('Source file')
    expect(row.cells.artist).toMatchObject({ text: '', tone: 'muted', deferred: true })
    expect(row.cells.title.text).toBe('Loose Audio.wav')
  })

  it('leaves state and load-more rows uninspectable', () => {
    const state = stateRow()
    const more = loadMoreRow()
    const list = projectList(projection([state, more]))

    expect(list.rows.map((row) => row.family)).toEqual(['state', 'loadMore'])
    expect(list.rows.map((row) => listRowSubject(row))).toEqual([undefined, undefined])
    expect(list.rows[0].cells.ready.text).toBe('Loading')
    expect(list.rows[1].cells.ready.text).toBe('More available')
  })

  it('renders source inventory rows without musical promotion', () => {
    const list = projectList(
      projection(
        [
          {
            id: 'local-folder:albums',
            kind: 'directory',
            label: 'Albums',
            detail: 'C:\\Music\\Albums',
            icon: 'folder'
          }
        ],
        { surfaceKind: 'sourceInventory' }
      )
    )
    const row = list.rows[0]

    expect(row.family).toBe('sourceInventory')
    expect(row.subject).toBeUndefined()
    expect(row.cells.index.text).toBe('')
    expect(row.cells.title.text).toBe('Albums')
    expect(row.cells.artist.text).toBe('')
    expect(row.cells.ready.text).toBe('Present')
    expect(row.cells.source.text).toBe('C:\\Music\\Albums')
  })

  it('keeps search source-file rows as sourceFile subjects', () => {
    const list = projectList(
      projection(
        [
          sourceFileRow({
            id: 'search-result:1',
            label: 'Search Hit.wav',
            relativePath: 'Search/Search Hit.wav'
          })
        ],
        { title: 'Search results' }
      )
    )
    const row = list.rows[0]

    expect(row.family).toBe('searchResult')
    expect(row.subject?.kind).toBe('sourceFile')
    expect(row.cells.source.text).toBe('Search/Search Hit.wav')
  })

  it('renders missing musical metadata as deferred blanks', () => {
    const row = projectList(
      projection([
        playableRow({
          id: 'playable-blank',
          label: 'Unknown Source.wav'
        })
      ])
    ).rows[0]

    for (const key of ['artist', 'album', 'bpm', 'key', 'time', 'rating'] as const) {
      expect(row.cells[key]).toEqual({
        text: '',
        tone: 'muted',
        deferred: true
      })
    }
    expect(row.cells.title.text).toBe('Unknown Source.wav')
    expect(row.cells.source.text).toBe('')
  })

  it('keeps selection valid through list projection only while the subject remains valid', () => {
    const first = playableRow({ id: 'playable-1', label: 'Track One.wav' })
    const selected = selectionRowSubject(projectList(projection([first])).rows[0].base)

    expect(isValidSelection(selected, projection([first]))).toBe(true)
    expect(
      isValidSelection(
        selected,
        projection([
          playableRow({
            id: 'playable-1',
            label: 'Track One.wav',
            playableMediaId: 'playable-2'
          })
        ])
      )
    ).toBe(false)
  })
})

function projection(
  rows: readonly ContentRow[],
  options: { readonly title?: string; readonly surfaceKind?: ContentProjection['surfaceKind'] } = {}
): ContentProjection {
  return {
    surfaceKind: options.surfaceKind ?? 'indexedContents',
    surfaceLabel: 'Contents',
    header: {
      surfaceLabel: 'Library Browse',
      scopeLabel: options.title ?? 'Source Fixture',
      health: { label: 'Ready', tone: 'ready' }
    },
    kind: 'ready',
    title: options.title ?? 'Source Fixture',
    rows
  }
}

function playableRow(options: {
  readonly id: string
  readonly label: string
  readonly relativePath?: string
  readonly playableMediaId?: string
}): ContentRow {
  const playableMediaId = options.playableMediaId ?? options.id
  return {
    id: options.id,
    kind: 'file',
    label: options.label,
    presence: 'present',
    icon: 'music',
    fileClass: 'audio',
    subject: {
      kind: 'playableMedia',
      sourceId: '7',
      sourceFileId: `file-${options.id}`,
      playableMediaId,
      attachmentId: `attachment-${playableMediaId}`,
      label: options.label,
      ...(options.relativePath === undefined ? {} : { relativePath: options.relativePath }),
      mediaKind: 'audio',
      presence: 'present'
    }
  }
}

function sourceFileRow(options: {
  readonly id: string
  readonly label: string
  readonly fileClass?: ContentRow['fileClass']
  readonly relativePath?: string
}): ContentRow {
  return {
    id: options.id,
    kind: 'file',
    label: options.label,
    presence: 'present',
    icon: 'music',
    fileClass: options.fileClass ?? 'audio',
    subject: {
      kind: 'sourceFile',
      sourceId: '7',
      sourceFileId: options.id,
      label: options.label,
      ...(options.relativePath === undefined ? {} : { relativePath: options.relativePath }),
      presence: 'present'
    }
  }
}

function stateRow(): ContentRow {
  return {
    id: 'contents-state:indexing:loading',
    kind: 'state',
    label: 'Still indexing',
    detail: 'More tracks may appear.',
    state: 'loading',
    icon: 'loading'
  }
}

function loadMoreRow(): ContentRow {
  return {
    id: 'contents-load-more:source',
    kind: 'more',
    label: 'More tracks available',
    detail: 'Load more',
    icon: 'more',
    action: {
      kind: 'loadContentsPage',
      nodeId: 'source:7',
      label: 'Load more tracks',
      cursor: 'cursor-1'
    }
  }
}
