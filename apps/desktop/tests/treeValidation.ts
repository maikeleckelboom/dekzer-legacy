import { strict as assert } from 'node:assert'

import { projectState } from '../src/renderer/libraryBrowser/projection/tree'
import {
  getTreeItemAriaExpanded,
  getTreeItemAriaSelected
} from '../src/renderer/libraryBrowser/tree/aria'
import {
  resolveTreeKeyboardIntent,
  type TreeKeyboardIntent
} from '../src/renderer/libraryBrowser/tree/keys'
import {
  canActivateBrowserTreeAction,
  canRevealBrowserTreeChildren,
  flattenVisibleTree,
  getFirstChildVisibleNodeId,
  getFirstVisibleNodeId,
  getLastVisibleNodeId,
  getLoadedBrowserTreeChildren,
  getNextVisibleNodeId,
  getParentVisibleNodeId,
  getPreviousVisibleNodeId,
  isBrowserTreeActionLoading,
  isBrowserTreeBranch,
  isBrowserTreeLeaf
} from '../src/renderer/libraryBrowser/tree/projection'
import type { LoadedChildren } from '../src/renderer/libraryBrowser/runtime/state'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId,
  BrowserTreeRowRole,
  BrowserTreeVisibleItem
} from '../src/renderer/libraryBrowser/tree/types'
import type { ReadResult, ChildWindow } from '../src/shared/libraryHierarchy/readChildren'
import type { NavigationReadRowsResult } from '../src/shared/libraryNavigation/readRows'

const libraryHierarchyFixtureTree = {
  name: 'Tree validation fixture',
  detail: 'Test-owned fixture input for generic tree interactions.',
  nodes: [
    {
      id: 'fixture-root',
      role: 'source',
      label: 'Fixture Library Root',
      badge: { value: 'Root', tone: 'muted' },
      detail: 'Demo root for renderer tree behavior.',
      children: {
        kind: 'loaded',
        nodes: [
          {
            id: 'fixture-artists',
            role: 'collectionView',
            label: 'Artists',
            badge: { value: 'Leaf', tone: 'muted' },
            detail: 'Fixture grouping placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-albums',
            role: 'collectionView',
            label: 'Albums',
            badge: { value: 'Leaf', tone: 'muted' },
            detail: 'Fixture grouping placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-tracks',
            role: 'collectionView',
            label: 'Tracks',
            badge: { value: 'Branch', tone: 'muted' },
            detail: 'Fixture grouping placeholder.',
            children: {
              kind: 'loaded',
              nodes: [
                {
                  id: 'fixture-tracks-group',
                  role: 'literalDirectory',
                  label: 'Fixture Track Group',
                  badge: { value: 'Branch', tone: 'muted' },
                  detail: 'Demo child row for nested hierarchy rendering.',
                  children: { kind: 'none' }
                }
              ]
            }
          },
          {
            id: 'fixture-playlists',
            role: 'playlistSurface',
            label: 'Nested Branch',
            badge: { value: 'Branch', tone: 'muted' },
            detail: 'Fixture grouping placeholder.',
            children: {
              kind: 'loaded',
              nodes: [
                {
                  id: 'fixture-playlist-group',
                  role: 'literalDirectory',
                  label: 'Fixture Nested Group',
                  badge: { value: 'Branch', tone: 'muted' },
                  detail: 'Demo child row for nested hierarchy rendering.',
                  children: { kind: 'none' }
                }
              ]
            }
          },
          {
            id: 'fixture-preparation',
            role: 'preparationSurface',
            label: 'Preparation',
            badge: { value: 'Leaf', tone: 'muted' },
            detail: 'Fixture workflow placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-history',
            role: 'collectionView',
            label: 'History',
            badge: { value: 'Leaf', tone: 'muted' },
            detail: 'Fixture history placeholder.',
            children: { kind: 'none' }
          }
        ]
      }
    }
  ]
} satisfies {
  readonly name: string
  readonly detail: string
  readonly nodes: readonly BrowserTreeNode[]
}

const expandedFixtureIds = new Set<BrowserTreeNodeId>([
  'fixture-root',
  'fixture-tracks',
  'fixture-playlists'
])
const rootOnlyExpandedIds = new Set<BrowserTreeNodeId>(['fixture-root'])
const collapsedFixtureIds = new Set<BrowserTreeNodeId>()
void main()

