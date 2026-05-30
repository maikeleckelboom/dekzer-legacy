export const boundaryEventChannels = {
  readAfter: 'desktop:library-boundary:read-after-events',
  subscribe: 'desktop:library-boundary:events:subscribe',
  unsubscribe: 'desktop:library-boundary:events:unsubscribe',
  batch: 'desktop:library-boundary:events:batch'
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

export type BoundaryEventBatchPayload = {
  readonly kind: 'batch'
  readonly events: readonly unknown[]
  readonly latestEventSequence: number | null
  readonly earliestRetainedSequence: number | null
  readonly gapDetected: boolean
}

export type BoundaryEventFailurePayload = {
  readonly kind: 'failed'
  readonly detail: string
}

export type BoundaryEventDeliveryPayload = BoundaryEventBatchPayload | BoundaryEventFailurePayload

export type BoundaryEventSubscribeResult =
  | { readonly kind: 'subscribed' }
  | { readonly kind: 'failed'; readonly detail: string }

export type BoundaryEventUnsubscribeResult =
  | { readonly kind: 'unsubscribed' }
  | { readonly kind: 'failed'; readonly detail: string }
