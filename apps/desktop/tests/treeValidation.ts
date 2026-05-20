import { strict as assert } from 'node:assert'

import { libraryHierarchyFixtureTree } from '../src/renderer/src/libraryBrowser/libraryHierarchyFixture'
import {
  resolveTreeKeyboardIntent,
  type TreeKeyboardIntent
} from '../src/renderer/src/libraryBrowser/tree/keys'
import {
  flattenVisibleTree,
  getFirstChildVisibleNodeId,
  getFirstVisibleNodeId,
  getLastVisibleNodeId,
  getNextVisibleNodeId,
  getParentVisibleNodeId,
  getPreviousVisibleNodeId
} from '../src/renderer/src/libraryBrowser/tree/projection'
import type {
  TreeNode,
  TreeNodeId,
  TreeVisibleItem
} from '../src/renderer/src/libraryBrowser/tree/types'

const expandedFixtureIds = new Set<TreeNodeId>([
  'fixture-root',
  'fixture-tracks',
  'fixture-playlists'
])
const rootOnlyExpandedIds = new Set<TreeNodeId>(['fixture-root'])
const collapsedFixtureIds = new Set<TreeNodeId>()

void main()

function main(): void {
  validatesVisibleProjection()
  validatesVisibleHelpers()
  validatesKeyboardNavigation()
  validatesKeyboardExpansion()
  validatesKeyboardSelection()
  validatesFocusAndSelectionSeparation()
}

function validatesVisibleProjection(): void {
  const expandedItems = fixtureVisibleItems(expandedFixtureIds, {
    selectedNodeId: 'fixture-playlist-group',
    activeNodeId: 'fixture-albums'
  })

  assert.deepEqual(
    expandedItems.map((item) => item.id),
    [
      'fixture-root',
      'fixture-artists',
      'fixture-albums',
      'fixture-tracks',
      'fixture-tracks-group',
      'fixture-playlists',
      'fixture-playlist-group',
      'fixture-preparation',
      'fixture-history'
    ]
  )
  assert.equal(getItem(expandedItems, 'fixture-root').level, 1)
  assert.equal(getItem(expandedItems, 'fixture-artists').level, 2)
  assert.equal(getItem(expandedItems, 'fixture-tracks-group').level, 3)
  assert.equal(getItem(expandedItems, 'fixture-root').ariaPosInSet, 1)
  assert.equal(getItem(expandedItems, 'fixture-root').ariaSetSize, 1)
  assert.equal(getItem(expandedItems, 'fixture-artists').ariaPosInSet, 1)
  assert.equal(getItem(expandedItems, 'fixture-artists').ariaSetSize, 6)
  assert.equal(getItem(expandedItems, 'fixture-albums').ariaPosInSet, 2)
  assert.equal(getItem(expandedItems, 'fixture-tracks').ariaPosInSet, 3)
  assert.equal(getItem(expandedItems, 'fixture-playlist-group').ariaPosInSet, 1)
  assert.equal(getItem(expandedItems, 'fixture-playlist-group').ariaSetSize, 1)

  const rootOnlyItems = fixtureVisibleItems(rootOnlyExpandedIds)
  assert.deepEqual(
    rootOnlyItems.map((item) => item.id),
    [
      'fixture-root',
      'fixture-artists',
      'fixture-albums',
      'fixture-tracks',
      'fixture-playlists',
      'fixture-preparation',
      'fixture-history'
    ]
  )

  const collapsedItems = fixtureVisibleItems(collapsedFixtureIds)
  assert.deepEqual(
    collapsedItems.map((item) => item.id),
    ['fixture-root']
  )
}

function validatesVisibleHelpers(): void {
  const items = fixtureVisibleItems(expandedFixtureIds)

  assert.equal(getFirstVisibleNodeId(items), 'fixture-root')
  assert.equal(getLastVisibleNodeId(items), 'fixture-history')
  assert.equal(getNextVisibleNodeId(items, 'fixture-root'), 'fixture-artists')
  assert.equal(getNextVisibleNodeId(items, 'fixture-history'), null)
  assert.equal(getPreviousVisibleNodeId(items, 'fixture-artists'), 'fixture-root')
  assert.equal(getPreviousVisibleNodeId(items, 'fixture-root'), null)
  assert.equal(getParentVisibleNodeId(items, 'fixture-tracks-group'), 'fixture-tracks')
  assert.equal(getParentVisibleNodeId(items, 'fixture-root'), null)
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-root'), 'fixture-artists')
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-tracks'), 'fixture-tracks-group')
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-artists'), null)
}

function validatesKeyboardNavigation(): void {
  const items = fixtureVisibleItems(expandedFixtureIds)

  assertIntent(resolveIntent(items, 'fixture-artists', 'ArrowUp'), {
    kind: 'focus',
    nodeId: 'fixture-root'
  })
  assertIntent(resolveIntent(items, 'fixture-root', 'ArrowDown'), {
    kind: 'focus',
    nodeId: 'fixture-artists'
  })
  assertIntent(resolveIntent(items, 'fixture-history', 'Home'), {
    kind: 'focus',
    nodeId: 'fixture-root'
  })
  assertIntent(resolveIntent(items, 'fixture-root', 'End'), {
    kind: 'focus',
    nodeId: 'fixture-history'
  })
}

