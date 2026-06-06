import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  contentsRequestKey,
  createContentsReadController,
  type ContentsBoundaryState
} from '../../../../src/renderer/library/boundary/contentsRead'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type {
  ContentsFileRow,
  ContentsReadRequest,
  ContentsReadResult
} from '../../../../src/shared/libraryContents/read'
import type { LibraryContentsApi } from '../../../../src/shared/rendererApi'

describe('createContentsReadController', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('requests recursive audio-only audio-browse contents for selected directories', async () => {
    let capturedRequest: ContentsReadRequest | undefined
    const contentsApi: LibraryContentsApi = {
      async read(request): Promise<ContentsReadResult> {
        capturedRequest = request
        return {
          state: 'ready',
          result: {
            state: 'empty',
            scope: request.scope,
            policy: request.policy,
            recursion: request.recursion,
            rows: [],
            coverage: {
              state: 'complete',
              recursiveScopeComplete: true,
              emptyResultAuthoritative: true
            }
          }
        }
      }
    }
    const controller = createContentsReadController(contentsApi)

    controller.start()
    await controller.readForBinding(directoryBinding())

    expect(capturedRequest).toEqual({
      scope: {
        kind: 'directory',
        sourceId: '7',
        sourceDirectoryId: '11'
      },
      policy: {
        kind: 'audioBrowse'
      },
      recursion: 'recursive',
      limit: 100
    })
    expect(capturedRequest?.policy).toEqual({ kind: 'audioBrowse' })
  })

  it('includes profile-specific policy in request keys', () => {
    const scope = {
      kind: 'directory' as const,
      sourceId: '7',
      sourceDirectoryId: '11'
    }

    expect(
      contentsRequestKey(
        scope,
        {
          kind: 'audioBrowse'
        },
        'recursive'
      )
    ).toBe('directory:7:11:audioBrowse:recursive')
    expect(
      contentsRequestKey(
        scope,
        {
          kind: 'sourceFileInventory',
          fileClasses: ['audio']
        },
        'recursive'
      )
    ).toBe('directory:7:11:sourceFileInventory:audio:recursive')
    expect(
      contentsRequestKey(
        scope,
        {
          kind: 'primaryMedia',
          mediaKinds: ['audio', 'video']
        },
        'recursive'
      )
    ).toBe('directory:7:11:primaryMedia:audio,video:recursive')
  })

  it('preloadForBinding schedules no IPC before the rest threshold', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())

    await vi.advanceTimersByTimeAsync(124)
    expect(contentsApi.requests).toEqual([])
    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      detail: 'No contents scope has been requested.'
    })

    await vi.advanceTimersByTimeAsync(1)
    expect(contentsApi.requests).toHaveLength(1)
    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      detail: 'No contents scope has been requested.'
    })
  })

  it('cancelPreloadForBinding before threshold prevents IPC', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(60)
    controller.cancelPreloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    expect(contentsApi.requests).toEqual([])
  })

  it('preloadForBinding does not mutate visible contents state before or after response', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      detail: 'No contents scope has been requested.'
    })

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()

    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      detail: 'No contents scope has been requested.'
    })
  })

  it('caps speculative in-flight prefetch reads', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding('11'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    controller.preloadForBinding(directoryBinding('12'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    expect(contentsApi.requests).toHaveLength(1)
    expect(contentsApi.requests[0]?.scope).toMatchObject({
      kind: 'directory',
      sourceDirectoryId: '11'
    })
  })

  it('normal selected reads are not blocked by the speculative in-flight cap', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding('11'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    const read = controller.readForBinding(directoryBinding('12'))

    expect(contentsApi.requests).toHaveLength(2)
    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      pending: {
        requestKey: 'directory:7:12:audioBrowse:recursive',
        sequence: 1
      }
    })

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('readForBinding consumes a fresh warmed snapshot without a new IPC call', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()

    await expect(controller.readForBinding(directoryBinding())).resolves.toBe(true)

    expect(contentsApi.requests).toHaveLength(1)
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')
  })

  it('readForBinding reuses a matching in-flight prefetch and creates normal pending identity', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    const read = controller.readForBinding(directoryBinding())

    expect(contentsApi.requests).toHaveLength(1)
    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      pending: {
        requestKey: 'directory:7:11:audioBrowse:recursive',
        sequence: 1
      }
    })

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])
  })

  it('stale in-flight prefetch cannot commit to the wrong selected scope', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding('11'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    const staleRead = controller.readForBinding(directoryBinding('11'))
    const currentRead = controller.readForBinding(directoryBinding('12'))

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(currentRead).resolves.toBe(true)

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await expect(staleRead).resolves.toBe(false)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('force reads bypass warmed snapshots and refresh the selected scope', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()

    const read = controller.readForBinding(directoryBinding(), { force: true })

    expect(contentsApi.requests).toHaveLength(2)
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('late prefetch completion after force does not overwrite the refreshed warm snapshot', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding('11'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    const forceRead = controller.readForBinding(directoryBinding('11'), { force: true })
    expect(contentsApi.requests).toHaveLength(2)

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(forceRead).resolves.toBe(true)

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()

    const otherRead = controller.readForBinding(directoryBinding('12'))
    contentsApi.resolveAt(2, readyContents(requestAt(contentsApi, 2), [contentsRow('c', 'C.wav')]))
    await expect(otherRead).resolves.toBe(true)

    await expect(controller.readForBinding(directoryBinding('11'))).resolves.toBe(true)
    expect(contentsApi.requests).toHaveLength(3)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('load-more reads do not consume base warm cache entries', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()

    const read = controller.readForBinding(directoryBinding(), { cursor: 'cursor-a' })

    expect(contentsApi.requests).toHaveLength(2)
    expect(contentsApi.requests[1]).toMatchObject({
      cursor: 'cursor-a',
      policy: {
        kind: 'audioBrowse'
      }
    })
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('a2', 'A2.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['A2.wav'])
  })

  it('warm cache entries expire before selection consumes them', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding())
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await flushPromises()
    await vi.advanceTimersByTimeAsync(10_001)

    const read = controller.readForBinding(directoryBinding())

    expect(contentsApi.requests).toHaveLength(2)
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('warm cache is bounded to the most recent entries', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()

    for (let index = 0; index < 17; index++) {
      controller.preloadForBinding(directoryBinding(`${index + 1}`))
      await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
      contentsApi.resolveNext(
        readyContents(requestAt(contentsApi, index), [contentsRow(`${index}`, `${index}.wav`)])
      )
      await flushPromises()
    }

    const evictedRead = controller.readForBinding(directoryBinding('1'))
    expect(contentsApi.requests).toHaveLength(18)
    contentsApi.resolveNext(
      readyContents(requestAt(contentsApi, 17), [contentsRow('fresh', 'Fresh.wav')])
    )
    await expect(evictedRead).resolves.toBe(true)

    await expect(controller.readForBinding(directoryBinding('17'))).resolves.toBe(true)
    expect(contentsApi.requests).toHaveLength(18)
    expect(visibleLabels(controller.state.value)).toEqual(['16.wav'])
  })

  it('clear removes warm entries and scheduled preload timers', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(directoryBinding('11'))
    controller.clear()
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    expect(contentsApi.requests).toEqual([])

    controller.preloadForBinding(directoryBinding('12'))
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('b', 'B.wav')]))
    await flushPromises()
    controller.clearWarmSnapshots()

    const read = controller.readForBinding(directoryBinding('12'))
    expect(contentsApi.requests).toHaveLength(2)
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('c', 'C.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])
  })

  it('unsupported bindings do not preload contents', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    controller.preloadForBinding(readStateBinding())
    controller.preloadForBinding(undefined)
    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())

    expect(contentsApi.requests).toEqual([])
  })

  it('threshold-gates initial loading when there is no accepted snapshot', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const read = controller.readForBinding(directoryBinding())

    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      pending: {
        requestKey: 'directory:7:11:audioBrowse:recursive',
        sequence: 1
      }
    })

    await vi.advanceTimersByTimeAsync(124)
    expect(controller.state.value.kind).toBe('idle')

    await vi.advanceTimersByTimeAsync(1)
    expect(controller.state.value).toMatchObject({
      kind: 'loading',
      requestKey: 'directory:7:11:audioBrowse:recursive',
      sequence: 1
    })

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await expect(read).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])
  })

  it('retains accepted rows while a same-scope refresh is pending', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const refresh = controller.readForBinding(directoryBinding(), { force: true })

    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        requestKey: 'directory:7:11:audioBrowse:recursive',
        sequence: 2
      }
    })
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await refresh

    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')
  })

  it('defers cross-scope pending presentation until the local read threshold', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding('11'))
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const nextScope = controller.readForBinding(directoryBinding('12'))

    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        requestKey: 'directory:7:12:audioBrowse:recursive',
        sequence: 2,
        presentation: 'deferred'
      }
    })
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])

    await vi.advanceTimersByTimeAsync(124)
    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        presentation: 'deferred'
      }
    })

    await vi.advanceTimersByTimeAsync(1)
    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        requestKey: 'directory:7:12:audioBrowse:recursive',
        sequence: 2,
        presentation: 'visible'
      }
    })
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(nextScope).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')
  })

  it('accepts fast cross-scope responses without promoting pending presentation', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding('11'))
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const nextScope = controller.readForBinding(directoryBinding('12'))

    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        presentation: 'deferred'
      }
    })

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(nextScope).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')

    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')
  })

  it('cancels stale cross-scope threshold timers when selection changes again', async () => {
    vi.useFakeTimers()
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding('11'))
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const staleScope = controller.readForBinding(directoryBinding('12'))
    await vi.advanceTimersByTimeAsync(60)
    const currentScope = controller.readForBinding(directoryBinding('13'))
    await vi.advanceTimersByTimeAsync(65)

    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      pending: {
        requestKey: 'directory:7:13:audioBrowse:recursive',
        sequence: 3,
        presentation: 'deferred'
      }
    })

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 2), [contentsRow('c', 'C.wav')]))
    await expect(currentScope).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')

    await vi.advanceTimersByTimeAsync(loadingThresholdPlusMargin())
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(staleScope).resolves.toBe(false)
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])
  })

  it('does not let an older same-scope refresh overwrite a newer accepted response', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const staleRefresh = controller.readForBinding(directoryBinding(), { force: true })
    const currentRefresh = controller.readForBinding(directoryBinding(), { force: true })

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 2), [contentsRow('c', 'C.wav')]))
    await expect(currentRefresh).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(staleRefresh).resolves.toBe(false)
    expect(visibleLabels(controller.state.value)).toEqual(['C.wav'])
  })

  it('keeps a newer selected scope pending when an older scope responds late', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const scopeA = controller.readForBinding(directoryBinding('11'))
    const scopeB = controller.readForBinding(directoryBinding('12'))

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await expect(scopeA).resolves.toBe(false)

    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      pending: {
        requestKey: 'directory:7:12:audioBrowse:recursive',
        sequence: 2
      }
    })

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(scopeB).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('commits the newer scope first and discards an older late response', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const scopeA = controller.readForBinding(directoryBinding('11'))
    const scopeB = controller.readForBinding(directoryBinding('12'))

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await expect(scopeB).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])

    contentsApi.resolveAt(0, readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await expect(scopeA).resolves.toBe(false)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('accepts an empty result only after the current backend response arrives', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const read = controller.readForBinding(directoryBinding())

    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      pending: expect.any(Object)
    })

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [], 'empty'))
    await expect(read).resolves.toBe(true)

    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      result: {
        state: 'ready',
        result: {
          state: 'empty',
          rows: []
        }
      }
    })
  })

  it('does not append stale load-more rows to a newer scope', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding('11'))
    contentsApi.resolveNext(
      readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')], 'ready', 'cursor-a')
    )
    await initial

    const loadMore = controller.readForBinding(directoryBinding('11'), { cursor: 'cursor-a' })
    const scopeB = controller.readForBinding(directoryBinding('12'))

    contentsApi.resolveAt(1, readyContents(requestAt(contentsApi, 2), [contentsRow('b', 'B.wav')]))
    await expect(scopeB).resolves.toBe(true)

    contentsApi.resolveAt(
      0,
      readyContents(requestAt(contentsApi, 1), [contentsRow('a2', 'A2.wav')])
    )
    await expect(loadMore).resolves.toBe(false)
    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
  })

  it('retains accepted rows when loading more contents fails', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding('11'))
    contentsApi.resolveNext(
      readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')], 'ready', 'cursor-a')
    )
    await initial

    const loadMore = controller.readForBinding(directoryBinding('11'), { cursor: 'cursor-a' })
    contentsApi.rejectNext(new Error('read failed'))

    await expect(loadMore).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])
    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      refreshError: 'Unable to request library contents.'
    })
  })

  it('retains accepted rows when a refresh fails', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    const refresh = controller.readForBinding(directoryBinding(), { force: true })
    contentsApi.rejectNext(new Error('read failed'))

    await expect(refresh).resolves.toBe(true)
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])
    expect(controller.state.value).toMatchObject({
      kind: 'ready',
      refreshError: 'Unable to request library contents.'
    })
  })

  it('clears stale contents when readForBinding receives undefined binding', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    expect(controller.state.value.kind).toBe('ready')

    await expect(controller.readForBinding(undefined, { force: true })).resolves.toBe(false)
    expect(controller.state.value.kind).toBe('idle')
    expect(contentsApi.requests).toHaveLength(1)
  })

  it('clears stale contents when readForBinding receives a readState binding', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    const initial = controller.readForBinding(directoryBinding())
    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 0), [contentsRow('a', 'A.wav')]))
    await initial

    expect(controller.state.value.kind).toBe('ready')

    await expect(controller.readForBinding(readStateBinding(), { force: true })).resolves.toBe(
      false
    )
    expect(controller.state.value.kind).toBe('idle')
  })

  it('clears contents and returns false for undefined binding without stale state', async () => {
    const contentsApi = deferredContentsApi()
    const controller = createContentsReadController(contentsApi)

    controller.start()
    await expect(controller.readForBinding(undefined)).resolves.toBe(false)
    expect(controller.state.value).toMatchObject({
      kind: 'idle',
      detail: 'No contents scope is active.'
    })
  })
})

