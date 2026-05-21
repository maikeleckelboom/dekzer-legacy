import { strict as assert } from 'node:assert'

import { projectState } from '../src/renderer/libraryBrowser/hierarchyProjection'
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
import type { LoadedChildren } from '../src/renderer/libraryBrowser/hierarchyState'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId,
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
      label: 'Fixture Library Root',
      badgeLabel: 'Root',
      detail: 'Demo root for renderer tree behavior.',
      children: {
        kind: 'loaded',
        children: [
          {
            id: 'fixture-artists',
            label: 'Artists',
            badgeLabel: 'Leaf',
            detail: 'Fixture grouping placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-albums',
            label: 'Albums',
            badgeLabel: 'Leaf',
            detail: 'Fixture grouping placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-tracks',
            label: 'Tracks',
            badgeLabel: 'Branch',
            detail: 'Fixture grouping placeholder.',
            children: {
              kind: 'loaded',
              children: [
                {
                  id: 'fixture-tracks-group',
                  label: 'Fixture Track Group',
                  badgeLabel: 'Branch',
                  detail: 'Demo child row for nested hierarchy rendering.',
                  children: { kind: 'none' }
                }
              ]
            }
          },
          {
            id: 'fixture-playlists',
            label: 'Nested Branch',
            badgeLabel: 'Branch',
            detail: 'Fixture grouping placeholder.',
            children: {
              kind: 'loaded',
              children: [
                {
                  id: 'fixture-playlist-group',
                  label: 'Fixture Nested Group',
                  badgeLabel: 'Branch',
                  detail: 'Demo child row for nested hierarchy rendering.',
                  children: { kind: 'none' }
                }
              ]
            }
          },
          {
            id: 'fixture-preparation',
            label: 'Preparation',
            badgeLabel: 'Leaf',
            detail: 'Fixture workflow placeholder.',
            children: { kind: 'none' }
          },
          {
            id: 'fixture-history',
            label: 'History',
            badgeLabel: 'Leaf',
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
  validatesKeyboardNavigation()
  validatesKeyboardExpansion()
  validatesKeyboardSelection()
  validatesFocusAndSelectionSeparation()
  validatesAriaAttributes()
  validatesCollapseKeepsLoadedChildren()
  validatesLibraryHierarchyReadProjection()
  validatesFixtureFallbackRemainsExplicit()
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
    assert.equal(item.isExpanded, false)
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
      label: 'Source Fixture',
      badgeLabel: 'Source',
      icon: 'source',
      detail: 'Navigation source row. Updated 100.',
      children: {
        kind: 'loaded',
        children: [
          {
            id: 'source-file:11',
            label: 'track.wav',
            badgeLabel: 'File',
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
      label: 'Source Fixture',
      badgeLabel: 'Source',
      icon: 'source',
      detail: 'Navigation source row. Updated 100.',
      children: {
        kind: 'loaded',
        children: [
          {
            id: 'source-directory:12',
            label: 'Album',
            badgeLabel: 'Folder',
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
    partialSourceNode.children.children.map((node) => node.id),
    ['source-file:11', 'more:navigation-row:7:1']
  )
  assert.equal(partialSourceNode.children.children[0]?.badgeLabel, 'File')
  assert.equal(partialSourceNode.children.children[0]?.icon, 'music')
  assert.equal(partialSourceNode.children.children[1]?.badgeLabel, 'More')
  assert.equal(partialSourceNode.children.children[1]?.icon, 'more')
  assert.equal(partialSourceNode.children.children[1]?.children.kind, 'none')
  assert.equal(partialSourceNode.children.children[1]?.action?.kind, 'loadMore')
  assert.equal(partialSourceNode.children.children[1]?.action?.state.kind, 'idle')
  assert.equal(
    partialSourceNode.children.children[1]?.action?.state.detail,
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
  assert.equal(libraryHierarchyFixtureTree.nodes[0]?.badgeLabel, 'Root')
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
    label: 'Leaf root',
    badgeLabel: 'Branch',
    children: { kind: 'none' }
  }
}

function loadedBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-root',
    label: 'Loaded root',
    badgeLabel: 'Branch',
    children: {
      kind: 'loaded',
      children: [
        {
          id: 'loaded-child',
          label: 'Loaded child',
          badgeLabel: 'Leaf',
          children: { kind: 'none' }
        }
      ]
    }
  }
}

function loadedEmptyBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-empty-root',
    label: 'Loaded empty root',
    badgeLabel: 'Branch',
    children: {
      kind: 'loaded',
      children: []
    }
  }
}

function unloadedBranchNode(): BrowserTreeNode {
  const detail = 'Children not loaded yet.'

  return {
    id: 'unloaded-root',
    label: 'Unloaded root',
    badgeLabel: 'Branch',
    children: {
      kind: 'deferred',
      detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'idle',
        detail
      }
    }
  }
}

function loadingBranchNode(): BrowserTreeNode {
  const detail = 'Loading children.'

  return {
    id: 'loading-root',
    label: 'Loading root',
    badgeLabel: 'Branch',
    children: {
      kind: 'deferred',
      detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'loading',
        detail
      }
    }
  }
}

function failedBranchNode(): BrowserTreeNode {
  const detail = 'Unable to load children.'

  return {
    id: 'failed-root',
    label: 'Failed root',
    badgeLabel: 'Branch',
    children: {
      kind: 'deferred',
      detail
    },
    action: {
      kind: 'loadChildren',
      state: {
        kind: 'failed',
        detail
      }
    }
  }
}

function moreActionNode(): BrowserTreeNode {
  return {
    id: 'more-root',
    label: 'Load more rows',
    badgeLabel: 'More',
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
