import { describe, expect, it } from 'vitest'

import {
  createSearchFilterReadController,
  createSearchQueryIdFromIdentity,
  createSearchQueryId,
  type SearchQueryState
} from '../../../../src/renderer/library/runtime/searchFilterState'
import type {
  SearchFilterQueryIdentity,
  SearchFilterReadRequest,
  SearchFilterReadResult,
  SearchFilterResult,
  SearchFilterResultRow
} from '../../../../src/shared/library/searchFilter/read'

describe('createSearchFilterReadController', () => {
  it('1. submit moves state to Pending', () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    void controller.submit(request())

    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      queryId: createSearchQueryId(request()),
      requestToken: 1
    })
  })

  it('2. accepted first response requires token and backend echoed identity', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const submitted = controller.submit(request())
    api.resolveNext(readyResult(requestAt(api, 0), [row('source-file:1', 'Amen.wav')]))

    await expect(submitted).resolves.toBe(true)
    expect(retainedRows(controller.state.value).map((resultRow) => resultRow.displayLabel)).toEqual(
      ['Amen.wav']
    )
  })

  it('3. response with mismatched backend echoed identity is discarded', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const submitted = controller.submit(request({ textQuery: 'amen' }))
    api.resolveNext(
      readyResult(request({ textQuery: 'break' }), [row('source-file:1', 'Break.wav')])
    )

    await expect(submitted).resolves.toBe(false)
    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      requestToken: 1
    })
  })

  it('accepts backend default targetKinds echo for an empty request targetKinds', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({ targetKinds: [] })
    const backendIdentity = echoIdentity(readRequest, {
      targetKinds: ['directory', 'source', 'sourceFile', 'sourceLocation']
    })

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Amen.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(true)
    expect(retainedRows(controller.state.value).map((resultRow) => resultRow.displayLabel)).toEqual(
      ['Amen.wav']
    )
  })

  it('accepts backend normalized textQuery echo for whitespace and uppercase input', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({ textQuery: '  AMEN  ' })
    const backendIdentity = echoIdentity(readRequest, { textQuery: 'amen' })

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Amen.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(true)
    expect(retainedRows(controller.state.value)).toHaveLength(1)
  })

  it('accepts backend sorted and deduped targetKinds echo', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({
      targetKinds: ['sourceFile', 'source', 'sourceFile', 'directory']
    })
    const backendIdentity = echoIdentity(readRequest, {
      targetKinds: ['directory', 'source', 'sourceFile']
    })

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Amen.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(true)
    expect(retainedRows(controller.state.value)).toHaveLength(1)
  })

  it('accepts backend sorted and deduped filter array echo', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({
      filters: {
        fileClasses: ['video', 'audio', 'audio'],
        fileKinds: ['video', 'archive', 'archive'],
        mediaRelevance: ['playableMedia', 'audioWorkflow', 'playableMedia'],
        presenceStates: ['removed', 'present', 'present'],
        sourceAccessStates: ['unknown', 'blocked', 'blocked'],
        blake3: 'hasCurrent',
        probe: 'missingCurrent',
        attachmentLinkStates: ['stale', 'current', 'current']
      }
    })
    const backendIdentity = echoIdentity(readRequest, {
      filters: {
        fileClasses: ['audio', 'video'],
        fileKinds: ['archive', 'video'],
        mediaRelevance: ['audioWorkflow', 'playableMedia'],
        presenceStates: ['present', 'removed'],
        sourceAccessStates: ['blocked', 'unknown'],
        blake3: 'hasCurrent',
        probe: 'missingCurrent',
        attachmentLinkStates: ['current', 'stale']
      }
    })

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Amen.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(true)
    expect(retainedRows(controller.state.value)).toHaveLength(1)
  })

  it('still rejects a genuinely different backend echoed identity', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({ textQuery: 'amen' })
    const backendIdentity = echoIdentity(readRequest, { textQuery: 'break' })

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Break.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(false)
    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      requestToken: 1
    })
  })

  it('uses the backend echo-derived queryId for Retained and preserves it while Accumulating', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)
    const readRequest = request({ textQuery: '  AMEN  ', targetKinds: [] })
    const backendIdentity = echoIdentity(readRequest, {
      textQuery: 'amen',
      targetKinds: ['directory', 'source', 'sourceFile', 'sourceLocation']
    })
    const expectedQueryId = createSearchQueryIdFromIdentity(backendIdentity)

    controller.start()
    const submitted = controller.submit(readRequest)
    api.resolveNext(
      readyResult(
        requestAt(api, 0),
        [row('source-file:1', 'Amen.wav')],
        'c1',
        'ready',
        '1',
        backendIdentity
      )
    )

    await expect(submitted).resolves.toBe(true)
    expect(controller.state.value).toMatchObject({
      kind: 'Retained',
      queryId: expectedQueryId
    })

    const loaded = controller.loadNext()
    expect(controller.state.value).toMatchObject({
      kind: 'Accumulating',
      queryId: expectedQueryId
    })
    api.resolveNext(
      readyResult(
        requestAt(api, 1),
        [row('source-file:2', 'Amen 2.wav')],
        undefined,
        'ready',
        '1',
        backendIdentity
      )
    )
    await expect(loaded).resolves.toBe(true)
    expect(controller.state.value).toMatchObject({
      kind: 'Retained',
      queryId: expectedQueryId
    })
  })

  it('4. response with mismatched request token is discarded', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const stale = controller.submit(request({ textQuery: 'amen' }))
    const current = controller.submit(request({ textQuery: 'break' }))

    api.resolveAt(1, readyResult(requestAt(api, 1), [row('source-file:2', 'Break.wav')]))
    await expect(current).resolves.toBe(true)

    api.resolveAt(0, readyResult(requestAt(api, 0), [row('source-file:1', 'Amen.wav')]))
    await expect(stale).resolves.toBe(false)
    expect(retainedRows(controller.state.value).map((resultRow) => resultRow.displayLabel)).toEqual(
      ['Break.wav']
    )
  })

  it('5. new submit while Pending discards the old response by token', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const oldSubmit = controller.submit(request({ textQuery: 'old' }))
    const newSubmit = controller.submit(request({ textQuery: 'new' }))

    api.resolveAt(0, readyResult(requestAt(api, 0), [row('source-file:1', 'Old.wav')]))
    await expect(oldSubmit).resolves.toBe(false)
    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      requestToken: 2
    })

    api.resolveAt(1, readyResult(requestAt(api, 1), [row('source-file:2', 'New.wav')]))
    await expect(newSubmit).resolves.toBe(true)
    expect(retainedRows(controller.state.value).map((resultRow) => resultRow.displayLabel)).toEqual(
      ['New.wav']
    )
  })

  it('6. loadNext from Retained with nextCursor moves to Accumulating', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')], 'c1')

    const loaded = controller.loadNext()

    expect(controller.state.value).toMatchObject({
      kind: 'Accumulating',
      accumulated: [{ displayLabel: 'One.wav' }],
      nextCursor: 'c1',
      requestToken: 2
    })
    api.resolveNext(readyResult(requestAt(api, 1), [row('source-file:2', 'Two.wav')]))
    await loaded
  })

  it('7. loadNext from Retained with no cursor is a no-op', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')])

    await expect(controller.loadNext()).resolves.toBe(false)
    expect(api.requests).toHaveLength(1)
    expect(controller.state.value.kind).toBe('Retained')
  })

  it('8. loadNext from Accumulating is a no-op', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')], 'c1')

    const loading = controller.loadNext()
    await expect(controller.loadNext()).resolves.toBe(false)

    expect(api.requests).toHaveLength(2)
    api.resolveNext(readyResult(requestAt(api, 1), [row('source-file:2', 'Two.wav')]))
    await loading
  })

  it('9. accepted next-page response appends rows and returns to Retained', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')], 'c1')

    const loaded = controller.loadNext()
    api.resolveNext(readyResult(requestAt(api, 1), [row('source-file:2', 'Two.wav')]))

    await expect(loaded).resolves.toBe(true)
    expect(controller.state.value).toMatchObject({
      kind: 'Retained',
      rows: [{ displayLabel: 'One.wav' }, { displayLabel: 'Two.wav' }]
    })
  })

  it('10. next-page response with mismatched backend echoed identity is discarded', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')], 'c1')

    const loaded = controller.loadNext()
    api.resolveNext(readyResult(request({ textQuery: 'other' }), [row('source-file:2', 'Two.wav')]))

    await expect(loaded).resolves.toBe(false)
    expect(controller.state.value).toMatchObject({
      kind: 'Accumulating',
      accumulated: [{ displayLabel: 'One.wav' }],
      nextCursor: 'c1',
      requestToken: 2
    })
    expect(api.requests).toHaveLength(2)
  })

  it('11. generation mismatch mid-accumulation discards accumulated rows and clears cursor', async () => {
    const { controller, api } = await retainedController(
      [row('source-file:1', 'One.wav')],
      'c1',
      '1'
    )

    const loaded = controller.loadNext()
    api.resolveNext(
      readyResult(requestAt(api, 1), [row('source-file:2', 'Two.wav')], undefined, 'ready', '2')
    )

    await expect(loaded).resolves.toBe(true)
    expect(controller.state.value).toEqual({
      kind: 'Pending',
      queryId: createSearchQueryId(request()),
      requestToken: 3
    })
    expect(api.requests).toHaveLength(3)
    expect(api.requests[2]).not.toHaveProperty('cursor')
    expect(controller.viewModels.value).toEqual([])
  })

  it('12. CursorInvalid from Pending clears cursor and reissues first page', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const submitted = controller.submit({ ...request(), cursor: 'bad-cursor' })
    api.resolveNext(cursorInvalidResult(requestAt(api, 0)))

    await expect(submitted).resolves.toBe(true)
    expect(api.requests).toHaveLength(2)
    expect(api.requests[1]).not.toHaveProperty('cursor')
    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      requestToken: 2
    })
  })

  it('13. invalidationSignal clears cursor and rereads the same logical query', async () => {
    const { controller, api } = await retainedController([row('source-file:1', 'One.wav')], 'c1')

    const refreshed = controller.invalidationSignal()

    expect(api.requests).toHaveLength(2)
    expect(api.requests[1]).toEqual(request())
    expect(controller.state.value).toMatchObject({
      kind: 'Pending',
      queryId: createSearchQueryId(request()),
      priorRetained: {
        rows: [{ displayLabel: 'One.wav' }]
      }
    })
    api.resolveNext(readyResult(requestAt(api, 1), [row('source-file:2', 'Fresh.wav')]))
    await refreshed
  })

  it('13b. invalidationSignal is a successful no-op without an active query', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()

    await expect(controller.invalidationSignal()).resolves.toBe(true)
    expect(api.requests).toHaveLength(0)
    expect(controller.state.value).toEqual({ kind: 'Idle' })
  })

  it('14. partial response does not prove the result set empty, current, or complete', async () => {
    const api = deferredSearchApi()
    const controller = createSearchFilterReadController(api)

    controller.start()
    const submitted = controller.submit(request())
    api.resolveNext(readyResult(requestAt(api, 0), [], undefined, 'partial'))

    await expect(submitted).resolves.toBe(true)
    expect(controller.state.value).toMatchObject({
      kind: 'Retained',
      rows: [],
      resultState: 'partial',
      nextCursor: null
    })
  })
})

