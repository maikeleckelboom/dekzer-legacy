import type { ContentProjection, ContentRow, ContentRowSubject } from '../contents/projection'
import type { StatusContext } from '../sourceStatus/context'

export type SelectionKind = 'none' | 'row' | 'source'

export type SourceSelectionContext = Extract<
  StatusContext,
  { readonly kind: 'localBrowse' | 'registeredDirectory' | 'registeredSource' }
>

export type PrimarySelection =
  | {
      readonly kind: 'none'
    }
  | {
      readonly kind: 'row'
      readonly rowId: string
      readonly subject: ContentRowSubject
    }
  | {
      readonly kind: 'source'
      readonly context: SourceSelectionContext
    }

export function rowSubject(row: ContentRow): PrimarySelection {
  if (row.subject === undefined) {
    return clearSelection()
  }

  return {
    kind: 'row',
    rowId: row.id,
    subject: row.subject
  }
}

export function sourceSubject(context: StatusContext): PrimarySelection {
  switch (context.kind) {
    case 'registeredSource':
    case 'registeredDirectory':
    case 'localBrowse':
      return {
        kind: 'source',
        context
      }
    case 'registeredFile':
    case 'none':
    case 'navigation':
    case 'readState':
      return clearSelection()
  }
}

export function clearSelection(): PrimarySelection {
  return { kind: 'none' }
}

export function isValidSelection(
  selection: PrimarySelection,
  projection: ContentProjection | undefined
): boolean {
  if (selection.kind !== 'row') {
    return true
  }

  const row = projection?.rows.find((candidate) => candidate.id === selection.rowId)
  return row?.subject !== undefined && sameSubject(row.subject, selection.subject)
}

function sameSubject(left: ContentRowSubject, right: ContentRowSubject): boolean {
  switch (left.kind) {
    case 'playableMedia':
      if (right.kind !== 'playableMedia') {
        return false
      }

      return (
        left.playableMediaId === right.playableMediaId &&
        left.sourceId === right.sourceId &&
        left.sourceFileId === right.sourceFileId
      )
    case 'sourceFile':
      if (right.kind !== 'sourceFile') {
        return false
      }

      return left.sourceId === right.sourceId && left.sourceFileId === right.sourceFileId
  }
}
