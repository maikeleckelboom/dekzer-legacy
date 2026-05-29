import type {
  ReadLibraryBoundaryEventsAfterReply,
  ReadLibraryBoundaryEventsReply
} from '@dekzer/library-boundary-contract'

import type { LibraryBoundaryHost } from '../libraryBoundary/host'
import {
  boundaryEventChannels,
  type BoundaryEventReadAfterRequest,
  type BoundaryEventReadPendingRequest
} from '../../shared/libraryBoundary/events'

export type { BoundaryEventReadAfterRequest, BoundaryEventReadPendingRequest }

export type BoundaryEventIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<unknown>
  ): void
}

function isBoundaryEventReadPendingRequest(
  request: unknown
): request is BoundaryEventReadPendingRequest {
  return (
    typeof request === 'object' &&
    request !== null &&
    'maxEvents' in request &&
    typeof (request as BoundaryEventReadPendingRequest).maxEvents === 'number' &&
    !('lastSeenEventSequence' in request)
  )
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
    boundaryEventChannels.readPending,
    async (_event, request): Promise<ReadLibraryBoundaryEventsReply> => {
      if (!isBoundaryEventReadPendingRequest(request)) {
        return { events: [] }
      }

      try {
        const client = host.client
        return await client.readPendingBoundaryEvents({
          maxEvents: request.maxEvents
        })
      } catch {
        return { events: [] }
      }
    }
  )

  ipcMain.handle(
    boundaryEventChannels.readAfter,
    async (_event, request): Promise<ReadLibraryBoundaryEventsAfterReply> => {
      if (!isBoundaryEventReadAfterRequest(request)) {
        return { events: [], latestEventSequence: null }
      }

      try {
        const client = host.client
        return await client.readAfterBoundaryEvents({
          lastSeenEventSequence: request.lastSeenEventSequence,
          maxEvents: request.maxEvents
        })
      } catch {
        return { events: [], latestEventSequence: null }
      }
    }
  )
}
