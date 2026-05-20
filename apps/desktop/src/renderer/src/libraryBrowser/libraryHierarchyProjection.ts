import type { LibraryHierarchyReadResult } from '../../../shared/libraryHierarchyRead'
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

  const firstDirectory = result.window.nodes.find((node) => node.kind === 'directory')

  if (firstDirectory !== undefined) {
    return {
      kind: 'unsupported',
      message: `Folder "${firstDirectory.label}" needs unloaded child state before it can be shown as a live tree row.`
    }
  }

  const children = result.window.nodes.map(
    (node): BrowserTreeNode => ({
      id: node.id,
      label: node.label,
      kind: 'file',
      detail: formatNodeDetail(node.presenceState)
    })
  )

  return {
    kind: 'tree',
    nodes: [
      {
        id: result.window.root.id,
        label: result.window.root.label,
        kind: 'source',
        detail: formatRootDetail(result.window.totalRows),
        ...(children.length > 0 ? { children } : {})
      }
    ]
  }
}

function formatRootDetail(totalRows: number): string {
  if (totalRows === 1) {
    return '1 literal hierarchy row loaded read-only.'
  }

  return `${totalRows} literal hierarchy rows loaded read-only.`
}

function formatNodeDetail(presenceState: string): string {
  switch (presenceState) {
    case 'present':
      return 'Present file.'
    case 'missing':
      return 'Missing file.'
    case 'removed':
      return 'Removed file.'
    default:
      return 'File.'
  }
}
