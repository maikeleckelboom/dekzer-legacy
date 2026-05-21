import type { ReadRequest } from '../../src/shared/libraryHierarchy/readChildren'

export function firstAvailableSourceReadRequest(): ReadRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
}
