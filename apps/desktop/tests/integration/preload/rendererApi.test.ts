import { describe, expect, it } from 'vitest'

import { createRendererApi, exposeRendererApi } from '../../../src/preload/rendererApi'
import {
  hostStatusChannels,
  type LibraryBoundaryHostStatus
} from '../../../src/shared/libraryBoundary/status'
import {
  libraryViewStateChannels,
  type LibraryViewStateReadResult,
  type LibraryViewStateWriteResult,
  type PersistedLibraryViewState
} from '../../../src/shared/libraryViewState/viewState'
import {
  hierarchyReadChannels,
  type ReadResult
} from '../../../src/shared/libraryHierarchy/readChildren'
import {
  navigationReadChannels,
  type NavigationReadRowsResult
} from '../../../src/shared/libraryNavigation/readRows'
import { rootChannels } from '../../../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../../../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { ReadLocalRootsOutcome } from '../../../src/shared/libraryRoots/readLocalRoots'
import type { LocalRootScanResult } from '../../../src/shared/libraryRoots/runScan'
import type { CancelRootScanResult } from '../../../src/shared/libraryRoots/cancelScan'
import type { UnregisterLocalRootResult } from '../../../src/shared/libraryRoots/unregisterLocalRoot'
import {
  contentsReadChannels,
  type ContentsReadResult
} from '../../../src/shared/libraryContents/read'
import { sourceLifecycleReadChannels } from '../../../src/shared/librarySourceLifecycle/channels'
import type { ReadSourceLifecycleResult } from '../../../src/shared/librarySourceLifecycle/readSourceLifecycle'
import {
  boundaryEventChannels,
  type BoundaryEventDeliveryPayload
} from '../../../src/shared/libraryBoundary/events'
import { emitStatus, testStatus } from '../../support/libraryBoundary'
import { firstAvailableSourceReadRequest } from '../../support/libraryHierarchy'