function validatesKeyboardExpansion(): void {
  const expandedItems = fixtureVisibleItems(expandedFixtureIds)
  const rootOnlyItems = fixtureVisibleItems(rootOnlyExpandedIds)
  const collapsedRootItems = fixtureVisibleItems(collapsedFixtureIds)
  const leafRootItems = flattenVisibleTree({
    nodes: [leafRootNode()],
    expandedNodeIds: new Set(),
    selectedNodeId: null
  })

  assertIntent(resolveIntent(rootOnlyItems, 'fixture-tracks', 'ArrowRight'), {
    kind: 'expand',
    nodeId: 'fixture-tracks'
  })
  assertIntent(resolveIntent(expandedItems, 'fixture-tracks', 'ArrowRight'), {
    kind: 'focus',
    nodeId: 'fixture-tracks-group'
  })
  assertHandledNoop(resolveIntent(expandedItems, 'fixture-artists', 'ArrowRight'))
  assertIntent(resolveIntent(expandedItems, 'fixture-tracks', 'ArrowLeft'), {
    kind: 'collapse',
    nodeId: 'fixture-tracks'
  })
  assertIntent(resolveIntent(expandedItems, 'fixture-tracks-group', 'ArrowLeft'), {
    kind: 'focus',
    nodeId: 'fixture-tracks'
  })
  assertIntent(resolveIntent(rootOnlyItems, 'fixture-tracks', 'ArrowLeft'), {
    kind: 'focus',
    nodeId: 'fixture-root'
  })
  assertHandledNoop(resolveIntent(collapsedRootItems, 'fixture-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(leafRootItems, 'leaf-root', 'ArrowLeft'))
  assertUnhandled(resolveIntent(expandedItems, 'fixture-root', 'Escape'))
}

function validatesKeyboardSelection(): void {
  const items = fixtureVisibleItems(expandedFixtureIds)

  assertIntent(resolveIntent(items, 'fixture-albums', 'Enter'), {
    kind: 'select',
    nodeId: 'fixture-albums'
  })
  assertIntent(resolveIntent(items, 'fixture-albums', ' '), {
    kind: 'select',
    nodeId: 'fixture-albums'
  })
}

function validatesFocusAndSelectionSeparation(): void {
  const items = fixtureVisibleItems(expandedFixtureIds, {
    selectedNodeId: 'fixture-tracks',
    activeNodeId: 'fixture-albums'
  })

  assert.equal(getItem(items, 'fixture-tracks').isSelected, true)
  assert.equal(getItem(items, 'fixture-tracks').isActive, false)
  assert.equal(getItem(items, 'fixture-albums').isSelected, false)
  assert.equal(getItem(items, 'fixture-albums').isActive, true)
}

function fixtureVisibleItems(
  expandedNodeIds: ReadonlySet<TreeNodeId>,
  options: {
    readonly selectedNodeId?: TreeNodeId | null
    readonly activeNodeId?: TreeNodeId | null
  } = {}
): readonly TreeVisibleItem[] {
  return flattenVisibleTree({
    nodes: libraryHierarchyFixtureTree.nodes,
    expandedNodeIds,
    selectedNodeId: options.selectedNodeId ?? null,
    activeNodeId: options.activeNodeId ?? null
  })
}

function resolveIntent(
  visibleItems: readonly TreeVisibleItem[],
  activeNodeId: TreeNodeId,
  key: string
): TreeKeyboardIntent {
  return resolveTreeKeyboardIntent({
    key,
    activeNodeId,
    visibleItems
  })
}

function assertIntent(
  intent: TreeKeyboardIntent,
  expected: {
    readonly kind: Exclude<TreeKeyboardIntent['kind'], 'none'>
    readonly nodeId: TreeNodeId
  }
): void {
  assert.equal(intent.kind, expected.kind)
  assert.equal(intent.shouldPreventDefault, true)

  if (intent.kind === 'none') {
    assert.fail('expected a state-changing keyboard intent')
  }

  assert.equal(intent.nodeId, expected.nodeId)
}

function assertHandledNoop(intent: TreeKeyboardIntent): void {
  assert.equal(intent.kind, 'none')
  assert.equal(intent.shouldPreventDefault, true)
}

function assertUnhandled(intent: TreeKeyboardIntent): void {
  assert.equal(intent.kind, 'none')
  assert.equal(intent.shouldPreventDefault, false)
}

function getItem(visibleItems: readonly TreeVisibleItem[], nodeId: TreeNodeId): TreeVisibleItem {
  const item = visibleItems.find((candidate) => candidate.id === nodeId)

  if (item === undefined) {
    throw new Error(`Expected visible tree item ${nodeId}.`)
  }

  return item
}

function leafRootNode(): TreeNode {
  return {
    id: 'leaf-root',
    label: 'Leaf root',
    kind: 'folder'
  }
}
