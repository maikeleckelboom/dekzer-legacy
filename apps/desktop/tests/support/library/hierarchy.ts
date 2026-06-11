import type { ReadRequest } from '../../../src/shared/library/hierarchy/read'

export function firstAvailableSourceReadRequest(): ReadRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
}
