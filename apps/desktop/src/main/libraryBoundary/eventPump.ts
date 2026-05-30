import type { ReadLibraryBoundaryEventsAfterReply } from '@dekzer/library-boundary-contract'

import type { LibraryBoundaryHost } from './host'
import {
  boundaryEventChannels,
  type BoundaryEventBatchPayload,
  type BoundaryEventDeliveryPayload,
  type BoundaryEventSubscribeResult,
  type BoundaryEventUnsubscribeResult
} from '../../shared/libraryBoundary/events'

const pollIntervalMs = 250
const maxEventsPerPoll = 32

export type BoundaryEventPumpIpcMain = {
  handle(
    channel: string,
    listener: (event: BoundaryEventPumpIpcEvent) => Promise<unknown> | unknown
  ): void
}

export type BoundaryEventPumpIpcEvent = {
  readonly sender: BoundaryEventPumpWebContents
}

export type BoundaryEventPumpWebContents = {
  readonly id: number
  send(channel: string, payload: BoundaryEventDeliveryPayload): void
  isDestroyed?(): boolean
  once?(eventName: 'destroyed', listener: () => void): void
  off?(eventName: 'destroyed', listener: () => void): void
}

type BoundaryEventPumpSubscriber = {
  readonly webContents: BoundaryEventPumpWebContents
  readonly removeDestroyedListener?: () => void
  count: number
}

export class BoundaryEventPump {
  readonly #host: LibraryBoundaryHost
  readonly #subscribers = new Map<number, BoundaryEventPumpSubscriber>()
  #hostStarted = false
  #lastSeenEventSequence: number | null = null
  #pollTimer: ReturnType<typeof setInterval> | undefined
  #pollInFlight = false

  constructor(host: LibraryBoundaryHost) {
    this.#host = host
  }

  get subscriberCount(): number {
    let count = 0

    for (const subscriber of this.#subscribers.values()) {
      count += subscriber.count
    }

    return count
  }

  get lastSeenEventSequence(): number | null {
    return this.#lastSeenEventSequence
  }

  setHostStarted(started: boolean): void {
    this.#hostStarted = started

    if (!started) {
      this.#stopPolling()
      this.#lastSeenEventSequence = null
      return
    }

    this.#syncPolling()
  }

  subscribe(webContents: BoundaryEventPumpWebContents): () => void {
    const existingSubscriber = this.#subscribers.get(webContents.id)

    if (existingSubscriber !== undefined) {
      existingSubscriber.count += 1
      this.#syncPolling()
      return () => this.unsubscribe(webContents)
    }

    const removeDestroyedListener = this.#trackWebContentsDestroyed(webContents)
    this.#subscribers.set(webContents.id, {
      webContents,
      ...(removeDestroyedListener === undefined ? {} : { removeDestroyedListener }),
      count: 1
    })
    this.#syncPolling()

    return () => this.unsubscribe(webContents)
  }

  unsubscribe(webContents: BoundaryEventPumpWebContents): void {
    const subscriber = this.#subscribers.get(webContents.id)

    if (subscriber === undefined) {
      return
    }

    if (subscriber.count > 1) {
      subscriber.count -= 1
      return
    }

    subscriber.removeDestroyedListener?.()
    this.#subscribers.delete(webContents.id)
    this.#syncPolling()
  }

  stop(): void {
    this.#stopPolling()

    for (const subscriber of this.#subscribers.values()) {
      subscriber.removeDestroyedListener?.()
    }

    this.#subscribers.clear()
    this.#lastSeenEventSequence = null
  }

  async pollOnce(): Promise<void> {
    if (!this.#hostStarted || this.subscriberCount === 0 || this.#pollInFlight) {
      return
    }

    this.#pollInFlight = true

    try {
      const reply: ReadLibraryBoundaryEventsAfterReply =
        await this.#host.client.readAfterBoundaryEvents({
          lastSeenEventSequence: this.#lastSeenEventSequence,
          maxEvents: maxEventsPerPoll
        })

      if (reply.latestEventSequence !== null) {
        this.#lastSeenEventSequence = reply.latestEventSequence
      }

      this.#publish({
        kind: 'batch',
        events: reply.events,
        latestEventSequence: reply.latestEventSequence,
        earliestRetainedSequence: reply.earliestRetainedSequence,
        gapDetected: reply.gapDetected
      } satisfies BoundaryEventBatchPayload)
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'boundary event read failed'
      this.#publish({ kind: 'failed', detail })
    } finally {
      this.#pollInFlight = false
    }
  }

  #syncPolling(): void {
    if (!this.#hostStarted || this.subscriberCount === 0) {
      this.#stopPolling()
      return
    }

    if (this.#pollTimer !== undefined) {
      return
    }

    this.#pollTimer = setInterval(() => {
      void this.pollOnce()
    }, pollIntervalMs)
    void this.pollOnce()
  }

  #stopPolling(): void {
    if (this.#pollTimer === undefined) {
      return
    }

    clearInterval(this.#pollTimer)
    this.#pollTimer = undefined
  }

  #publish(payload: BoundaryEventDeliveryPayload): void {
    for (const subscriber of this.#subscribers.values()) {
      const webContents = subscriber.webContents

      if (webContents.isDestroyed?.()) {
        this.#subscribers.delete(webContents.id)
        subscriber.removeDestroyedListener?.()
        continue
      }

      webContents.send(boundaryEventChannels.batch, payload)
    }

    this.#syncPolling()
  }

  #trackWebContentsDestroyed(webContents: BoundaryEventPumpWebContents): (() => void) | undefined {
    if (webContents.once === undefined) {
      return undefined
    }

    const removeSubscriber = (): void => {
      const subscriber = this.#subscribers.get(webContents.id)
      subscriber?.removeDestroyedListener?.()
      this.#subscribers.delete(webContents.id)
      this.#syncPolling()
    }

    webContents.once('destroyed', removeSubscriber)

    if (webContents.off === undefined) {
      return undefined
    }

    return () => {
      webContents.off?.('destroyed', removeSubscriber)
    }
  }
}

export function registerBoundaryEventPumpIpc(
  ipcMain: BoundaryEventPumpIpcMain,
  pump: BoundaryEventPump
): void {
  ipcMain.handle(
    boundaryEventChannels.subscribe,
    async (event): Promise<BoundaryEventSubscribeResult> => {
      pump.subscribe(event.sender)
      return { kind: 'subscribed' }
    }
  )

  ipcMain.handle(
    boundaryEventChannels.unsubscribe,
    async (event): Promise<BoundaryEventUnsubscribeResult> => {
      pump.unsubscribe(event.sender)
      return { kind: 'unsubscribed' }
    }
  )
}