describe('preload renderer API', () => {
  it('exposes the library API and forwards calls over owned IPC channels', async () => {
    const status = testStatus()
    const hierarchyRequest = firstAvailableSourceReadRequest()
    const navigationRequest = { parentNavigationRowId: null }
    const contentsRequest = {
      scope: {
        kind: 'source' as const,
        sourceId: '7'
      },
      policy: {
        mediaClasses: ['audio', 'video'] as const,
        rowProfile: { kind: 'primaryMedia' as const }
      },
      recursion: 'recursive' as const,
      limit: 100
    }
    const sourceLifecycleRequest = { sourceId: '7' }
    const scanRequest = { rootId: 'root-1' }
    const persistedViewState: PersistedLibraryViewState = {
      version: 1,
      selectedNodeId: 'navigation-row:1',
      expandedNodeIds: ['navigation-row:1']
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
    const contentsResult: ContentsReadResult = {
      state: 'noTarget',
      error: {
        code: 'noTarget',
        message: 'No contents scope was provided.'
      }
    }
    const sourceLifecycleResult: ReadSourceLifecycleResult = {
      state: 'ready',
      lifecycle: {
        sourceId: '7',
        sourceClass: 'externalMounted',
        isUserVisible: true,
        mountStatus: 'mounted',
        accessState: 'accessible',
        scanPhase: 'complete',
        lastScanStartedAtMs: 10,
        lastScanFinishedAtMs: 20,
        lastSuccessfulScanAtMs: 20,
        lastSeenAtMs: 9,
        updatedAtMs: 21
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
      state: 'started',
      scanRunId: 'scan-1'
    }
    const cancelScanResult: CancelRootScanResult = {
      state: 'accepted',
      status: 'accepted'
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
    const viewStateReadResult: LibraryViewStateReadResult = {
      state: 'ready',
      viewState: {
        version: 1,
        selectedNodeId: 'navigation-row:1',
        expandedNodeIds: ['navigation-row:1', 'source-directory:2']
      }
    }
    const viewStateWriteResult: LibraryViewStateWriteResult = { state: 'written' }
    let receivedChoiceArgs: readonly unknown[] | undefined
    let receivedHierarchyRequest: unknown
    let receivedNavigationRequest: unknown
    let receivedScanRequest: unknown
    let receivedCancelScanRequest: unknown
    let receivedContentsRequest: unknown
    let receivedSourceLifecycleRequest: unknown
    let receivedViewStatePayload: unknown
    let subscribeCount = 0
    let unsubscribeCount = 0
    const listeners = new Map<string, Set<(event: unknown, payload: unknown) => void>>()
    const ipcRenderer = {
      invoke: async (channel: string, ...args: readonly unknown[]): Promise<unknown> => {
        if (channel === hostStatusChannels.getStatus) {
          expect(args).toEqual([])
          return status
        }

        if (channel === navigationReadChannels.readRows) {
          receivedNavigationRequest = args[0]
          return navigationResult
        }

        if (channel === hierarchyReadChannels.readChildren) {
          receivedHierarchyRequest = args[0]
          return hierarchyResult
        }

        if (channel === contentsReadChannels.read) {
          receivedContentsRequest = args[0]
          return contentsResult
        }

        if (channel === sourceLifecycleReadChannels.readSourceLifecycle) {
          receivedSourceLifecycleRequest = args[0]
          return sourceLifecycleResult
        }

        if (channel === rootChannels.chooseAndRegisterLocal) {
          receivedChoiceArgs = args
          return choiceResult
        }

        if (channel === rootChannels.runScan) {
          receivedScanRequest = args[0]
          return scanResult
        }

        if (channel === rootChannels.cancelScan) {
          receivedCancelScanRequest = args[0]
          return cancelScanResult
        }

        if (channel === rootChannels.readLocalRoots) {
          expect(args).toEqual([])
          return readLocalRootsResult
        }

        if (channel === rootChannels.unregisterLocalRoot) {
          return unregisterLocalRootResult
        }

        if (channel === libraryViewStateChannels.readViewState) {
          expect(args).toEqual([])
          return viewStateReadResult
        }

        if (channel === libraryViewStateChannels.writeViewState) {
          receivedViewStatePayload = args[0]
          return viewStateWriteResult
        }

        if (channel === boundaryEventChannels.subscribe) {
          expect(args).toEqual([])
          subscribeCount += 1
          return { kind: 'subscribed' }
        }

        if (channel === boundaryEventChannels.unsubscribe) {
          expect(args).toEqual([])
          unsubscribeCount += 1
          return { kind: 'unsubscribed' }
        }

        throw new Error(`Unexpected preload invoke channel ${channel}.`)
      },
      on: (channel: string, listener: (event: unknown, payload: unknown) => void): void => {
        const channelListeners = listeners.get(channel) ?? new Set()
        channelListeners.add(listener)
        listeners.set(channel, channelListeners)
      },
      off: (channel: string, listener: (event: unknown, payload: unknown) => void): void => {
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
    expect(exposedApis.get('dekzer')).toBeDefined()

    const api = createRendererApi(ipcRenderer)

    await expect(api.library.host.getStatus()).resolves.toBe(status)
    await expect(
      (
        api.library.roots.chooseAndRegisterLocal as (
          request?: unknown
        ) => Promise<LocalRootChoiceResult>
      )({ absolutePath: 'C:/RendererMustNotControlThis' })
    ).resolves.toBe(choiceResult)
    expect(receivedChoiceArgs).toEqual([])
    await expect(api.library.roots.runScan(scanRequest)).resolves.toBe(scanResult)
    expect(receivedScanRequest).toBe(scanRequest)
    await expect(api.library.roots.cancelScan({ scanRunId: 'scan-1' })).resolves.toBe(
      cancelScanResult
    )
    expect(receivedCancelScanRequest).toEqual({ scanRunId: 'scan-1' })
    await expect(api.library.roots.readLocalRoots()).resolves.toBe(readLocalRootsResult)
    await expect(api.library.roots.unregisterLocalRoot({ rootId: '7' })).resolves.toBe(
      unregisterLocalRootResult
    )
    await expect(api.library.navigation.readRows(navigationRequest)).resolves.toBe(navigationResult)
    expect(receivedNavigationRequest).toBe(navigationRequest)
    await expect(api.library.hierarchy.readChildren(hierarchyRequest)).resolves.toBe(
      hierarchyResult
    )
    expect(receivedHierarchyRequest).toBe(hierarchyRequest)
    await expect(api.library.contents.read(contentsRequest)).resolves.toBe(contentsResult)
    expect(receivedContentsRequest).toBe(contentsRequest)
    await expect(
      api.library.sourceLifecycle.readSourceLifecycle(sourceLifecycleRequest)
    ).resolves.toBe(sourceLifecycleResult)
    expect(receivedSourceLifecycleRequest).toBe(sourceLifecycleRequest)
    await expect(api.library.viewState.readViewState()).resolves.toBe(viewStateReadResult)
    await expect(api.library.viewState.writeViewState(persistedViewState)).resolves.toBe(
      viewStateWriteResult
    )
    expect(receivedViewStatePayload).toBe(persistedViewState)

    let receivedStatus: LibraryBoundaryHostStatus | null = null
    const unsubscribe = api.library.host.onStatusChanged((changedStatus) => {
      receivedStatus = changedStatus
    })

    emitStatus(listeners, status)
    expect(receivedStatus).toBe(status)

    receivedStatus = null
    unsubscribe()
    emitStatus(listeners, status)
    expect(receivedStatus).toBeNull()

    const eventPayload: BoundaryEventDeliveryPayload = {
      kind: 'batch',
      events: [],
      latestEventSequence: null,
      earliestRetainedSequence: null,
      gapDetected: false
    }
    let receivedEventPayload: BoundaryEventDeliveryPayload | undefined
    const unsubscribeEvents = api.library.events.subscribe((payload) => {
      receivedEventPayload = payload
    })

    await waitForMicrotasks()
    expect(subscribeCount).toBe(1)
    for (const listener of listeners.get(boundaryEventChannels.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBe(eventPayload)

    unsubscribeEvents()
    await waitForMicrotasks()
    expect(unsubscribeCount).toBe(1)
    receivedEventPayload = undefined
    for (const listener of listeners.get(boundaryEventChannels.batch) ?? []) {
      listener({}, eventPayload)
    }
    expect(receivedEventPayload).toBeUndefined()
  })
})

async function waitForMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}