function main(): void {
  validatesExplicitChildrenAndActionModel()
  validatesVisibleProjection()
  validatesVisibleHelpers()
  validatesDeferredActionProjection()
  validatesExpansionIndependence()
  validatesMoreActionNotExpandable()
  validatesLoadedBranchExpansion()
  validatesKeyboardNavigation()
  validatesKeyboardExpansion()
  validatesKeyboardSelection()
  validatesFocusAndSelectionSeparation()
  validatesAriaAttributes()
  validatesCollapseKeepsLoadedChildren()
  validatesLibraryHierarchyReadProjection()
  validatesFixtureFallbackRemainsExplicit()
  validatesRowRoleDistinctions()
  validatesFolderOpenClosedFromExpansion()
  validatesFileRoleIcons()
  validatesStateAndActionRoleIcons()
  validatesPrimaryActivation()
}

function validatesExplicitChildrenAndActionModel(): void {
  const leaf = leafRootNode()
  const loadedBranch = loadedBranchRootNode()
  const unloadedBranch = unloadedBranchNode()
  const loadingBranch = loadingBranchNode()
  const failedBranch = failedBranchNode()
  const moreAction = moreActionNode()

  assert.equal(isBrowserTreeLeaf(leaf), true)
  assert.equal(isBrowserTreeBranch(leaf), false)
  assert.equal(canRevealBrowserTreeChildren(leaf), false)
  assert.equal(canActivateBrowserTreeAction(leaf), false)
  assert.equal(isBrowserTreeActionLoading(leaf), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(leaf), [])

  assert.equal(isBrowserTreeLeaf(loadedBranch), false)
  assert.equal(isBrowserTreeBranch(loadedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadedBranch), true)
  assert.equal(canActivateBrowserTreeAction(loadedBranch), false)
  assert.equal(isBrowserTreeActionLoading(loadedBranch), false)
  assert.deepEqual(
    getLoadedBrowserTreeChildren(loadedBranch).map((node) => node.id),
    ['loaded-child']
  )

  const loadedEmptyBranch = loadedEmptyBranchRootNode()
  assert.equal(isBrowserTreeLeaf(loadedEmptyBranch), false)
  assert.equal(isBrowserTreeBranch(loadedEmptyBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadedEmptyBranch), false)
  assert.equal(canActivateBrowserTreeAction(loadedEmptyBranch), false)
  assert.equal(isBrowserTreeActionLoading(loadedEmptyBranch), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(loadedEmptyBranch), [])

  assert.equal(isBrowserTreeLeaf(unloadedBranch), false)
  assert.equal(isBrowserTreeBranch(unloadedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(unloadedBranch), false)
  assert.equal(canActivateBrowserTreeAction(unloadedBranch), true)
  assert.equal(isBrowserTreeActionLoading(unloadedBranch), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(unloadedBranch), [])
  assert.equal(unloadedBranch.children.kind, 'deferred')
  assert.equal(unloadedBranch.action?.kind, 'loadChildren')
  assert.equal(unloadedBranch.action?.state.kind, 'idle')

  assert.equal(isBrowserTreeBranch(loadingBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadingBranch), false)
  assert.equal(canActivateBrowserTreeAction(loadingBranch), false)
  assert.equal(isBrowserTreeActionLoading(loadingBranch), true)
  assert.equal(loadingBranch.children.kind, 'deferred')
  assert.equal(loadingBranch.action?.kind, 'loadChildren')
  assert.equal(loadingBranch.action?.state.kind, 'loading')
  assert.notEqual(loadingBranch.action?.state.kind, unloadedBranch.action?.state.kind)

  assert.equal(isBrowserTreeBranch(failedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(failedBranch), false)
  assert.equal(canActivateBrowserTreeAction(failedBranch), true)
  assert.equal(isBrowserTreeActionLoading(failedBranch), false)
  assert.equal(failedBranch.children.kind, 'deferred')
  assert.equal(failedBranch.action?.kind, 'loadChildren')
  assert.equal(failedBranch.action?.state.kind, 'failed')
  assert.equal(failedBranch.action?.state.detail, 'Unable to load children.')

  assert.equal(isBrowserTreeLeaf(moreAction), true)
  assert.equal(isBrowserTreeBranch(moreAction), false)
  assert.equal(canRevealBrowserTreeChildren(moreAction), false)
  assert.equal(canActivateBrowserTreeAction(moreAction), true)
  assert.equal(isBrowserTreeActionLoading(moreAction), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(moreAction), [])
  assert.equal(moreAction.children.kind, 'none')
  assert.equal(moreAction.action?.kind, 'loadMore')
  assert.equal(moreAction.action?.state.kind, 'idle')
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
  assert.equal(getItem(expandedItems, 'fixture-root').isBranch, true)
  assert.equal(getItem(expandedItems, 'fixture-root').canRevealChildren, true)
  assert.equal(getItem(expandedItems, 'fixture-root').canActivateAction, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').isBranch, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').canRevealChildren, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').canActivateAction, false)

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

function validatesDeferredActionProjection(): void {
  const items = flattenVisibleTree({
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode(), moreActionNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root', 'more-root'])
  })

  assert.deepEqual(
    items.map((item) => item.id),
    ['unloaded-root', 'loading-root', 'failed-root', 'more-root']
  )

  for (const item of items.filter((candidate) => candidate.id !== 'more-root')) {
    assert.equal(item.isBranch, true)
    assert.equal(item.canRevealChildren, false)
    assert.equal(item.isExpanded, true)
    assert.equal(getFirstChildVisibleNodeId(items, item.id), undefined)
  }

  assert.equal(getItem(items, 'unloaded-root').canActivateAction, true)
  assert.equal(getItem(items, 'unloaded-root').isActionLoading, false)
  assert.equal(getItem(items, 'loading-root').canActivateAction, false)
  assert.equal(getItem(items, 'loading-root').isActionLoading, true)
  assert.equal(getItem(items, 'failed-root').canActivateAction, true)
  assert.equal(getItem(items, 'failed-root').isActionLoading, false)
  assert.equal(getItem(items, 'more-root').isBranch, false)
  assert.equal(getItem(items, 'more-root').canRevealChildren, false)
  assert.equal(getItem(items, 'more-root').canActivateAction, true)
  assert.equal(getItem(items, 'more-root').isActionLoading, false)
  assert.equal(getItem(items, 'more-root').isActionItem, true)
  assert.equal(getItem(items, 'more-root').isExpanded, false)
}

function validatesExpansionIndependence(): void {
  const expandedDeferredItems = flattenVisibleTree({
    nodes: [unloadedBranchNode()],
    expandedNodeIds: new Set(['unloaded-root'])
  })
  const deferredItem = getItem(expandedDeferredItems, 'unloaded-root')

  assert.equal(deferredItem.isBranch, true)
  assert.equal(deferredItem.canRevealChildren, false)
  assert.equal(deferredItem.isExpanded, true)
  assert.equal(getFirstChildVisibleNodeId(expandedDeferredItems, deferredItem.id), undefined)

  const collapsedDeferredItems = flattenVisibleTree({
    nodes: [unloadedBranchNode()],
    expandedNodeIds: new Set()
  })
  assert.equal(getItem(collapsedDeferredItems, 'unloaded-root').isExpanded, false)

  const expandedLoadingItems = flattenVisibleTree({
    nodes: [loadingBranchNode()],
    expandedNodeIds: new Set(['loading-root'])
  })
  const loadingItem = getItem(expandedLoadingItems, 'loading-root')

  assert.equal(loadingItem.isExpanded, true)
  assert.equal(loadingItem.canRevealChildren, false)
  assert.equal(loadingItem.isActionLoading, true)
  assert.equal(getFirstChildVisibleNodeId(expandedLoadingItems, loadingItem.id), undefined)

  const expandedFailedItems = flattenVisibleTree({
    nodes: [failedBranchNode()],
    expandedNodeIds: new Set(['failed-root'])
  })
  const failedItem = getItem(expandedFailedItems, 'failed-root')

  assert.equal(failedItem.isExpanded, true)
  assert.equal(failedItem.canRevealChildren, false)
  assert.equal(failedItem.canActivateAction, true)
  assert.equal(getFirstChildVisibleNodeId(expandedFailedItems, failedItem.id), undefined)
}

function validatesMoreActionNotExpandable(): void {
  const items = flattenVisibleTree({
    nodes: [moreActionNode()],
    expandedNodeIds: new Set(['more-root'])
  })
  const moreItem = getItem(items, 'more-root')

  assert.equal(moreItem.isActionItem, true)
  assert.equal(moreItem.isBranch, false)
  assert.equal(moreItem.isExpanded, false)
  assert.equal(moreItem.canRevealChildren, false)
  assert.equal(moreItem.canActivateAction, true)
}

function validatesLoadedBranchExpansion(): void {
  const expandedLoadedItems = flattenVisibleTree({
    nodes: [loadedBranchRootNode()],
    expandedNodeIds: new Set(['loaded-root'])
  })

  assert.deepEqual(
    expandedLoadedItems.map((item) => item.id),
    ['loaded-root', 'loaded-child']
  )
  const expandedItem = getItem(expandedLoadedItems, 'loaded-root')
  assert.equal(expandedItem.isExpanded, true)
  assert.equal(expandedItem.canRevealChildren, true)

  const collapsedLoadedItems = flattenVisibleTree({
    nodes: [loadedBranchRootNode()],
    expandedNodeIds: new Set()
  })

  assert.deepEqual(
    collapsedLoadedItems.map((item) => item.id),
    ['loaded-root']
  )
  assert.equal(getItem(collapsedLoadedItems, 'loaded-root').isExpanded, false)
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
  const unloadedBranchItems = flattenVisibleTree({
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode(), moreActionNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root', 'more-root'])
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
  assertIntent(resolveIntent(unloadedBranchItems, 'unloaded-root', 'ArrowRight'), {
    kind: 'activateAction',
    nodeId: 'unloaded-root'
  })
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'loading-root', 'ArrowRight'))
  assertIntent(resolveIntent(unloadedBranchItems, 'failed-root', 'ArrowRight'), {
    kind: 'activateAction',
    nodeId: 'failed-root'
  })
  assertIntent(resolveIntent(unloadedBranchItems, 'more-root', 'ArrowRight'), {
    kind: 'activateAction',
    nodeId: 'more-root'
  })
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'unloaded-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'loading-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'failed-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'more-root', 'ArrowLeft'))
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

  const actionItems = flattenVisibleTree({
    nodes: [moreActionNode(), unloadedBranchNode()],
    expandedNodeIds: new Set(['more-root', 'unloaded-root'])
  })

  assertIntent(resolveIntent(actionItems, 'more-root', 'Enter'), {
    kind: 'activateAction',
    nodeId: 'more-root'
  })
  assertIntent(resolveIntent(actionItems, 'more-root', ' '), {
    kind: 'activateAction',
    nodeId: 'more-root'
  })

  assertIntent(resolveIntent(actionItems, 'unloaded-root', 'Enter'), {
    kind: 'select',
    nodeId: 'unloaded-root'
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

  const loadedEmptyItems = flattenVisibleTree({
    nodes: [loadedEmptyBranchRootNode()],
    expandedNodeIds: new Set(['loaded-empty-root'])
  })
  assert.equal(getTreeItemAriaExpanded(getItem(loadedEmptyItems, 'loaded-empty-root')), undefined)

  const unloadedItems = flattenVisibleTree({
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode(), moreActionNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root', 'more-root'])
  })
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'unloaded-root')), 'false')
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'loading-root')), 'false')
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'failed-root')), 'false')
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'more-root')), undefined)
}

