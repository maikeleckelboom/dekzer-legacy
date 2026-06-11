import { libraryPublicationChannels } from '../../../../src/shared/library/boundary/publicationPlane'
import { libraryControlChannels } from '../../../../src/shared/library/boundary/controlPlane'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  BoundaryEventPump,
  registerBoundaryEventPumpIpc,
  type BoundaryEventPumpIpcMain,
  type BoundaryEventPumpWebContents
} from '../../../../src/main/library/boundary/eventPump'

type TestHost = {
  readonly client: {
    readonly readAfterBoundaryEvents: ReturnType<typeof vi.fn>
  }
}

describe('BoundaryEventPump', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('reads boundary events, emits batches, and advances the Main-owned cursor', async () => {
    vi.useFakeTimers()
    const host = testHost([
      {
        events: [{ type: 'sourceScanEvent', payload: { eventSequence: 1 } }],
        latestEventSequence: 1,
        earliestRetainedSequence: 1,
        gapDetected: false
      },
      {
        events: [{ type: 'maintainedSnapshotInvalidated', payload: { eventSequence: 2 } }],
        latestEventSequence: 2,
        earliestRetainedSequence: 1,
        gapDetected: false
      }
    ])
    const pump = new BoundaryEventPump(host as never)
    const subscriber = testWebContents(1)

    pump.subscribe(subscriber)
    pump.setHostStarted(true)
    await waitForMicrotasks()

    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledWith({
      lastSeenEventSequence: null,
      maxEvents: 32
    })
    expect(pump.lastSeenEventSequence).toBe(1)
    expect(subscriber.sent[0]).toMatchObject({
      channel: libraryPublicationChannels.boundary.events.batch,
      payload: {
        kind: 'batch',
        latestEventSequence: 1,
        events: [{ type: 'sourceScanEvent', payload: { eventSequence: 1 } }]
      }
    })

    await pump.pollOnce()
    expect(host.client.readAfterBoundaryEvents).toHaveBeenLastCalledWith({
      lastSeenEventSequence: 1,
      maxEvents: 32
    })
    expect(pump.lastSeenEventSequence).toBe(2)
    expect(subscriber.sent.at(-1)).toMatchObject({
      payload: {
        kind: 'batch',
        latestEventSequence: 2,
        events: [{ type: 'maintainedSnapshotInvalidated', payload: { eventSequence: 2 } }]
      }
    })

    pump.stop()
  })

  it('uses one polling loop for multiple subscribers', async () => {
    vi.useFakeTimers()
    const host = testHost([emptyReply(null), emptyReply(null)])
    const pump = new BoundaryEventPump(host as never)

    pump.subscribe(testWebContents(1))
    pump.subscribe(testWebContents(2))
    pump.setHostStarted(true)
    await waitForMicrotasks()

    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(1)

    await vi.advanceTimersByTimeAsync(250)
    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(2)

    pump.stop()
  })

  it('idles without a started host or active subscribers', async () => {
    vi.useFakeTimers()
    const host = testHost([emptyReply(null), emptyReply(null)])
    const pump = new BoundaryEventPump(host as never)
    const subscriber = testWebContents(1)

    const unsubscribe = pump.subscribe(subscriber)
    await pump.pollOnce()
    expect(host.client.readAfterBoundaryEvents).not.toHaveBeenCalled()

    pump.setHostStarted(true)
    await waitForMicrotasks()
    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(1)

    unsubscribe()
    await vi.advanceTimersByTimeAsync(250)
    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(1)
  })

  it('isolates a subscriber send failure while delivering to healthy subscribers', async () => {
    vi.useFakeTimers()
    const host = testHost([
      {
        events: [{ type: 'sourceScanEvent', payload: { eventSequence: 1 } }],
        latestEventSequence: 1,
        earliestRetainedSequence: 1,
        gapDetected: false
      }
    ])
    const pump = new BoundaryEventPump(host as never)
    const failingSubscriber = testWebContents(1, { throwOnSend: true })
    const healthySubscriber = testWebContents(2)

    pump.subscribe(failingSubscriber)
    pump.subscribe(healthySubscriber)
    pump.setHostStarted(true)
    await waitForMicrotasks()

    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(1)
    expect(healthySubscriber.sent).toHaveLength(1)
    expect(healthySubscriber.sent[0]).toMatchObject({
      channel: libraryPublicationChannels.boundary.events.batch,
      payload: { kind: 'batch', latestEventSequence: 1 }
    })
    expect(pump.subscriberCount).toBe(1)

    pump.stop()
  })

  it('removes destroyed subscribers without affecting healthy subscribers', async () => {
    vi.useFakeTimers()
    const host = testHost([
      {
        events: [{ type: 'sourceScanEvent', payload: { eventSequence: 1 } }],
        latestEventSequence: 1,
        earliestRetainedSequence: 1,
        gapDetected: false
      }
    ])
    const pump = new BoundaryEventPump(host as never)
    const destroyedSubscriber = testWebContents(1, { destroyed: true })
    const healthySubscriber = testWebContents(2)

    pump.subscribe(destroyedSubscriber)
    pump.subscribe(healthySubscriber)
    pump.setHostStarted(true)
    await waitForMicrotasks()

    expect(destroyedSubscriber.sent).toHaveLength(0)
    expect(healthySubscriber.sent).toHaveLength(1)
    expect(pump.subscriberCount).toBe(1)

    pump.stop()
  })

  it('delivers service read failures to healthy subscribers', async () => {
    vi.useFakeTimers()
    const host = {
      client: {
        readAfterBoundaryEvents: vi.fn(async () => {
          throw new Error('host read failed')
        })
      }
    }
    const pump = new BoundaryEventPump(host as never)
    const subscriber = testWebContents(1)

    pump.subscribe(subscriber)
    pump.setHostStarted(true)
    await waitForMicrotasks()

    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledTimes(1)
    expect(subscriber.sent).toEqual([
      {
        channel: libraryPublicationChannels.boundary.events.batch,
        payload: { kind: 'failed', detail: 'host read failed' }
      }
    ])
    expect(pump.subscriberCount).toBe(1)

    pump.stop()
  })

  it('registers explicit subscribe and unsubscribe IPC handlers', async () => {
    const host = testHost([emptyReply(null)])
    const pump = new BoundaryEventPump(host as never)
    const registered: Array<{
      readonly channel: string
      readonly listener: Parameters<BoundaryEventPumpIpcMain['handle']>[1]
    }> = []
    const webContents = testWebContents(7)

    registerBoundaryEventPumpIpc(
      {
        handle(channel, listener): void {
          registered.push({ channel, listener })
        }
      },
      pump
    )

    await expect(
      registered
        .find((handler) => handler.channel === libraryControlChannels.boundary.events.subscribe)
        ?.listener({ sender: webContents })
    ).resolves.toEqual({ kind: 'subscribed' })
    expect(pump.subscriberCount).toBe(1)

    await expect(
      registered
        .find((handler) => handler.channel === libraryControlChannels.boundary.events.unsubscribe)
        ?.listener({ sender: webContents })
    ).resolves.toEqual({ kind: 'unsubscribed' })
    expect(pump.subscriberCount).toBe(0)
  })
})

function testHost(replies: readonly ReturnType<typeof emptyReply>[]): TestHost {
  const queuedReplies = [...replies]

  return {
    client: {
      readAfterBoundaryEvents: vi.fn(async () => queuedReplies.shift() ?? emptyReply(null))
    }
  }
}

function emptyReply(latestEventSequence: number | null): {
  readonly events: readonly unknown[]
  readonly latestEventSequence: number | null
  readonly earliestRetainedSequence: number | null
  readonly gapDetected: boolean
} {
  return {
    events: [],
    latestEventSequence,
    earliestRetainedSequence: null,
    gapDetected: false
  }
}

function testWebContents(
  id: number,
  options: { readonly throwOnSend?: boolean; readonly destroyed?: boolean } = {}
): BoundaryEventPumpWebContents & {
  readonly sent: Array<{ readonly channel: string; readonly payload: unknown }>
} {
  const sent: Array<{ readonly channel: string; readonly payload: unknown }> = []

  return {
    id,
    sent,
    isDestroyed: () => options.destroyed === true,
    send(channel, payload): void {
      if (options.throwOnSend === true) {
        throw new Error('webContents unavailable')
      }

      sent.push({ channel, payload })
    }
  }
}

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
