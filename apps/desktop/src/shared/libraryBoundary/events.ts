export const boundaryEventChannels = {
  readAfter: 'desktop:library-boundary:read-after-events'
} as const

export type BoundaryEventReadAfterRequest = {
  readonly lastSeenEventSequence: number | null
  readonly maxEvents: number
}

export type BoundaryEventReadAfterReply = {
  readonly events: readonly unknown[]
  readonly latestEventSequence: number | null
  readonly earliestRetainedSequence: number | null
  readonly gapDetected: boolean
}

export type BoundaryEventReadResult =
  | { readonly kind: 'ready'; readonly reply: BoundaryEventReadAfterReply }
  | { readonly kind: 'failed'; readonly detail: string }
