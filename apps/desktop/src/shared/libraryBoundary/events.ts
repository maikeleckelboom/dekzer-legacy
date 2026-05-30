export const boundaryEventChannels = {
  subscribe: 'desktop:library-boundary:events:subscribe',
  unsubscribe: 'desktop:library-boundary:events:unsubscribe',
  batch: 'desktop:library-boundary:events:batch'
} as const

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
