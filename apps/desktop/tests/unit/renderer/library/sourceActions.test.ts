import { describe, expect, it } from 'vitest'

import type { LocalRootsReadState } from '../../../../src/renderer/library/boundary/localRootActions'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import {
  deriveSourceActionModel,
  hasVisibleSourceRootBinding
} from '../../../../src/renderer/library/runtime/sourceActions'

describe('deriveSourceActionModel', () => {
  it('finds visible removable source rows from source bindings and local roots', () => {
    const projection = sourceProjection(['root-1', 'root-2'])
    const model = deriveSourceActionModel({
      projection,
      selectedNodeId: 'navigation-row:root-2',
      localRootsReadState: readyRoots(['root-1', 'root-2']),
      scanStatus: 'idle',
      removeSourceStatus: 'idle',
      refreshStatus: 'idle'
    })

    expect(model.visibleSourceRows.map((row) => row.rootId)).toEqual(['root-1', 'root-2'])
    expect(model.selectedRemovableSourceRootId).toBe('root-2')
    expect(model.removeVisible).toBe(true)
    expect(model.removeEnabled).toBe(true)
    expect(model.reasonUnavailable).toBeUndefined()
  })

  it('falls back to the only visible removable source when no row is selected', () => {
    const model = deriveSourceActionModel({
      projection: sourceProjection(['root-1']),
      selectedNodeId: undefined,
      localRootsReadState: readyRoots(['root-1']),
      scanStatus: 'idle',
      removeSourceStatus: 'idle',
      refreshStatus: 'idle'
    })

    expect(model.selectedRemovableSourceRootId).toBe('root-1')
    expect(model.removeVisible).toBe(true)
    expect(model.removeEnabled).toBe(true)
  })

  it('keeps remove visible but disabled while scan, refresh, or removal is active', () => {
    const base = {
      projection: sourceProjection(['root-1']),
      selectedNodeId: 'navigation-row:root-1',
      localRootsReadState: readyRoots(['root-1'])
    }

    expect(
      deriveSourceActionModel({
        ...base,
        scanStatus: 'scanning',
        removeSourceStatus: 'idle',
        refreshStatus: 'idle'
      })
    ).toMatchObject({
      removeVisible: true,
      removeEnabled: false,
      reasonUnavailable: 'A source scan is still running.'
    })
    expect(
      deriveSourceActionModel({
        ...base,
        scanStatus: 'idle',
        removeSourceStatus: 'removing',
        refreshStatus: 'idle'
      })
    ).toMatchObject({
      removeVisible: true,
      removeEnabled: false,
      reasonUnavailable: 'A source removal is already in progress.'
    })
    expect(
      deriveSourceActionModel({
        ...base,
        scanStatus: 'idle',
        removeSourceStatus: 'idle',
        refreshStatus: 'refreshing'
      })
    ).toMatchObject({
      removeVisible: true,
      removeEnabled: false,
      reasonUnavailable: 'The library view is refreshing.'
    })
  })

  it('reports why no removable source action is available', () => {
    expect(
      deriveSourceActionModel({
        projection: sourceProjection(['root-1']),
        selectedNodeId: undefined,
        localRootsReadState: { kind: 'unread' },
        scanStatus: 'idle',
        removeSourceStatus: 'idle',
        refreshStatus: 'idle'
      })
    ).toMatchObject({
      removeVisible: false,
      removeEnabled: false,
      reasonUnavailable: 'Local source actions are not ready yet.'
    })

    expect(
      deriveSourceActionModel({
        projection: sourceProjection(['root-1', 'root-2']),
        selectedNodeId: undefined,
        localRootsReadState: readyRoots(['root-1', 'root-2']),
        scanStatus: 'idle',
        removeSourceStatus: 'idle',
        refreshStatus: 'idle'
      })
    ).toMatchObject({
      removeVisible: false,
      removeEnabled: false,
      reasonUnavailable: 'Select a local source to remove it.'
    })

    expect(
      deriveSourceActionModel({
        projection: sourceProjection(['root-1']),
        selectedNodeId: 'navigation-row:root-2',
        localRootsReadState: readyRoots(['root-1']),
        scanStatus: 'idle',
        removeSourceStatus: 'idle',
        refreshStatus: 'idle'
      })
    ).toMatchObject({
      removeVisible: false,
      removeEnabled: false,
      reasonUnavailable: 'The selected row does not belong to a removable local source.'
    })
  })

  it('checks visible local source bindings without requiring local root records', () => {
    const projection = sourceProjection(['root-1'])

    expect(hasVisibleSourceRootBinding(projection, 'root-1')).toBe(true)
    expect(hasVisibleSourceRootBinding(projection, 'root-2')).toBe(false)
    expect(hasVisibleSourceRootBinding(undefined, 'root-1')).toBe(false)
  })
})

function sourceProjection(rootIds: readonly string[]): BrowserProjection {
  const bindingsById: BrowserProjection['bindingsById'] = new Map(
    rootIds.map((rootId) => [
      `navigation-row:${rootId}`,
      {
        kind: 'source',
        navigationRow: {
          navigationRowId: rootId,
          stableKey: `source:${rootId}`,
          parentNavigationRowId: null,
          family: 'sources',
          rowKind: 'source',
          displayName: `Source ${rootId}`,
          siblingPosition: 0,
          selectable: true,
          selectorKind: 'source',
          selectorPayload: rootId,
          updatedAtMs: 100,
          rowVersion: '1'
        },
        target: {
          navigationRowId: rootId,
          entryPoint: {
            kind: 'source',
            sourceId: rootId
          },
          label: `Source ${rootId}`
        }
      }
    ])
  )

  return {
    kind: 'tree',
    nodes: rootIds.map<BrowserTreeNode>((rootId) => ({
      id: `navigation-row:${rootId}`,
      role: 'source',
      label: `Source ${rootId}`,
      children: { kind: 'none' }
    })),
    bindingsById
  }
}

function readyRoots(rootIds: readonly string[]): LocalRootsReadState {
  return {
    kind: 'ready',
    roots: rootIds.map((rootId) => ({
      rootId,
      canonicalPath: `C:/Music/${rootId}`,
      availability: 'available'
    }))
  }
}
