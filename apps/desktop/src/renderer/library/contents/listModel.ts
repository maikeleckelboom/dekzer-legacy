import type { ContentProjection, ContentRow, ContentRowIcon, ContentRowSubject } from './projection'

export type colKey =
  | 'index'
  | 'title'
  | 'artist'
  | 'album'
  | 'bpm'
  | 'key'
  | 'time'
  | 'rating'
  | 'ready'
  | 'source'

export type ListCol = {
  readonly key: colKey
  readonly label: string
}

export type Cell = {
  readonly text: string
  readonly tone: 'normal' | 'muted' | 'warning' | 'danger'
  readonly deferred?: boolean
  readonly detail?: string
  readonly icon?: ContentRowIcon
}

export type ListRow = {
  readonly id: string
  readonly family:
    | 'playableMedia'
    | 'sourceFile'
    | 'sourceInventory'
    | 'state'
    | 'loadMore'
    | 'searchResult'
  readonly base: ContentRow
  readonly subject?: ContentRowSubject
  readonly cells: Readonly<Record<colKey, Cell>>
}

export type List = {
  readonly columns: readonly ListCol[]
  readonly rows: readonly ListRow[]
}

export const listCols = [
  { key: 'index', label: '#' },
  { key: 'title', label: 'Title' },
  { key: 'artist', label: 'Artist' },
  { key: 'album', label: 'Album' },
  { key: 'bpm', label: 'BPM' },
  { key: 'key', label: 'Key' },
  { key: 'time', label: 'Time' },
  { key: 'rating', label: 'Rating' },
  { key: 'ready', label: 'Ready' },
  { key: 'source', label: 'Source' }
] satisfies readonly ListCol[]

export function projectList(projection: ContentProjection): List {
  return {
    columns: listCols,
    rows: projection.rows.map((row, index) => projectRow(projection, row, index))
  }
}

export function rowSubject(row: ListRow): ContentRowSubject | undefined {
  return row.subject
}

export function shouldSelectRowForKey(key: string): boolean {
  return key === 'Enter'
}

function projectRow(projection: ContentProjection, row: ContentRow, index: number): ListRow {
  const family = rowFamily(projection, row)
  const cells = listCells(row, family, index)

  return {
    id: row.id,
    family,
    base: row,
    ...(row.subject === undefined ? {} : { subject: row.subject }),
    cells
  }
}

function rowFamily(projection: ContentProjection, row: ContentRow): ListRow['family'] {
  if (row.kind === 'state') {
    return 'state'
  }

  if (row.kind === 'more') {
    return 'loadMore'
  }

  if (row.subject?.kind === 'playableMedia') {
    return 'playableMedia'
  }

  if (row.subject?.kind === 'sourceFile') {
    return projection.title === 'Search results' ? 'searchResult' : 'sourceFile'
  }

  return 'sourceInventory'
}

function listCells(
  row: ContentRow,
  family: ListRow['family'],
  index: number
): Readonly<Record<colKey, Cell>> {
  const source = sourceText(row)

  return {
    index: cell(family === 'playableMedia' ? `${index + 1}` : '', 'muted'),
    title: {
      text: row.label,
      tone: titleTone(row),
      ...(row.detail === undefined ? {} : { detail: row.detail }),
      ...(row.icon === undefined ? {} : { icon: row.icon })
    },
    artist: deferredCell(),
    album: deferredCell(),
    bpm: deferredCell(),
    key: deferredCell(),
    time: deferredCell(),
    rating: deferredCell(),
    ready: readyCell(row, family),
    source: cell(source, source.length === 0 ? 'muted' : 'normal')
  }
}

function deferredCell(): Cell {
  return {
    text: '',
    tone: 'muted',
    deferred: true
  }
}

function cell(text: string, tone: Cell['tone'] = 'normal'): Cell {
  return { text, tone }
}

function readyCell(row: ContentRow, family: ListRow['family']): Cell {
  if (row.presence === 'missing') {
    return cell('Missing', 'warning')
  }

  if (row.presence === 'removed') {
    return cell('Removed', 'danger')
  }

  if (row.kind === 'more') {
    return cell(row.action === undefined ? 'Loading' : 'More available', 'muted')
  }

  if (row.kind === 'state') {
    return stateCell(row)
  }

  if (row.detail === 'Indexing') {
    return cell('Indexing', 'muted')
  }

  if (row.detail === 'Source missing' || row.detail === 'File missing') {
    return cell('Missing', 'warning')
  }

  if (row.detail === 'Source blocked') {
    return cell('Blocked', 'warning')
  }

  if (row.detail === 'Source file removed' || row.detail === 'File removed') {
    return cell('Removed', 'danger')
  }

  if (family === 'playableMedia') {
    return cell('Ready')
  }

  if (family === 'sourceFile' || family === 'searchResult') {
    return cell('Source file', 'muted')
  }

  return cell('Present', 'muted')
}

function stateCell(row: ContentRow): Cell {
  switch (row.state) {
    case 'attention':
      return cell('Needs attention', 'warning')
    case 'failed':
    case 'unsupported':
      return cell('Unavailable', 'warning')
    case 'loading':
      return cell('Loading', 'muted')
    case 'notLoaded':
      return cell('Not loaded', 'muted')
    case 'empty':
      return cell('Empty', 'muted')
    case 'file':
      return cell('Selected', 'muted')
    default:
      return cell('', 'muted')
  }
}

function sourceText(row: ContentRow): string {
  if (row.kind === 'state' || row.kind === 'more') {
    return ''
  }

  if (row.subject?.relativePath !== undefined) {
    return row.subject.relativePath
  }

  if (row.subject?.kind === 'sourceFile') {
    return row.subject.detail ?? ''
  }

  if (row.subject === undefined && (row.kind === 'directory' || row.kind === 'file')) {
    return row.detail ?? ''
  }

  return ''
}

function titleTone(row: ContentRow): Cell['tone'] {
  if (row.state === 'attention' || row.state === 'failed' || row.state === 'unsupported') {
    return 'warning'
  }

  if (row.presence === 'missing') {
    return 'warning'
  }

  if (row.presence === 'removed') {
    return 'danger'
  }

  if (row.kind === 'state' || row.icon === 'cueSheet' || row.icon === 'metadata') {
    return 'muted'
  }

  return 'normal'
}
