import { describe, expect, it } from 'vitest'
import type { ReadSourceActivityReply } from '@dekzer/library-boundary-contract'

import {
  projectSourceAdmissionHandoff,
  sourceAdmissionHandoffFromRoot
} from '../../../../src/renderer/library/runtime/sourceAdmissionHandoff'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type { RowBinding } from '../../../../src/renderer/library/state'

describe('source admission handoff projection', () => {
  it('projects successful source admission with view and add-another actions', () => {
    const projection = projectSourceAdmissionHandoff({
      handoff: sourceAdmissionHandoffFromRoot({
        rootId: '7',
        admittedRootPath: 'C:/Music'
      }),
      projection: sourceProjection('7', 'Music'),
      sourceReadinessByNodeId: new Map([
        [
          'navigation-row:7',
          {
            kind: 'scanning',
            sourceNodeId: 'navigation-row:7',
            rootId: '7',
            detail: 'The source is still being indexed.'
          }
        ]
      ])
    })

    expect(projection).toMatchObject({
      title: 'Source added',
      sourceId: '7',
      sourceName: 'Music',
      sourcePath: 'C:/Music',
      detail: 'Music was added from C:/Music.',
      readinessDetail: 'The source is still being indexed.'
    })
    expect(projection?.actions).toEqual([
      {
        kind: 'viewSource',
        label: 'View source',
        sourceId: '7',
        enabled: true
      },
      {
        kind: 'addAnotherSource',
        label: 'Add another source',
        enabled: true
      }
    ])
  })

  it('uses truthful pending readiness until the admitted source is visible', () => {
    const projection = projectSourceAdmissionHandoff({
      handoff: {
        sourceId: '7',
        sourcePath: 'C:/Music'
      }
    })

    expect(projection).toMatchObject({
      title: 'Source added',
      detail: 'Source added from C:/Music.',
      readinessDetail: 'Checking source readiness.'
    })
  })

  it('uses source activity for preparation handoff detail', () => {
    const projection = projectSourceAdmissionHandoff({
      handoff: sourceAdmissionHandoffFromRoot({
        rootId: '7',
        admittedRootPath: 'C:/Music'
      }),
      projection: sourceProjection('7', 'Music'),
      sourceActivityBySourceId: new Map([
        [
          '7',
          sourceActivity({
            preparation: {
              state: 'running'
            }
          })
        ]
      ])
    })

    expect(projection?.readinessDetail).toBe('Preparing source.')
  })
})

function sourceProjection(sourceId: string, label: string): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [sourceNode(sourceId, label)],
    bindingsById: new Map([[`navigation-row:${sourceId}`, sourceBinding(sourceId, label)]])
  }
}

function sourceNode(sourceId: string, label: string): BrowserTreeNode {
  return {
    id: `navigation-row:${sourceId}`,
    role: 'source',
    label,
    children: { kind: 'none' }
  }
}

function sourceBinding(sourceId: string, label: string): RowBinding {
  return {
    kind: 'source',
    navigationRow: {
      navigationRowId: sourceId,
      stableKey: `source:${sourceId}`,
      parentNavigationRowId: null,
      family: 'sources',
      rowKind: 'source',
      displayName: label,
      siblingPosition: 0,
      selectable: true,
      selectorKind: 'source',
      selectorPayload: sourceId,
      updatedAtMs: 100,
      rowVersion: '1'
    },
    target: {
      navigationRowId: sourceId,
      entryPoint: { kind: 'source', sourceId },
      label
    }
  }
}

function sourceActivity(
  overrides: {
    readonly preparation?: Partial<ReadSourceActivityReply['preparation']>
  } = {}
): ReadSourceActivityReply {
  const base: ReadSourceActivityReply = {
    sourceId: '7',
    admission: 'active',
    browse: {
      state: 'ready'
    },
    scan: {
      state: 'completed',
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
