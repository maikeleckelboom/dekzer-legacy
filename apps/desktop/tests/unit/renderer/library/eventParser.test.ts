import { describe, expect, it } from 'vitest'
import { parseBoundaryEvent } from '../../../../src/shared/libraryBoundary/eventParser'

describe('parseBoundaryEvent', () => {
  it('rejects null and non-object inputs as unsupported', () => {
    expect(parseBoundaryEvent(null)).toMatchObject({ type: 'unsupported' })
    expect(parseBoundaryEvent(undefined)).toMatchObject({ type: 'unsupported' })
    expect(parseBoundaryEvent(42)).toMatchObject({ type: 'unsupported' })
    expect(parseBoundaryEvent('hello')).toMatchObject({ type: 'unsupported' })
    expect(parseBoundaryEvent(true)).toMatchObject({ type: 'unsupported' })
  })

  it('rejects unknown event families as unsupported', () => {
    expect(
      parseBoundaryEvent({ type: 'unknownFamily', payload: {} })
    ).toMatchObject({ type: 'unsupported' })
    expect(
      parseBoundaryEvent({ type: 'sourceScanEvent' })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with missing or invalid payload', () => {
    expect(
      parseBoundaryEvent({ type: 'sourceScanEvent', payload: null })
    ).toMatchObject({ type: 'unsupported' })
    expect(
      parseBoundaryEvent({ type: 'sourceScanEvent', payload: undefined })
    ).toMatchObject({ type: 'unsupported' })
    expect(
      parseBoundaryEvent({ type: 'sourceScanEvent', payload: 'not-an-object' })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with invalid kind', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'unknownKind',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with missing eventSequence', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with negative eventSequence', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: -1,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 1,
          filesVisited: 2,
          filesDiscovered: 3,
          mediaCandidates: 4,
          queuedWorkItems: 5
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with non-integer eventSequence', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 1.5,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with missing occurredAtMs', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with negative occurredAtMs', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: -1,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with missing rootId', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with empty rootId', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with missing scanRunId', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('parses valid sourceScanEvent with all required fields', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 3,
        occurredAtMs: 1700000000000,
        kind: 'sourceScanProgressed',
        rootId: '7',
        scanRunId: '14',
        phase: 'scanning',
        directoriesVisited: 42,
        filesVisited: 100,
        filesDiscovered: 95,
        mediaCandidates: 12,
        queuedWorkItems: 8,
        detail: null
      }
    })

    expect(result).toMatchObject({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 3,
        occurredAtMs: 1700000000000,
        kind: 'sourceScanProgressed',
        rootId: '7',
        scanRunId: '14',
        phase: 'scanning',
        directoriesVisited: 42,
        filesVisited: 100,
        filesDiscovered: 95,
        mediaCandidates: 12,
        queuedWorkItems: 8,
        detail: null
      }
    })
  })

  it('parses sourceScanEvent with string eventSequence as valid number', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: '5',
        occurredAtMs: 1700000000000,
        kind: 'sourceScanCompleted',
        rootId: '1',
        scanRunId: '2',
        phase: 'scanning',
        directoriesVisited: 1,
        filesVisited: 2,
        filesDiscovered: 3,
        mediaCandidates: 4,
        queuedWorkItems: 5,
        detail: null
      }
    })

    expect(result.type).toBe('sourceScanEvent')
    if (result.type === 'sourceScanEvent') {
      expect(result.payload.eventSequence).toBe(5)
    }
  })

  it('defaults optional counters to 0 when absent from valid event', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 0,
        occurredAtMs: 1000,
        kind: 'sourceScanStarted',
        rootId: '1',
        scanRunId: '1',
        phase: 'scanning',
        detail: null
      }
    })

    expect(result.type).toBe('sourceScanEvent')
    if (result.type === 'sourceScanEvent') {
      expect(result.payload.directoriesVisited).toBe(0)
      expect(result.payload.filesVisited).toBe(0)
      expect(result.payload.filesDiscovered).toBe(0)
      expect(result.payload.mediaCandidates).toBe(0)
      expect(result.payload.queuedWorkItems).toBe(0)
    }
  })

  it('rejects maintainedSnapshotInvalidated with missing payload', () => {
    expect(
      parseBoundaryEvent({ type: 'maintainedSnapshotInvalidated', payload: null })
    ).toMatchObject({ type: 'unsupported' })
    expect(
      parseBoundaryEvent({ type: 'maintainedSnapshotInvalidated', payload: undefined })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects maintainedSnapshotInvalidated with missing eventSequence', () => {
    expect(
      parseBoundaryEvent({
        type: 'maintainedSnapshotInvalidated',
        payload: {
          occurredAtMs: 1000,
          invalidation: { scope: 'libraryBrowser', revision: '1' }
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects maintainedSnapshotInvalidated with missing scope', () => {
    expect(
      parseBoundaryEvent({
        type: 'maintainedSnapshotInvalidated',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          invalidation: { revision: '1' }
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects maintainedSnapshotInvalidated with empty scope', () => {
    expect(
      parseBoundaryEvent({
        type: 'maintainedSnapshotInvalidated',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          invalidation: { scope: '', revision: '1' }
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('parses valid maintainedSnapshotInvalidated with null revision', () => {
    const result = parseBoundaryEvent({
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence: 5,
        occurredAtMs: 1700000000000,
        invalidation: { scope: 'libraryBrowser', revision: null }
      }
    })

    expect(result).toMatchObject({
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence: 5,
        occurredAtMs: 1700000000000,
        invalidation: { scope: 'libraryBrowser', revision: null }
      }
    })
  })

  it('parses valid maintainedSnapshotInvalidated with revision', () => {
    const result = parseBoundaryEvent({
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence: 5,
        occurredAtMs: 1700000000000,
        invalidation: { scope: 'navigationRows', revision: '42' }
      }
    })

    expect(result).toMatchObject({
      type: 'maintainedSnapshotInvalidated',
      payload: {
        eventSequence: 5,
        occurredAtMs: 1700000000000,
        invalidation: { scope: 'navigationRows', revision: '42' }
      }
    })
  })

  it('rejects sourceScanEvent with missing phase', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with empty phase', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: '',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with unknown phase', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'unknown',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('accepts sourceScanEvent with phase scanning', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 0,
        occurredAtMs: 1000,
        kind: 'sourceScanStarted',
        rootId: '1',
        scanRunId: '1',
        phase: 'scanning',
        directoriesVisited: 0,
        filesVisited: 0,
        filesDiscovered: 0,
        mediaCandidates: 0,
        queuedWorkItems: 0
      }
    })
    expect(result).toMatchObject({ type: 'sourceScanEvent' })
    if (result.type === 'sourceScanEvent') {
      expect(result.payload.phase).toBe('scanning')
    }
  })

  it('accepts sourceScanEvent with phase blocked', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 0,
        occurredAtMs: 1000,
        kind: 'sourceScanBlocked',
        rootId: '1',
        scanRunId: '1',
        phase: 'blocked',
        directoriesVisited: 0,
        filesVisited: 0,
        filesDiscovered: 0,
        mediaCandidates: 0,
        queuedWorkItems: 0
      }
    })
    expect(result).toMatchObject({ type: 'sourceScanEvent' })
    if (result.type === 'sourceScanEvent') {
      expect(result.payload.phase).toBe('blocked')
    }
  })

  it('accepts sourceScanEvent with phase interrupted', () => {
    const result = parseBoundaryEvent({
      type: 'sourceScanEvent',
      payload: {
        eventSequence: 0,
        occurredAtMs: 1000,
        kind: 'sourceScanFailed',
        rootId: '1',
        scanRunId: '1',
        phase: 'interrupted',
        directoriesVisited: 0,
        filesVisited: 0,
        filesDiscovered: 0,
        mediaCandidates: 0,
        queuedWorkItems: 0
      }
    })
    expect(result).toMatchObject({ type: 'sourceScanEvent' })
    if (result.type === 'sourceScanEvent') {
      expect(result.payload.phase).toBe('interrupted')
    }
  })

  it('rejects sourceScanEvent with fractional occurredAtMs', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1700000000000.5,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects maintainedSnapshotInvalidated with fractional occurredAtMs', () => {
    expect(
      parseBoundaryEvent({
        type: 'maintainedSnapshotInvalidated',
        payload: {
          eventSequence: 0,
          occurredAtMs: 1700000000000.5,
          invalidation: { scope: 'libraryBrowser', revision: null }
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('handles unknown event families as unsupported', () => {
    expect(
      parseBoundaryEvent({ type: 'futureEventKind', payload: {} })
    ).toMatchObject({ type: 'unsupported' })
  })

  it('rejects sourceScanEvent with NaN eventSequence', () => {
    expect(
      parseBoundaryEvent({
        type: 'sourceScanEvent',
        payload: {
          eventSequence: NaN,
          occurredAtMs: 1000,
          kind: 'sourceScanStarted',
          rootId: '1',
          scanRunId: '1',
          phase: 'scanning',
          directoriesVisited: 0,
          filesVisited: 0,
          filesDiscovered: 0,
          mediaCandidates: 0,
          queuedWorkItems: 0
        }
      })
    ).toMatchObject({ type: 'unsupported' })
  })
})
