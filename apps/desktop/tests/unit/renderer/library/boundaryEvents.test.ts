import { describe, expect, it, vi } from 'vitest'

import {
  createBoundaryEventsController,
  type BoundaryEventApi
} from '../../../../src/renderer/library/boundary/boundaryEvents'
import type { BoundaryEventDeliveryPayload } from '../../../../src/shared/library/boundary/events'

describe('createBoundaryEventsController', () => {
  it('consumes Main-delivered event batches without polling', () => {
    const setIntervalSpy = vi.spyOn(globalThis, 'setInterval')
    const api = testEventApi()
    const controller = createBoundaryEventsController(api)

    controller.start()

    expect(api.subscribe).toHaveBeenCalledTimes(1)
    expect(setIntervalSpy).not.toHaveBeenCalled()

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
    expect(controller.maintainedSnapshotInvalidationSignal.value).toBe(1)
    expect(controller.consumeMaintainedSnapshotInvalidations()).toEqual([
      {
        eventSequence: 2,
        occurredAtMs: 1001,
        invalidation: { scope: 'libraryBrowser', revision: '8' }
      }
    ])
    expect(controller.consumeMaintainedSnapshotInvalidations()).toEqual([])

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
      controller.consumeMaintainedSnapshotInvalidations().map((event) => event.invalidation)
    ).toEqual([{ scope: 'navigationRows', revision: '9' }])

    controller.acknowledgedGap()
    expect(controller.recoveryNeeded.value).toBe(false)
    expect(controller.consumeMaintainedSnapshotInvalidations()).toEqual([])
  })

  it('consumes multiple maintained invalidation batches once without retaining old events', () => {
    const api = testEventApi()
    const controller = createBoundaryEventsController(api)

    controller.start()
    api.deliver({
      kind: 'batch',
      events: [
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 4,
            occurredAtMs: 1004,
            invalidation: { scope: 'libraryBrowser', revision: '10' }
          }
        }
      ],
      latestEventSequence: 4,
      earliestRetainedSequence: 4,
      gapDetected: false
    })
    api.deliver({
      kind: 'batch',
      events: [
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 5,
            occurredAtMs: 1005,
            invalidation: { scope: 'navigationRows', revision: '11' }
          }
        },
        {
          type: 'maintainedSnapshotInvalidated',
          payload: {
            eventSequence: 6,
            occurredAtMs: 1006,
            invalidation: { scope: 'libraryBrowser', revision: '12' }
          }
        }
      ],
      latestEventSequence: 6,
      earliestRetainedSequence: 4,
      gapDetected: false
    })

    expect(controller.maintainedSnapshotInvalidationSignal.value).toBe(2)
    expect(
      controller.consumeMaintainedSnapshotInvalidations().map((event) => event.eventSequence)
    ).toEqual([4, 5, 6])
    expect(controller.consumeMaintainedSnapshotInvalidations()).toEqual([])
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
