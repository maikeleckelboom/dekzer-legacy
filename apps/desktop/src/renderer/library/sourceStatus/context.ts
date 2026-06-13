import { sourceAdmissionOperation } from '../localBrowse/projection'
import type { RowBinding } from '../state'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNode, BrowserTreeNodeId } from '../tree/types'

export type StatusContext =
  | {
      readonly kind: 'none'
    }
  | {
      readonly kind: 'localBrowse'
      readonly title: string
      readonly itemRole: 'folder' | 'file'
      readonly detail?: string
      readonly admission?: StatusAdmission
    }
  | {
      readonly kind: 'registeredSource'
      readonly title: string
      readonly sourceId: string
      readonly nodeId: BrowserTreeNodeId
    }
  | {
      readonly kind: 'registeredDirectory'
      readonly title: string
      readonly sourceId: string
      readonly directoryId: string
      readonly nodeId: BrowserTreeNodeId
    }
  | {
      readonly kind: 'registeredFile'
      readonly title: string
      readonly sourceId: string
      readonly fileId: string
      readonly nodeId: BrowserTreeNodeId
    }
  | {
      readonly kind: 'navigation'
      readonly title: string
      readonly detail?: string
    }
  | {
      readonly kind: 'readState'
      readonly title: string
      readonly detail: string
    }

export type StatusAdmission = {
  readonly resolvedPath: string
  readonly label: 'Add this folder' | 'Add parent folder'
}

export type StatusContextInput = {
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly selectedTitle?: string
}

export function projectStatusContext(input: StatusContextInput): StatusContext {
  const selectedNodeId = input.selectedNodeId
  const projection = input.projection

  if (selectedNodeId === undefined || projection === undefined) {
    return { kind: 'none' }
  }

  const binding = projection.bindingsById.get(selectedNodeId)
  const node = findNode(projection.nodes, selectedNodeId)
  const title = input.selectedTitle ?? node?.label ?? 'Selected item'

  if (binding === undefined) {
    return {
      kind: 'navigation',
      title,
      detail: 'Selection unavailable.'
    }
  }

  return contextForBinding(binding, selectedNodeId, title, node?.detail)
}

function contextForBinding(
  binding: RowBinding,
  nodeId: BrowserTreeNodeId,
  title: string,
  nodeDetail: string | undefined
): StatusContext {
  switch (binding.kind) {
    case 'source':
      if (binding.target.entryPoint.kind !== 'source') {
        return {
          kind: 'navigation',
          title,
          detail: 'Navigation row.'
        }
      }

      return {
        kind: 'registeredSource',
        title,
        sourceId: binding.target.entryPoint.sourceId,
        nodeId
      }
    case 'directory':
      return {
        kind: 'registeredDirectory',
        title,
        sourceId: binding.sourceId,
        directoryId: binding.directoryId,
        nodeId
      }
    case 'file':
      return {
        kind: 'registeredFile',
        title,
        sourceId: binding.sourceId,
        fileId: binding.fileId,
        nodeId
      }
    case 'localBrowseEntryPoint': {
      const entryDetail = binding.entry.identity.resolvedPath ?? nodeDetail
      return {
        kind: 'localBrowse',
        title,
        itemRole: 'folder',
        ...(entryDetail === undefined ? {} : { detail: entryDetail }),
        ...admissionField(binding.entry.availableOperations)
      }
    }
    case 'localBrowseItem':
      return {
        kind: 'localBrowse',
        title,
        itemRole: binding.target === undefined ? 'file' : 'folder',
        detail: binding.item.identity.resolvedItemPath,
        ...admissionField(binding.item.availableOperations)
      }
    case 'readState':
      return {
        kind: 'readState',
        title,
        detail: binding.detail
      }
    case 'navigation':
      return {
        kind: 'navigation',
        title,
        ...(nodeDetail === undefined ? {} : { detail: nodeDetail })
      }
    case 'more':
    case 'localBrowseSection':
    case 'localBrowseMore':
      return {
        kind: 'navigation',
        title,
        ...(nodeDetail === undefined ? {} : { detail: nodeDetail })
      }
  }
}

function admissionField(operations: Parameters<typeof sourceAdmissionOperation>[0]): {
  readonly admission?: StatusAdmission
} {
  const operation = sourceAdmissionOperation(operations)

  if (operation === undefined) {
    return {}
  }

  return {
    admission: {
      resolvedPath: operation.resolvedPath,
      label: operation.requestKind === 'parentDirectory' ? 'Add parent folder' : 'Add this folder'
    }
  }
}

function findNode(
  nodes: readonly BrowserTreeNode[],
  nodeId: BrowserTreeNodeId
): BrowserTreeNode | undefined {
  for (const node of nodes) {
    if (node.id === nodeId) {
      return node
    }

    if (node.children.kind === 'loaded') {
      const child = findNode(node.children.nodes, nodeId)
      if (child !== undefined) {
        return child
      }
    }
  }

  return undefined
}
