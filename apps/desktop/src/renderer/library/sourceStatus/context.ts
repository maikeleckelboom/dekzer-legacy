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
      readonly localState: LocalBrowseStatusState
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
  readonly label: 'Add as music source' | 'Add parent as music source'
}

export type LocalBrowseStatusState =
  | 'localBrowseOnly'
  | 'eligible'
  | 'broadRoot'
  | 'protected'
  | 'resolutionFailed'
  | 'alreadyAdded'

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
      const localState = localBrowseEntryState(binding.entry)
      return {
        kind: 'localBrowse',
        title,
        itemRole: 'folder',
        localState,
        ...(entryDetail === undefined ? {} : { detail: entryDetail }),
        ...admissionField(binding.entry.availableOperations, localState)
      }
    }
    case 'localBrowseItem': {
      const localState = localBrowseItemState(binding.item)
      return {
        kind: 'localBrowse',
        title,
        itemRole: binding.target === undefined ? 'file' : 'folder',
        localState,
        detail: binding.item.failure?.detail ?? binding.item.identity.resolvedItemPath,
        ...admissionField(binding.item.availableOperations, localState)
      }
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
    case 'localBrowseMore':
      return {
        kind: 'navigation',
        title,
        ...(nodeDetail === undefined ? {} : { detail: nodeDetail })
      }
    case 'localBrowseSection':
      return {
        kind: 'navigation',
        title,
        detail: nodeDetail ?? 'Choose a folder to add as a music source.'
      }
  }
}

function admissionField(
  operations: Parameters<typeof sourceAdmissionOperation>[0],
  localState: LocalBrowseStatusState
): {
  readonly admission?: StatusAdmission
} {
  if (localState !== 'eligible') {
    return {}
  }

  const operation = sourceAdmissionOperation(operations)

  if (operation === undefined) {
    return {}
  }

  return {
    admission: {
      resolvedPath: operation.resolvedPath,
      label:
        operation.requestKind === 'parentDirectory'
          ? 'Add parent as music source'
          : 'Add as music source'
    }
  }
}

function localBrowseEntryState(
  entry: Extract<RowBinding, { readonly kind: 'localBrowseEntryPoint' }>['entry']
): LocalBrowseStatusState {
  if (entry.status === 'duplicateOfAdmittedSource') {
    return 'alreadyAdded'
  }

  if (entry.status !== 'available' && entry.status !== 'resolving') {
    return 'resolutionFailed'
  }

  if (entry.identity.entryPointKind === 'systemDriveRoot') {
    return 'broadRoot'
  }

  return sourceAdmissionOperation(entry.availableOperations) === undefined
    ? 'localBrowseOnly'
    : 'eligible'
}

function localBrowseItemState(
  item: Extract<RowBinding, { readonly kind: 'localBrowseItem' }>['item']
): LocalBrowseStatusState {
  if (item.status === 'duplicateOfAdmittedSource') {
    return 'alreadyAdded'
  }

  if (item.itemKind === 'rejectedRoot' || item.status === 'rejected') {
    return 'protected'
  }

  if (item.status === 'permissionBlocked') {
    return 'protected'
  }

  if (item.status !== 'available' && item.status !== 'unknown') {
    return 'resolutionFailed'
  }

  if (item.status === 'unknown' || item.itemKind === 'unknown') {
    return 'resolutionFailed'
  }

  if (
    isUnsafeSystemDriveAdmission(
      item.identity.entryPointKind,
      item.identity.resolvedRootPath,
      item.availableOperations
    )
  ) {
    return 'broadRoot'
  }

  return sourceAdmissionOperation(item.availableOperations) === undefined
    ? 'localBrowseOnly'
    : 'eligible'
}

function isUnsafeSystemDriveAdmission(
  entryPointKind: string,
  resolvedRootPath: string,
  operations: Parameters<typeof sourceAdmissionOperation>[0]
): boolean {
  const operation = sourceAdmissionOperation(operations)

  return (
    entryPointKind === 'systemDriveRoot' &&
    operation !== undefined &&
    normalizePathKey(operation.resolvedPath) === normalizePathKey(resolvedRootPath)
  )
}

function normalizePathKey(path: string): string {
  return path.replaceAll('/', '\\').replace(/\\+$/, '').toLowerCase()
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