function directoryBinding(directoryId = '11'): RowBinding {
  return {
    kind: 'directory',
    sourceId: '7',
    directoryId,
    entryPoint: {
      kind: 'source',
      sourceId: '7'
    },
    label: 'Album'
  }
}

function readStateBinding(): RowBinding {
  return {
    kind: 'readState',
    state: 'notLoaded',
    ownerId: 'navigation-row:7',
    detail: 'Contents not loaded yet.'
  }
}

function loadingThresholdPlusMargin(): number {
  return 126
}

async function flushPromises(): Promise<void> {
  for (let index = 0; index < 5; index++) {
    await Promise.resolve()
  }
}

type DeferredContentsApi = LibraryContentsApi & {
  readonly requests: ContentsReadRequest[]
  readonly resolveNext: (result: ContentsReadResult) => void
  readonly resolveAt: (index: number, result: ContentsReadResult) => void
  readonly rejectNext: (error: unknown) => void
}

function deferredContentsApi(): DeferredContentsApi {
  const requests: ContentsReadRequest[] = []
  const pendingReads: Deferred<ContentsReadResult>[] = []

  return {
    requests,
    async read(request): Promise<ContentsReadResult> {
      requests.push(request)
      const pending = deferred<ContentsReadResult>()
      pendingReads.push(pending)
      return pending.promise
    },
    resolveNext(result): void {
      pendingReads.shift()?.resolve(result)
    },
    resolveAt(index, result): void {
      pendingReads[index]?.resolve(result)
    },
    rejectNext(error): void {
      pendingReads.shift()?.reject(error)
    }
  }
}

