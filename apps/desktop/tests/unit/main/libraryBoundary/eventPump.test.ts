import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  BoundaryEventPump,
  registerBoundaryEventPumpIpc,
  type BoundaryEventPumpIpcMain,
  type BoundaryEventPumpWebContents
} from '../../../../src/main/libraryBoundary/eventPump'
import { boundaryEventChannels } from '../../../../src/shared/libraryBoundary/events'

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
      channel: boundaryEventChannels.batch,
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
        .find((handler) => handler.channel === boundaryEventChannels.subscribe)
        ?.listener({ sender: webContents })
    ).resolves.toEqual({ kind: 'subscribed' })
    expect(pump.subscriberCount).toBe(1)

    await expect(
      registered
        .find((handler) => handler.channel === boundaryEventChannels.unsubscribe)
        ?.listener({ sender: webContents })
    ).resolves.toEqual({ kind: 'unsubscribed' })
    expect(pump.subscriberCount).toBe(0)
  })
})

function testHost(replies: readonly ReturnType<typeof emptyReply>[]) {
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

function testWebContents(id: number): BoundaryEventPumpWebContents & {
  readonly sent: Array<{ readonly channel: string; readonly payload: unknown }>
} {
  const sent: Array<{ readonly channel: string; readonly payload: unknown }> = []

  return {
    id,
    sent,
    send(channel, payload): void {
      sent.push({ channel, payload })
    }
  }
}

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
