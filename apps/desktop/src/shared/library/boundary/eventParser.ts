import type { ScanRunPhase } from '@dekzer/library-boundary-contract'

export type AppSourceScanEventKind =
  | 'sourceScanStarted'
  | 'sourceScanProgressed'
  | 'sourceScanCompleted'
  | 'sourceScanFailed'
  | 'sourceScanBlocked'
  | 'sourceScanCancelled'

export type AppSourceScanEvent = {
  readonly eventSequence: number
  readonly occurredAtMs: number
  readonly kind: AppSourceScanEventKind
  readonly rootId: string
  readonly scanRunId: string
  readonly phase: ScanRunPhase
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
  | {
      readonly type: 'maintainedSnapshotInvalidated'
      readonly payload: AppMaintainedSnapshotInvalidatedEvent
    }
  | { readonly type: 'unsupported'; readonly payload: unknown }

const SCAN_RUN_PHASES: ReadonlySet<string> = new Set(['scanning', 'blocked', 'interrupted'])

function isAppSourceScanEventKind(value: unknown): value is AppSourceScanEventKind {
  return (
    typeof value === 'string' &&
    [
      'sourceScanStarted',
      'sourceScanProgressed',
      'sourceScanCompleted',
      'sourceScanFailed',
      'sourceScanBlocked',
      'sourceScanCancelled'
    ].includes(value)
  )
}

function isScanRunPhase(value: unknown): value is ScanRunPhase {
  return typeof value === 'string' && SCAN_RUN_PHASES.has(value)
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
    const eventSequence = requireNonNegativeSafeInteger(payload.eventSequence)
    if (eventSequence === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    const occurredAtMs = requireNonNegativeSafeInteger(payload.occurredAtMs)
    if (occurredAtMs === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    const rootId = requireNonEmptyString(payload.rootId)
    if (rootId === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    const scanRunId = requireNonEmptyString(payload.scanRunId)
    if (scanRunId === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    if (!isScanRunPhase(payload.phase)) {
      return { type: 'unsupported', payload: raw }
    }
    const directoriesVisited = requireNonNegativeSafeInteger(payload.directoriesVisited) ?? 0
    const filesVisited = requireNonNegativeSafeInteger(payload.filesVisited) ?? 0
    const filesDiscovered = requireNonNegativeSafeInteger(payload.filesDiscovered) ?? 0
    const mediaCandidates = requireNonNegativeSafeInteger(payload.mediaCandidates) ?? 0
    const queuedWorkItems = requireNonNegativeSafeInteger(payload.queuedWorkItems) ?? 0
    return {
      type: 'sourceScanEvent',
      payload: {
        eventSequence,
        occurredAtMs,
        kind: payload.kind,
        rootId,
        scanRunId,
        phase: payload.phase,
        directoriesVisited,
        filesVisited,
        filesDiscovered,
        mediaCandidates,
        queuedWorkItems,
        detail:
          payload.detail === null || payload.detail === undefined ? null : String(payload.detail)
      }
    }
  }

  if (event.type === 'maintainedSnapshotInvalidated') {
    const payload = event.payload as Record<string, unknown> | undefined
    if (payload === undefined || payload === null || typeof payload !== 'object') {
      return { type: 'unsupported', payload: raw }
    }
    const eventSequence = requireNonNegativeSafeInteger(payload.eventSequence)
    if (eventSequence === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    const occurredAtMs = requireNonNegativeSafeInteger(payload.occurredAtMs)
    if (occurredAtMs === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    const invalidation = payload.invalidation as Record<string, unknown> | undefined
    const scope = requireNonEmptyString(invalidation?.scope)
    if (scope === undefined) {
      return { type: 'unsupported', payload: raw }
    }
    return {
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence,
        occurredAtMs,
        invalidation: {
          scope,
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

function requireNonNegativeSafeInteger(value: unknown): number | undefined {
  if (typeof value === 'number' && Number.isSafeInteger(value) && value >= 0) {
    return value
  }
  if (typeof value === 'string') {
    const parsed = Number(value)
    if (Number.isSafeInteger(parsed) && parsed >= 0) {
      return parsed
    }
  }
  return undefined
}

function requireNonEmptyString(value: unknown): string | undefined {
  if (typeof value === 'string' && value.length > 0) {
    return value
  }
  return undefined
}
