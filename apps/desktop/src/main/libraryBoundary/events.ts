import type {
  ReadLibraryBoundaryEventsAfterReply
} from '@dekzer/library-boundary-contract'

import type { LibraryBoundaryHost } from '../libraryBoundary/host'
import {
  boundaryEventChannels,
  type BoundaryEventReadAfterRequest,
  type BoundaryEventReadResult
} from '../../shared/libraryBoundary/events'

export type { BoundaryEventReadAfterRequest }

export type BoundaryEventIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<unknown>
  ): void
}

function isBoundaryEventReadAfterRequest(
  request: unknown
): request is BoundaryEventReadAfterRequest {
  return (
    typeof request === 'object' &&
    request !== null &&
    'maxEvents' in request &&
    'lastSeenEventSequence' in request
  )
}

export function registerBoundaryEventIpc(
  ipcMain: BoundaryEventIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(
    boundaryEventChannels.readAfter,
    async (_event, request): Promise<BoundaryEventReadResult> => {
      if (!isBoundaryEventReadAfterRequest(request)) {
        return { kind: 'failed', detail: 'invalid request shape' }
      }

      try {
        const client = host.client
        const reply: ReadLibraryBoundaryEventsAfterReply =
          await client.readAfterBoundaryEvents({
            lastSeenEventSequence: request.lastSeenEventSequence,
            maxEvents: request.maxEvents
          })
        return {
          kind: 'ready',
          reply: {
            events: reply.events,
            latestEventSequence: reply.latestEventSequence,
            earliestRetainedSequence: reply.earliestRetainedSequence,
            gapDetected: reply.gapDetected
          }
        }
      } catch (error) {
        const detail =
          error instanceof Error ? error.message : 'boundary event read failed'
        return { kind: 'failed', detail }
      }
    }
  )
}
