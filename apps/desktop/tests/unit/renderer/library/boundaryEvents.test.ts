import { describe, expect, it, vi } from 'vitest'

import {
  createBoundaryEventsController,
  type BoundaryEventApi
} from '../../../../src/renderer/library/boundary/boundaryEvents'
import type { BoundaryEventDeliveryPayload } from '../../../../src/shared/libraryBoundary/events'

describe('createBoundaryEventsController', () => {
  it('consumes Main-delivered event batches without polling or owning a cursor', () => {
    const setIntervalSpy = vi.spyOn(globalThis, 'setInterval')
    const api = testEventApi()
    const controller = createBoundaryEventsController(api)

    controller.start()

    expect(api.subscribe).toHaveBeenCalledTimes(1)
    expect(setIntervalSpy).not.toHaveBeenCalled()
    expect((controller as Record<string, unknown>).lastSeenEventSequence).toBeUndefined()

    api.deliver({
      kind: 'batch',
      events: [
        {
          type: 'sourceScanEvent',
          payload: {
            eventSequence: 1,
            occurredAtMs: 1000,
            kind: 'sourceScanCompleted',
            rootId: 'root-1',
            scanRunId: 'scan-1',
            phase: 'scanning',
            directoriesVisited: 1,
            filesVisited: 2,
            filesDiscovered: 3,
            mediaCandidates: 4,
            queuedWorkItems: 5,
            detail: null
          }
        },
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 2,
            occurredAtMs: 1001,
            invalidation: { scope: 'libraryBrowser', revision: '8' }
          }
        }
      ],
      latestEventSequence: 2,
      earliestRetainedSequence: 1,
      gapDetected: false
    })

    expect(controller.scanProgress.value.get('root-1')).toMatchObject({
      kind: 'completed',
      filesDiscovered: 3,
      queuedWorkItems: 5
    })
    expect(controller.maintainedSnapshotInvalidations.value).toEqual([
      {
        eventSequence: 2,
        occurredAtMs: 1001,
        invalidation: { scope: 'libraryBrowser', revision: '8' }
      }
    ])

    controller.stop()
    expect(api.unsubscribe).toHaveBeenCalledTimes(1)
    setIntervalSpy.mockRestore()
  })

  it('keeps gap recovery separate from normal maintained snapshot invalidation', () => {
    const api = testEventApi()
    const controller = createBoundaryEventsController(api)

    controller.start()
    api.deliver({
      kind: 'batch',
      events: [
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 3,
            occurredAtMs: 1002,
            invalidation: { scope: 'navigationRows', revision: '9' }
          }
        }
      ],
      latestEventSequence: 3,
      earliestRetainedSequence: 2,
      gapDetected: true
    })

    expect(controller.recoveryNeeded.value).toBe(true)
    expect(controller.gapDetected.value).toBe(true)
    expect(
      controller.maintainedSnapshotInvalidations.value.map((event) => event.invalidation)
    ).toEqual([{ scope: 'navigationRows', revision: '9' }])

    controller.acknowledgedGap()
    expect(controller.recoveryNeeded.value).toBe(false)
    expect(controller.maintainedSnapshotInvalidations.value).toHaveLength(1)
  })

  it('reports read failures from the Main delivery channel', () => {
    const api = testEventApi()
    const controller = createBoundaryEventsController(api)

    controller.start()
    api.deliver({ kind: 'failed', detail: 'host unavailable' })
    expect(controller.lastReadFailed.value).toBe(true)

    api.deliver({
      kind: 'batch',
      events: [],
      latestEventSequence: null,
      earliestRetainedSequence: null,
      gapDetected: false
    })
    expect(controller.lastReadFailed.value).toBe(false)
  })
})

function testEventApi(): BoundaryEventApi & {
  readonly deliver: (payload: BoundaryEventDeliveryPayload) => void
  readonly unsubscribe: ReturnType<typeof vi.fn>
} {
  let callback: ((payload: BoundaryEventDeliveryPayload) => void) | undefined
  const unsubscribe = vi.fn()
  const subscribe = vi.fn((next: (payload: BoundaryEventDeliveryPayload) => void) => {
    callback = next
    return unsubscribe
  })

  return {
    subscribe,
    unsubscribe,
    deliver(payload): void {
      callback?.(payload)
    }
  }
}
