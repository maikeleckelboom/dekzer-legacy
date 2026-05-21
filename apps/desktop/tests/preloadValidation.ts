import { strict as assert } from 'node:assert'

import { firstAvailableSourceReadRequest } from './support/libraryHierarchy'
import { createRendererApi, exposeRendererApi } from '../src/preload/rendererApi'
import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../src/shared/libraryBoundary/status'
import {
  hierarchyReadChannels,
  type LibraryHierarchyReadChildrenResult
} from '../src/shared/libraryHierarchy/readChildren'
import { rootChannels } from '../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import { emitStatus, testStatus } from './support/libraryBoundary'

void main()

async function main(): Promise<void> {
  await validatesPreloadApiSurface()
}

async function validatesPreloadApiSurface(): Promise<void> {
  const status = testStatus()
  const hierarchyRequest = firstAvailableSourceReadRequest()
  const scanRequest = {
    rootId: 'root-1'
  }
  const hierarchyResult: LibraryHierarchyReadChildrenResult = {
    state: 'noTarget',
    error: {
      code: 'noTarget',
      message: 'No library source is available for a literal hierarchy read.'
    }
  }
  const choiceResult: LocalRootChoiceResult = {
    state: 'registered',
    root: {
      rootId: '7',
      canonicalPath: 'C:/Music'
    }
  }
  const scanResult: LocalRootScanResult = {
    state: 'scanned',
    rootId: 'root-1',
    scanRunId: 'scan-1',
    discoveredFileCount: 12,
    queuedSourceWorkItems: 8
  }
  let receivedHierarchyRequest: unknown = null
  let receivedChoiceArgs: readonly unknown[] | undefined
  let receivedScanRequest: unknown = null
  const listeners = new Map<
    string,
    Set<(event: unknown, changedStatus: LibraryBoundaryHostStatus) => void>
  >()
  const ipcRenderer = {
    invoke: async (channel, ...args) => {
      if (channel === hostStatusChannels.getStatus) {
        assert.deepEqual(args, [])
        return status
      }

      if (channel === hierarchyReadChannels.readChildren) {
        assert.equal(args.length, 1)
        receivedHierarchyRequest = args[0]
        return hierarchyResult
      }

      if (channel === rootChannels.chooseAndRegisterLocal) {
        receivedChoiceArgs = args
        return choiceResult
      }

      if (channel === rootChannels.runScan) {
        assert.equal(args.length, 1)
        receivedScanRequest = args[0]
        return scanResult
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

  exposeRendererApi(
    {
      exposeInMainWorld(apiKey, api): void {
        exposedApis.set(apiKey, api)
      }
    },
    ipcRenderer
  )

  assert.deepEqual([...exposedApis.keys()], ['dekzer'])
  assert.equal(exposedApis.has('desktop'), false)

  const api = createRendererApi(ipcRenderer)

  assert.deepEqual(Object.keys(api), ['library'])
  assert.equal('libraryBoundary' in api, false)
  assert.deepEqual(Object.keys(api.library).sort(), ['hierarchy', 'host', 'roots'])
  assert.deepEqual(Object.keys(api.library.host).sort(), ['getStatus', 'onStatusChanged'])
  assert.deepEqual(Object.keys(api.library.hierarchy), ['readChildren'])
  assert.deepEqual(Object.keys(api.library.roots).sort(), ['chooseAndRegisterLocal', 'runScan'])
  assert.equal('ipcRenderer' in api, false)
  assert.equal('libraryBoundary' in api, false)
  assert.equal('client' in api, false)
  assert.equal('transport' in api, false)
  assert.equal('client' in api.library, false)
  assert.equal('transport' in api.library, false)
  assert.equal('ipcRenderer' in api.library, false)
  assert.equal('runRootScan' in api.library, false)
  assert.equal('getStatus' in api.library, false)
  assert.equal('onStatusChanged' in api.library, false)
  assert.equal('readLiteralHierarchyChildren' in api.library, false)
  assert.equal('registerLocal' in api.library, false)
  assert.equal('registerLocalRoot' in api.library, false)
  assert.equal('registerLocal' in api.library.roots, false)
  assert.equal('readLiteralHierarchyChildren' in api.library.hierarchy, false)
  assert.equal('registerLocalRoot' in api.library.roots, false)
  assert.equal('runRootScan' in api.library.roots, false)
  assert.equal('client' in api.library.roots, false)
  assert.equal('transport' in api.library.roots, false)
  assert.equal('ipcRenderer' in api.library.roots, false)
  assert.equal(await api.library.host.getStatus(), status)
  assert.equal(
    await (
      api.library.roots.chooseAndRegisterLocal as (
        request?: unknown
      ) => Promise<LocalRootChoiceResult>
    )({
      absolutePath: 'C:/RendererMustNotControlThis'
    }),
    choiceResult
  )
  assert.deepEqual(receivedChoiceArgs, [])
  assert.equal(await api.library.roots.runScan(scanRequest), scanResult)
  assert.equal(receivedScanRequest, scanRequest)
  assert.equal(await api.library.hierarchy.readChildren(hierarchyRequest), hierarchyResult)
  assert.equal(receivedHierarchyRequest, hierarchyRequest)

  let receivedStatus: LibraryBoundaryHostStatus | null = null
  const unsubscribe = api.library.host.onStatusChanged((changedStatus) => {
    receivedStatus = changedStatus
  })

  emitStatus(listeners, status)
  assert.equal(receivedStatus, status)

  receivedStatus = null
  unsubscribe()
  emitStatus(listeners, status)
  assert.equal(receivedStatus, null)
}
