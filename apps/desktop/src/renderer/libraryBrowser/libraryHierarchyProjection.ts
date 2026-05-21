import type {
  LibraryHierarchyReadNode,
  LibraryHierarchyReadResult
} from '../../shared/libraryHierarchyRead'
import type { BrowserTreeNode } from './tree/types'

export type LibraryHierarchyBrowserProjection =
  | {
      readonly kind: 'tree'
      readonly nodes: readonly BrowserTreeNode[]
    }
  | {
      readonly kind: 'unavailable'
      readonly message: string
    }
  | {
      readonly kind: 'unsupported'
      readonly message: string
    }

export function projectLibraryHierarchyReadToBrowserTree(
  result: LibraryHierarchyReadResult
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

  const children = result.window.nodes.map(projectReadNodeToBrowserTreeNode)

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
    ]
  }
}

function formatRootDetail(totalRows: number): string {
  if (totalRows === 1) {
    return 'Loaded read-only from library backend. 1 literal hierarchy row available.'
  }

  return `Loaded read-only from library backend. ${totalRows} literal hierarchy rows available.`
}

function projectReadNodeToBrowserTreeNode(node: LibraryHierarchyReadNode): BrowserTreeNode {
  if (node.kind === 'directory') {
    return {
      id: node.id,
      label: node.label,
      kind: 'folder',
      detail: formatDirectoryDetail(node.presenceState),
      childrenState: {
        kind: 'unloaded',
        detail: 'Children not loaded yet.'
      }
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

function formatFileDetail(presenceState: LibraryHierarchyReadNode['presenceState']): string {
  switch (presenceState) {
    case 'present':
      return 'Present file.'
    case 'missing':
      return 'Missing file.'
    case 'removed':
      return 'Removed file.'
  }
}

function formatDirectoryDetail(presenceState: LibraryHierarchyReadNode['presenceState']): string {
  switch (presenceState) {
    case 'present':
      return 'Present directory.'
    case 'missing':
      return 'Missing directory.'
    case 'removed':
      return 'Removed directory.'
  }
}