function validatesCollapseKeepsLoadedChildren(): void {
  const loadedBranch = loadedBranchRootNode()
  const expandedItems = flattenVisibleTree({
    nodes: [loadedBranch],
    expandedNodeIds: new Set(['loaded-root'])
  })
  const collapsedItems = flattenVisibleTree({
    nodes: [loadedBranch],
    expandedNodeIds: new Set()
  })

  assert.deepEqual(
    expandedItems.map((item) => item.id),
    ['loaded-root', 'loaded-child']
  )
  assert.deepEqual(
    collapsedItems.map((item) => item.id),
    ['loaded-root']
  )
  assert.deepEqual(
    getLoadedBrowserTreeChildren(loadedBranch).map((node) => node.id),
    ['loaded-child']
  )
}

function validatesLibraryHierarchyReadProjection(): void {
  const projected = projectState({
    navigationReadResult: navigationSourceReadRowsResult(),
    sourceReadStates: new Map([
      [
        'navigation-row:7',
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(fileOnlyHierarchyReadResult().window)
        }
      ]
    ]),
    directoryReadStates: new Map()
  })

  assert.equal(projected?.kind, 'tree')
  if (projected?.kind !== 'tree') {
    assert.fail('expected file-only hierarchy read result to project to browser tree')
  }

  assert.deepEqual(projected.nodes, [
    {
      id: 'navigation-row:7',
      role: 'source',
      label: 'Source Fixture',
      badge: { value: 'Local Library', tone: 'muted' },
      icon: 'source',
      detail: 'Navigation source row. Updated 1970-01-01.',
      children: {
        kind: 'loaded',
        nodes: [
          {
            id: 'source-file:11',
            role: 'literalFile',
            label: 'track.wav',
            badge: { value: 'Audio', tone: 'muted' },
            icon: 'music',
            detail: 'Present file.',
            children: { kind: 'none' }
          }
        ]
      }
    }
  ])

  const directoryProjection = projectState({
    navigationReadResult: navigationSourceReadRowsResult(),
    sourceReadStates: new Map([
      [
        'navigation-row:7',
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow(directoryHierarchyReadResult().window)
        }
      ]
    ]),
    directoryReadStates: new Map()
  })
  assert.equal(directoryProjection?.kind, 'tree')
  if (directoryProjection?.kind !== 'tree') {
    assert.fail('expected directory hierarchy read result to project to browser tree')
  }
  assert.deepEqual(directoryProjection.nodes, [
    {
      id: 'navigation-row:7',
      role: 'source',
      label: 'Source Fixture',
      badge: { value: 'Local Library', tone: 'muted' },
      icon: 'source',
      detail: 'Navigation source row. Updated 1970-01-01.',
      children: {
        kind: 'loaded',
        nodes: [
          {
            id: 'source-directory:12',
            role: 'literalDirectory',
            label: 'Album',
            badge: { value: 'Folder', tone: 'muted' },
            icon: 'folder',
            detail: 'Present directory.',
            children: {
              kind: 'deferred',
              detail: 'Children not loaded yet.'
            },
            action: {
              kind: 'loadChildren',
              state: {
                kind: 'idle',
                detail: 'Children not loaded yet.'
              }
            }
          }
        ]
      }
    }
  ])
  const directoryItems = flattenVisibleTree({
    nodes: directoryProjection.nodes,
    expandedNodeIds: new Set(['navigation-row:7', 'source-directory:12'])
  })
  assert.deepEqual(
    directoryItems.map((item) => item.id),
    ['navigation-row:7', 'source-directory:12']
  )
  assert.equal(getItem(directoryItems, 'source-directory:12').isBranch, true)
  assert.equal(getItem(directoryItems, 'source-directory:12').canRevealChildren, false)
  assert.equal(getItem(directoryItems, 'source-directory:12').canActivateAction, true)
  assert.deepEqual(
    [...directoryProjection.bindingsById.entries()].filter(
      ([, binding]) => binding.kind === 'directory'
    ),
    [
      [
        'source-directory:12',
        {
          kind: 'directory',
          directoryId: '12',
          entryPoint: {
            kind: 'source',
            sourceId: '7'
          },
          label: 'Source Fixture'
        }
      ]
    ]
  )

  const partialProjection = projectState({
    navigationReadResult: navigationSourceReadRowsResult(),
    sourceReadStates: new Map([
      [
        'navigation-row:7',
        {
          kind: 'loaded',
          children: loadedChildrenFromWindow({
            ...fileOnlyHierarchyReadResult().window,
            totalRows: 2
          })
        }
      ]
    ]),
    directoryReadStates: new Map()
  })
  assert.equal(partialProjection?.kind, 'tree')
  if (partialProjection?.kind !== 'tree') {
    assert.fail('expected partial hierarchy read result to project to browser tree')
  }
  const partialSourceNode = partialProjection.nodes[0]
  assert.equal(partialSourceNode?.children.kind, 'loaded')
  if (partialSourceNode?.children.kind !== 'loaded') {
    assert.fail('expected partial projection to render loaded children')
  }
  assert.deepEqual(
    partialSourceNode.children.nodes.map((node) => node.id),
    ['source-file:11', 'more:navigation-row:7:1']
  )
  assert.deepEqual(partialSourceNode.children.nodes[0]?.badge, { value: 'Audio', tone: 'muted' })
  assert.equal(partialSourceNode.children.nodes[0]?.icon, 'music')
  assert.deepEqual(partialSourceNode.children.nodes[1]?.badge, { value: 'More', tone: 'muted' })
  assert.equal(partialSourceNode.children.nodes[1]?.icon, 'more')
  assert.equal(partialSourceNode.children.nodes[1]?.children.kind, 'none')
  assert.equal(partialSourceNode.children.nodes[1]?.action?.kind, 'loadMore')
  assert.equal(partialSourceNode.children.nodes[1]?.action?.state.kind, 'idle')
  assert.equal(
    partialSourceNode.children.nodes[1]?.action?.state.detail,
    'Rows 2-2 of 2 are available.'
  )
  assert.deepEqual(
    (
      partialProjection.bindingsById.get('more:navigation-row:7:1') as
        | { target: unknown }
        | undefined
    )?.target,
    {
      ownerNodeId: 'navigation-row:7',
      entryPoint: {
        kind: 'source',
        sourceId: '7'
      },
      label: 'Source Fixture',
      offset: 1,
      limit: 50
    }
  )
}

