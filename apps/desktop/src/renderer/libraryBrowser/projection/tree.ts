import type { EntryPoint, ChildRow } from '../../../shared/libraryHierarchy/readChildren'
import type { NavigationRow } from '../../../shared/libraryNavigation/readRows'
import type {
  DirectoryState,
  LoadedChildren,
  MoreTarget,
  RowBinding,
  BrowserState,
  SourceState,
  SourceTarget
} from '../runtime/state'
import type {
  BrowserTreeAction,
  BrowserTreeActionState,
  BrowserTreeBadgeTone,
  BrowserTreeChildren,
  BrowserTreeIcon,
  BrowserTreeNode,
  BrowserTreeNodeId
} from '../tree/types'
import { copyEntryPoint } from '../runtime/entryPoint'
import { adaptLocationSourceDescriptor, getLocationSourcePresentation } from './sourcePresentation'
import { classifyLibraryEntryName, type LibraryEntryRole } from './entryPresentation'
import { browserRowRoleForNavigationRow } from './rowRoles'

export type BrowserProjection = {
  readonly kind: 'tree'
  readonly nodes: readonly BrowserTreeNode[]
  readonly bindingsById: ReadonlyMap<BrowserTreeNodeId, RowBinding>
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/

type ProjectedNodeState = {
  readonly children: BrowserTreeChildren
  readonly action?: BrowserTreeAction
}

export function projectState(state: BrowserState): BrowserProjection | undefined {
  if (state.navigationReadResult === undefined) {
    return undefined
  }

  return projectNavigationResult(state)
}

function projectNavigationResult(state: BrowserState): BrowserProjection {
  const bindingsById = new Map<BrowserTreeNodeId, RowBinding>()
  const result = state.navigationReadResult

  if (result === undefined) {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'loading',
      label: 'Loading library',
      detail: 'Loading your library.'
    })
  }

  if (result.state !== 'ready') {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'error',
      label: 'Library unavailable',
      detail: result.error.message
    })
  }

  if (result.rows.length === 0) {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'empty',
      label: 'No library sources',
      detail: 'Add a music folder to start building your library.'
    })
  }

  return {
    kind: 'tree',
    nodes: result.rows.map((row) =>
      projectNavigationRow({
        row,
        sourceReadStates: state.sourceReadStates,
        directoryReadStates: state.directoryReadStates,
        bindingsById
      })
    ),
    bindingsById
  }
}

