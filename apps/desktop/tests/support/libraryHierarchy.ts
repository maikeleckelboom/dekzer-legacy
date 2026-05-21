import type { LibraryHierarchyReadChildrenRequest } from '../../src/shared/libraryHierarchy/readChildren'

export function firstAvailableSourceReadRequest(): LibraryHierarchyReadChildrenRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
}