function validatesFixtureFallbackRemainsExplicit(): void {
  assert.equal(libraryHierarchyFixtureTree.name, 'Tree validation fixture')
  assert.match(libraryHierarchyFixtureTree.detail, /Test-owned fixture input/)
  assert.deepEqual(libraryHierarchyFixtureTree.nodes[0]?.badge, { value: 'Root', tone: 'muted' })
  assert.equal(libraryHierarchyFixtureTree.nodes[0]?.children.kind, 'loaded')
  assert.doesNotMatch(JSON.stringify(libraryHierarchyFixtureTree), /Loaded from literal hierarchy/)
  assert.doesNotMatch(JSON.stringify(libraryHierarchyFixtureTree), /Children not loaded yet/)
  assertFixtureNodesHaveExplicitChildrenState(libraryHierarchyFixtureTree.nodes)
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

function assertFixtureNodesHaveExplicitChildrenState(nodes: readonly BrowserTreeNode[]): void {
  for (const node of nodes) {
    assert.equal(typeof node.children.kind, 'string')
    assertFixtureNodesHaveExplicitChildrenState(getLoadedBrowserTreeChildren(node))
  }
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
    role: 'state',
    label: 'Leaf root',
    badge: { value: 'Branch', tone: 'muted' },
    children: { kind: 'none' }
  }
}

function loadedBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-root',
    role: 'source',
    label: 'Loaded root',
    badge: { value: 'Branch', tone: 'muted' },
    children: {
      kind: 'loaded',
      nodes: [
        {
          id: 'loaded-child',
          role: 'literalFile',
          label: 'Loaded child',
          badge: { value: 'Leaf', tone: 'muted' },
          children: { kind: 'none' }
        }
      ]
    }
  }
}

function loadedEmptyBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-empty-root',
    role: 'source',
    label: 'Loaded empty root',
    badge: { value: 'Branch', tone: 'muted' },
    children: {
      kind: 'loaded',
      nodes: []
    }
  }
}

function unloadedBranchNode(): BrowserTreeNode {
  const detail = 'Children not loaded yet.'

  return {
    id: 'unloaded-root',
    role: 'source',
    label: 'Unloaded root',
    badge: { value: 'Branch', tone: 'muted' },
    children: {
      kind: 'deferred',
      detail: detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'idle',
        detail: detail
      }
    }
  }
}

function loadingBranchNode(): BrowserTreeNode {
  const detail = 'Loading children.'

  return {
    id: 'loading-root',
    role: 'source',
    label: 'Loading root',
    badge: { value: 'Branch', tone: 'muted' },
    children: {
      kind: 'deferred',
      detail: detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'loading',
        detail: detail
      }
    }
  }
}

