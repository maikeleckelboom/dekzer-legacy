export type AppSourceScanEventKind =
  | 'sourceScanStarted'
  | 'sourceScanProgressed'
  | 'sourceScanCompleted'
  | 'sourceScanFailed'
  | 'sourceScanBlocked'

export type AppSourceScanEvent = {
  readonly eventSequence: number
  readonly occurredAtMs: number
  readonly kind: AppSourceScanEventKind
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

export type AppMaintainedSnapshotInvalidatedEvent = {
  readonly eventSequence: number
  readonly occurredAtMs: number
  readonly invalidation: {
    readonly scope: string
    readonly revision: string | null
  }
}

export type AppBoundaryEvent =
  | { readonly type: 'sourceScanEvent'; readonly payload: AppSourceScanEvent }
  | { readonly type: 'maintainedSnapshotInvalidated'; readonly payload: AppMaintainedSnapshotInvalidatedEvent }
  | { readonly type: 'unsupported'; readonly payload: unknown }

function isAppSourceScanEventKind(value: unknown): value is AppSourceScanEventKind {
  return (
    typeof value === 'string' &&
    [
      'sourceScanStarted',
      'sourceScanProgressed',
      'sourceScanCompleted',
      'sourceScanFailed',
      'sourceScanBlocked'
    ].includes(value)
  )
}

export function parseBoundaryEvent(raw: unknown): AppBoundaryEvent {
  if (raw === null || typeof raw !== 'object') {
    return { type: 'unsupported', payload: raw }
  }

  const event = raw as Record<string, unknown>

  if (event.type === 'sourceScanEvent') {
    const payload = event.payload as Record<string, unknown> | undefined
    if (payload === undefined || payload === null || typeof payload !== 'object') {
      return { type: 'unsupported', payload: raw }
    }
    if (!isAppSourceScanEventKind(payload.kind)) {
      return { type: 'unsupported', payload: raw }
    }
    return {
      type: 'sourceScanEvent',
      payload: {
        eventSequence: safeNumber(payload.eventSequence),
        occurredAtMs: safeNumber(payload.occurredAtMs),
        kind: payload.kind as AppSourceScanEventKind,
        rootId: safeString(payload.rootId),
        scanRunId: safeString(payload.scanRunId),
        phase: safeString(payload.phase),
        directoriesVisited: safeNumber(payload.directoriesVisited),
        filesVisited: safeNumber(payload.filesVisited),
        filesDiscovered: safeNumber(payload.filesDiscovered),
        mediaCandidates: safeNumber(payload.mediaCandidates),
        queuedWorkItems: safeNumber(payload.queuedWorkItems),
        detail: payload.detail === null || payload.detail === undefined ? null : String(payload.detail)
      }
    }
  }

  if (event.type === 'maintainedSnapshotInvalidated') {
    const payload = event.payload as Record<string, unknown> | undefined
    if (payload === undefined || payload === null || typeof payload !== 'object') {
      return { type: 'unsupported', payload: raw }
    }
    const invalidation = payload.invalidation as Record<string, unknown> | undefined
    return {
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence: safeNumber(payload.eventSequence),
        occurredAtMs: safeNumber(payload.occurredAtMs),
        invalidation: {
          scope: safeString(invalidation?.scope),
          revision:
            invalidation?.revision === null || invalidation?.revision === undefined
              ? null
              : String(invalidation.revision)
        }
      }
    }
  }

  return { type: 'unsupported', payload: raw }
}

function safeNumber(value: unknown): number {
  if (typeof value === 'number') {
    return value
  }
  if (typeof value === 'string') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : 0
  }
  return 0
}

function safeString(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }
  return String(value ?? '')
}
