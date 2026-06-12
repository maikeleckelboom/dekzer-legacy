import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { DirectoryState, SourceState } from '../state'

export type DisclosureReconciler = {
  readonly reconcile: (input: DisclosureReconciliationInput) => readonly BrowserTreeNodeId[]
}

export type DisclosureReconciliationInput = {
  readonly projection: BrowserProjection | undefined
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
}

export function createDisclosureReconciler(options: {
  readonly requestNodeChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean> | boolean | void
}): DisclosureReconciler {
  const requestedNodeIds = new Set<BrowserTreeNodeId>()

  function reconcile(input: DisclosureReconciliationInput): readonly BrowserTreeNodeId[] {
    const requestedThisPass: BrowserTreeNodeId[] = []

    if (input.projection?.kind !== 'tree') {
      requestedNodeIds.clear()
      return requestedThisPass
    }

    const bindableExpandedNodeIds = new Set<BrowserTreeNodeId>()
    const loadedExpandedNodeIds = new Set<BrowserTreeNodeId>()

    for (const nodeId of input.expandedNodeIds) {
      const binding = input.projection.bindingsById.get(nodeId)

      if (binding?.kind === 'source') {
        bindableExpandedNodeIds.add(nodeId)
        const state = input.sourceReadStates.get(nodeId)

        if (state?.kind === 'loaded') {
          loadedExpandedNodeIds.add(nodeId)
        }

        if (!hasLoadedOrInFlightChildren(state) && !requestedNodeIds.has(nodeId)) {
          requestedNodeIds.add(nodeId)
          requestedThisPass.push(nodeId)
          void options.requestNodeChildren(nodeId)
        }
      } else if (binding?.kind === 'directory') {
        bindableExpandedNodeIds.add(nodeId)
        const state = input.directoryReadStates.get(binding.directoryId)

        if (state?.kind === 'loaded') {
          loadedExpandedNodeIds.add(nodeId)
        }

        if (!hasLoadedOrInFlightChildren(state) && !requestedNodeIds.has(nodeId)) {
          requestedNodeIds.add(nodeId)
          requestedThisPass.push(nodeId)
          void options.requestNodeChildren(nodeId)
        }
      }
    }

    for (const nodeId of requestedNodeIds) {
      if (
        !input.expandedNodeIds.has(nodeId) ||
        !bindableExpandedNodeIds.has(nodeId) ||
        loadedExpandedNodeIds.has(nodeId)
      ) {
        requestedNodeIds.delete(nodeId)
      }
    }

    return requestedThisPass
  }

  return { reconcile }
}

function hasLoadedOrInFlightChildren(state: SourceState | DirectoryState | undefined): boolean {
  return state?.kind === 'loaded' || state?.kind === 'loading' || state?.kind === 'refreshing'
}
