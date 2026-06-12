import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { DirectoryState, SourceState } from '../state'

export type DisclosureReconciler = {
  readonly reconcile: (input: DisclosureReconciliationInput) => readonly BrowserTreeNodeId[]
  readonly getLedgerEntry: (nodeId: BrowserTreeNodeId) => LedgerEntry | undefined
  readonly clearFailed: () => void
  readonly clearFailedForNode: (nodeId: BrowserTreeNodeId) => void
}

export type LedgerEntry =
  | {
      readonly status: 'pending'
    }
  | {
      readonly status: 'failed'
      readonly reason: string
      readonly failedAt: number
    }

export type DisclosureReconciliationInput = {
  readonly projection: BrowserProjection | undefined
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
}

export function createDisclosureReconciler(options: {
  readonly requestNodeChildren: (nodeId: BrowserTreeNodeId) => Promise<boolean> | boolean | void
  readonly now?: () => number
}): DisclosureReconciler {
  const ledger = new Map<BrowserTreeNodeId, LedgerEntry>()
  const now = options.now ?? Date.now

  function reconcile(input: DisclosureReconciliationInput): readonly BrowserTreeNodeId[] {
    const requestedThisPass: BrowserTreeNodeId[] = []

    if (input.projection?.kind !== 'tree') {
      ledger.clear()
      return requestedThisPass
    }

    const bindableExpandedNodeIds = new Set<BrowserTreeNodeId>()
    const loadedExpandedNodeIds = new Set<BrowserTreeNodeId>()

    for (const nodeId of input.expandedNodeIds) {
      const binding = input.projection.bindingsById.get(nodeId)

      if (binding?.kind === 'source') {
        bindableExpandedNodeIds.add(nodeId)
        const state = input.sourceReadStates.get(nodeId)
        reconcileLedgerFailure(nodeId, state)

        if (state?.kind === 'loaded') {
          loadedExpandedNodeIds.add(nodeId)
        }

        if (!hasLoadedOrInFlightChildren(state) && !ledger.has(nodeId)) {
          ledger.set(nodeId, { status: 'pending' })
          requestedThisPass.push(nodeId)
          void options.requestNodeChildren(nodeId)
        }
      } else if (binding?.kind === 'directory') {
        bindableExpandedNodeIds.add(nodeId)
        const state = input.directoryReadStates.get(binding.directoryId)
        reconcileLedgerFailure(nodeId, state)

        if (state?.kind === 'loaded') {
          loadedExpandedNodeIds.add(nodeId)
        }

        if (!hasLoadedOrInFlightChildren(state) && !ledger.has(nodeId)) {
          ledger.set(nodeId, { status: 'pending' })
          requestedThisPass.push(nodeId)
          void options.requestNodeChildren(nodeId)
        }
      }
    }

    for (const nodeId of ledger.keys()) {
      if (
        !input.expandedNodeIds.has(nodeId) ||
        !bindableExpandedNodeIds.has(nodeId) ||
        loadedExpandedNodeIds.has(nodeId)
      ) {
        ledger.delete(nodeId)
      }
    }

    return requestedThisPass
  }

  function reconcileLedgerFailure(
    nodeId: BrowserTreeNodeId,
    state: SourceState | DirectoryState | undefined
  ): void {
    const entry = ledger.get(nodeId)

    if (entry?.status !== 'pending' || state?.kind !== 'failed') {
      return
    }

    ledger.set(nodeId, {
      status: 'failed',
      reason: state.detail.length > 0 ? state.detail : state.errorCode,
      failedAt: now()
    })
  }

  function getLedgerEntry(nodeId: BrowserTreeNodeId): LedgerEntry | undefined {
    const entry = ledger.get(nodeId)
    return entry === undefined ? undefined : { ...entry }
  }

  function clearFailed(): void {
    for (const [nodeId, entry] of ledger) {
      if (entry.status === 'failed') {
        ledger.delete(nodeId)
      }
    }
  }

  function clearFailedForNode(nodeId: BrowserTreeNodeId): void {
    if (ledger.get(nodeId)?.status === 'failed') {
      ledger.delete(nodeId)
    }
  }

  return { reconcile, getLedgerEntry, clearFailed, clearFailedForNode }
}

function hasLoadedOrInFlightChildren(state: SourceState | DirectoryState | undefined): boolean {
  return state?.kind === 'loaded' || state?.kind === 'loading' || state?.kind === 'refreshing'
}
