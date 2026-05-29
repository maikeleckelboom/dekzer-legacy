import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'

import type { RendererApi } from '../../../shared/rendererApi'

// Minimal local event types to avoid cross-package dependency in web build.
type SourceScanEventKind =
  | 'sourceScanStarted'
  | 'sourceScanProgressed'
  | 'sourceScanCompleted'
  | 'sourceScanFailed'
  | 'sourceScanBlocked'

type SourceScanEvent = {
  readonly eventSequence: number
  readonly occurredAtMs: number
  readonly kind: SourceScanEventKind
  readonly rootId: string
  readonly scanRunId: string
  readonly phase: string
  readonly directoriesVisited: number
  readonly filesVisited: number
  readonly filesDiscovered: number
  readonly mediaCandidates: number
  readonly queuedWorkItems: number
  readonly detail: string | null
}

type LibraryBoundaryEvent =
  | { readonly type: 'sourceScanEvent'; readonly payload: SourceScanEvent }
  | {
      readonly type: 'maintainedSnapshotInvalidated'
      readonly payload: unknown
    }

export type ScanProgressState =
  | { readonly kind: 'idle' }
  | {
      readonly kind: 'scanning'
      readonly rootId: string
      readonly scanRunId: string
      readonly directoriesVisited: number
      readonly filesVisited: number
      readonly filesDiscovered: number
      readonly mediaCandidates: number
      readonly queuedWorkItems: number
    }
  | {
      readonly kind: 'completed'
      readonly rootId: string
      readonly scanRunId: string
      readonly filesDiscovered: number
      readonly queuedWorkItems: number
    }
  | {
      readonly kind: 'failed'
      readonly rootId: string
      readonly detail: string | null
    }
  | {
      readonly kind: 'blocked'
      readonly rootId: string
      readonly detail: string | null
    }

export type BoundaryEventScannerController = {
  readonly scanProgress: Ref<ReadonlyMap<string, ScanProgressState>>
  readonly start: () => void
  readonly stop: () => void
}

export type BoundaryEventApi = RendererApi['library']['events']

const POLL_INTERVAL_MS = 250
const MAX_EVENTS_PER_POLL = 32

export function useBoundaryEventScanner(
  eventApi: BoundaryEventApi = getRendererApi().library.events
): BoundaryEventScannerController {
  const controller = createBoundaryEventScannerController(eventApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createBoundaryEventScannerController(
  eventApi: BoundaryEventApi
): BoundaryEventScannerController {
  const scanProgress = shallowRef<ReadonlyMap<string, ScanProgressState>>(new Map())
  let lastSeenEventSequence: number | null = null
  let pollTimer: ReturnType<typeof setInterval> | null = null
  let running = false

  async function poll(): Promise<void> {
    if (!running) {
      return
    }

    try {
      const reply = await eventApi.readAfter({
        lastSeenEventSequence,
        maxEvents: MAX_EVENTS_PER_POLL
      })

      const events = reply.events as readonly LibraryBoundaryEvent[]

      if (events.length > 0) {
        applyEvents(events)
      }

      if (reply.latestEventSequence !== null) {
        lastSeenEventSequence = reply.latestEventSequence
      }
    } catch {
      // Silently ignore poll errors
    }
  }

  function applyEvents(events: readonly LibraryBoundaryEvent[]): void {
    const nextProgress = new Map(scanProgress.value)

    for (const event of events) {
      if (event.type !== 'sourceScanEvent') {
        continue
      }

      const progress = scanProgressFromEvent(event.payload)
      nextProgress.set(event.payload.rootId, progress)
    }

    scanProgress.value = nextProgress
  }

  function start(): void {
    if (running) {
      return
    }

    running = true
    pollTimer = setInterval(poll, POLL_INTERVAL_MS)
    void poll()
  }

  function stop(): void {
    running = false

    if (pollTimer !== null) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  return {
    scanProgress,
    start,
    stop
  }
}

function scanProgressFromEvent(event: SourceScanEvent): ScanProgressState {
  switch (event.kind) {
    case 'sourceScanStarted':
    case 'sourceScanProgressed':
      return {
        kind: 'scanning',
        rootId: event.rootId,
        scanRunId: event.scanRunId,
        directoriesVisited: event.directoriesVisited,
        filesVisited: event.filesVisited,
        filesDiscovered: event.filesDiscovered,
        mediaCandidates: event.mediaCandidates,
        queuedWorkItems: event.queuedWorkItems
      }
    case 'sourceScanCompleted':
      return {
        kind: 'completed',
        rootId: event.rootId,
        scanRunId: event.scanRunId,
        filesDiscovered: event.filesDiscovered,
        queuedWorkItems: event.queuedWorkItems
      }
    case 'sourceScanFailed':
      return {
        kind: 'failed',
        rootId: event.rootId,
        detail: event.detail
      }
    case 'sourceScanBlocked':
      return {
        kind: 'blocked',
        rootId: event.rootId,
        detail: event.detail
      }
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
