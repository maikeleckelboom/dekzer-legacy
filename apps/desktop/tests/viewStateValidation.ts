import { strict as assert } from 'node:assert'

import { createViewStateStore } from '../src/renderer/libraryBrowser/runtime/viewState'
import type { ViewStateApi } from '../src/renderer/libraryBrowser/runtime/viewState'
import type { PersistedLibraryBrowserViewState } from '../src/shared/libraryBrowser/viewState'

void main()

async function main(): Promise<void> {
  await validatesWriteQueueIsLastStateWins()
  await validatesRapidSelectToggleDoesNotPersistStaleState()
  await validatesLoadDelegatesToApi()
  await validatesClearedStatePersisted()
  await validatesWriteFailureDoesNotThrowAndDoesNotBlockSubsequentWrites()
  await validatesSaveCopiesAndNormalizesInput()
}

type RecordedWrite = {
  readonly state: PersistedLibraryBrowserViewState
  readonly sequence: number
}

function createFakeApi(): {
  readonly api: ViewStateApi
  readonly writes: RecordedWrite[]
  readonly resolveAll: () => Promise<void>
} {
  const writes: RecordedWrite[] = []
  let writeSequence = 0
  const pendingResolvers: (() => void)[] = []

  const api: ViewStateApi = {
    library: {
      browser: {
        viewState: {
          async readViewState() {
            return { state: 'empty' as const }
          },
          writeViewState(state: PersistedLibraryBrowserViewState): Promise<{ state: 'written' }> {
            const sequence = writeSequence++
            writes.push({ state, sequence })

            return new Promise((resolve) => {
              pendingResolvers.push(() => resolve({ state: 'written' }))
            })
          }
        }
      }
    }
  }

  return {
    api,
    writes,
    async resolveAll(): Promise<void> {
      while (pendingResolvers.length > 0) {
        const resolver = pendingResolvers.shift()!
        resolver()
        await new Promise((resolve) => setTimeout(resolve, 0))
      }

      await new Promise((resolve) => setTimeout(resolve, 0))
    }
  }
}

async function validatesWriteQueueIsLastStateWins(): Promise<void> {
  const { api, writes, resolveAll } = createFakeApi()
  const store = createViewStateStore(api)

  const state1: PersistedLibraryBrowserViewState = {
    version: 1,
    expandedNodeIds: ['node-1']
  }
  const state2: PersistedLibraryBrowserViewState = {
    version: 1,
    expandedNodeIds: ['node-2']
  }
  const state3: PersistedLibraryBrowserViewState = {
    version: 1,
    expandedNodeIds: ['node-3']
  }

  store.save(state1)
  store.save(state2)
  store.save(state3)

  await resolveAll()

  assert.equal(writes.length, 2, 'should coalesce 3 rapid schedules to 2 writes')

  assert.deepEqual(
    writes[0]!.state.expandedNodeIds,
    ['node-1'],
    'first write is the initial in-flight state'
  )
  assert.deepEqual(
    writes[1]!.state.expandedNodeIds,
    ['node-3'],
    'second write is the newest state, not the stale second'
  )
  assert.ok(writes[0]!.sequence < writes[1]!.sequence, 'writes should be ordered')
}

async function validatesRapidSelectToggleDoesNotPersistStaleState(): Promise<void> {
  const { api, writes, resolveAll } = createFakeApi()
  const store = createViewStateStore(api)

  const selectState: PersistedLibraryBrowserViewState = {
    version: 1,
    selectedNodeId: 'node-A',
    expandedNodeIds: ['node-A']
  }
  const toggleState: PersistedLibraryBrowserViewState = {
    version: 1,
    selectedNodeId: 'node-A',
    expandedNodeIds: ['node-A', 'node-B']
  }

  store.save(selectState)
  store.save(toggleState)

  await resolveAll()

  assert.equal(writes.length, 2, 'select then toggle produces two writes')

  const lastWrite = writes[writes.length - 1]!
  assert.deepEqual(
    lastWrite.state.expandedNodeIds,
    ['node-A', 'node-B'],
    'final write reflects the toggle, not stale select state'
  )
}

async function validatesLoadDelegatesToApi(): Promise<void> {
  let readCount = 0
  const api = {
    library: {
      browser: {
        viewState: {
          async readViewState() {
            readCount++
            return { state: 'empty' as const }
          },
          async writeViewState() {
            return { state: 'written' as const }
          }
        }
      }
    }
  } satisfies ViewStateApi

  const store = createViewStateStore(api)
  const result = await store.load()

  assert.equal(readCount, 1)
  assert.equal(result.state, 'empty')
}

async function validatesClearedStatePersisted(): Promise<void> {
  const { api, writes, resolveAll } = createFakeApi()
  const store = createViewStateStore(api)

  store.save({
    version: 1,
    expandedNodeIds: []
  })

  await resolveAll()

  assert.equal(writes.length, 1)
  assert.deepEqual(writes[0]!.state.expandedNodeIds, [])
  assert.equal(writes[0]!.state.selectedNodeId, undefined)
}

async function validatesWriteFailureDoesNotThrowAndDoesNotBlockSubsequentWrites(): Promise<void> {
  let callCount = 0
  const writes: PersistedLibraryBrowserViewState[] = []

  const api = {
    library: {
      browser: {
        viewState: {
          async readViewState() {
            return { state: 'empty' as const }
          },
          async writeViewState(state: PersistedLibraryBrowserViewState) {
            callCount++

            if (callCount === 1) {
              throw new Error('simulated write failure')
            }

            writes.push(state)
            return { state: 'written' as const }
          }
        }
      }
    }
  } satisfies ViewStateApi

  const store = createViewStateStore(api)

  store.save({
    version: 1,
    selectedNodeId: 'will-fail',
    expandedNodeIds: ['will-fail']
  })

  await new Promise((resolve) => setTimeout(resolve, 0))

  store.save({
    version: 1,
    selectedNodeId: 'will-succeed',
    expandedNodeIds: ['will-succeed']
  })

  await new Promise((resolve) => setTimeout(resolve, 50))

  assert.equal(callCount, 2, 'should retry after failure')
  assert.equal(writes.length, 1, 'only the successful write should be recorded')
  assert.deepEqual(writes[0]!.selectedNodeId, 'will-succeed')
}

async function validatesSaveCopiesAndNormalizesInput(): Promise<void> {
  const { api, writes, resolveAll } = createFakeApi()
  const store = createViewStateStore(api)

  const expandedNodeIds = ['node-a', 'node-a']
  store.save({
    version: 1,
    selectedNodeId: 'node-a',
    expandedNodeIds
  })
  expandedNodeIds.push('node-b')
  await resolveAll()

  assert.deepEqual(writes[0]!.state, {
    version: 1,
    selectedNodeId: 'node-a',
    expandedNodeIds: ['node-a']
  })
}