async function retainedController(
  rows: readonly SearchFilterResultRow[],
  nextCursor?: string,
  indexGeneration = '1'
): Promise<{
  readonly controller: ReturnType<typeof createSearchFilterReadController>
  readonly api: DeferredSearchApi
}> {
  const api = deferredSearchApi()
  const controller = createSearchFilterReadController(api)

  controller.start()
  const submitted = controller.submit(request())
  api.resolveNext(readyResult(requestAt(api, 0), rows, nextCursor, 'ready', indexGeneration))
  await submitted

  return { controller, api }
}

function retainedRows(state: SearchQueryState): readonly SearchFilterResultRow[] {
  expect(state.kind).toBe('Retained')
  return state.kind === 'Retained' ? state.rows : []
}

type DeferredSearchApi = {
  readonly requests: SearchFilterReadRequest[]
  readonly read: (request: SearchFilterReadRequest) => Promise<SearchFilterReadResult>
  readonly resolveNext: (result: SearchFilterReadResult) => void
  readonly resolveAt: (index: number, result: SearchFilterReadResult) => void
}

function deferredSearchApi(): DeferredSearchApi {
  const requests: SearchFilterReadRequest[] = []
  const pendingReads: Deferred<SearchFilterReadResult>[] = []

  return {
    requests,
    async read(readRequest): Promise<SearchFilterReadResult> {
      requests.push(readRequest)
      const pending = deferred<SearchFilterReadResult>()
      pendingReads.push(pending)
      return pending.promise
    },
    resolveNext(result): void {
      pendingReads.shift()?.resolve(result)
    },
    resolveAt(index, result): void {
      pendingReads[index]?.resolve(result)
    }
  }
}

