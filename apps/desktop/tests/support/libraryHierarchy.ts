import type { LibraryHierarchyReadRequest } from '../../src/shared/libraryHierarchy/read'

export function firstAvailableSourceReadRequest(): LibraryHierarchyReadRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
}
