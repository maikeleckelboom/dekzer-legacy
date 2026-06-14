import { describe, expect, it, vi } from 'vitest'

import { invalidateSourceStatus } from '../../../../src/renderer/library/runtime/sourceStatusInvalidation'

describe('invalidateSourceStatus', () => {
  it('invalidates every source status reader for one source id', () => {
    const sourceLifecycleRead = invalidationController()
    const integrityRead = invalidationController()
    const maintenanceRead = invalidationController()
    const activityRead = invalidationController()

    invalidateSourceStatus(
      {
        sourceLifecycleRead,
        integrityRead,
        maintenanceRead,
        activityRead
      },
      'source-7'
    )

    expect(sourceLifecycleRead.invalidateSource).toHaveBeenCalledWith('source-7')
    expect(integrityRead.invalidateSource).toHaveBeenCalledWith('source-7')
    expect(maintenanceRead.invalidateSource).toHaveBeenCalledWith('source-7')
    expect(activityRead.invalidateSource).toHaveBeenCalledWith('source-7')
  })
})

function invalidationController(): { readonly invalidateSource: (sourceId: string) => void } {
  return {
    invalidateSource: vi.fn()
  }
}
