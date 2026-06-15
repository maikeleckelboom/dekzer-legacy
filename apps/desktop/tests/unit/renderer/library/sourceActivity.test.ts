import { describe, expect, it } from 'vitest'
import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

import {
  projectSourceActivity,
  sourceActivityBacklogCategories,
  sourceActivityBacklogTotal,
  sourceActivityPreparationSummary,
  sourceActivityScanSummary
} from '../../../../src/renderer/library/runtime/sourceActivity'

describe('source activity projection', () => {
  it('maps running scan progress with known counters and no fake percentage', () => {
    const projected = projectSourceActivity({
      activity: sourceActivity(),
      scanProgress: {
        kind: 'scanning',
        rootId: '7',
        scanRunId: 'scan-1',
        directoriesVisited: 3,
        filesVisited: 20,
        filesDiscovered: 12,
        mediaCandidates: 8,
        queuedWorkItems: 5
      }
    })

    expect(projected?.scan).toMatchObject({
      state: 'running',
      counters: {
        directoriesVisited: 3,
        filesVisited: 20,
        filesDiscovered: 12,
        mediaCandidates: 8,
        queuedWorkItems: 5
      },
      scanRunId: 'scan-1'
    })
    expect(JSON.stringify(projected)).not.toContain('percentage')
    expect(sourceActivityScanSummary(projected!)).toBe(
      'Scanning source: 12 files discovered, 3 folders visited, 5 work items queued.'
    )
  })

  it.each([
    { kind: 'completed', state: 'completed', detail: 'Scan completed.' },
    { kind: 'failed', state: 'failed', detail: 'Read failed.' },
    { kind: 'blocked', state: 'blocked', detail: 'Blocked by permissions.' },
    { kind: 'cancelled', state: 'cancelled', detail: 'User cancelled.' }
  ] as const)('maps $kind scan progress terminal state honestly', (expected) => {
    const scanProgress =
      expected.kind === 'completed'
        ? {
            kind: expected.kind,
            rootId: '7',
            scanRunId: 'scan-1',
            filesDiscovered: 2,
            queuedWorkItems: 1
          }
        : expected.kind === 'cancelled'
          ? {
              kind: expected.kind,
              rootId: '7',
              scanRunId: 'scan-1',
              detail: expected.detail
            }
          : {
              kind: expected.kind,
              rootId: '7',
              detail: expected.detail
            }

    const projected = projectSourceActivity({
      activity: sourceActivity(),
      scanProgress
    })

    expect(projected?.scan.state).toBe(expected.state)
    expect(sourceActivityScanSummary(projected!)).toBe(expected.detail)
  })

  it('preserves preparation category counts and bounded-batch copy', () => {
    const activity = sourceActivity({
      preparation: {
        state: 'idle',
        backlog: {
          hash: 1,
          probe: 2,
          attachment: 3,
          promotion: 4,
          identity: 5
        }
      }
    })

    expect(sourceActivityBacklogTotal(activity)).toBe(15)
    expect(sourceActivityBacklogCategories(activity)).toEqual([
      { label: 'hash', count: 1 },
      { label: 'probe', count: 2 },
      { label: 'attachment', count: 3 },
      { label: 'promotion', count: 4 },
      { label: 'identity', count: 5 }
    ])
    expect(sourceActivityPreparationSummary(activity)).toBe(
      'Maintenance needed: hash 1, probe 2, attachment 3, promotion 4, identity 5. Run maintenance processes a bounded batch.'
    )
  })

  it('distinguishes completed maintenance with remaining work from complete preparation', () => {
    expect(
      sourceActivityPreparationSummary(
        sourceActivity({
          preparation: {
            state: 'completedWithRemainingWork',
            backlog: {
              hash: 7,
              probe: 0,
              attachment: 0,
              promotion: 0,
              identity: 0
            }
          }
        })
      )
    ).toBe(
      'Maintenance completed; pending work remains: hash 7. Run maintenance processes a bounded batch.'
    )

    expect(
      sourceActivityPreparationSummary(
        sourceActivity({
          preparation: {
            state: 'complete'
          }
        })
      )
    ).toBe('Preparation complete.')
  })

  it('preserves run-result provenance when maintenance just completed', () => {
    const projected = projectSourceActivity({
      activity: sourceActivity({
        preparation: {
          state: 'completedWithRemainingWork',
          lastRunStatus: 'completed'
        }
      }),
      maintenanceRunState: 'completed'
    })

    expect(projected?.preparation.provenance).toBe('runResult')
  })
})

function sourceActivity(
  overrides: {
    readonly preparation?: Partial<ReadSourceActivityReply['preparation']>
  } = {}
): ReadSourceActivityReply {
  const base: ReadSourceActivityReply = {
    sourceId: '7',
    admission: 'active',
    browse: {
      state: 'ready',
      detail: 'Source is ready to browse.'
    },
    scan: {
      state: 'idle',
      counters: {}
    },
    preparation: {
      state: 'complete',
      backlog: {
        hash: 0,
        probe: 0,
        attachment: 0,
        promotion: 0,
        identity: 0
      },
      provenance: 'maintenanceSnapshot',
      boundedBatch: true
    }
  }

  return {
    ...base,
    preparation:
      overrides.preparation === undefined
        ? base.preparation
        : {
            ...base.preparation,
            ...overrides.preparation,
            backlog: {
              ...base.preparation.backlog,
              ...(overrides.preparation.backlog ?? {})
            }
          }
  }
}
