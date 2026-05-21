import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

import { desktopRoot, rendererSourceRoot, listSourceFiles, normalizePath } from './support/files'
import { libraryHierarchyFixtureTree } from '../src/renderer/libraryBrowser/fixture'
import { projectReadResult } from '../src/renderer/libraryBrowser/hierarchyProjection'
import {
  getTreeItemAriaExpanded,
  getTreeItemAriaSelected
} from '../src/renderer/libraryBrowser/tree/aria'
import {
  resolveTreeKeyboardIntent,
  type TreeKeyboardIntent
} from '../src/renderer/libraryBrowser/tree/keys'
import {
  canRequestBrowserTreeChildren,
  canRevealBrowserTreeChildren,
  flattenVisibleTree,
  getFirstChildVisibleNodeId,
  getFirstVisibleNodeId,
  getLastVisibleNodeId,
  getLoadedBrowserTreeChildren,
  getNextVisibleNodeId,
  getParentVisibleNodeId,
  getPreviousVisibleNodeId,
  isLoadingBrowserTreeChildren,
  isBrowserTreeBranch,
  isBrowserTreeLeaf
} from '../src/renderer/libraryBrowser/tree/projection'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId,
  BrowserTreeVisibleItem
} from '../src/renderer/libraryBrowser/tree/types'
import type { LibraryHierarchyReadChildrenResult } from '../src/shared/libraryHierarchy/readChildren'

const expandedFixtureIds = new Set<BrowserTreeNodeId>([
  'fixture-root',
  'fixture-tracks',
  'fixture-playlists'
])
const rootOnlyExpandedIds = new Set<BrowserTreeNodeId>(['fixture-root'])
const collapsedFixtureIds = new Set<BrowserTreeNodeId>()
const approvedRunScanRendererOwner = 'src/renderer/libraryBrowser/localRootActions.ts'
void main()

function main(): void {
  validatesExplicitChildrenStateModel()
  validatesVisibleProjection()
  validatesVisibleHelpers()
  validatesUnloadedBranchProjection()
  validatesKeyboardNavigation()
  validatesKeyboardExpansion()
  validatesKeyboardSelection()
  validatesFocusAndSelectionSeparation()
  validatesAriaAttributes()
  validatesRequestChildrenWiring()
  validatesCollapseKeepsLoadedChildren()
  validatesLibraryHierarchyReadProjection()
  validatesFixtureFallbackRemainsExplicit()
  validatesRendererBoundaryOwnership()
  validatesStrictTypecheckFlagsRemainEnabled()
}

