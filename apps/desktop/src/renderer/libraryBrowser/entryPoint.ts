import type { EntryPoint } from '../../shared/libraryHierarchy/readChildren'

export function sameEntryPoint(left: EntryPoint, right: EntryPoint): boolean {
  if (left.kind !== right.kind) {
    return false
  }

  if (left.kind === 'source') {
    return right.kind === 'source' && left.sourceId === right.sourceId
  }

  return right.kind === 'sourceLocation' && left.sourceLocationId === right.sourceLocationId
}

export function copyReadEntryPoint(entryPoint: EntryPoint): EntryPoint {
  switch (entryPoint.kind) {
    case 'source':
      return {
        kind: 'source',
        sourceId: entryPoint.sourceId
      }
    case 'sourceLocation':
      return {
        kind: 'sourceLocation',
        sourceLocationId: entryPoint.sourceLocationId
      }
  }
}