function requestAt(api: DeferredSearchApi, index: number): SearchFilterReadRequest {
  const readRequest = api.requests[index]
  expect(readRequest).toBeDefined()

  if (readRequest === undefined) {
    throw new Error(`Expected search/filter request at index ${index}.`)
  }

  return readRequest
}

function request(overrides: Partial<SearchFilterReadRequest> = {}): SearchFilterReadRequest {
  return {
    scope: { type: 'library' },
    recursion: 'recursive',
    textQuery: 'amen',
    targetKinds: ['sourceFile'],
    filters: {},
    sort: 'pathName',
    limit: 100,
    ...overrides
  }
}

function readyResult(
  readRequest: SearchFilterReadRequest,
  rows: readonly SearchFilterResultRow[],
  nextCursor?: string,
  state: SearchFilterResult['state'] = 'ready',
  indexGeneration = '1',
  queryIdentity: SearchFilterQueryIdentity = echoIdentity(readRequest, {}, indexGeneration)
): SearchFilterReadResult {
  return {
    state: 'ready',
    reply: {
      result: {
        state,
        queryIdentity,
        indexGeneration,
        indexState: 'ready',
        rows: [...rows],
        ...(nextCursor === undefined ? {} : { nextCursor })
      }
    }
  }
}

function echoIdentity(
  readRequest: SearchFilterReadRequest,
  overrides: Partial<SearchFilterQueryIdentity> = {},
  indexGeneration = '1'
): SearchFilterQueryIdentity {
  return {
    scope: readRequest.scope,
    recursion: readRequest.recursion,
    ...(readRequest.textQuery === undefined ? {} : { textQuery: readRequest.textQuery }),
    targetKinds: readRequest.targetKinds ?? [],
    filters: readRequest.filters,
    sort: readRequest.sort,
    pageSize: readRequest.limit ?? 100,
    indexGeneration,
    ...overrides
  }
}