function validatesExplicitChildrenStateModel(): void {
  const leaf = leafRootNode()
  const loadedBranch = loadedBranchRootNode()
  const unloadedBranch = unloadedBranchNode()
  const loadingBranch = loadingBranchNode()
  const failedBranch = failedBranchNode()

  assert.equal(isBrowserTreeLeaf(leaf), true)
  assert.equal(isBrowserTreeBranch(leaf), false)
  assert.equal(canRevealBrowserTreeChildren(leaf), false)
  assert.equal(canRequestBrowserTreeChildren(leaf), false)
  assert.equal(isLoadingBrowserTreeChildren(leaf), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(leaf), [])

  assert.equal(isBrowserTreeLeaf(loadedBranch), false)
  assert.equal(isBrowserTreeBranch(loadedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadedBranch), true)
  assert.equal(canRequestBrowserTreeChildren(loadedBranch), false)
  assert.equal(isLoadingBrowserTreeChildren(loadedBranch), false)
  assert.deepEqual(
    getLoadedBrowserTreeChildren(loadedBranch).map((node) => node.id),
    ['loaded-child']
  )

  const loadedEmptyBranch = loadedEmptyBranchRootNode()
  assert.equal(isBrowserTreeLeaf(loadedEmptyBranch), false)
  assert.equal(isBrowserTreeBranch(loadedEmptyBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadedEmptyBranch), false)
  assert.equal(canRequestBrowserTreeChildren(loadedEmptyBranch), false)
  assert.equal(isLoadingBrowserTreeChildren(loadedEmptyBranch), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(loadedEmptyBranch), [])

  assert.equal(isBrowserTreeLeaf(unloadedBranch), false)
  assert.equal(isBrowserTreeBranch(unloadedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(unloadedBranch), false)
  assert.equal(canRequestBrowserTreeChildren(unloadedBranch), true)
  assert.equal(isLoadingBrowserTreeChildren(unloadedBranch), false)
  assert.deepEqual(getLoadedBrowserTreeChildren(unloadedBranch), [])
  assert.equal(unloadedBranch.childrenState.kind, 'unloaded')

  assert.equal(isBrowserTreeBranch(loadingBranch), true)
  assert.equal(canRevealBrowserTreeChildren(loadingBranch), false)
  assert.equal(canRequestBrowserTreeChildren(loadingBranch), false)
  assert.equal(isLoadingBrowserTreeChildren(loadingBranch), true)
  assert.equal(loadingBranch.childrenState.kind, 'loading')
  assert.notEqual(loadingBranch.childrenState.kind, unloadedBranch.childrenState.kind)

  assert.equal(isBrowserTreeBranch(failedBranch), true)
  assert.equal(canRevealBrowserTreeChildren(failedBranch), false)
  assert.equal(canRequestBrowserTreeChildren(failedBranch), true)
  assert.equal(isLoadingBrowserTreeChildren(failedBranch), false)
  assert.equal(failedBranch.childrenState.kind, 'failed')
  assert.equal(failedBranch.childrenState.detail, 'Unable to load children.')

  const treeTypesSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/tree/types.ts', import.meta.url),
    'utf8'
  )
  assert.match(treeTypesSource, /readonly childrenState: BrowserTreeChildrenState/)
  assert.doesNotMatch(treeTypesSource, /readonly children\?:/)
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
  assert.equal(getItem(expandedItems, 'fixture-root').canRequestChildren, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').isBranch, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').canRevealChildren, false)
  assert.equal(getItem(expandedItems, 'fixture-artists').canRequestChildren, false)

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

function validatesUnloadedBranchProjection(): void {
  const items = flattenVisibleTree({
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root'])
  })

  assert.deepEqual(
    items.map((item) => item.id),
    ['unloaded-root', 'loading-root', 'failed-root']
  )

  for (const item of items) {
    assert.equal(item.isBranch, true)
    assert.equal(item.canRevealChildren, false)
    assert.equal(item.isExpanded, false)
    assert.equal(getFirstChildVisibleNodeId(items, item.id), undefined)
  }

  assert.equal(getItem(items, 'unloaded-root').canRequestChildren, true)
  assert.equal(getItem(items, 'unloaded-root').isLoadingChildren, false)
  assert.equal(getItem(items, 'loading-root').canRequestChildren, false)
  assert.equal(getItem(items, 'loading-root').isLoadingChildren, true)
  assert.equal(getItem(items, 'failed-root').canRequestChildren, true)
  assert.equal(getItem(items, 'failed-root').isLoadingChildren, false)
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
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root'])
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
    kind: 'requestChildren',
    nodeId: 'unloaded-root'
  })
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'loading-root', 'ArrowRight'))
  assertIntent(resolveIntent(unloadedBranchItems, 'failed-root', 'ArrowRight'), {
    kind: 'requestChildren',
    nodeId: 'failed-root'
  })
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'unloaded-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'loading-root', 'ArrowLeft'))
  assertHandledNoop(resolveIntent(unloadedBranchItems, 'failed-root', 'ArrowLeft'))
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

  const loadedEmptyItems = flattenVisibleTree({
    nodes: [loadedEmptyBranchRootNode()],
    expandedNodeIds: new Set(['loaded-empty-root'])
  })
  assert.equal(getTreeItemAriaExpanded(getItem(loadedEmptyItems, 'loaded-empty-root')), undefined)

  const unloadedItems = flattenVisibleTree({
    nodes: [unloadedBranchNode(), loadingBranchNode(), failedBranchNode()],
    expandedNodeIds: new Set(['unloaded-root', 'loading-root', 'failed-root'])
  })
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'unloaded-root')), undefined)
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'loading-root')), undefined)
  assert.equal(getTreeItemAriaExpanded(getItem(unloadedItems, 'failed-root')), undefined)
}