function failedBranchNode(): BrowserTreeNode {
  const detail = 'Unable to load children.'

  return {
    id: 'failed-root',
    role: 'source',
    label: 'Failed root',
    badge: { value: 'Branch', tone: 'muted' },
    children: {
      kind: 'deferred',
      detail: detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'failed',
        detail: detail
      }
    }
  }
}

function moreActionNode(): BrowserTreeNode {
  return {
    id: 'more-root',
    role: 'action',
    label: 'Load more rows',
    badge: { value: 'More', tone: 'muted' },
    icon: 'more',
    children: {
      kind: 'none'
    },
    action: {
      kind: 'loadMore',
      state: {
        kind: 'idle',
        detail: 'Rows 1-50 of 100 are available.'
      }
    }
  }
}

function fileOnlyHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
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
          fileId: '11',
          presence: 'present',
          updatedAtMs: 100
        }
      ]
    }
  }
}

function navigationSourceReadRowsResult(): NavigationReadRowsResult {
  return {
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
}

function directoryHierarchyReadResult(): Extract<ReadResult, { state: 'ready' }> {
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
          directoryId: '12',
          presence: 'present',
          updatedAtMs: 100
        }
      ]
    }
  }
}

function loadedChildrenFromWindow(window: ChildWindow): LoadedChildren {
  const nextOffset = window.nodes.length < window.totalRows ? window.nodes.length : undefined

  return {
    entryPoint: window.root.entryPoint,
    ...(window.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: window.parentDirectoryId }),
    ...(window.root.label === undefined ? {} : { label: window.root.label }),
    rows: window.nodes,
    totalRows: window.totalRows,
    ...(nextOffset === undefined ? {} : { nextOffset }),
    limit: window.limit
  }
}

