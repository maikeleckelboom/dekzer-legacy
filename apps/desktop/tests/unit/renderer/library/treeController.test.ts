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

  it('revealing a deferred branch expands before activating the load action', () => {
    const harness = treeHarness({
      nodes: [deferredBranchNode('source-a', 'Source A')]
    })

    harness.controller.revealNode('source-a')

    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.controller.activeNodeId.value).toBe('source-a')
    expect(harness.expandedNodeIds.value.has('source-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:source-a', 'activateAction:source-a'])
  })

  it('revealing a loaded branch toggles expansion without activating a load action', () => {
    const harness = treeHarness({
      expandedNodeIds: new Set(['branch-a'])
    })

    harness.controller.revealNode('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)

    harness.controller.revealNode('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)

    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:branch-a', 'toggle:branch-a'])
  })

  it('revealing a loading branch keeps it visible without duplicating a load action', () => {
    const harness = treeHarness({
      nodes: [loadingBranchNode('source-a', 'Source A')]
    })

    harness.controller.revealNode('source-a')
    harness.controller.revealNode('source-a')

    expect(harness.expandedNodeIds.value.has('source-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:source-a'])
  })

  it('revealing a failed retryable branch retries and keeps it revealed', () => {
    const harness = treeHarness({
      nodes: [failedBranchNode('source-a', 'Source A', { retryable: true })]
    })

    harness.controller.revealNode('source-a')

    expect(harness.expandedNodeIds.value.has('source-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:source-a', 'activateAction:source-a'])
  })

  it('revealing a failed non-retryable branch preserves the error without a fake load', () => {
    const harness = treeHarness({
      nodes: [failedBranchNode('source-a', 'Source A', { retryable: false })],
      expandedNodeIds: new Set(['source-a'])
    })

    harness.controller.revealNode('source-a')

    expect(harness.expandedNodeIds.value.has('source-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual([])
  })

  it('row-body selection does not expand or load deferred children', () => {
    const harness = treeHarness({
      nodes: [deferredBranchNode('source-a', 'Source A')]
    })

    harness.controller.selectNode('source-a')

    expect(harness.selectedNodeId.value).toBe('source-a')
    expect(harness.expandedNodeIds.value.has('source-a')).toBe(false)
    expect(harness.events.log).toEqual(['select:source-a'])
  })

  it('preparing a visible row emits prefetch intent without mutating tree state', () => {
    const harness = treeHarness()

    harness.controller.prepareNode('branch-a')

    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.prepare).toEqual(['branch-a'])
    expect(harness.events.select).toEqual([])
    expect(harness.events.toggle).toEqual([])
    expect(harness.events.activateAction).toEqual([])
  })

  it('cancelling a prepared row emits cancel intent without selecting or expanding', () => {
    const harness = treeHarness()
    const activeNodeId = harness.controller.activeNodeId.value

    harness.controller.cancelPrepareNode('branch-a')

    expect(harness.controller.activeNodeId.value).toBe(activeNodeId)
    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.cancelPrepare).toEqual(['branch-a'])
    expect(harness.events.select).toEqual([])
    expect(harness.events.toggle).toEqual([])
    expect(harness.events.activateAction).toEqual([])
    expect(harness.events.log).toEqual(['cancelPrepare:branch-a'])
  })

  it('does not prepare rows that are no longer visible', () => {
    const harness = treeHarness()

    harness.controller.prepareNode('missing-row')

    expect(harness.events.prepare).toEqual([])
    expect(harness.events.log).toEqual([])
  })

  it('reveals a newly added row without focusing, selecting, or expanding it', async () => {
    const harness = treeHarness()
    const scrollIntoView = vi.fn()
    const element = { scrollIntoView } as unknown as HTMLElement
    harness.controller.registerItemElement('branch-b', element)

    harness.controller.scrollNodeIntoView('branch-b')
    await Promise.resolve()

    expect(scrollIntoView).toHaveBeenCalledWith({ block: 'nearest' })
    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.expandedNodeIds.value).toEqual(new Set())
    expect(harness.events.log).toEqual([])
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

  it('selects focused branches on Enter without changing expansion', () => {
    const harness = treeHarness({
      expandedNodeIds: new Set(['branch-a'])
    })
    const branch = visibleItem(harness, 'branch-a')
    const intent = harness.controller.resolveKeyboardIntent(branch, 'Enter')

    expect(intent).toMatchObject({ kind: 'select', nodeId: 'branch-a' })
    if (intent.kind === 'select') {
      harness.controller.selectNode(intent.nodeId)
    }

    expect(harness.selectedNodeId.value).toBe('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)
    expect(harness.events.select).toEqual(['branch-a'])
    expect(harness.events.toggle).toEqual([])
    expect(harness.events.activateAction).toEqual([])
  })

  it('keeps primary activation select-only for non-action rows', () => {
    const harness = treeHarness({
      expandedNodeIds: new Set(['branch-a'])
    })

    harness.controller.activatePrimary('branch-a')

    expect(harness.selectedNodeId.value).toBe('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)
    expect(harness.events.log).toEqual(['select:branch-a'])
  })

  it('keeps Enter activation for explicit action rows', () => {
    const harness = treeHarness({
      nodes: [actionNode('action-a', 'Load more')]
    })
    const action = visibleItem(harness, 'action-a')
    const intent = harness.controller.resolveKeyboardIntent(action, 'Enter')

    expect(intent).toMatchObject({ kind: 'activateAction', nodeId: 'action-a' })
    if (intent.kind === 'activateAction') {
      harness.controller.activateAction(intent.nodeId)
    }

    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.events.log).toEqual(['activateAction:action-a'])
  })

  it('keeps ArrowRight and ArrowLeft expansion intents on the reveal and toggle paths', () => {
    const harness = treeHarness()
    const collapsedBranch = visibleItem(harness, 'branch-a')
    const expandIntent = harness.controller.resolveKeyboardIntent(collapsedBranch, 'ArrowRight')

    expect(expandIntent).toMatchObject({ kind: 'revealNode', nodeId: 'branch-a' })
    if (expandIntent.kind === 'revealNode') {
      harness.controller.revealNode(expandIntent.nodeId)
    }
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(true)

    const expandedBranch = visibleItem(harness, 'branch-a')
    const expandedRightIntent = harness.controller.resolveKeyboardIntent(
      expandedBranch,
      'ArrowRight'
    )

    expect(expandedRightIntent).toEqual({ kind: 'none', shouldPreventDefault: true })
    expect(harness.controller.activeNodeId.value).toBe('branch-a')

    const collapseIntent = harness.controller.resolveKeyboardIntent(expandedBranch, 'ArrowLeft')

    expect(collapseIntent).toMatchObject({ kind: 'collapse', nodeId: 'branch-a' })
    if (collapseIntent.kind === 'collapse') {
      harness.controller.toggleNode(collapseIntent.nodeId)
    }
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.select).toEqual([])
    expect(harness.events.toggle).toEqual(['branch-a', 'branch-a'])
  })

  it('keeps ArrowLeft on collapsed branches as a no-op', () => {
    const harness = treeHarness()
    const branch = visibleItem(harness, 'branch-a')
    const intent = harness.controller.resolveKeyboardIntent(branch, 'ArrowLeft')

    expect(intent).toEqual({ kind: 'none', shouldPreventDefault: true })
    expect(harness.controller.activeNodeId.value).toBe('branch-a')
    expect(harness.expandedNodeIds.value.has('branch-a')).toBe(false)
    expect(harness.events.log).toEqual([])
  })

  it('does not introduce Space as tree selection or activation', () => {
    const harness = treeHarness()
    const branch = visibleItem(harness, 'branch-a')

    for (const key of [' ', 'Space', 'Spacebar']) {
      expect(harness.controller.resolveKeyboardIntent(branch, key)).toEqual({
        kind: 'none',
        shouldPreventDefault: false
      })
    }

    expect(harness.selectedNodeId.value).toBeUndefined()
    expect(harness.events.select).toEqual([])
    expect(harness.events.activateAction).toEqual([])
  })

  it('routes ArrowRight on a deferred branch through revealNode', () => {
    const harness = treeHarness({
      nodes: [deferredBranchNode('source-a', 'Source A')]
    })
    const deferredBranch = visibleItem(harness, 'source-a')
    const intent = harness.controller.resolveKeyboardIntent(deferredBranch, 'ArrowRight')

    expect(intent).toMatchObject({ kind: 'revealNode', nodeId: 'source-a' })
    if (intent.kind === 'revealNode') {
      harness.controller.revealNode(intent.nodeId)
    }

    expect(harness.expandedNodeIds.value.has('source-a')).toBe(true)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:source-a', 'activateAction:source-a'])
  })

  it('collapses an expanded deferred branch with ArrowLeft', () => {
    const harness = treeHarness({
      nodes: [deferredBranchNode('source-a', 'Source A')],
      expandedNodeIds: new Set(['source-a'])
    })
    const deferredBranch = visibleItem(harness, 'source-a')
    const intent = harness.controller.resolveKeyboardIntent(deferredBranch, 'ArrowLeft')

    expect(intent).toMatchObject({ kind: 'collapse', nodeId: 'source-a' })
    if (intent.kind === 'collapse') {
      harness.controller.toggleNode(intent.nodeId)
    }

    expect(harness.expandedNodeIds.value.has('source-a')).toBe(false)
    expect(harness.events.select).toEqual([])
    expect(harness.events.log).toEqual(['toggle:source-a'])
  })

  it('does not expand or load unknown child-readiness rows through reveal or ArrowRight', () => {
    const harness = treeHarness({
      nodes: [unknownChildReadinessNode('folder-a', 'Folder A')]
    })
    const unknownRow = visibleItem(harness, 'folder-a')
    const intent = harness.controller.resolveKeyboardIntent(unknownRow, 'ArrowRight')

    expect(unknownRow).toMatchObject({
      isBranch: false,
      canRevealChildren: false,
      isExpanded: false
    })
    expect(unknownRow.childReadinessNode).toMatchObject({
      role: 'state',
      label: 'Folder child scopes pending'
    })
    expect(intent).toEqual({ kind: 'none', shouldPreventDefault: true })

    harness.controller.revealNode('folder-a')
    harness.controller.activatePrimary('folder-a')

    expect(harness.expandedNodeIds.value.has('folder-a')).toBe(false)
    expect(harness.events.toggle).toEqual([])
    expect(harness.events.activateAction).toEqual([])
    expect(harness.events.select).toEqual(['folder-a'])
  })
})

function treeHarness(
  options: {
    readonly nodes?: readonly BrowserTreeNode[]
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
    readonly prepare: BrowserTreeNodeId[]
    readonly cancelPrepare: BrowserTreeNodeId[]
    readonly log: string[]
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
    activateAction: [] as BrowserTreeNodeId[],
    prepare: [] as BrowserTreeNodeId[],
    cancelPrepare: [] as BrowserTreeNodeId[],
    log: [] as string[]
  }

  const activateAction = vi.fn((nodeId: BrowserTreeNodeId) => {
    events.activateAction.push(nodeId)
    events.log.push(`activateAction:${nodeId}`)
  })

  const controller = useTreeController({
    nodes: computed(() => options.nodes ?? treeNodes()),
    selectedNodeId: computed(() => selectedNodeId.value),
    expandedNodeIds: computed(() => expandedNodeIds.value),
    selectNode: (nodeId) => {
      events.select.push(nodeId)
      events.log.push(`select:${nodeId}`)
      selectedNodeId.value = nodeId
    },
    toggleNode: (nodeId) => {
      events.toggle.push(nodeId)
      events.log.push(`toggle:${nodeId}`)
      const nextExpandedIds = new Set(expandedNodeIds.value)

      if (nextExpandedIds.has(nodeId)) {
        nextExpandedIds.delete(nodeId)
      } else {
        nextExpandedIds.add(nodeId)
      }

      expandedNodeIds.value = nextExpandedIds
    },
    activateAction,
    prepareNode: (nodeId) => {
      events.prepare.push(nodeId)
      events.log.push(`prepare:${nodeId}`)
    },
    cancelPrepareNode: (nodeId) => {
      events.cancelPrepare.push(nodeId)
      events.log.push(`cancelPrepare:${nodeId}`)
    }
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

function actionNode(id: BrowserTreeNodeId, label: string): BrowserTreeNode {
  return {
    id,
    label,
    role: 'action',
    icon: 'more',
    children: { kind: 'none' },
    action: { kind: 'loadMore', state: { kind: 'idle' } }
  }
}

function deferredBranchNode(id: BrowserTreeNodeId, label: string): BrowserTreeNode {
  return {
    id,
    label,
    role: 'source',
    icon: 'source',
    children: {
      kind: 'deferred',
      stateNode: stateNode(id, 'Source contents not loaded', 'state')
    },
    action: { kind: 'loadChildren', state: { kind: 'idle' } }
  }
}

function loadingBranchNode(id: BrowserTreeNodeId, label: string): BrowserTreeNode {
  return {
    id,
    label,
    role: 'source',
    icon: 'source',
    children: {
      kind: 'loading',
      stateNode: stateNode(id, 'Loading source contents', 'loading')
    }
  }
}

function failedBranchNode(
  id: BrowserTreeNodeId,
  label: string,
  options: { readonly retryable: boolean }
): BrowserTreeNode {
  return {
    id,
    label,
    role: 'source',
    icon: 'source',
    children: {
      kind: 'failed',
      stateNode: stateNode(id, 'Hierarchy read failed', 'warning')
    },
    ...(options.retryable
      ? {
          action: {
            kind: 'loadChildren' as const,
            state: { kind: 'failed' as const, detail: 'Retry' }
          }
        }
      : {})
  }
}

function unknownChildReadinessNode(id: BrowserTreeNodeId, label: string): BrowserTreeNode {
  return {
    id,
    label,
    role: 'literalDirectory',
    icon: 'folder',
    children: {
      kind: 'unknown',
      stateNode: stateNode(id, 'Folder child scopes pending', 'loading')
    }
  }
}

function stateNode(
  parentNodeId: BrowserTreeNodeId,
  label: string,
  icon: NonNullable<BrowserTreeNode['icon']>
): BrowserTreeNode {
  return {
    id: `read-state:${parentNodeId}`,
    label,
    role: 'state',
    icon,
    children: { kind: 'none' }
  }
}
