import type {
  EntryPoint,
  ChildRow,
  HierarchyCoverage,
  SourceFileVisibility
} from '../../../shared/libraryHierarchy/readChildren'
import type { NavigationRow } from '../../../shared/libraryNavigation/readRows'
import type {
  DirectoryState,
  LoadedChildren,
  MoreTarget,
  RowBinding,
  BrowserState,
  SourceState,
  SourceTarget
} from '../state'
import type {
  BrowserTreeAction,
  BrowserTreeActionState,
  BrowserTreeBadge,
  BrowserTreeChildren,
  BrowserTreeIcon,
  BrowserTreeNode,
  BrowserTreeNodeId
} from './types'
import { copyEntryPoint } from '../runtime/entryPoint'
import { formatSourceDisplayName } from './sourcePresentation'
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

type BrowserTreeReadState = 'notLoaded' | 'loading' | 'empty' | 'unavailable' | 'error'

export function projectState(state: BrowserState): BrowserProjection | undefined {
  const hostProjection = projectHostStatus(state.hostStatus)

  if (hostProjection !== undefined) {
    return hostProjection
  }

  if (state.navigationReadResult === undefined) {
    return undefined
  }

  return projectNavigationResult(state)
}

function projectHostStatus(hostStatus: BrowserState['hostStatus']): BrowserProjection | undefined {
  if (hostStatus === undefined) {
    return undefined
  }

  if (hostStatus.state === 'failed') {
    return projectHostFailed(hostStatus)
  }

  if (hostStatus.state === 'stopping' || hostStatus.state === 'stopped') {
    return emptyProjection({
      ownerId: 'host',
      state: 'unavailable',
      label: 'Library engine unavailable',
      detail: 'The library engine is not running.'
    })
  }

  return undefined
}

function projectHostFailed(hostStatus: NonNullable<BrowserState['hostStatus']>): BrowserProjection {
  const lastError = hostStatus.lastError

  return emptyProjection({
    ownerId: 'host',
    state: 'error',
    label: 'Library engine failed to start',
    detail: lastError?.message ?? 'No additional detail available.'
  })
}

function isRendererVisibleNavigationRow(row: NavigationRow): boolean {
  return row.selectorKind === 'source' || row.selectorKind === 'sourceLocation'
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

  const visibleRows = result.rows.filter(isRendererVisibleNavigationRow)

  if (visibleRows.length === 0) {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'empty',
      label: 'No library sources',
      detail: 'Add a music folder to start building your library.'
    })
  }

  return {
    kind: 'tree',
    nodes: visibleRows.map((row) =>
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
    const sourceFailed = sourceState?.kind === 'failed'

    options.bindingsById.set(nodeId, {
      kind: 'source',
      navigationRow: options.row,
      target: sourceTarget
    })

    return {
      id: nodeId,
      role: browserRowRoleForNavigationRow(options.row),
      label: formatSourceDisplayName(options.row.displayName),
      ...(sourceFailed
        ? { badge: { value: 'Unavailable', tone: 'warning' } as BrowserTreeBadge }
        : {}),
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
    label: formatSourceDisplayName(options.row.displayName),
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
      children: deferredChildren(
        {
          ownerId: options.ownerId,
          label: 'Source contents not loaded',
          detail
        },
        options.bindingsById
      ),
      action: loadChildrenAction('idle', detail)
    }
  }

  if (state.kind === 'loading') {
    return {
      children: loadingChildren(
        {
          ownerId: options.ownerId,
          label: 'Loading source contents',
          detail: state.detail ?? 'Loading source contents.'
        },
        options.bindingsById
      )
    }
  }

  if (state.kind === 'failed') {
    return {
      children: failedChildren(
        {
          ownerId: options.ownerId,
          label: 'Source contents unavailable',
          detail: state.detail
        },
        options.bindingsById
      )
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
    const stateNode = hierarchyCoverageStateNode(
      options.ownerId,
      options.children.coverage,
      options.bindingsById
    )
    return stateNode === undefined ? [] : [stateNode]
  }

  const projectedNodes = projectLiteralNodes({
    nodes: options.children.rows,
    entryPoint: options.children.entryPoint,
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    sourceFileVisibility: options.children.sourceFileVisibility,
    directoryReadStates: options.directoryReadStates,
    bindingsById: options.bindingsById
  })

  if (projectedNodes.length === 0 && options.children.nextOffset === undefined) {
    const stateNode = hierarchyCoverageStateNode(
      options.ownerId,
      options.children.coverage,
      options.bindingsById
    )
    return stateNode === undefined ? [] : [stateNode]
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

function hierarchyCoverageStateNode(
  ownerId: string,
  coverage: HierarchyCoverage,
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode | undefined {
  switch (coverage.state) {
    case 'complete':
      if (!coverage.emptyResultAuthoritative) {
        return undefined
      }
      return trackedReadStateNode(
        {
          ownerId,
          state: 'empty',
          label: 'No visible items',
          detail: coverage.detail ?? 'No visible items in this scope.'
        },
        bindingsById
      )
    case 'pending':
    case 'scanning':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'loading',
          label: coverage.state === 'scanning' ? 'Indexing source contents' : 'Indexing pending',
          detail: coverage.detail ?? 'Source contents are still being indexed.'
        },
        bindingsById
      )
    case 'sourceUnavailable':
    case 'locationMissing':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'unavailable',
          label: coverage.state === 'locationMissing' ? 'Location missing' : 'Source unavailable',
          detail: coverage.detail ?? 'The selected source location is unavailable.'
        },
        bindingsById
      )
    case 'blocked':
    case 'failed':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'error',
          label: coverage.state === 'blocked' ? 'Access blocked' : 'Scan failed',
          detail: coverage.detail ?? 'The selected scope could not be fully scanned.'
        },
        bindingsById
      )
  }
}