function cursorInvalidResult(readRequest: SearchFilterReadRequest): SearchFilterReadResult {
  return readyResult(readRequest, [], undefined, 'cursorInvalid')
}

function row(stableKey: string, displayLabel: string): SearchFilterResultRow {
  return {
    resultKind: 'sourceFile',
    authorityLayer: 'sourceFileInventory',
    stableKey,
    sourceId: '7',
    sourceLocationId: null,
    sourceDirectoryId: '11',
    parentSourceDirectoryId: null,
    sourceFileId: stableKey,
    displayLabel,
    displayPath: `Music/${displayLabel}`,
    relativePath: displayLabel,
    fileClass: 'audio',
    fileKind: 'audio',
    mediaRelevance: 'audioWorkflow',
    presenceState: 'present',
    sourceAccessState: 'accessible',
    sourceScanPhase: 'complete',
    hasCurrentBlake3: true,
    hasCurrentProbe: false,
    attachmentLinkState: 'notApplicable',
    attachmentId: null,
    evidenceCoverageState: 'indexed',
    matchReason: 'label',
    updatedAtMs: 100
  }
}

type Deferred<T> = {
  readonly promise: Promise<T>
  readonly resolve: (value: T) => void
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((promiseResolve) => {
    resolve = promiseResolve
  })

  return {
    promise,
    resolve
  }
}
