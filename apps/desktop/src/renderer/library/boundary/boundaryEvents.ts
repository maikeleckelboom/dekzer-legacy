import { onMounted, onUnmounted, shallowRef } from 'vue'
import type { Ref } from 'vue'

import type { RendererApi } from '../../../shared/rendererApi'
import {
  parseBoundaryEvent,
  type AppBoundaryEvent,
  type AppMaintainedSnapshotInvalidatedEvent,
  type AppSourceScanEvent
} from '../../../shared/libraryBoundary/eventParser'

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
  | {
      readonly kind: 'cancelled'
      readonly rootId: string
      readonly scanRunId: string
      readonly detail: string | null
    }

export type MaintainedSnapshotInvalidationSignal = AppMaintainedSnapshotInvalidatedEvent

export type BoundaryEventsController = {
  readonly scanProgress: Ref<ReadonlyMap<string, ScanProgressState>>
  readonly maintainedSnapshotInvalidations: Ref<readonly MaintainedSnapshotInvalidationSignal[]>
  readonly lastReadFailed: Ref<boolean>
  readonly gapDetected: Ref<boolean>
  readonly recoveryNeeded: Ref<boolean>
  readonly acknowledgedGap: () => void
  readonly start: () => void
  readonly stop: () => void
}

export type BoundaryEventApi = RendererApi['library']['events']

export function useBoundaryEvents(
  eventApi: BoundaryEventApi = getRendererApi().library.events
): BoundaryEventsController {
  const controller = createBoundaryEventsController(eventApi)

  onMounted(() => {
    controller.start()
  })

  onUnmounted(() => {
    controller.stop()
  })

  return controller
}

export function createBoundaryEventsController(
  eventApi: BoundaryEventApi
): BoundaryEventsController {
  const scanProgress = shallowRef<ReadonlyMap<string, ScanProgressState>>(new Map())
  const maintainedSnapshotInvalidations = shallowRef<
    readonly MaintainedSnapshotInvalidationSignal[]
  >([])
  const lastReadFailed = shallowRef(false)
  const gapDetected = shallowRef(false)
  const recoveryNeeded = shallowRef(false)
  let unsubscribe: (() => void) | undefined

  function applyEvents(events: ReadonlyArray<AppBoundaryEvent>): void {
    const nextProgress = new Map(scanProgress.value)
    const nextInvalidations = [...maintainedSnapshotInvalidations.value]

    for (const event of events) {
      if (event.type === 'sourceScanEvent') {
        const progress = scanProgressFromEvent(event.payload)
        nextProgress.set(event.payload.rootId, progress)
      } else if (event.type === 'maintainedSnapshotInvalidated') {
        nextInvalidations.push(event.payload)
      }
    }

    scanProgress.value = nextProgress
    maintainedSnapshotInvalidations.value = nextInvalidations
  }

  function acknowledgedGap(): void {
    recoveryNeeded.value = false
  }

  function start(): void {
    if (unsubscribe !== undefined) {
      return
    }

    unsubscribe = eventApi.subscribe((payload) => {
      if (payload.kind === 'failed') {
        lastReadFailed.value = true
        return
      }

      lastReadFailed.value = false

      if (payload.gapDetected) {
        gapDetected.value = true
        recoveryNeeded.value = true
      }

      const parsed = payload.events
        .map(parseBoundaryEvent)
        .filter(
          (event): event is Exclude<AppBoundaryEvent, { type: 'unsupported' }> =>
            event.type !== 'unsupported'
        )

      if (parsed.length > 0) {
        applyEvents(parsed)
      }
    })
  }

  function stop(): void {
    unsubscribe?.()
    unsubscribe = undefined
  }

  return {
    scanProgress,
    maintainedSnapshotInvalidations,
    lastReadFailed,
    gapDetected,
    recoveryNeeded,
    acknowledgedGap,
    start,
    stop
  }
}

function scanProgressFromEvent(event: AppSourceScanEvent): ScanProgressState {
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
    case 'sourceScanCancelled':
      return {
        kind: 'cancelled',
        rootId: event.rootId,
        scanRunId: event.scanRunId,
        detail: event.detail
      }
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}
