import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  isValidViewState,
  readViewStateFromHost,
  writeViewStateToHost
} from '../src/main/libraryBrowser/viewState'
import type { LibraryBoundaryHost } from '../src/main/libraryBoundary/host'
import type { LibraryBoundaryHostConfig } from '../src/main/libraryBoundary/config'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-view-state-validation-'))

void main().finally(() => {
  rmSync(tempRoot, { recursive: true, force: true })
})

async function main(): Promise<void> {
  await validatesMissingFileReturnsEmpty()
  await validatesMalformedPayloadIgnored()
  await validatesWriteStoresOnlyVersionAndIds()
  validatesUnknownVersionIgnored()
  validatesNonStringIdsRejected()
  validatesDeduplicatedExpandedIds()
  await validatesWriteFailureReturnsFailed()
}

function fakeHost(userDataPath: string): LibraryBoundaryHost {
  return {
    config: {
      storageEnvironment: {
        userDataPath
      }
    } as LibraryBoundaryHostConfig
  } as LibraryBoundaryHost
}

async function validatesMissingFileReturnsEmpty(): Promise<void> {
  const dir = join(tempRoot, 'missing-file')
  const result = await readViewStateFromHost(fakeHost(dir))

  assert.deepEqual(result, { state: 'empty' })
}

async function validatesMalformedPayloadIgnored(): Promise<void> {
  const dir = join(tempRoot, 'malformed')
  const writeOk = await writeViewStateToHost(fakeHost(dir), {
    version: 1,
    selectedNodeId: 'valid-node',
    expandedNodeIds: ['valid-node']
  })

  assert.equal(writeOk.state, 'written')

  const filePath = join(dir, 'library-browser-view-state.json')
  writeFileSync(filePath, '{ malformed }', 'utf-8')

  const result = await readViewStateFromHost(fakeHost(dir))
  assert.deepEqual(result, { state: 'empty' })
}

async function validatesWriteStoresOnlyVersionAndIds(): Promise<void> {
  const dir = join(tempRoot, 'write-stores')
  const writeResult = await writeViewStateToHost(fakeHost(dir), {
    version: 1,
    selectedNodeId: 'navigation-row:7',
    expandedNodeIds: ['navigation-row:7', 'source-directory:12']
  })

  assert.deepEqual(writeResult, { state: 'written' })

  const readResult = await readViewStateFromHost(fakeHost(dir))

  assert.equal(readResult.state, 'ready')
  if (readResult.state !== 'ready') {
    assert.fail('expected view state to be read back')
  }

  assert.deepEqual(readResult.viewState, {
    version: 1,
    selectedNodeId: 'navigation-row:7',
    expandedNodeIds: ['navigation-row:7', 'source-directory:12']
  })

  assert.deepEqual(Object.keys(readResult.viewState).sort(), [
    'expandedNodeIds',
    'selectedNodeId',
    'version'
  ])
}

function validatesUnknownVersionIgnored(): void {
  assert.equal(isValidViewState({ version: 1, expandedNodeIds: [] }), true)
  assert.equal(isValidViewState({ version: 2, expandedNodeIds: [] }), false)
  assert.equal(isValidViewState({ version: 0, expandedNodeIds: [] }), false)
  assert.equal(isValidViewState({ expandedNodeIds: [] }), false)
  assert.equal(isValidViewState({ version: '1', expandedNodeIds: [] }), false)
  assert.equal(isValidViewState({ version: 1 }), false)
}

function validatesNonStringIdsRejected(): void {
  assert.equal(
    isValidViewState({
      version: 1,
      selectedNodeId: 42,
      expandedNodeIds: []
    }),
    false
  )

  assert.equal(
    isValidViewState({
      version: 1,
      expandedNodeIds: ['valid', 42]
    }),
    false
  )

  assert.equal(
    isValidViewState({
      version: 1,
      expandedNodeIds: []
    }),
    true
  )

  assert.equal(
    isValidViewState({
      version: 1,
      selectedNodeId: 'valid-string-id',
      expandedNodeIds: []
    }),
    true
  )
}

function validatesDeduplicatedExpandedIds(): void {
  assert.equal(
    isValidViewState({
      version: 1,
      expandedNodeIds: ['a', 'b', 'a']
    }),
    true
  )
}

async function validatesWriteFailureReturnsFailed(): Promise<void> {
  const dir = join(tempRoot, 'write-failure')
  writeFileSync(dir, 'not-a-directory')

  const result = await writeViewStateToHost(fakeHost(dir), {
    version: 1,
    expandedNodeIds: []
  })

  assert.equal(result.state, 'failed')
  assert.equal(typeof result.detail, 'string')
}
