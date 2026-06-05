import { computed, ref, shallowRef, type Ref, type ShallowRef } from 'vue'
import { describe, expect, it, vi } from 'vitest'

import { useTreeController } from '../../../../src/renderer/library/tree/controller'
import type {
  BrowserTreeNode,
  BrowserTreeNodeId
} from '../../../../src/renderer/library/tree/types'

describe('useTreeController', () => {
  it('selecting a row does not toggle expansion', () => {
    const harness = treeHarness()

    harness.controller.selectNode('branch-a')

    expect(harness.selectedNodeId.value).toBe('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.select).toEqual(['branch-a'])
    expect(harness.events.toggle).toEqual([])
    expect(harness.events.activateAction).toEqual([])
  })

  it('toggling disclosure does not select or activate the row', () => {
    const harness = treeHarness()

    harness.controller.toggleNode('branch-a')

    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.toggle).toEqual(['branch-a'])
    expect(harness.events.activateAction).toEqual([])
  })

  it('keeps expansion stable when selecting another row', () => {
    const harness = treeHarness({
      expandedNodeIds: new Set(['branch-a'])
    })

    harness.controller.selectNode('branch-b')

    expect(harness.selectedNodeId.value).toBe('branch-b')
    expect([...harness.expandedNodeIds.value]).toEqual(['branch-a'])
    expect(harness.events.toggle).toEqual([])
  })

  it('keeps existing keyboard primary activation contract separate from pointer selection', () => {
    const harness = treeHarness({
      expandedNodeIds: new Set(['branch-a'])
    })

    harness.controller.activatePrimary('branch-a')

    expect(harness.selectedNodeId.value).toBe('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.select).toEqual(['branch-a'])
    expect(harness.events.toggle).toEqual(['branch-a'])
  })

  it('keeps ArrowRight and ArrowLeft expansion intents on the toggle path', () => {
    const harness = treeHarness()
    const collapsedBranch = visibleItem(harness, 'branch-a')
    const expandIntent = harness.controller.resolveKeyboardIntent(collapsedBranch, 'ArrowRight')

    expect(expandIntent).toMatchObject({ kind: 'expand', nodeId: 'branch-a' })
    if (expandIntent.kind === 'expand') {
      harness.controller.toggleNode(expandIntent.nodeId)
    }
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)

    const expandedBranch = visibleItem(harness, 'branch-a')
    const collapseIntent = harness.controller.resolveKeyboardIntent(expandedBranch, 'ArrowLeft')

    expect(collapseIntent).toMatchObject({ kind: 'collapse', nodeId: 'branch-a' })
    if (collapseIntent.kind === 'collapse') {
      harness.controller.toggleNode(collapseIntent.nodeId)
    }
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.select).toEqual([])
    expect(harness.events.toggle).toEqual(['branch-a', 'branch-a'])
  })
})

function treeHarness(
  options: {
    readonly selectedNodeId?: BrowserTreeNodeId
    readonly expandedNodeIds?: ReadonlySet<BrowserTreeNodeId>
  } = {}
): {
  readonly selectedNodeId: Ref<BrowserTreeNodeId | undefined>
  readonly expandedNodeIds: ShallowRef<ReadonlySet<BrowserTreeNodeId>>
  readonly events: {
    readonly select: BrowserTreeNodeId[]
    readonly toggle: BrowserTreeNodeId[]
    readonly activateAction: BrowserTreeNodeId[]
  }
  readonly controller: ReturnType<typeof useTreeController>
} {
  const selectedNodeId = ref<BrowserTreeNodeId | undefined>(options.selectedNodeId)
  const expandedNodeIds = shallowRef<ReadonlySet<BrowserTreeNodeId>>(
    options.expandedNodeIds ?? new Set()
  )
  const events = {
    select: [] as BrowserTreeNodeId[],
    toggle: [] as BrowserTreeNodeId[],
    activateAction: [] as BrowserTreeNodeId[]
  }

  const activateAction = vi.fn((nodeId: BrowserTreeNodeId) => {
    events.activateAction.push(nodeId)
  })

  const controller = useTreeController({
    nodes: computed(() => treeNodes()),
    selectedNodeId: computed(() => selectedNodeId.value),
    expandedNodeIds: computed(() => expandedNodeIds.value),
    selectNode: (nodeId) => {
      events.select.push(nodeId)
      selectedNodeId.value = nodeId
    },
    toggleNode: (nodeId) => {
      events.toggle.push(nodeId)
      const nextExpandedIds = new Set(expandedNodeIds.value)

      if (nextExpandedIds.has(nodeId)) {
        nextExpandedIds.delete(nodeId)
      } else {
        nextExpandedIds.add(nodeId)
      }

      expandedNodeIds.value = nextExpandedIds
    },
    activateAction
  })

  return {
    selectedNodeId,
    expandedNodeIds,
    events,
    controller
  }
}

function visibleItem(
  harness: ReturnType<typeof treeHarness>,
  nodeId: BrowserTreeNodeId
): ReturnType<typeof useTreeController>['visibleItems']['value'][number] {
  const item = harness.controller.visibleItems.value.find((visible) => visible.id === nodeId)

  expect(item).toBeDefined()
  if (item === undefined) {
    throw new Error(`Expected visible item ${nodeId}.`)
  }

  return item
}

function treeNodes(): readonly BrowserTreeNode[] {
  return [
    branchNode('branch-a', 'Branch A', [leafNode('leaf-a')]),
    branchNode('branch-b', 'Branch B', [leafNode('leaf-b')])
  ]
}

function branchNode(
  id: BrowserTreeNodeId,
  label: string,
  nodes: readonly BrowserTreeNode[]
): BrowserTreeNode {
  return {
    id,
    label,
    role: 'literalDirectory',
    icon: 'folder',
    children: {
      kind: 'loaded',
      nodes
    }
  }
}

function leafNode(id: BrowserTreeNodeId): BrowserTreeNode {
  return {
    id,
    label: id,
    role: 'literalDirectory',
    icon: 'folder',
    children: { kind: 'none' }
  }
}
