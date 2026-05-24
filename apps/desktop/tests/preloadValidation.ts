import { strict as assert } from 'node:assert'

import { firstAvailableSourceReadRequest } from './support/libraryHierarchy'
import { createRendererApi, exposeRendererApi } from '../src/preload/rendererApi'
import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../src/shared/libraryBoundary/status'
import { hierarchyReadChannels, type ReadResult } from '../src/shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type NavigationReadRowsResult
} from '../src/shared/libraryNavigation/readRows'
import { rootChannels } from '../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../src/shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../src/shared/libraryRoots/runScan'
import type { UnregisterLocalRootResult } from '../src/shared/libraryRoots/unregisterLocalRoot'
import {
  libraryBrowserChannels,
  type LibraryBrowserViewStateReadResult,
  type LibraryBrowserViewStateWriteResult,
  type PersistedLibraryBrowserViewState
} from '../src/shared/libraryBrowser/viewState'
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
  const navigationRequest = {
    parentNavigationRowId: null
  }
  const navigationResult: NavigationReadRowsResult = {
    state: 'ready',
    rows: [
      {
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
    ]
  }
  const hierarchyResult: ReadResult = {
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
  const readLocalRootsResult: ReadLocalRootsOutcome = {
    state: 'read',
    roots: [
      {
        rootId: '7',
        canonicalPath: 'C:/Music',
        availability: 'available'
      }
    ]
  }
  const unregisterLocalRootResult: UnregisterLocalRootResult = {
    state: 'unregistered',
    unregistered: true
  }
  const viewStateReadResult: LibraryBrowserViewStateReadResult = {
    state: 'ready',
    viewState: {
      version: 1,
      selectedNodeId: 'navigation-row:1',
      expandedNodeIds: ['navigation-row:1', 'source-directory:2']
    }
  }
  const viewStateWriteResult: LibraryBrowserViewStateWriteResult = {
    state: 'written'
  }
  const persistedViewState: PersistedLibraryBrowserViewState = {
    version: 1,
    selectedNodeId: 'navigation-row:1',
    expandedNodeIds: ['navigation-row:1']
  }
  let receivedViewStatePayload: unknown = null
  let receivedNavigationRequest: unknown = null
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

      if (channel === navigationReadChannels.readRows) {
        assert.equal(args.length, 1)
        receivedNavigationRequest = args[0]
        return navigationResult
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

      if (channel === rootChannels.readLocalRoots) {
        assert.deepEqual(args, [])
        return readLocalRootsResult
      }

      if (channel === rootChannels.unregisterLocalRoot) {
        assert.equal(args.length, 1)
        return unregisterLocalRootResult
      }

      if (channel === libraryBrowserChannels.readViewState) {
        assert.deepEqual(args, [])
        return viewStateReadResult
      }

      if (channel === libraryBrowserChannels.writeViewState) {
        assert.equal(args.length, 1)
        receivedViewStatePayload = args[0]
        return viewStateWriteResult
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

  const api = createRendererApi(ipcRenderer)

  assert.deepEqual(Object.keys(api), ['library'])
  assert.deepEqual(Object.keys(api.library).sort(), [
    'browser',
    'hierarchy',
    'host',
    'navigation',
    'roots',
    'selectedContents'
  ])
  assert.deepEqual(Object.keys(api.library.host).sort(), ['getStatus', 'onStatusChanged'])
  assert.deepEqual(Object.keys(api.library.hierarchy), ['readChildren'])
  assert.deepEqual(Object.keys(api.library.navigation), ['readRows'])
  assert.deepEqual(Object.keys(api.library.selectedContents), ['read'])
  assert.deepEqual(Object.keys(api.library.roots).sort(), [
    'chooseAndRegisterLocal',
    'readLocalRoots',
    'runScan',
    'unregisterLocalRoot'
  ])
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
  assert.equal(await api.library.roots.readLocalRoots(), readLocalRootsResult)
  assert.equal(
    await api.library.roots.unregisterLocalRoot({ rootId: '7' }),
    unregisterLocalRootResult
  )
  assert.equal(await api.library.navigation.readRows(navigationRequest), navigationResult)
  assert.equal(receivedNavigationRequest, navigationRequest)
  assert.equal(await api.library.hierarchy.readChildren(hierarchyRequest), hierarchyResult)
  assert.equal(receivedHierarchyRequest, hierarchyRequest)

  assert.deepEqual(Object.keys(api.library.browser), ['viewState'])
  assert.deepEqual(Object.keys(api.library.browser.viewState).sort(), [
    'readViewState',
    'writeViewState'
  ])

  assert.equal(await api.library.browser.viewState.readViewState(), viewStateReadResult)

  assert.equal(
    await api.library.browser.viewState.writeViewState(persistedViewState),
    viewStateWriteResult
  )
  assert.deepEqual(receivedViewStatePayload, persistedViewState)

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
