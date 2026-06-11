import { describe, expect, it } from 'vitest'

import {
  createViewStateStore,
  type ViewStateApi
} from '../../../../src/renderer/library/runtime/viewState'
import type { PersistedLibraryViewState } from '../../../../src/shared/library/viewState/persistence'

describe('createViewStateStore', () => {
  it('coalesces queued writes to the newest pending state', async () => {
    const { api, writes, resolveAll } = createQueuedWriteApi()
    const store = createViewStateStore(api)

    store.save({ version: 1, expandedNodeIds: ['node-1'] })
    store.save({ version: 1, expandedNodeIds: ['node-2'] })
    store.save({ version: 1, expandedNodeIds: ['node-3'] })

    await resolveAll()

    expect(writes.map((write) => write.state.expandedNodeIds)).toEqual([['node-1'], ['node-3']])
    expect(writes[0]?.sequence).toBeLessThan(writes[1]?.sequence ?? 0)
  })

  it('delegates load and normalizes persisted state', async () => {
    const { api, writes, resolveAll } = createQueuedWriteApi()
    const store = createViewStateStore(api)
    const expandedNodeIds = ['node-a', 'node-a']

    await expect(store.load()).resolves.toEqual({ state: 'empty' })
    store.save({
      version: 1,
      selectedNodeId: 'node-a',
      expandedNodeIds
    })
    expandedNodeIds.push('node-b')
    await resolveAll()

    expect(writes[0]?.state).toEqual({
      version: 1,
      selectedNodeId: 'node-a',
      expandedNodeIds: ['node-a']
    })
  })

  it('does not let a failed write block later writes', async () => {
    const writes: PersistedLibraryViewState[] = []
    let callCount = 0
    const api = createViewStateApi({
      writeViewState: async (state) => {
        callCount += 1

        if (callCount === 1) {
          throw new Error('simulated write failure')
        }

        writes.push(state)
        return { state: 'written' }
      }
    })
    const store = createViewStateStore(api)

    store.save({
      version: 1,
      selectedNodeId: 'will-fail',
      expandedNodeIds: ['will-fail']
    })
    await waitForMicrotasks()

    store.save({
      version: 1,
      selectedNodeId: 'will-succeed',
      expandedNodeIds: ['will-succeed']
    })
    await waitForWrites()

    expect(callCount).toBe(2)
    expect(writes).toEqual([
      {
        version: 1,
        selectedNodeId: 'will-succeed',
        expandedNodeIds: ['will-succeed']
      }
    ])
  })
})

type RecordedWrite = {
  readonly state: PersistedLibraryViewState
  readonly sequence: number
}

function createQueuedWriteApi(): {
  readonly api: ViewStateApi
  readonly writes: RecordedWrite[]
  readonly resolveAll: () => Promise<void>
} {
  const writes: RecordedWrite[] = []
  const pendingResolvers: Array<() => void> = []
  let writeSequence = 0

  return {
    api: createViewStateApi({
      writeViewState: (state) => {
        const sequence = writeSequence
        writeSequence += 1
        writes.push({ state, sequence })

        return new Promise<{ readonly state: 'written' }>((resolve) => {
          pendingResolvers.push(() => resolve({ state: 'written' }))
        })
      }
    }),
    writes,
    async resolveAll(): Promise<void> {
      while (pendingResolvers.length > 0) {
        const resolver = pendingResolvers.shift()
        if (resolver === undefined) {
          break
        }

        resolver()
        await waitForMicrotasks()
      }

      await waitForMicrotasks()
    }
  }
}

function createViewStateApi(options: {
  readonly writeViewState: ViewStateApi['library']['viewState']['writeViewState']
}): ViewStateApi {
  return {
    library: {
      viewState: {
        readViewState: async () => ({ state: 'empty' }),
        writeViewState: options.writeViewState
      }
    }
  }
}

function waitForMicrotasks(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0))
}

function waitForWrites(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 50))
}
