import type {
  LibraryHierarchyReadChildrenNode,
  LibraryHierarchyReadChildrenResult
} from '../../shared/libraryHierarchy/readChildren'
import type {
  LibraryHierarchyBrowserState,
  LibraryHierarchyDirectoryReadState,
  LibraryHierarchyDirectoryReadTarget
} from './hierarchyState'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'

export type LibraryHierarchyBrowserProjection =
  | {
      readonly kind: 'tree'
      readonly nodes: readonly BrowserTreeNode[]
      readonly directoryReadTargetsByNodeId: ReadonlyMap<
        BrowserTreeNodeId,
        LibraryHierarchyDirectoryReadTarget
      >
    }
  | {
      readonly kind: 'unavailable'
      readonly message: string
    }
  | {
      readonly kind: 'unsupported'
      readonly message: string
    }

export function projectBrowserState(
  state: LibraryHierarchyBrowserState
): LibraryHierarchyBrowserProjection | undefined {
  if (state.rootReadResult === undefined) {
    return undefined
  }

  return projectResult(state.rootReadResult, state.directoryReadStates)
}

export function projectReadResult(
  result: LibraryHierarchyReadChildrenResult
): LibraryHierarchyBrowserProjection {
  return projectResult(result, new Map())
}

function projectResult(
  result: LibraryHierarchyReadChildrenResult,
  directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
): LibraryHierarchyBrowserProjection {
  if (result.state !== 'ready') {
    return {
      kind: 'unavailable',
      message: result.error.message
    }
  }

  if (result.window.root.label === undefined) {
    return {
      kind: 'unsupported',
      message: 'The hierarchy read target does not have a display label.'
    }
  }

  if (result.window.offset !== 0 || result.window.nodes.length !== result.window.totalRows) {
    return {
      kind: 'unsupported',
      message: 'The hierarchy read returned a partial window.'
    }
  }

  const directoryReadTargetsByNodeId = new Map<
    BrowserTreeNodeId,
    LibraryHierarchyDirectoryReadTarget
  >()
  const children = projectNodes({
    nodes: result.window.nodes,
    directoryReadStates,
    directoryReadTargetsByNodeId
  })

  return {
    kind: 'tree',
    nodes: [
      {
        id: result.window.root.id,
        label: result.window.root.label,
        kind: 'source',
        detail: formatRootDetail(result.window.totalRows),
        childrenState: {
          kind: 'loaded',
          children
        }
      }
    ],
    directoryReadTargetsByNodeId
  }
}

function formatRootDetail(totalRows: number): string {
  if (totalRows === 1) {
    return 'Loaded read-only from library backend. 1 literal hierarchy row available.'
  }

  return `Loaded read-only from library backend. ${totalRows} literal hierarchy rows available.`
}

function projectNodes(options: {
  readonly nodes: readonly LibraryHierarchyReadChildrenNode[]
  readonly directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, LibraryHierarchyDirectoryReadTarget>
}): readonly BrowserTreeNode[] {
  return options.nodes.map((node) =>
    projectNode({
      node,
      directoryReadStates: options.directoryReadStates,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
    })
  )
}

function projectNode(options: {
  readonly node: LibraryHierarchyReadChildrenNode
  readonly directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, LibraryHierarchyDirectoryReadTarget>
}): BrowserTreeNode {
  const node = options.node

  if (node.kind === 'directory') {
    options.directoryReadTargetsByNodeId.set(node.id, {
      sourceDirectoryId: node.sourceDirectoryId
    })

    return {
      id: node.id,
      label: node.label,
      kind: 'folder',
      detail: formatDirectoryDetail(node.presenceState),
      childrenState: projectDirectoryChildren({
        state: options.directoryReadStates.get(node.sourceDirectoryId),
        directoryReadStates: options.directoryReadStates,
        directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
      })
    }
  }

  return {
    id: node.id,
    label: node.label,
    kind: 'file',
    detail: formatFileDetail(node.presenceState),
    childrenState: { kind: 'leaf' }
  }
}

function projectDirectoryChildren(options: {
  readonly state: LibraryHierarchyDirectoryReadState | undefined
  readonly directoryReadStates: ReadonlyMap<string, LibraryHierarchyDirectoryReadState>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, LibraryHierarchyDirectoryReadTarget>
}): BrowserTreeNode['childrenState'] {
  const state = options.state

  if (state === undefined || state.kind === 'unloaded') {
    return {
      kind: 'unloaded',
      detail: state?.detail ?? 'Children not loaded yet.'
    }
  }

  if (state.kind === 'loading') {
    return {
      kind: 'loading',
      detail: state.detail ?? 'Loading children.'
    }
  }

  if (state.kind === 'failed') {
    return {
      kind: 'failed',
      detail: state.detail
    }
  }

  if (!isCompleteWindow(state.window)) {
    return {
      kind: 'failed',
      detail: 'The hierarchy read returned a partial child window.'
    }
  }

  return {
    kind: 'loaded',
    children: projectNodes({
      nodes: state.window.nodes,
      directoryReadStates: options.directoryReadStates,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
    })
  }
}

function isCompleteWindow(window: {
  readonly offset: number
  readonly nodes: readonly unknown[]
  readonly totalRows: number
}): boolean {
  return window.offset === 0 && window.nodes.length === window.totalRows
}

function formatFileDetail(
  presenceState: LibraryHierarchyReadChildrenNode['presenceState']
): string {
  switch (presenceState) {
    case 'present':
      return 'Present file.'
    case 'missing':
      return 'Missing file.'
    case 'removed':
      return 'Removed file.'
  }
}

function formatDirectoryDetail(
  presenceState: LibraryHierarchyReadChildrenNode['presenceState']
): string {
  switch (presenceState) {
    case 'present':
      return 'Present directory.'
    case 'missing':
      return 'Missing directory.'
    case 'removed':
      return 'Removed directory.'
  }
}