function nodeWithRole(role: BrowserTreeRowRole, icon?: BrowserTreeNode['icon']): BrowserTreeNode {
  return {
    id: `role-test:${role}`,
    role,
    label: `Test ${role}`,
    ...(icon === undefined ? {} : { icon }),
    children: { kind: 'none' }
  }
}

function resolvesToDifferentCategory(
  roleA: BrowserTreeRowRole,
  roleB: BrowserTreeRowRole
): boolean {
  return roleA !== roleB
}

function validatesRowRoleDistinctions(): void {
  const everyRole: readonly BrowserTreeRowRole[] = [
    'collectionView',
    'locationGroup',
    'source',
    'sourceLocation',
    'literalDirectory',
    'literalFile',
    'preparationSurface',
    'playlistSurface',
    'smartView',
    'state',
    'action'
  ]

  for (const role of everyRole) {
    const node = nodeWithRole(role)
    assert.equal(node.role, role, `node must carry role ${role}`)
  }

  assert.ok(
    resolvesToDifferentCategory('locationGroup', 'source'),
    'locationGroup must not share category with source'
  )
  assert.ok(
    resolvesToDifferentCategory('locationGroup', 'sourceLocation'),
    'locationGroup must not share category with sourceLocation'
  )
  assert.ok(
    resolvesToDifferentCategory('collectionView', 'source'),
    'collectionView must differ from source'
  )

  const sourceNode = nodeWithRole('source')
  assert.equal(sourceNode.role, 'source', 'source role must be explicit')

  const locationGroupNode = nodeWithRole('locationGroup')
  assert.equal(locationGroupNode.role, 'locationGroup', 'locationGroup role must be explicit')

  assert.notEqual(
    locationGroupNode.role,
    sourceNode.role,
    'locationGroup and source must resolve to different roles'
  )
}

function validatesFolderOpenClosedFromExpansion(): void {
  const dir = nodeWithRole('literalDirectory', 'folder')
  const sourceLocation = nodeWithRole('sourceLocation')
  const locationGroup = nodeWithRole('locationGroup')

  assert.equal(typeof dir.role, 'string', 'literalDirectory must have role')
  assert.equal(typeof sourceLocation.role, 'string', 'sourceLocation must have role')
  assert.equal(typeof locationGroup.role, 'string', 'locationGroup must have role')

  const expandedDir: BrowserTreeVisibleItem = {
    id: dir.id,
    node: dir,
    level: 1,
    visibleIndex: 0,
    isBranch: true,
    canRevealChildren: true,
    canActivateAction: false,
    isActionLoading: false,
    isActionItem: false,
    isExpanded: true,
    isSelected: false,
    isActive: false,
    ariaSetSize: 1,
    ariaPosInSet: 1
  }

  const closedDir: BrowserTreeVisibleItem = {
    ...expandedDir,
    isExpanded: false
  }

  assert.ok(expandedDir.isExpanded, 'expanded state must be true')
  assert.ok(!closedDir.isExpanded, 'closed state must be false')
  assert.notEqual(
    expandedDir.isExpanded,
    closedDir.isExpanded,
    'folder open/closed must derive from expansion state, not baked into role'
  )
}

