export const boundaryEventChannels = {
  readPending: 'desktop:library-boundary:read-pending-events',
  readAfter: 'desktop:library-boundary:read-after-events',
  eventsChanged: 'desktop:library-boundary:events-changed'
} as const

export type BoundaryEventReadPendingRequest = {
  readonly maxEvents: number
}

export type BoundaryEventReadPendingReply = {
  readonly events: readonly unknown[]
}

export type BoundaryEventReadAfterRequest = {
  readonly lastSeenEventSequence: number | null
  readonly maxEvents: number
}

export type BoundaryEventReadAfterReply = {
  readonly events: readonly unknown[]
  readonly latestEventSequence: number | null
}
