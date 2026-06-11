import type {
  SearchFilterAuthorityLayer,
  SearchFilterMatchReason,
  SearchFilterResultKind,
  SearchFilterResultRow
} from '../../../shared/library/searchFilter/read'

export type SearchResultIconRole = 'source' | 'sourceLocation' | 'directory' | 'sourceFile'

export type SearchResultViewModel = {
  readonly id: string
  readonly resultKind: SearchFilterResultKind
  readonly authorityLayer: SearchFilterAuthorityLayer
  readonly label: string
  readonly pathDisplay?: string
  readonly matchReason: SearchFilterMatchReason
  readonly iconRole: SearchResultIconRole
  readonly statusSummary?: string
}

export function projectSearchFilterResultRow(row: SearchFilterResultRow): SearchResultViewModel {
  const pathDisplay = searchPathDisplay(row)
  const statusSummary = searchStatusSummary(row)

  return {
    id: row.stableKey,
    resultKind: row.resultKind,
    authorityLayer: row.authorityLayer,
    label: row.displayLabel,
    ...(pathDisplay === undefined ? {} : { pathDisplay }),
    matchReason: row.matchReason,
    iconRole: searchResultIconRole(row.resultKind),
    ...(statusSummary === undefined ? {} : { statusSummary })
  }
}

export function projectSearchFilterResultRows(
  rows: readonly SearchFilterResultRow[]
): readonly SearchResultViewModel[] {
  return rows.map(projectSearchFilterResultRow)
}

function searchPathDisplay(row: SearchFilterResultRow): string | undefined {
  return row.displayPath ?? row.relativePath
}

function searchResultIconRole(resultKind: SearchFilterResultKind): SearchResultIconRole {
  switch (resultKind) {
    case 'source':
      return 'source'
    case 'sourceLocation':
      return 'sourceLocation'
    case 'directory':
      return 'directory'
    case 'sourceFile':
      return 'sourceFile'
  }
}

function searchStatusSummary(row: SearchFilterResultRow): string | undefined {
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

  return undefined
}
