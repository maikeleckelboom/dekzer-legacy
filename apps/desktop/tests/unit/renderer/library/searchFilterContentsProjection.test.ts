import { describe, expect, it } from 'vitest'

import { projectSearchFilterContents } from '../../../../src/renderer/library/searchFilter/contentsProjection'
import type { QueryId } from '../../../../src/renderer/library/runtime/searchFilterState'
import type { SearchFilterResultRow } from '../../../../src/shared/library/searchFilter/read'

const queryId = 'query' as QueryId
const libraryIdentity = {
  scope: { type: 'library' },
  textQuery: 'amen'
} as const
const sourceIdentity = {
  scope: { type: 'source', payload: { sourceId: '7' } },
  textQuery: 'amen'
} as const
const folderIdentity = {
  scope: { type: 'directory', payload: { sourceId: '7', sourceDirectoryId: '11' } },
  textQuery: 'break'
} as const

describe('search/filter contents projection', () => {
  it('projects active search results into contents rows', () => {
    const contents = projectSearchFilterContents({
      state: {
        kind: 'Retained',
        queryId,
        identity: libraryIdentity,
        rows: [row('1', 'Amen.wav', 'audio'), row('2', 'Clip.mp4', 'video')],
        nextCursor: null,
        indexGeneration: '1',
        resultState: 'ready'
      },
      activeQuery: 'amen',
      profile: 'playable'
    })

    expect(contents.kind).toBe('ready')
    expect(contents.surfaceLabel).toBe('Contents')
    expect(contents.surfaceKind).toBe('indexedContents')
    expect(contents.title).toBe('Search results')
    expect(contents.header).toMatchObject({
      surfaceLabel: 'Library Browse',
      scopeLabel: 'Search results',
      profileLabel: 'Audio + Video',
      searchLabel: 'Search: "amen"',
      searchScopeLabel: 'Library-wide',
      health: {
        label: 'Ready',
        tone: 'ready'
      }
    })
    expect(contents.rows.map((projected) => projected.label)).toEqual(['Amen.wav', 'Clip.mp4'])
    expect(contents.rows.map((projected) => projected.icon)).toEqual(['music', 'video'])
    expect(contents.rows[0]).toMatchObject({
      subject: {
        kind: 'sourceFile',
        sourceId: '7',
        sourceFileId: '1'
      }
    })
  })

  it('uses existing search next-page path for load more rows', () => {
    const contents = projectSearchFilterContents({
      state: {
        kind: 'Retained',
        queryId,
        identity: libraryIdentity,
        rows: [row('1', 'Amen.wav', 'audio')],
        nextCursor: 'cursor-1',
        indexGeneration: '1',
        resultState: 'ready'
      },
      activeQuery: 'amen',
      profile: 'audio'
    })

    expect(contents.rows.at(-1)).toMatchObject({
      kind: 'more',
      action: {
        kind: 'loadSearchPage',
        cursor: 'cursor-1',
        label: 'Load more search results'
      }
    })
    expect(contents.rows.at(-1)).not.toHaveProperty('subject')
  })

  it('retains prior rows while a refreshed search is pending', () => {
    const contents = projectSearchFilterContents({
      state: {
        kind: 'Pending',
        queryId,
        identity: libraryIdentity,
        requestToken: 2,
        priorRetained: {
          queryId,
          identity: libraryIdentity,
          rows: [row('1', 'Amen.wav', 'audio')],
          nextCursor: null,
          indexGeneration: '1',
          resultState: 'ready'
        }
      },
      activeQuery: 'amen',
      profile: 'audio'
    })

    expect(contents.rows.map((projected) => projected.label)).toEqual(['Amen.wav'])
  })

  it('labels retained rows as previous results while another scope is pending', () => {
    const contents = projectSearchFilterContents({
      state: {
        kind: 'Pending',
        queryId,
        identity: folderIdentity,
        requestToken: 2,
        priorRetained: {
          queryId,
          identity: sourceIdentity,
          rows: [row('1', 'Amen.wav', 'audio')],
          nextCursor: null,
          indexGeneration: '1',
          resultState: 'ready'
        }
      },
      activeQuery: 'break',
      profile: 'audio'
    })

    expect(contents.detail).toBe(
      'Searching in selected folder for "break". Showing previous results in selected source for "amen".'
    )
    expect(contents.header).toMatchObject({
      searchLabel: 'Search: "break"',
      searchScopeLabel: 'Inside selected folder'
    })
    expect(contents.rows.map((projected) => projected.label)).toEqual(['Amen.wav'])
  })

  it('uses profile-aware empty copy for profile-mapped searches', () => {
    for (const [profile, expected] of [
      ['audio', 'No matching audio tracks.'],
      ['playable', 'No matching playable media.'],
      ['allFiles', 'No matching files.']
    ] as const) {
      const contents = projectSearchFilterContents({
        state: {
          kind: 'Retained',
          queryId,
          identity: {
            scope: { type: 'library' },
            textQuery: 'missing'
          },
          rows: [],
          nextCursor: null,
          indexGeneration: '1',
          resultState: 'empty'
        },
        activeQuery: 'missing',
        profile
      })

      expect(contents.rows[0]).toMatchObject({
        kind: 'state',
        state: 'empty',
        label: expected
      })
    }
  })

  it('keeps accumulated rows visible while loading more search results', () => {
    const contents = projectSearchFilterContents({
      state: {
        kind: 'Accumulating',
        queryId,
        identity: libraryIdentity,
        accumulated: [row('1', 'Amen.wav', 'audio')],
        nextCursor: 'cursor-1',
        indexGeneration: '1',
        requestToken: 2
      },
      activeQuery: 'amen',
      profile: 'audio'
    })

    expect(contents.rows.map((projected) => projected.label)).toEqual([
      'Amen.wav',
      'Loading more results'
    ])
    expect(contents.rows.at(-1)).not.toHaveProperty('action')
  })
})

function row(
  id: string,
  displayLabel: string,
  fileClass: SearchFilterResultRow['fileClass']
): SearchFilterResultRow {
  return {
    resultKind: 'sourceFile',
    authorityLayer: 'sourceFileInventory',
    stableKey: `source-file:${id}`,
    sourceId: '7',
    sourceLocationId: null,
    sourceDirectoryId: '11',
    parentSourceDirectoryId: null,
    sourceFileId: id,
    displayLabel,
    displayPath: `Music/${displayLabel}`,
    relativePath: displayLabel,
    ...(fileClass === undefined ? {} : { fileClass }),
    ...(fileClass === undefined ? {} : { fileKind: searchFileKind(fileClass) }),
    mediaRelevance: 'playableMedia',
    presenceState: 'present',
    sourceAccessState: 'accessible',
    sourceScanPhase: 'complete',
    hasCurrentBlake3: true,
    hasCurrentProbe: false,
    attachmentLinkState: 'notApplicable',
    attachmentId: null,
    evidenceCoverageState: 'indexed',
    matchReason: 'label',
    updatedAtMs: 100
  }
}

function searchFileKind(
  fileClass: Exclude<SearchFilterResultRow['fileClass'], undefined>
): Exclude<SearchFilterResultRow['fileKind'], undefined> {
  switch (fileClass) {
    case 'audio':
      return 'audio'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'unsupported':
      return 'other'
    case 'none':
      return 'unknown'
  }
}
