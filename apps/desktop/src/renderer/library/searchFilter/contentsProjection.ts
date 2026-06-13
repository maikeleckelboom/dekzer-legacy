import type { ProfileKey } from '../browseProfile/types'
import type { ContentProjection, ContentRow, ContentRowIcon } from '../contents/projection'
import type { SearchQueryState } from '../runtime/searchFilterState'
import type { SearchFilterResultRow } from '../../../shared/library/searchFilter/read'

export function projectSearchFilterContents(options: {
  readonly state: SearchQueryState
  readonly activeQuery: string
  readonly profile: ProfileKey
}): ContentProjection {
  const rows = rowsForSearchState(options.state)
  const nextCursor = nextCursorForSearchState(options.state)
  const loadingMore = options.state.kind === 'Accumulating'
  const detail = searchDetail(options.activeQuery, rows.length, loadingMore)

  if (rows.length === 0) {
    return {
      kind: searchProjectionKind(options.state),
      title: 'Search results',
      detail,
      rows: [emptyOrPendingSearchRow(options.state, options.profile)]
    }
  }

  const projectedRows = rows.map(searchResultContentRow)
  return {
    kind: 'ready',
    title: 'Search results',
    detail,
    rows:
      nextCursor === null
        ? projectedRows
        : [...projectedRows, searchLoadMoreRow(nextCursor, loadingMore)]
  }
}

function rowsForSearchState(state: SearchQueryState): readonly SearchFilterResultRow[] {
  switch (state.kind) {
    case 'Retained':
      return state.rows
    case 'Pending':
      return state.priorRetained?.rows ?? []
    case 'Accumulating':
      return state.accumulated
    case 'Idle':
      return []
  }
}

function nextCursorForSearchState(state: SearchQueryState): string | null {
  switch (state.kind) {
    case 'Retained':
      return state.nextCursor
    case 'Accumulating':
      return state.nextCursor
    case 'Idle':
    case 'Pending':
      return null
  }
}

function searchProjectionKind(state: SearchQueryState): ContentProjection['kind'] {
  if (state.kind === 'Retained' && state.resultState === 'unsupported') {
    return 'unsupported'
  }

  if (state.kind === 'Retained' && state.resultState === 'empty') {
    return 'ready'
  }

  return state.kind === 'Idle' || state.kind === 'Pending' ? 'loading' : 'ready'
}

function emptyOrPendingSearchRow(state: SearchQueryState, profile: ProfileKey): ContentRow {
  if (state.kind === 'Retained' && state.resultState === 'unsupported') {
    return stateRow('search-unsupported', 'unsupported', 'Search unavailable', state.resultDetail)
  }

  if (state.kind === 'Retained' && state.resultState !== 'partial') {
    const label = emptySearchLabel(profile)
    return stateRow('search-empty', 'empty', label, label)
  }

  if (state.kind === 'Retained' && state.resultState === 'partial') {
    return stateRow(
      'search-partial',
      'loading',
      'Still indexing',
      state.resultDetail ?? 'Search results may be incomplete.'
    )
  }

  return stateRow('search-pending', 'loading', 'Searching library', 'Searching library.')
}

function searchResultContentRow(row: SearchFilterResultRow): ContentRow {
  const detail = searchRowDetail(row)
  const fileClass = contentFileClass(row.fileClass)
  return {
    id: `search-result:${row.stableKey}`,
    kind: row.resultKind === 'directory' ? 'directory' : 'file',
    label: row.displayLabel,
    icon: searchRowIcon(row),
    ...(detail === undefined ? {} : { detail }),
    ...(row.presenceState === undefined ? {} : { presence: row.presenceState }),
    ...(fileClass === undefined ? {} : { fileClass })
  }
}

function searchRowDetail(row: SearchFilterResultRow): string | undefined {
  if (row.sourceAccessState === 'missing') {
    return 'Source missing'
  }

  if (row.sourceAccessState === 'blocked') {
    return 'Source blocked'
  }

  if (row.presenceState === 'missing') {
    return 'Source file missing'
  }

  if (row.presenceState === 'removed') {
    return 'Source file removed'
  }

  if (row.sourceScanPhase === 'scanning' || row.sourceScanPhase === 'partial') {
    return 'Indexing'
  }

  return row.displayPath ?? row.relativePath
}

function searchRowIcon(row: SearchFilterResultRow): ContentRowIcon {
  if (row.resultKind === 'directory') {
    return 'folder'
  }

  switch (row.fileClass) {
    case 'audio':
      return 'music'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'unsupported':
      return row.fileKind === 'cueSheet' ? 'cueSheet' : 'metadata'
    case 'none':
    case undefined:
      return 'state'
  }

  return 'state'
}

function contentFileClass(
  fileClass: SearchFilterResultRow['fileClass']
): ContentRow['fileClass'] | undefined {
  switch (fileClass) {
    case 'audio':
    case 'video':
    case 'image':
    case 'unsupported':
      return fileClass
    case 'none':
    case undefined:
      return undefined
  }
}

function searchLoadMoreRow(nextCursor: string, loadingMore: boolean): ContentRow {
  return {
    id: 'search-results-load-more',
    kind: 'more',
    label: loadingMore ? 'Loading more results' : 'More search results available',
    detail: loadingMore ? 'Loading more' : 'Load more',
    icon: loadingMore ? 'loading' : 'more',
    ...(loadingMore
      ? {}
      : {
          action: {
            kind: 'loadSearchPage',
            label: 'Load more search results',
            cursor: nextCursor
          }
        })
  }
}

function stateRow(
  id: string,
  state: Exclude<ContentRow['state'], undefined>,
  label: string,
  detail: string | undefined
): ContentRow {
  return {
    id: `contents-state:${id}:${state}`,
    kind: 'state',
    label,
    state,
    icon: state === 'loading' ? 'loading' : state === 'empty' ? 'state' : 'warning',
    ...(detail === undefined ? {} : { detail })
  }
}

function emptySearchLabel(profile: ProfileKey): string {
  switch (profile) {
    case 'audio':
      return 'No matching audio files.'
    case 'playable':
      return 'No matching playable media.'
    case 'allFiles':
      return 'No matching tracks.'
  }
}

function searchDetail(activeQuery: string, rowCount: number, loadingMore: boolean): string {
  if (rowCount === 0) {
    return `Search results for "${activeQuery}".`
  }

  const count = rowCount === 1 ? '1 result' : `${rowCount} results`
  return loadingMore
    ? `${count} for "${activeQuery}". Loading more.`
    : `${count} for "${activeQuery}".`
}