function projectNavigationRow(options: {
  readonly row: NavigationRow
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const nodeId = navigationNodeId(options.row)
  const sourceTarget = sourceReadTargetFor(options.row)

  if (sourceTarget !== undefined) {
    const sourceState = options.sourceReadStates.get(nodeId)
    const descriptor = adaptLocationSourceDescriptor({
      rowKind: options.row.rowKind,
      selectorKind: options.row.selectorKind,
      sourceStateKind: sourceState?.kind,
      sourceStateFailed: sourceState?.kind === 'failed'
    })
    const presentation = getLocationSourcePresentation(descriptor)

    options.bindingsById.set(nodeId, {
      kind: 'source',
      navigationRow: options.row,
      target: sourceTarget
    })

    return {
      id: nodeId,
      role: browserRowRoleForNavigationRow(options.row),
      label: options.row.displayName,
      badge: { value: presentation.label, tone: 'muted' },
      icon: 'source',
      detail: formatNavigationSourceDetail(options.row),
      ...projectSourceChildren({
        ownerId: nodeId,
        target: sourceTarget,
        state: sourceState,
        directoryReadStates: options.directoryReadStates,
        bindingsById: options.bindingsById
      })
    }
  }

  options.bindingsById.set(nodeId, {
    kind: 'navigation',
    navigationRow: options.row
  })

  return {
    id: nodeId,
    role: browserRowRoleForNavigationRow(options.row),
    label: options.row.displayName,
    badge: { value: 'Navigation', tone: 'muted' },
    icon: 'navigation',
    detail: formatNavigationDetail(options.row),
    children: { kind: 'none' }
  }
}

function projectSourceChildren(options: {
  readonly ownerId: string
  readonly target: SourceTarget
  readonly state: SourceState | undefined
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): ProjectedNodeState {
  const state = options.state

  if (state === undefined || state.kind === 'unloaded') {
    const detail = state?.detail ?? 'Contents not loaded yet.'

    return {
      children: {
        kind: 'deferred',
        detail
      },
      action: loadChildrenAction('idle', detail)
    }
  }

  if (state.kind === 'loading') {
    return {
      children: {
        kind: 'loaded',
        nodes: [
          trackedReadStateNode(
            {
              ownerId: options.ownerId,
              state: 'loading',
              label: 'Loading folder contents',
              detail: state.detail ?? 'Loading folder contents.'
            },
            options.bindingsById
          )
        ]
      }
    }
  }

  if (state.kind === 'failed') {
    return {
      children: {
        kind: 'loaded',
        nodes: [
          trackedReadStateNode(
            {
              ownerId: options.ownerId,
              state: 'error',
              label: 'Folder contents unavailable',
              detail: state.detail
            },
            options.bindingsById
          )
        ]
      }
    }
  }

  return {
    children: childrenForProjectedNodes(
      projectLoadedHierarchyChildren({
        ownerId: options.ownerId,
        children: state.children,
        directoryReadStates: options.directoryReadStates,
        bindingsById: options.bindingsById
      })
    )
  }
}

function projectLoadedHierarchyChildren(options: {
  readonly ownerId: string
  readonly children: LoadedChildren
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): readonly BrowserTreeNode[] {
  if (options.children.rows.length === 0 && options.children.nextOffset === undefined) {
    return []
  }

  const projectedNodes = projectLiteralNodes({
    nodes: options.children.rows,
    entryPoint: options.children.entryPoint,
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    directoryReadStates: options.directoryReadStates,
    bindingsById: options.bindingsById
  })

  if (projectedNodes.length === 0 && options.children.nextOffset === undefined) {
    return []
  }

  if (options.children.nextOffset === undefined) {
    return projectedNodes
  }

  return [
    ...projectedNodes,
    trackedMoreNode(
      {
        ownerId: options.ownerId,
        children: options.children
      },
      options.bindingsById
    )
  ]
}

function projectLiteralNodes(options: {
  readonly nodes: readonly ChildRow[]
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): readonly BrowserTreeNode[] {
  return options.nodes
    .filter((node) => isVisibleLiteralNode(node))
    .map((node) =>
      projectLiteralNode({
        node,
        entryPoint: options.entryPoint,
        ...(options.label === undefined ? {} : { label: options.label }),
        directoryReadStates: options.directoryReadStates,
        bindingsById: options.bindingsById
      })
    )
}

function projectLiteralNode(options: {
  readonly node: ChildRow
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const node = options.node

  if (node.kind === 'directory') {
    options.bindingsById.set(node.id, {
      kind: 'directory',
      directoryId: node.directoryId,
      ...(node.parentDirectoryId === undefined
        ? {}
        : { parentDirectoryId: node.parentDirectoryId }),
      entryPoint: copyEntryPoint(options.entryPoint),
      ...(options.label === undefined ? {} : { label: options.label })
    })

    const directoryState = options.directoryReadStates.get(node.directoryId)
    const isUnloaded = directoryState === undefined || directoryState.kind === 'unloaded'

    if (isUnloaded && node.browseability === 'empty') {
      return {
        id: node.id,
        role: 'literalDirectory',
        label: node.label,
        badge: { value: 'Folder', tone: 'muted' },
        icon: 'folder',
        detail: formatDirectoryDetail(node.presence),
        children: { kind: 'none' }
      }
    }

    return {
      id: node.id,
      role: 'literalDirectory',
      label: node.label,
      badge: { value: 'Folder', tone: 'muted' },
      icon: 'folder',
      detail: formatDirectoryDetail(node.presence),
      ...projectDirectoryChildren({
        ownerId: node.id,
        state: directoryState,
        directoryReadStates: options.directoryReadStates,
        bindingsById: options.bindingsById
      })
    }
  }

  const presentation = classifyLibraryEntryName(node.label)

  options.bindingsById.set(node.id, {
    kind: 'file',
    fileId: node.fileId,
    ...(node.parentDirectoryId === undefined ? {} : { parentDirectoryId: node.parentDirectoryId }),
    entryPoint: copyEntryPoint(options.entryPoint)
  })

  return {
    id: node.id,
    role: 'literalFile',
    label: node.label,
    badge: presentation.badge,
    icon: browserTreeIconForEntryRole(presentation.role),
    detail: formatFileDetail(node.presence),
    children: { kind: 'none' }
  }
}

function browserTreeIconForEntryRole(
  role: LibraryEntryRole
): import('../tree/types').BrowserTreeIcon {
  switch (role) {
    case 'folder':
      return 'folder'
    case 'audio':
      return 'music'
    case 'video':
      return 'video'
    case 'cueSheet':
      return 'cueSheet'
    case 'playlist':
      return 'playlist'
    case 'artwork':
      return 'image'
    case 'metadata':
      return 'metadata'
    case 'unknown':
    case 'nonMedia':
      return 'file'
  }
}

function isVisibleLiteralNode(node: ChildRow): boolean {
  if (node.kind === 'directory') {
    return true
  }

  const presentation = classifyLibraryEntryName(node.label)

  return presentation.visibility !== 'hiddenNonMedia'
}

function childrenForProjectedNodes(nodes: readonly BrowserTreeNode[]): BrowserTreeChildren {
  return nodes.length === 0 ? { kind: 'none' } : { kind: 'loaded', nodes }
}

function projectDirectoryChildren(options: {
  readonly ownerId: string
  readonly state: DirectoryState | undefined
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): ProjectedNodeState {
  const state = options.state

  if (state === undefined || state.kind === 'unloaded') {
    const detail = state?.detail ?? 'Contents not loaded yet.'

    return {
      children: {
        kind: 'deferred',
        detail
      },
      action: loadChildrenAction('idle', detail)
    }
  }

  if (state.kind === 'loading') {
    return {
      children: {
        kind: 'loaded',
        nodes: [
          trackedReadStateNode(
            {
              ownerId: options.ownerId,
              state: 'loading',
              label: 'Loading contents',
              detail: state.detail ?? 'Loading contents.'
            },
            options.bindingsById
          )
        ]
      }
    }
  }

  if (state.kind === 'failed') {
    return {
      children: {
        kind: 'loaded',
        nodes: [
          trackedReadStateNode(
            {
              ownerId: options.ownerId,
              state: 'error',
              label: 'Contents unavailable',
              detail: state.detail
            },
            options.bindingsById
          )
        ]
      }
    }
  }

  return {
    children: childrenForProjectedNodes(
      projectLoadedHierarchyChildren({
        ownerId: options.ownerId,
        children: state.children,
        directoryReadStates: options.directoryReadStates,
        bindingsById: options.bindingsById
      })
    )
  }
}

function emptyProjection(options: {
  readonly ownerId: string
  readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
  readonly label: string
  readonly detail: string
}): BrowserProjection {
  const bindingsById = new Map<BrowserTreeNodeId, RowBinding>()
  const node = trackedReadStateNode(options, bindingsById)

  return {
    kind: 'tree',
    nodes: [node],
    bindingsById
  }
}

function readStateNode(options: {
  readonly ownerId: string
  readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
  readonly label: string
  readonly detail: string
}): BrowserTreeNode {
  return {
    id: `read-state:${options.ownerId}:${options.state}`,
    role: 'state',
    label: options.label,
    badge: { value: 'Status', tone: readStateBadgeTone(options.state) },
    detail: options.detail,
    icon: readStateIcon(options.state),
    children: { kind: 'none' }
  }
}

function loadChildrenAction(
  stateKind: 'idle' | 'loading' | 'failed',
  detail?: string
): BrowserTreeAction {
  const state: BrowserTreeActionState =
    stateKind === 'loading'
      ? {
          kind: 'loading',
          ...(detail === undefined ? {} : { detail })
        }
      : stateKind === 'failed'
        ? { kind: 'failed', detail: detail ?? 'Load failed.' }
        : {
            kind: 'idle',
            ...(detail === undefined ? {} : { detail })
          }

  return { kind: 'loadChildren', state }
}

function trackedReadStateNode(
  options: {
    readonly ownerId: string
    readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode {
  const node = readStateNode(options)

  bindingsById.set(node.id, {
    kind: 'readState',
    state: options.state,
    ownerId: options.ownerId,
    detail: options.detail
  })

  return node
}

function moreNode(options: { readonly ownerId: string; readonly children: LoadedChildren }): {
  readonly node: BrowserTreeNode
  readonly target: MoreTarget
  readonly detail: string
} {
  const offset = options.children.nextOffset

  if (offset === undefined) {
    throw new Error('More rows require a next offset.')
  }

  const target = {
    ownerNodeId: options.ownerId,
    entryPoint: copyEntryPoint(options.children.entryPoint),
    ...(options.children.parentDirectoryId === undefined
      ? {}
      : { parentDirectoryId: options.children.parentDirectoryId }),
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    offset,
    limit: options.children.limit
  }
  const more = options.children.more
  const detail =
    more?.kind === 'failed'
      ? more.detail
      : more?.kind === 'loading'
        ? (more.detail ?? 'Loading more items.')
        : formatMoreDetail(offset, options.children.limit, options.children.totalRows)
  const actionState: BrowserTreeActionState =
    more?.kind === 'failed'
      ? { kind: 'failed', detail }
      : more?.kind === 'loading'
        ? { kind: 'loading', detail }
        : { kind: 'idle', detail }

  return {
    node: {
      id: `more:${options.ownerId}:${offset}`,
      role: 'action',
      label:
        more?.kind === 'failed'
          ? 'Retry loading more'
          : more?.kind === 'loading'
            ? 'Loading more'
            : 'Load more',
      badge: { value: 'More', tone: moreBadgeTone(more) },
      icon: moreIcon(more),
      children: { kind: 'none' },
      action: { kind: 'loadMore', state: actionState }
    },
    target,
    detail
  }
}

function trackedMoreNode(
  options: {
    readonly ownerId: string
    readonly children: LoadedChildren
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode {
  const { node, target, detail } = moreNode(options)
  const more = options.children.more

  bindingsById.set(node.id, {
    kind: 'more',
    state: more?.kind === 'failed' ? 'error' : more?.kind === 'loading' ? 'loading' : 'available',
    ownerId: options.ownerId,
    target,
    detail
  })

  return node
}

function readStateIcon(state: 'loading' | 'empty' | 'unavailable' | 'error'): BrowserTreeIcon {
  switch (state) {
    case 'loading':
      return 'loading'
    case 'error':
    case 'unavailable':
      return 'warning'
    case 'empty':
      return 'state'
  }
}

function moreIcon(more: LoadedChildren['more']): BrowserTreeIcon {
  switch (more?.kind) {
    case 'loading':
      return 'loading'
    case 'failed':
      return 'warning'
    default:
      return 'more'
  }
}

function readStateBadgeTone(
  state: 'loading' | 'empty' | 'unavailable' | 'error'
): BrowserTreeBadgeTone {
  switch (state) {
    case 'loading':
      return 'neutral'
    case 'error':
    case 'unavailable':
      return 'warning'
    case 'empty':
      return 'muted'
  }
}

function moreBadgeTone(more: LoadedChildren['more']): BrowserTreeBadgeTone {
  switch (more?.kind) {
    case 'loading':
      return 'neutral'
    case 'failed':
      return 'warning'
    default:
      return 'muted'
  }
}

function sourceReadTargetFor(row: NavigationRow): SourceTarget | undefined {
  if (row.selectorKind === 'source' && isPositiveOpaqueId(row.selectorPayload)) {
    return {
      navigationRowId: row.navigationRowId,
      entryPoint: {
        kind: 'source',
        sourceId: row.selectorPayload
      },
      label: row.displayName
    }
  }

  if (row.selectorKind === 'sourceLocation' && isPositiveOpaqueId(row.selectorPayload)) {
    return {
      navigationRowId: row.navigationRowId,
      entryPoint: {
        kind: 'sourceLocation',
        sourceLocationId: row.selectorPayload
      },
      label: row.displayName
    }
  }

  return undefined
}

function navigationNodeId(row: NavigationRow): BrowserTreeNodeId {
  return `navigation-row:${row.navigationRowId}`
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function formatNavigationSourceDetail(row: NavigationRow): string {
  return `Library source. Updated ${formatRowFreshness(row)}.`
}

function formatNavigationDetail(row: NavigationRow): string {
  return `${formatNavigationRowKind(row.rowKind)}. Updated ${formatRowFreshness(row)}.`
}

function formatNavigationRowKind(rowKind: NavigationRow['rowKind']): string {
  switch (rowKind) {
    case 'view':
      return 'View'
    case 'collectionGroup':
      return 'Collection group'
    case 'playlist':
      return 'Playlist'
    case 'prepPolicyGroup':
      return 'Preparation group'
    case 'prepPolicyScope':
      return 'Preparation scope'
    case 'source':
      return 'Source'
    case 'locationGroup':
      return 'Location group'
    case 'location':
      return 'Location'
  }
}

function formatRowFreshness(row: NavigationRow): string {
  return new Date(row.updatedAtMs).toISOString().slice(0, 10)
}

function formatMoreDetail(offset: number, limit: number, totalRows: number): string {
  return `Items ${offset + 1}-${Math.min(offset + limit, totalRows)} of ${totalRows} are available.`
}

function formatFileDetail(presence: ChildRow['presence']): string {
  switch (presence) {
    case 'present':
      return 'File available.'
    case 'missing':
      return 'File missing.'
    case 'removed':
      return 'File removed.'
  }
}

function formatDirectoryDetail(presence: ChildRow['presence']): string {
  switch (presence) {
    case 'present':
      return 'Folder available.'
    case 'missing':
      return 'Folder missing.'
    case 'removed':
      return 'Folder removed.'
  }
}
