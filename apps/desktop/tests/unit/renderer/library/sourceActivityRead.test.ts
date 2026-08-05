import { describe, expect, it, vi } from 'vitest'
import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

import {
  createController,
  type ReadApi
} from '../../../../src/renderer/library/sourceActivity/read'
import type { SourceActivityReadResult } from '../../../../src/shared/library/source/activity'

describe('source activity read controller', () => {
  it('invalidates one source activity snapshot and rejects late reads', async () => {
    const staleRead = deferred<SourceActivityReadResult>()
    const api = activityApi(
      vi
        .fn()
        .mockResolvedValueOnce({
          state: 'ready',
          activity: sourceActivity({ sourceId: '7', scan: { state: 'completed' } })
        })
        .mockResolvedValueOnce({
          state: 'ready',
          activity: sourceActivity({ sourceId: '8', scan: { state: 'idle' } })
        })
        .mockResolvedValueOnce({
          state: 'readFailed',
          error: { code: 'readFailed', message: 'Unable to read source activity.' }
        })
        .mockReturnValueOnce(staleRead.promise)
        .mockResolvedValue({
          state: 'ready',
          activity: sourceActivity({ sourceId: '7', scan: { state: 'running' } })
        })
    )
    const controller = createController(api)

    await expect(controller.read('7')).resolves.toBe(true)
    await expect(controller.read('8')).resolves.toBe(true)
    await expect(controller.read('7')).resolves.toBe(false)

    const stale = controller.read('7')
    controller.invalidateSource('7')

    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()
    expect(controller.readErrorsBySourceId.value.get('7')).toBeUndefined()
    expect(controller.snapshotBySourceId.value.get('8')).toMatchObject({
      sourceId: '8',
      scan: { state: 'idle' }
    })

    staleRead.resolve({
      state: 'ready',
      activity: sourceActivity({ sourceId: '7', scan: { state: 'running' } })
    })

    await expect(stale).resolves.toBe(false)
    expect(controller.snapshotBySourceId.value.get('7')).toBeUndefined()

    await expect(controller.refresh(['7'])).resolves.toBe(true)
    expect(controller.snapshotBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      scan: { state: 'running' }
    })
  })
})

function activityApi(readSourceActivity: ReadApi['sourceActivity']['readSourceActivity']): ReadApi {
  return {
    sourceActivity: {
      readSourceActivity
    }
  }
}

function sourceActivity(
  overrides: {
    readonly sourceId?: string
    readonly scan?: Partial<ReadSourceActivityReply['scan']>
  } = {}
): ReadSourceActivityReply {
  const sourceId = overrides.sourceId ?? '7'

  return {
    sourceId,
    admission: 'active',
    browse: {
      state: 'ready',
      detail: 'Source is ready to browse.'
    },
    scan: {
      state: 'idle',
      counters: {},
      ...(overrides.scan ?? {})
    },
    preparation: {
      state: 'complete',
      backlog: {
        hash: 0,
        probe: 0,
        attachment: 0,
        promotion: 0,
        identity: 0
      },
      provenance: 'maintenanceSnapshot',
      boundedBatch: true
    }
  }
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolveDeferred: (value: T) => void = () => undefined
  const promise = new Promise<T>((resolve) => {
    resolveDeferred = resolve
  })

  return {
    promise,
    resolve: resolveDeferred
  }
}
