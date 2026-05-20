import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'

import { libraryHierarchyFixtureTree } from '../src/renderer/src/libraryBrowser/libraryHierarchyFixture'
import { projectLibraryHierarchyReadToBrowserTree } from '../src/renderer/src/libraryBrowser/libraryHierarchyProjection'
import {
  getTreeItemAriaExpanded,
  getTreeItemAriaSelected
} from '../src/renderer/src/libraryBrowser/tree/aria'
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
  BrowserTreeNode,
  BrowserTreeNodeId,
  BrowserTreeVisibleItem
} from '../src/renderer/src/libraryBrowser/tree/types'
import type { LibraryHierarchyReadResult } from '../src/shared/libraryHierarchyRead'

const expandedFixtureIds = new Set<BrowserTreeNodeId>([
  'fixture-root',
  'fixture-tracks',
  'fixture-playlists'
])
const rootOnlyExpandedIds = new Set<BrowserTreeNodeId>(['fixture-root'])
const collapsedFixtureIds = new Set<BrowserTreeNodeId>()

void main()

function main(): void {
  validatesVisibleProjection()
  validatesVisibleHelpers()
  validatesKeyboardNavigation()
  validatesKeyboardExpansion()
  validatesKeyboardSelection()
  validatesFocusAndSelectionSeparation()
  validatesAriaAttributes()
  validatesLibraryHierarchyReadProjection()
  validatesFixtureFallbackRemainsExplicit()
  validatesRootQualityGateIncludesTreeValidation()
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
  assert.equal(getNextVisibleNodeId(items, 'fixture-history'), undefined)
  assert.equal(getPreviousVisibleNodeId(items, 'fixture-artists'), 'fixture-root')
  assert.equal(getPreviousVisibleNodeId(items, 'fixture-root'), undefined)
  assert.equal(getParentVisibleNodeId(items, 'fixture-tracks-group'), 'fixture-tracks')
  assert.equal(getParentVisibleNodeId(items, 'fixture-root'), undefined)
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-root'), 'fixture-artists')
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-tracks'), 'fixture-tracks-group')
  assert.equal(getFirstChildVisibleNodeId(items, 'fixture-artists'), undefined)
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
    expandedNodeIds: new Set()
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

function validatesAriaAttributes(): void {
  const selectedItems = fixtureVisibleItems(expandedFixtureIds, {
    selectedNodeId: 'fixture-tracks',
    activeNodeId: 'fixture-tracks'
  })
  const expandedBranch = getItem(selectedItems, 'fixture-tracks')
  const expandedRoot = getItem(selectedItems, 'fixture-root')
  const leaf = getItem(selectedItems, 'fixture-artists')

  assert.equal(getTreeItemAriaExpanded(expandedBranch), 'true')
  assert.equal(getTreeItemAriaExpanded(expandedRoot), 'true')
  assert.equal(getTreeItemAriaExpanded(leaf), undefined)
  assert.equal(getTreeItemAriaSelected(expandedBranch), 'true')
  assert.equal(getTreeItemAriaSelected(leaf), 'false')

  const rootOnlyItems = fixtureVisibleItems(rootOnlyExpandedIds)
  assert.equal(getTreeItemAriaExpanded(getItem(rootOnlyItems, 'fixture-tracks')), 'false')
}

function validatesLibraryHierarchyReadProjection(): void {
  const projected = projectLibraryHierarchyReadToBrowserTree(fileOnlyHierarchyReadResult())

  assert.equal(projected.kind, 'tree')
  if (projected.kind !== 'tree') {
    assert.fail('expected file-only hierarchy read result to project to browser tree')
  }

  assert.deepEqual(projected.nodes, [
    {
      id: 'source:7',
      label: 'Source Fixture',
      kind: 'source',
      detail: '1 literal hierarchy row loaded read-only.',
      children: [
        {
          id: 'source-file:11',
          label: 'track.wav',
          kind: 'file',
          detail: 'Present file.'
        }
      ]
    }
  ])

  const directoryProjection = projectLibraryHierarchyReadToBrowserTree(
    directoryHierarchyReadResult()
  )
  assert.equal(directoryProjection.kind, 'unsupported')
  if (directoryProjection.kind !== 'unsupported') {
    assert.fail('expected directory rows to stay out of the current browser tree model')
  }
  assert.match(directoryProjection.message, /unloaded child state/)

  const partialProjection = projectLibraryHierarchyReadToBrowserTree({
    ...fileOnlyHierarchyReadResult(),
    window: {
      ...fileOnlyHierarchyReadResult().window,
      totalRows: 2
    }
  })
  assert.equal(partialProjection.kind, 'unsupported')
}

function validatesFixtureFallbackRemainsExplicit(): void {
  assert.equal(libraryHierarchyFixtureTree.name, 'Library hierarchy fixture')
  assert.match(libraryHierarchyFixtureTree.detail, /Renderer-only demo input/)
  assert.equal(libraryHierarchyFixtureTree.nodes[0]?.kind, 'fixtureRoot')
}

function validatesRootQualityGateIncludesTreeValidation(): void {
  const rootPackage = JSON.parse(
    readFileSync(new URL('../../../package.json', import.meta.url), 'utf8')
  ) as {
    readonly scripts?: Record<string, string>
  }
  const scripts = rootPackage.scripts ?? {}

  assert.equal(scripts['desktop:validate:tree'], 'pnpm --filter @dekzer/desktop run validate:tree')
  assert.match(scripts.check ?? '', /pnpm run desktop:validate:tree/)
}

function fixtureVisibleItems(
  expandedNodeIds: ReadonlySet<BrowserTreeNodeId>,
  options: {
    readonly selectedNodeId?: BrowserTreeNodeId
    readonly activeNodeId?: BrowserTreeNodeId
  } = {}
): readonly BrowserTreeVisibleItem[] {
  return flattenVisibleTree({
    nodes: libraryHierarchyFixtureTree.nodes,
    expandedNodeIds,
    ...(options.selectedNodeId === undefined ? {} : { selectedNodeId: options.selectedNodeId }),
    ...(options.activeNodeId === undefined ? {} : { activeNodeId: options.activeNodeId })
  })
}

function resolveIntent(
  visibleItems: readonly BrowserTreeVisibleItem[],
  activeNodeId: BrowserTreeNodeId,
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
    readonly nodeId: BrowserTreeNodeId
  }
): void {
  if (intent.kind === 'none') {
    assert.fail('expected a state-changing keyboard intent')
  }

  assert.equal(intent.kind, expected.kind)
  assert.equal(intent.shouldPreventDefault, true)
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

function getItem(
  visibleItems: readonly BrowserTreeVisibleItem[],
  nodeId: BrowserTreeNodeId
): BrowserTreeVisibleItem {
  const item = visibleItems.find((candidate) => candidate.id === nodeId)

  if (item === undefined) {
    throw new Error(`Expected visible tree item ${nodeId}.`)
  }

  return item
}

function leafRootNode(): BrowserTreeNode {
  return {
    id: 'leaf-root',
    label: 'Leaf root',
    kind: 'folder'
  }
}

function fileOnlyHierarchyReadResult(): Extract<LibraryHierarchyReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      offset: 0,
      limit: 50,
      totalRows: 1,
      nodes: [
        {
          id: 'source-file:11',
          kind: 'file',
          label: 'track.wav',
          sourceFileId: '11',
          presenceState: 'present',
          updatedAtMs: 100
        }
      ]
    }
  }
}

function directoryHierarchyReadResult(): Extract<LibraryHierarchyReadResult, { state: 'ready' }> {
  return {
    state: 'ready',
    window: {
      root: {
        id: 'source:7',
        label: 'Source Fixture',
        entryPoint: {
          kind: 'source',
          sourceId: '7'
        }
      },
      offset: 0,
      limit: 50,
      totalRows: 1,
      nodes: [
        {
          id: 'source-directory:12',
          kind: 'directory',
          label: 'Album',
          sourceDirectoryId: '12',
          presenceState: 'present',
          updatedAtMs: 100
        }
      ]
    }
  }
}
