import { describe, expect, it, vi } from 'vitest'
import type { BoundaryEventIpcMain } from '../../../../src/main/libraryBoundary/events'

function createRecordingIpc(): {
  ipcMain: BoundaryEventIpcMain
  registered: Array<{ channel: string; listener: (event: unknown, request: unknown) => Promise<unknown> }>
} {
  const registered: Array<{
    channel: string
    listener: (event: unknown, request: unknown) => Promise<unknown>
  }> = []
  const ipcMain: BoundaryEventIpcMain = {
    handle(channel, listener) {
      registered.push({ channel, listener })
    }
  }
  return { ipcMain, registered }
}

function createHost() {
  return {
    client: {
      readAfterBoundaryEvents: vi.fn().mockResolvedValue({
        events: [],
        latestEventSequence: null,
        earliestRetainedSequence: null,
        gapDetected: false
      })
    }
  }
}

describe('boundary event IPC request validation', () => {
  it('returns failed for non-object request', async () => {
    const { ipcMain, registered } = createRecordingIpc()

    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    expect(registered.length).toBe(1)
    const handler = registered[0].listener
    const result = await handler({}, 'not-an-object')
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for request with missing maxEvents', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for request with zero maxEvents', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null, maxEvents: 0 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for request with negative maxEvents', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null, maxEvents: -1 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for request with non-integer maxEvents', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null, maxEvents: 1.5 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for negative lastSeenEventSequence', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: -1, maxEvents: 16 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for non-integer lastSeenEventSequence', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: 1.5, maxEvents: 16 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('returns failed for string lastSeenEventSequence', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    registerBoundaryEventIpc(ipcMain, createHost() as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: '42', maxEvents: 16 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'invalid request shape' })
  })

  it('accepts valid request with null lastSeenEventSequence', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    const host = createHost()
    registerBoundaryEventIpc(ipcMain, host as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null, maxEvents: 16 })
    expect(result).toMatchObject({ kind: 'ready' })
    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledWith({
      lastSeenEventSequence: null,
      maxEvents: 16
    })
  })

  it('accepts valid request with numeric cursor', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    const host = createHost()
    host.client.readAfterBoundaryEvents = vi.fn().mockResolvedValue({
      events: [],
      latestEventSequence: 42,
      earliestRetainedSequence: 0,
      gapDetected: false
    })
    registerBoundaryEventIpc(ipcMain, host as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: 0, maxEvents: 32 })
    expect(result).toMatchObject({ kind: 'ready' })
    expect(host.client.readAfterBoundaryEvents).toHaveBeenCalledWith({
      lastSeenEventSequence: 0,
      maxEvents: 32
    })
  })

  it('returns failed when client throws', async () => {
    const { ipcMain, registered } = createRecordingIpc()
    const { registerBoundaryEventIpc } = await import(
      '../../../../src/main/libraryBoundary/events'
    )
    const host = createHost()
    host.client.readAfterBoundaryEvents = vi.fn().mockRejectedValue(new Error('host failure'))
    registerBoundaryEventIpc(ipcMain, host as never)

    const handler = registered[0].listener
    const result = await handler({}, { lastSeenEventSequence: null, maxEvents: 16 })
    expect(result).toMatchObject({ kind: 'failed', detail: 'host failure' })
  })
})
