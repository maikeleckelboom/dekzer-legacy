import { afterEach, describe, expect, it, vi } from 'vitest'

import {
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

  it('requests recursive audio-only source-file contents for selected directories', async () => {
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
        mediaClasses: ['audio'],
        rowProfile: { kind: 'sourceFile' }
      },
      recursion: 'recursive',
      limit: 100
    })
    expect(capturedRequest?.policy.mediaClasses).not.toContain('image')
    expect(capturedRequest?.policy.mediaClasses).not.toContain('video')
    expect(capturedRequest?.policy.mediaClasses).not.toContain('unsupported')
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
        requestKey: 'directory:7:11:sourceFile:audio:recursive',
        sequence: 1
      }
    })

    await vi.advanceTimersByTimeAsync(124)
    expect(controller.state.value.kind).toBe('idle')

    await vi.advanceTimersByTimeAsync(1)
    expect(controller.state.value).toMatchObject({
      kind: 'loading',
      requestKey: 'directory:7:11:sourceFile:audio:recursive',
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
        requestKey: 'directory:7:11:sourceFile:audio:recursive',
        sequence: 2
      }
    })
    expect(visibleLabels(controller.state.value)).toEqual(['A.wav'])

    contentsApi.resolveNext(readyContents(requestAt(contentsApi, 1), [contentsRow('b', 'B.wav')]))
    await refresh

    expect(visibleLabels(controller.state.value)).toEqual(['B.wav'])
    expect(controller.state.value).not.toHaveProperty('pending')
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
        requestKey: 'directory:7:12:sourceFile:audio:recursive',
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
