import { describe, expect, it, vi } from 'vitest'

import {
  createSourceLifecycleReadController,
  sourceLifecycleIdsForBrowserContext,
  type SourceLifecycleReadApi
} from '../../../../src/renderer/library/boundary/sourceLifecycleRead'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { NavigationRow } from '../../../../src/shared/libraryNavigation/readRows'
import type { SourceLifecycleRecord } from '../../../../src/shared/librarySourceLifecycle/readSourceLifecycle'

describe('createSourceLifecycleReadController', () => {
  it('hydrates lifecycle records by source id', async () => {
    const api = sourceLifecycleApi(async () => ({
      state: 'ready',
      lifecycle: sourceLifecycle({ sourceId: '7', accessState: 'blocked' })
    }))
    const controller = createSourceLifecycleReadController(api)

    await expect(controller.readSourceLifecycle('7')).resolves.toBe(true)

    expect(api.sourceLifecycle.readSourceLifecycle).toHaveBeenCalledWith({ sourceId: '7' })
    expect(controller.sourceLifecycleBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      accessState: 'blocked'
    })
    expect(controller.sourceLifecycleReadErrorsBySourceId.value.has('7')).toBe(false)
  })

  it('dedupes in-flight reads per source id', async () => {
    const deferredRead =
      deferred<
        Awaited<ReturnType<SourceLifecycleReadApi['sourceLifecycle']['readSourceLifecycle']>>
      >()
    const api = sourceLifecycleApi(() => deferredRead.promise)
    const controller = createSourceLifecycleReadController(api)

    const first = controller.readSourceLifecycle('7')
    const second = controller.readSourceLifecycle('7')

    expect(first).toBe(second)
    expect(api.sourceLifecycle.readSourceLifecycle).toHaveBeenCalledTimes(1)

    deferredRead.resolve({
      state: 'ready',
      lifecycle: sourceLifecycle({ sourceId: '7' })
    })
    await expect(first).resolves.toBe(true)

    await controller.readSourceLifecycle('7')
    expect(api.sourceLifecycle.readSourceLifecycle).toHaveBeenCalledTimes(2)
  })

  it('preserves last known lifecycle records on read failure', async () => {
    let fail = false
    const api = sourceLifecycleApi(async () => {
      if (fail) {
        throw new Error('host failed')
      }

      return {
        state: 'ready',
        lifecycle: sourceLifecycle({ sourceId: '7', mountStatus: 'mounted' })
      }
    })
    const controller = createSourceLifecycleReadController(api)

    await expect(controller.readSourceLifecycle('7')).resolves.toBe(true)
    fail = true
    await expect(controller.readSourceLifecycle('7')).resolves.toBe(false)

    expect(controller.sourceLifecycleBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      mountStatus: 'mounted'
    })
    expect(controller.sourceLifecycleReadErrorsBySourceId.value.get('7')).toMatchObject({
      state: 'readFailed',
      error: { code: 'readFailed' }
    })
  })

  it('records notFound separately without erasing a known source lifecycle', async () => {
    let missing = false
    const api = sourceLifecycleApi(async () => {
      if (missing) {
        return {
          state: 'notFound',
          error: { code: 'notFound', message: 'Source not found.' }
        }
      }

      return {
        state: 'ready',
        lifecycle: sourceLifecycle({ sourceId: '7', scanPhase: 'complete' })
      }
    })
    const controller = createSourceLifecycleReadController(api)

    await expect(controller.readSourceLifecycle('7')).resolves.toBe(true)
    missing = true
    await expect(controller.readSourceLifecycle('7')).resolves.toBe(false)

    expect(controller.sourceLifecycleBySourceId.value.get('7')).toMatchObject({
      sourceId: '7',
      scanPhase: 'complete'
    })
    expect(controller.sourceLifecycleReadErrorsBySourceId.value.get('7')).toMatchObject({
      state: 'notFound',
      error: { code: 'notFound' }
    })
  })
})

describe('sourceLifecycleIdsForBrowserContext', () => {
  it('collects current source rows without walking unrelated source ids', () => {
    expect(
      sourceLifecycleIdsForBrowserContext({
        projection: browserProjection(),
        selectedNodeId: 'source-directory:12',
        expandedNodeIds: new Set(['navigation-row:7'])
      })
    ).toEqual(new Set(['7']))
  })
})

function sourceLifecycleApi(
  readSourceLifecycle: SourceLifecycleReadApi['sourceLifecycle']['readSourceLifecycle']
): SourceLifecycleReadApi {
  return {
    sourceLifecycle: {
      readSourceLifecycle: vi.fn(readSourceLifecycle)
    }
  }
}

function sourceLifecycle(overrides: Partial<SourceLifecycleRecord> = {}): SourceLifecycleRecord {
  return {
    sourceId: '7',
    sourceClass: 'externalMounted',
    isUserVisible: true,
    mountStatus: 'mounted',
    accessState: 'accessible',
    scanPhase: 'idle',
    updatedAtMs: 100,
    ...overrides
  }
}

function browserProjection(): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [],
    bindingsById: new Map<string, RowBinding>([
      [
        'navigation-row:7',
        {
          kind: 'source',
          navigationRow: sourceNavigationRow(),
          target: {
            navigationRowId: '7',
            label: 'Source Fixture',
            entryPoint: { kind: 'source', sourceId: '7' }
          }
        }
      ],
      [
        'source-directory:12',
        {
          kind: 'directory',
          sourceId: '7',
          entryPoint: { kind: 'source', sourceId: '7' },
          directoryId: '12',
          label: 'Album'
        }
      ]
    ])
  }
}

function sourceNavigationRow(): NavigationRow {
  return {
    navigationRowId: '7',
    stableKey: 'source:7',
    parentNavigationRowId: null,
    family: 'sources',
    rowKind: 'source',
    displayName: 'Source Fixture',
    siblingPosition: 0,
    selectable: true,
    selectorKind: 'source',
    selectorPayload: '7',
    updatedAtMs: 100,
    rowVersion: '1'
  }
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolveDeferred: (value: T) => void = () => undefined
  const promise = new Promise<T>((resolve) => {
    resolveDeferred = resolve
  })

  return {
    promise,
    resolve: resolveDeferred
  }
}
