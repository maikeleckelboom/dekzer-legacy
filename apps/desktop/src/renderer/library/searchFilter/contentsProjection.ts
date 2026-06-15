import type { LibraryBrowseProfile } from '../libraryBrowseProfile/types'
import type {
  ContentProjection,
  ContentRow,
  ContentRowIcon,
  ContentScopeHealth,
  ContentScopeHeader
} from '../contents/projection'
import type { SearchQueryState, SearchResultIdentity } from '../runtime/searchFilterState'
import type { SearchFilterResultRow } from '../../../shared/library/searchFilter/read'

export function projectSearchFilterContents(options: {
  readonly state: SearchQueryState
  readonly activeQuery: string
  readonly profile: LibraryBrowseProfile
}): ContentProjection {
  const rows = rowsForSearchState(options.state)
  const nextCursor = nextCursorForSearchState(options.state)
  const loadingMore = options.state.kind === 'Accumulating'
  const identity = presentationIdentityForSearchState(options.state)
  const detail = searchDetail(identity, options.activeQuery, rows.length, loadingMore)

  if (rows.length === 0) {
    return {
      surfaceKind: 'indexedContents',
      surfaceLabel: 'Contents',
      header: searchScopeHeader(options.state, options.activeQuery, options.profile, identity),
      kind: searchProjectionKind(options.state),
      title: 'Search results',
      detail,
      rows: [emptyOrPendingSearchRow(options.state, options.profile, identity)]
    }
  }

  const projectedRows = rows.map(searchResultContentRow)
  return {
    surfaceKind: 'indexedContents',
    surfaceLabel: 'Contents',
    header: searchScopeHeader(options.state, options.activeQuery, options.profile, identity),
    kind: 'ready',
    title: 'Search results',
    detail,
    rows:
      nextCursor === null
        ? projectedRows
        : [...projectedRows, searchLoadMoreRow(nextCursor, loadingMore)]
  }
}

function searchScopeHeader(
  state: SearchQueryState,
  activeQuery: string,
  profile: LibraryBrowseProfile,
  identity: SearchResultIdentity | undefined
): ContentScopeHeader {
  const query = identity?.textQuery ?? activeQuery

  return {
    surfaceLabel: 'Library Browse',
    scopeLabel: 'Search results',
    profileLabel: profileHeaderLabel(profile),
    searchLabel: query.length === 0 ? 'Search active' : `Search: "${query}"`,
    searchScopeLabel: searchScopeHeaderLabel(identity),
    health: searchHealth(state)
  }
}

function profileHeaderLabel(profile: LibraryBrowseProfile): string {
  switch (profile) {
    case 'audio':
      return 'Audio'
    case 'playable':
      return 'Audio + Video'
    case 'allFiles':
      return 'All Files'
  }
}

function searchHealth(state: SearchQueryState): ContentScopeHealth {
  if (state.kind === 'Idle' || state.kind === 'Pending') {
    return { label: 'Loading', tone: 'active' }
  }

  if (state.kind === 'Retained') {
    if (state.resultState === 'unsupported') {
      return { label: 'Unavailable', tone: 'danger' }
    }

    if (state.resultState === 'partial') {
      return { label: 'Still indexing', tone: 'active' }
    }

    if (state.resultState === 'empty') {
      return { label: 'Empty', tone: 'muted' }
    }
  }

  return { label: 'Ready', tone: 'ready' }
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

function presentationIdentityForSearchState(
  state: SearchQueryState
): SearchResultIdentity | undefined {
  switch (state.kind) {
    case 'Retained':
    case 'Accumulating':
      return state.identity
    case 'Pending':
      return state.priorRetained?.identity ?? state.identity
    case 'Idle':
      return undefined
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

function emptyOrPendingSearchRow(
  state: SearchQueryState,
  profile: LibraryBrowseProfile,
  identity: SearchResultIdentity | undefined
): ContentRow {
  if (state.kind === 'Retained' && state.resultState === 'unsupported') {
    return stateRow('search-unsupported', 'unsupported', 'Search unavailable', state.resultDetail)
  }

  if (state.kind === 'Retained' && state.resultState !== 'partial') {
    const label = emptySearchLabel(profile, identity)
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

  const label = pendingSearchLabel(identity)
  return stateRow('search-pending', 'loading', label, `${label}.`)
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

function emptySearchLabel(
  profile: LibraryBrowseProfile,
  identity: SearchResultIdentity | undefined
): string {
  const scopeSuffix = searchScopeSuffix(identity)

  switch (profile) {
    case 'audio':
      return `No matching audio tracks${scopeSuffix}.`
    case 'playable':
      return `No matching playable media${scopeSuffix}.`
    case 'allFiles':
      return `No matching files${scopeSuffix}.`
  }
}

function searchDetail(
  identity: SearchResultIdentity | undefined,
  activeQuery: string,
  rowCount: number,
  loadingMore: boolean
): string {
  const query = identity?.textQuery ?? activeQuery
  const scopePhrase = searchScopePhrase(identity)

  if (rowCount === 0) {
    return `Search results ${scopePhrase} for "${query}".`
  }

  const count = rowCount === 1 ? '1 result' : `${rowCount} results`
  return loadingMore
    ? `${count} ${scopePhrase} for "${query}". Loading more.`
    : `${count} ${scopePhrase} for "${query}".`
}

function pendingSearchLabel(identity: SearchResultIdentity | undefined): string {
  switch (identity?.scope.type) {
    case 'source':
      return 'Searching selected source'
    case 'sourceLocation':
      return 'Searching selected source location'
    case 'directory':
      return 'Searching selected folder'
    case 'library':
    case undefined:
      return 'Searching library'
  }
}

function searchScopePhrase(identity: SearchResultIdentity | undefined): string {
  switch (identity?.scope.type) {
    case 'source':
      return 'in selected source'
    case 'sourceLocation':
      return 'in selected source location'
    case 'directory':
      return 'in selected folder'
    case 'library':
    case undefined:
      return 'in library'
  }
}

function searchScopeHeaderLabel(identity: SearchResultIdentity | undefined): string {
  switch (identity?.scope.type) {
    case 'source':
      return 'Inside selected source'
    case 'sourceLocation':
      return 'Inside selected source location'
    case 'directory':
      return 'Inside selected folder'
    case 'library':
    case undefined:
      return 'Library-wide'
  }
}

function searchScopeSuffix(identity: SearchResultIdentity | undefined): string {
  switch (identity?.scope.type) {
    case 'source':
      return ' in this source'
    case 'sourceLocation':
      return ' in this source location'
    case 'directory':
      return ' in this folder'
    case 'library':
    case undefined:
      return ''
  }
}