function validatesFileRoleIcons(): void {
  const fileRoles: readonly BrowserTreeNode['icon'][] = [
    'music',
    'video',
    'image',
    'cueSheet',
    'playlist',
    'metadata',
    'file'
  ]

  for (const icon of fileRoles) {
    const node = nodeWithRole('literalFile', icon)
    assert.equal(node.role, 'literalFile', `file with icon ${icon} must be literalFile`)
    assert.equal(node.icon, icon, `file must carry icon hint ${icon}`)
  }
}

function validatesStateAndActionRoleIcons(): void {
  const stateLoading = nodeWithRole('state', 'loading')
  const stateWarning = nodeWithRole('state', 'warning')
  const stateDefault = nodeWithRole('state')

  assert.equal(stateLoading.role, 'state', 'state loading must have state role')
  assert.equal(stateWarning.role, 'state', 'state warning must have state role')
  assert.equal(stateDefault.role, 'state', 'state default must have state role')

  const actionLoading = nodeWithRole('action', 'loading')
  const actionWarning = nodeWithRole('action', 'warning')
  const actionDefault = nodeWithRole('action', 'more')

  assert.equal(actionLoading.role, 'action', 'action loading must have action role')
  assert.equal(actionWarning.role, 'action', 'action warning must have action role')
  assert.equal(actionDefault.role, 'action', 'action default must have action role')

  assert.notEqual(
    stateLoading.role,
    actionLoading.role,
    'state and action must have distinct roles'
  )
}

function validatesPrimaryActivation(): void {
  const primaryActivationActions = recordPrimaryActivationActions(loadedBranchRootNode(), false)
  assert.deepEqual(
    primaryActivationActions,
    ['select', 'toggle'],
    'loaded collapsed branch primary activation must select and expand'
  )

  const expandedPrimary = recordPrimaryActivationActions(loadedBranchRootNode(), true)
  assert.deepEqual(
    expandedPrimary,
    ['select', 'toggle'],
    'loaded expanded branch primary activation must select and collapse'
  )

  const deferredPrimary = recordPrimaryActivationActions(unloadedBranchNode(), false)
  assert.deepEqual(
    deferredPrimary,
    ['select', 'toggle', 'activateAction'],
    'deferred branch primary activation must select, expand, and activate load'
  )

  const failedPrimary = recordPrimaryActivationActions(failedBranchNode(), false)
  assert.deepEqual(
    failedPrimary,
    ['select', 'toggle', 'activateAction'],
    'failed branch primary activation must select, expand, and activate retry'
  )

  const loadingPrimary = recordPrimaryActivationActions(loadingBranchNode(), false)
  assert.deepEqual(
    loadingPrimary,
    ['select'],
    'loading branch primary activation must select without duplicate load'
  )

  const leafPrimary = recordPrimaryActivationActions(leafRootNode(), false)
  assert.deepEqual(leafPrimary, ['select'], 'leaf primary activation must select only')

  const actionPrimary = recordPrimaryActivationActions(moreActionNode(), false)
  assert.deepEqual(
    actionPrimary,
    ['activateAction'],
    'action row primary activation must activate action only'
  )
}

function recordPrimaryActivationActions(
  node: BrowserTreeNode,
  isExpanded: boolean
): readonly string[] {
  const item = visibleItemFromNode(node, isExpanded)
  const actions: string[] = []

  if (item.isActionItem) {
    if (item.canActivateAction) {
      actions.push('activateAction')
    }

    return actions
  }

  actions.push('select')

  if (!item.isBranch) {
    return actions
  }

  if (item.isExpanded) {
    actions.push('toggle')
    return actions
  }

  if (item.canRevealChildren) {
    actions.push('toggle')
    return actions
  }

  if (item.canActivateAction) {
    actions.push('toggle')
    actions.push('activateAction')
    return actions
  }

  return actions
}

function visibleItemFromNode(node: BrowserTreeNode, isExpanded: boolean): BrowserTreeVisibleItem {
  const isBranch = isBrowserTreeBranch(node)

  return {
    id: node.id,
    node,
    level: 1,
    visibleIndex: 0,
    isBranch,
    canRevealChildren: canRevealBrowserTreeChildren(node),
    canActivateAction: canActivateBrowserTreeAction(node),
    isActionLoading: isBrowserTreeActionLoading(node),
    isActionItem: node.action !== undefined && !isBranch,
    isExpanded: isBranch && isExpanded,
    isSelected: false,
    isActive: false,
    ariaSetSize: 1,
    ariaPosInSet: 1
  }
}