function projectLiteralNodes(options: {
  readonly nodes: readonly ChildRow[]
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly sourceFileVisibility: SourceFileVisibility
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): readonly BrowserTreeNode[] {
  return options.nodes.map((node) =>
    projectLiteralNode({
      node,
      entryPoint: options.entryPoint,
      ...(options.label === undefined ? {} : { label: options.label }),
      sourceFileVisibility: options.sourceFileVisibility,
      directoryReadStates: options.directoryReadStates,
      bindingsById: options.bindingsById
    })
  )
}

function projectLiteralNode(options: {
  readonly node: ChildRow
  readonly entryPoint: EntryPoint
  readonly label?: string
  readonly sourceFileVisibility: SourceFileVisibility
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const node = options.node

  if (node.kind === 'directory') {
    options.bindingsById.set(node.id, {
      kind: 'directory',
      sourceId: node.sourceId,
      directoryId: node.directoryId,
      ...(node.parentDirectoryId === undefined
        ? {}
        : { parentDirectoryId: node.parentDirectoryId }),
      entryPoint: copyEntryPoint(options.entryPoint),
      ...(options.label === undefined ? {} : { label: options.label })
    })

    const directoryState = options.directoryReadStates.get(node.directoryId)
    const dirBadge = presenceBadge(node.presence)

    if (isConfirmedDirectoryLeaf(node, directoryState, options.sourceFileVisibility)) {
      return {
        id: node.id,
        role: 'literalDirectory',
        label: node.label,
        ...(dirBadge === undefined ? {} : { badge: dirBadge }),
        icon: 'folder',
        detail: formatDirectoryDetail(node.presence),
        children: { kind: 'none' }
      }
    }

    return {
      id: node.id,
      role: 'literalDirectory',
      label: node.label,
      ...(dirBadge === undefined ? {} : { badge: dirBadge }),
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

  const fileBadge = presenceBadge(node.presence)

  options.bindingsById.set(node.id, {
    kind: 'file',
    sourceId: node.sourceId,
    fileId: node.fileId,
    ...(node.parentDirectoryId === undefined ? {} : { parentDirectoryId: node.parentDirectoryId }),
    entryPoint: copyEntryPoint(options.entryPoint)
  })

  return {
    id: node.id,
    role: 'literalFile',
    label: node.label,
    ...(fileBadge === undefined ? {} : { badge: fileBadge }),
    icon: browserTreeIconForMediaClass(node.mediaClass),
    detail: formatFileDetail(node.presence, node.mediaClass),
    children: { kind: 'none' }
  }
}

function isConfirmedDirectoryLeaf(
  node: Extract<ChildRow, { readonly kind: 'directory' }>,
  state: DirectoryState | undefined,
  sourceFileVisibility: SourceFileVisibility
): boolean {
  if (state !== undefined && state.kind !== 'unloaded') {
    return false
  }

  return (
    !node.hasChildDirectories &&
    directoryHasNoRevealableMediaDescendants(node, sourceFileVisibility) &&
    node.directoryScanState === 'complete'
  )
}

function directoryHasNoRevealableMediaDescendants(
  node: Extract<ChildRow, { readonly kind: 'directory' }>,
  sourceFileVisibility: SourceFileVisibility
): boolean {
  if (node.directoryPrimaryMediaState.kind !== 'noPrimaryMediaDescendants') {
    return false
  }

  return (
    sourceFileVisibility === 'performance' ||
    node.directoryImageMediaState.kind === 'noImageMediaDescendants'
  )
}

function browserTreeIconForMediaClass(
  mediaClass: Extract<ChildRow, { readonly kind: 'file' }>['mediaClass']
): BrowserTreeIcon {
  switch (mediaClass) {
    case 'audio':
      return 'music'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'unsupported':
    case 'none':
      return 'file'
  }
}

function childrenForProjectedNodes(nodes: readonly BrowserTreeNode[]): BrowserTreeChildren {
  return nodes.length === 0 ? { kind: 'none' } : { kind: 'loaded', nodes }
}

function deferredChildren(
  options: {
    readonly ownerId: string
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeChildren {
  return {
    kind: 'deferred',
    stateNode: trackedReadStateNode(
      {
        ...options,
        state: 'notLoaded'
      },
      bindingsById
    )
  }
}

function loadingChildren(
  options: {
    readonly ownerId: string
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeChildren {
  return {
    kind: 'loading',
    stateNode: trackedReadStateNode(
      {
        ...options,
        state: 'loading'
      },
      bindingsById
    )
  }
}

function failedChildren(
  options: {
    readonly ownerId: string
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeChildren {
  return {
    kind: 'failed',
    stateNode: trackedReadStateNode(
      {
        ...options,
        state: 'error'
      },
      bindingsById
    )
  }
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
      children: deferredChildren(
        {
          ownerId: options.ownerId,
          label: 'Folder contents not loaded',
          detail
        },
        options.bindingsById
      ),
      action: loadChildrenAction('idle', detail)
    }
  }

  if (state.kind === 'loading') {
    return {
      children: loadingChildren(
        {
          ownerId: options.ownerId,
          label: 'Loading contents',
          detail: state.detail ?? 'Loading contents.'
        },
        options.bindingsById
      )
    }
  }

  if (state.kind === 'failed') {
    return {
      children: failedChildren(
        {
          ownerId: options.ownerId,
          label: 'Contents unavailable',
          detail: state.detail
        },
        options.bindingsById
      )
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
  readonly state: BrowserTreeReadState
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
  readonly state: BrowserTreeReadState
  readonly label: string
  readonly detail: string
}): BrowserTreeNode {
  return {
    id: `read-state:${options.ownerId}:${options.state}`,
    role: 'state',
    label: options.label,
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
    readonly state: BrowserTreeReadState
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
    sourceFileVisibility: options.children.sourceFileVisibility,
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

function readStateIcon(state: BrowserTreeReadState): BrowserTreeIcon {
  switch (state) {
    case 'loading':
      return 'loading'
    case 'error':
    case 'unavailable':
      return 'warning'
    case 'notLoaded':
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

function presenceBadge(presence: ChildRow['presence']): BrowserTreeBadge | undefined {
  switch (presence) {
    case 'present':
      return undefined
    case 'missing':
      return { value: 'Missing', tone: 'warning', ariaLabel: 'Missing item' }
    case 'removed':
      return { value: 'Removed', tone: 'danger', ariaLabel: 'Removed item' }
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
    case 'location':
      return 'Location'
    default:
      return 'Navigation item'
  }
}

function formatRowFreshness(row: NavigationRow): string {
  return new Date(row.updatedAtMs).toISOString().slice(0, 10)
}

function formatMoreDetail(offset: number, limit: number, totalRows: number): string {
  return `Items ${offset + 1}-${Math.min(offset + limit, totalRows)} of ${totalRows} are available.`
}

function formatFileDetail(
  presence: ChildRow['presence'],
  mediaClass: Extract<ChildRow, { readonly kind: 'file' }>['mediaClass']
): string {
  switch (presence) {
    case 'present':
      return formatPresentFileDetail(mediaClass)
    case 'missing':
      return 'File missing'
    case 'removed':
      return 'File removed'
  }
}

function formatPresentFileDetail(
  mediaClass: Extract<ChildRow, { readonly kind: 'file' }>['mediaClass']
): string {
  switch (mediaClass) {
    case 'audio':
      return 'Audio file'
    case 'video':
      return 'Video file'
    case 'image':
      return 'Image file'
    case 'unsupported':
    case 'none':
      return 'File'
  }
}

function formatDirectoryDetail(presence: ChildRow['presence']): string {
  switch (presence) {
    case 'present':
      return 'Folder'
    case 'missing':
      return 'Folder missing'
    case 'removed':
      return 'Folder removed'
  }
}
