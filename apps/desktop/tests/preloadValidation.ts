import { strict as assert } from 'node:assert'

import { createDekzerRendererApi, exposeDekzerRendererApi } from '../src/preload/libraryBoundary'
import {
  libraryBoundaryHostStatusIpcChannels,
  type LibraryBoundaryHostStatus
} from '../src/shared/libraryBoundary/status'
import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadRequest,
  type LibraryHierarchyReadResult
} from '../src/shared/libraryHierarchy/read'
import { emitStatus, testStatus } from './support/libraryBoundary'

void main()

async function main(): Promise<void> {
  await validatesPreloadApiSurface()
}

async function validatesPreloadApiSurface(): Promise<void> {
  const status = testStatus()
  const hierarchyRequest = firstAvailableSourceReadRequest()
  const hierarchyResult: LibraryHierarchyReadResult = {
    state: 'noTarget',
    error: {
      code: 'noTarget',
      message: 'No library source is available for a literal hierarchy read.'
    }
  }
  let receivedHierarchyRequest: unknown = null
  const listeners = new Map<
    string,
    Set<(event: unknown, changedStatus: LibraryBoundaryHostStatus) => void>
  >()
  const ipcRenderer = {
    invoke: async (channel, ...args) => {
      if (channel === libraryBoundaryHostStatusIpcChannels.getStatus) {
        assert.deepEqual(args, [])
        return status
      }

      if (channel === libraryHierarchyReadIpcChannels.readLiteralHierarchyChildren) {
        assert.equal(args.length, 1)
        receivedHierarchyRequest = args[0]
        return hierarchyResult
      }

      throw new Error(`unexpected preload invoke channel ${channel}`)
    },
    on: (channel, listener) => {
      const channelListeners = listeners.get(channel) ?? new Set()
      channelListeners.add(listener)
      listeners.set(channel, channelListeners)
    },
    off: (channel, listener) => {
      listeners.get(channel)?.delete(listener)
    }
  }
  const exposedApis = new Map<string, unknown>()

  exposeDekzerRendererApi(
    {
      exposeInMainWorld(apiKey, api): void {
        exposedApis.set(apiKey, api)
      }
    },
    ipcRenderer
  )

  assert.deepEqual([...exposedApis.keys()], ['dekzer'])
  assert.equal(exposedApis.has('desktop'), false)

  const api = createDekzerRendererApi(ipcRenderer)

  assert.deepEqual(Object.keys(api), ['libraryBoundary'])
  assert.deepEqual(Object.keys(api.libraryBoundary).sort(), [
    'getStatus',
    'onStatusChanged',
    'readLiteralHierarchyChildren'
  ])
  assert.equal('ipcRenderer' in api, false)
  assert.equal('client' in api, false)
  assert.equal('transport' in api, false)
  assert.equal('client' in api.libraryBoundary, false)
  assert.equal('transport' in api.libraryBoundary, false)
  assert.equal('ipcRenderer' in api.libraryBoundary, false)
  assert.equal('registerLocalRoot' in api.libraryBoundary, false)
  assert.equal('runRootScan' in api.libraryBoundary, false)
  assert.equal(await api.libraryBoundary.getStatus(), status)
  assert.equal(
    await api.libraryBoundary.readLiteralHierarchyChildren(hierarchyRequest),
    hierarchyResult
  )
  assert.equal(receivedHierarchyRequest, hierarchyRequest)

  let receivedStatus: LibraryBoundaryHostStatus | null = null
  const unsubscribe = api.libraryBoundary.onStatusChanged((changedStatus) => {
    receivedStatus = changedStatus
  })

  emitStatus(listeners, status)
  assert.equal(receivedStatus, status)

  receivedStatus = null
  unsubscribe()
  emitStatus(listeners, status)
  assert.equal(receivedStatus, null)
}

function firstAvailableSourceReadRequest(): LibraryHierarchyReadRequest {
  return {
    target: {
      kind: 'firstAvailableSource'
    },
    offset: 0,
    limit: 50
  }
}