function requestAt(contentsApi: DeferredContentsApi, index: number): ContentsReadRequest {
  const request = contentsApi.requests[index]
  expect(request).toBeDefined()

  if (request === undefined) {
    throw new Error(`Expected contents request at index ${index}.`)
  }

  return request
}

type Deferred<T> = {
  readonly promise: Promise<T>
  readonly resolve: (value: T) => void
  readonly reject: (error: unknown) => void
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve
    reject = promiseReject
  })

  return {
    promise,
    resolve,
    reject
  }
}

function readyContents(
  request: ContentsReadRequest,
  rows: readonly ContentsFileRow[],
  state: 'ready' | 'empty' = 'ready',
  nextCursor?: string
): ContentsReadResult {
  return {
    state: 'ready',
    result: {
      state,
      scope: request.scope,
      policy: request.policy,
      recursion: request.recursion,
      rows,
      coverage: {
        state: 'complete',
        recursiveScopeComplete: true,
        emptyResultAuthoritative: rows.length === 0
      },
      ...(nextCursor === undefined ? {} : { nextCursor })
    }
  }
}

function contentsRow(id: string, label: string): ContentsFileRow {
  return {
    id,
    sourceId: '7',
    sourceFileId: `file-${id}`,
    label,
    relativePath: label,
    fileName: label,
    mediaClass: 'audio',
    fileKind: 'audio',
    presence: 'present',
    updatedAtMs: 100
  }
}

function visibleLabels(state: ContentsBoundaryState): readonly string[] {
  expect(state.kind).toBe('ready')

  if (state.kind !== 'ready' || state.result.state !== 'ready') {
    return []
  }

  return (state.accumulatedRows ?? state.result.result.rows).map((row) => row.label)
}