function validatesRequestChildrenWiring(): void {
  const treeRootSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/tree/treeRoot.vue', import.meta.url),
    'utf8'
  )
  const treeItemSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/tree/treeItem.vue', import.meta.url),
    'utf8'
  )
  const treeRowSource = readFileSync(
    new URL('../src/renderer/libraryBrowser/tree/treeRow.vue', import.meta.url),
    'utf8'
  )

  assert.match(treeRootSource, /requestChildren: \[nodeId: BrowserTreeNodeId\]/)
  assert.match(treeRootSource, /requestChildren: \(nodeId\) => emit\('requestChildren', nodeId\)/)
  assert.match(treeItemSource, /if \(props\.item\.canRequestChildren\)/)
  assert.match(treeItemSource, /tree\.requestChildren\(props\.item\.id\)/)
  assert.match(treeItemSource, /case 'requestChildren':/)
  assert.match(treeRowSource, /:data-tree-affordance="item\.isBranch \? 'true' : undefined"/)
  assert.match(
    treeRowSource,
    /v-if="item\.canRevealChildren \|\| item\.canRequestChildren \|\| item\.isLoadingChildren"/
  )
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
  const projected = projectReadResult(fileOnlyHierarchyReadResult())

  assert.equal(projected.kind, 'tree')
  if (projected.kind !== 'tree') {
    assert.fail('expected file-only hierarchy read result to project to browser tree')
  }

  assert.deepEqual(projected.nodes, [
    {
      id: 'source:7',
      label: 'Source Fixture',
      kind: 'source',
      detail: 'Loaded read-only from library backend. 1 literal hierarchy row available.',
      childrenState: {
        kind: 'loaded',
        children: [
          {
            id: 'source-file:11',
            label: 'track.wav',
            kind: 'file',
            detail: 'Present file.',
            childrenState: { kind: 'leaf' }
          }
        ]
      }
    }
  ])

  const directoryProjection = projectReadResult(directoryHierarchyReadResult())
  assert.equal(directoryProjection.kind, 'tree')
  if (directoryProjection.kind !== 'tree') {
    assert.fail('expected directory hierarchy read result to project to browser tree')
  }
  assert.deepEqual(directoryProjection.nodes, [
    {
      id: 'source:7',
      label: 'Source Fixture',
      kind: 'source',
      detail: 'Loaded read-only from library backend. 1 literal hierarchy row available.',
      childrenState: {
        kind: 'loaded',
        children: [
          {
            id: 'source-directory:12',
            label: 'Album',
            kind: 'folder',
            detail: 'Present directory.',
            childrenState: {
              kind: 'unloaded',
              detail: 'Children not loaded yet.'
            }
          }
        ]
      }
    }
  ])
  const directoryItems = flattenVisibleTree({
    nodes: directoryProjection.nodes,
    expandedNodeIds: new Set(['source:7', 'source-directory:12'])
  })
  assert.deepEqual(
    directoryItems.map((item) => item.id),
    ['source:7', 'source-directory:12']
  )
  assert.equal(getItem(directoryItems, 'source-directory:12').isBranch, true)
  assert.equal(getItem(directoryItems, 'source-directory:12').canRevealChildren, false)
  assert.equal(getItem(directoryItems, 'source-directory:12').canRequestChildren, true)
  assert.deepEqual(
    [...directoryProjection.directoryReadTargetsByNodeId.entries()],
    [['source-directory:12', { sourceDirectoryId: '12' }]]
  )

  const partialProjection = projectReadResult({
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
  assert.equal(libraryHierarchyFixtureTree.nodes[0]?.childrenState.kind, 'loaded')
  assert.doesNotMatch(
    JSON.stringify(libraryHierarchyFixtureTree),
    /Loaded read-only from library backend/
  )
  assert.doesNotMatch(JSON.stringify(libraryHierarchyFixtureTree), /Children not loaded yet/)
  assertFixtureNodesHaveExplicitChildrenState(libraryHierarchyFixtureTree.nodes)
}

function validatesRendererBoundaryOwnership(): void {
  const violations: string[] = []
  const forbiddenPatterns = [
    /@dekzer\/library-boundary-client/,
    /@dekzer\/library-boundary-stdio-transport/,
    /\bLibraryBoundaryClient\b/,
    /\bipcRenderer\b/,
    /from ['"]electron['"]/,
    /from ['"]node:fs['"]/,
    /from ['"]fs['"]/,
    /from ['"]node:path['"]/,
    /from ['"]path['"]/,
    /from ['"].*\/main\//,
    /\bshowOpenDialog\b/,
    /\brunRootScan\b/
  ]

  for (const filePath of listSourceFiles(rendererSourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    for (const pattern of forbiddenPatterns) {
      if (pattern.test(contents)) {
        violations.push(`${relativePath}: ${String(pattern)}`)
      }
    }

    if (/\.runScan\(/.test(contents) && relativePath !== approvedRunScanRendererOwner) {
      violations.push(
        `${relativePath}: .runScan is only allowed in ${approvedRunScanRendererOwner}`
      )
    }
  }

  assert.deepEqual(violations, [])
}

function validatesStrictTypecheckFlagsRemainEnabled(): void {
  for (const tsconfigName of ['tsconfig.node.json', 'tsconfig.web.json']) {
    const tsconfig = JSON.parse(readFileSync(join(desktopRoot, tsconfigName), 'utf8')) as {
      readonly compilerOptions?: {
        readonly strictNullChecks?: boolean
        readonly exactOptionalPropertyTypes?: boolean
        readonly noUncheckedIndexedAccess?: boolean
      }
    }

    assert.equal(tsconfig.compilerOptions?.strictNullChecks, true)
    assert.equal(tsconfig.compilerOptions?.exactOptionalPropertyTypes, true)
    assert.equal(tsconfig.compilerOptions?.noUncheckedIndexedAccess, true)
  }
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
    assert.equal(typeof node.childrenState.kind, 'string')
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
    kind: 'folder',
    childrenState: { kind: 'leaf' }
  }
}

function loadedBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-root',
    label: 'Loaded root',
    kind: 'folder',
    childrenState: {
      kind: 'loaded',
      children: [
        {
          id: 'loaded-child',
          label: 'Loaded child',
          kind: 'file',
          childrenState: { kind: 'leaf' }
        }
      ]
    }
  }
}

function loadedEmptyBranchRootNode(): BrowserTreeNode {
  return {
    id: 'loaded-empty-root',
    label: 'Loaded empty root',
    kind: 'folder',
    childrenState: {
      kind: 'loaded',
      children: []
    }
  }
}

function unloadedBranchNode(): BrowserTreeNode {
  return {
    id: 'unloaded-root',
    label: 'Unloaded root',
    kind: 'folder',
    childrenState: {
      kind: 'unloaded',
      detail: 'Children not loaded yet.'
    }
  }
}

function loadingBranchNode(): BrowserTreeNode {
  return {
    id: 'loading-root',
    label: 'Loading root',
    kind: 'folder',
    childrenState: {
      kind: 'loading',
      detail: 'Loading children.'
    }
  }
}

function failedBranchNode(): BrowserTreeNode {
  return {
    id: 'failed-root',
    label: 'Failed root',
    kind: 'folder',
    childrenState: {
      kind: 'failed',
      detail: 'Unable to load children.'
    }
  }
}

function fileOnlyHierarchyReadResult(): Extract<
  LibraryHierarchyReadChildrenResult,
  { state: 'ready' }
> {
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

function directoryHierarchyReadResult(): Extract<
  LibraryHierarchyReadChildrenResult,
  { state: 'ready' }
> {
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
