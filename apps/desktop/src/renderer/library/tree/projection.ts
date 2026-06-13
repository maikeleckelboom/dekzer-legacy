import type {
  EntryPoint,
  ChildRow,
  HierarchyCoverage
} from '../../../shared/library/hierarchy/read'
import type { NavigationRow } from '../../../shared/library/navigation/read'
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
  BrowserTreeChildren,
  BrowserTreeIcon,
  BrowserTreeNode,
  BrowserTreeNodeId
} from './types'
import { copyEntryPoint } from '../runtime/entryPoint'
import type { SourceReadiness } from '../runtime/sourceReadiness'
import { formatSourceDisplayName } from './sourcePresentation'
import { browserRowRoleForNavigationRow } from './rowRoles'
import { projectAddSourceSection } from '../localBrowse/projection'
import {
  defaultLibraryBrowseProfile,
  type LibraryBrowseProfile
} from '../libraryBrowseProfile/types'
import { defaultLocalPreviewMode } from '../localBrowse/previewMode'

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

  if (state.navigationReadResult === undefined && state.localBrowseEntryPointsState === undefined) {
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
  const addSourceSection = projectAddSourceSection({
    localPreviewMode: state.localPreviewMode ?? defaultLocalPreviewMode,
    ...(state.localBrowseEntryPointsState === undefined
      ? {}
      : { entryPointsState: state.localBrowseEntryPointsState }),
    ...(state.localBrowseItemStates === undefined
      ? {}
      : { itemStates: state.localBrowseItemStates }),
    bindingsById
  })
  const localBrowseNodes =
    addSourceSection === undefined ? [] : ([addSourceSection] satisfies BrowserTreeNode[])

  if (result === undefined) {
    if (localBrowseNodes.length > 0) {
      return {
        kind: 'tree',
        nodes: localBrowseNodes,
        bindingsById
      }
    }

    return emptyProjectionWithBindings(
      {
        ownerId: 'navigation',
        state: 'loading',
        label: 'Loading library',
        detail: 'Loading your library.'
      },
      bindingsById
    )
  }

  if (result.state !== 'ready') {
    if (localBrowseNodes.length > 0) {
      return {
        kind: 'tree',
        nodes: localBrowseNodes,
        bindingsById
      }
    }

    return emptyProjectionWithBindings(
      {
        ownerId: 'navigation',
        state: 'error',
        label: 'Library unavailable',
        detail: result.error.message
      },
      bindingsById
    )
  }

  const visibleRows = result.rows.filter(isRendererVisibleNavigationRow)

  if (visibleRows.length === 0 && localBrowseNodes.length === 0) {
    return emptyProjectionWithBindings(
      {
        ownerId: 'navigation',
        state: 'empty',
        label: 'No library sources',
        detail: 'Add a music folder to start building your library.'
      },
      bindingsById
    )
  }

  return {
    kind: 'tree',
    nodes: [
      ...visibleRows.map((row) =>
        projectNavigationRow({
          row,
          profile: state.libraryBrowseProfile ?? defaultLibraryBrowseProfile,
          ...(state.sourceReadinessByNodeId === undefined
            ? {}
            : { sourceReadinessByNodeId: state.sourceReadinessByNodeId }),
          sourceReadStates: state.sourceReadStates,
          directoryReadStates: state.directoryReadStates,
          bindingsById
        })
      ),
      ...localBrowseNodes
    ],
    bindingsById
  }
}

function projectNavigationRow(options: {
  readonly row: NavigationRow
  readonly profile: LibraryBrowseProfile
  readonly sourceReadinessByNodeId?: ReadonlyMap<string, SourceReadiness>
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const nodeId = navigationNodeId(options.row)
  const sourceTarget = sourceReadTargetFor(options.row)

  if (sourceTarget !== undefined) {
    const sourceState = options.sourceReadStates.get(nodeId)
    const sourceReadiness = options.sourceReadinessByNodeId?.get(nodeId)

    options.bindingsById.set(nodeId, {
      kind: 'source',
      navigationRow: options.row,
      target: sourceTarget
    })

    return {
      id: nodeId,
      role: browserRowRoleForNavigationRow(options.row),
      label: formatSourceDisplayName(options.row.displayName),
      icon: 'source',
      detail: formatNavigationSourceDetail(options.row, sourceReadiness),
      ...projectSourceChildren({
        ownerId: nodeId,
        target: sourceTarget,
        state: sourceState,
        profile: options.profile,
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
  readonly profile: LibraryBrowseProfile
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
          label: sourceFailedLabel(state.errorCode),
          detail: state.detail
        },
        options.bindingsById
      ),
      ...(isRetryableChildrenReadError(state.errorCode)
        ? { action: loadChildrenAction('failed', state.detail) }
        : {})
    }
  }

  if (state.kind === 'refreshing' || state.kind === 'loaded') {
    return {
      children: childrenForProjectedNodes(
        projectLoadedHierarchyChildren({
          ownerId: options.ownerId,
          children: state.children,
          profile: options.profile,
          directoryReadStates: options.directoryReadStates,
          bindingsById: options.bindingsById,
          suppressFileOnlyTerminalState: state.kind === 'refreshing'
        })
      )
    }
  }

  return {
    children: childrenForProjectedNodes([])
  }
}

function projectLoadedHierarchyChildren(options: {
  readonly ownerId: string
  readonly children: LoadedChildren
  readonly profile: LibraryBrowseProfile
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
  readonly suppressFileOnlyTerminalState?: boolean
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
    profile: options.profile,
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    directoryReadStates: options.directoryReadStates,
    bindingsById: options.bindingsById
  })

  if (projectedNodes.length === 0 && options.children.nextOffset === undefined) {
    const stateNode =
      options.children.rows.length > 0 && options.suppressFileOnlyTerminalState !== true
        ? noChildFoldersStateNode(options.ownerId, options.bindingsById)
        : hierarchyCoverageStateNode(
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
          label: 'No matches',
          detail: coverage.detail ?? 'No matching content.'
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

function noChildFoldersStateNode(
  ownerId: string,
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode {
  return trackedReadStateNode(
    {
      ownerId,
      state: 'empty',
      label: 'No child folders in this view',
      detail: 'This scope has matching files but no child folders in the active view.'
    },
    bindingsById
  )
}

function projectLiteralNodes(options: {
  readonly nodes: readonly ChildRow[]
  readonly entryPoint: EntryPoint
  readonly profile: LibraryBrowseProfile
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): readonly BrowserTreeNode[] {
  return options.nodes.flatMap((node) =>
    node.kind === 'directory'
      ? [
          projectLiteralDirectoryNode({
            node,
            entryPoint: options.entryPoint,
            profile: options.profile,
            ...(options.label === undefined ? {} : { label: options.label }),
            directoryReadStates: options.directoryReadStates,
            bindingsById: options.bindingsById
          })
        ]
      : options.profile === 'allFiles'
        ? [
            projectLiteralFileNode({
              node,
              entryPoint: options.entryPoint,
              bindingsById: options.bindingsById
            })
          ]
        : []
  )
}

function projectLiteralDirectoryNode(options: {
  readonly node: Extract<ChildRow, { readonly kind: 'directory' }>
  readonly entryPoint: EntryPoint
  readonly profile: LibraryBrowseProfile
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const node = options.node

  options.bindingsById.set(node.id, {
    kind: 'directory',
    sourceId: node.sourceId,
    directoryId: node.directoryId,
    ...(node.parentDirectoryId === undefined ? {} : { parentDirectoryId: node.parentDirectoryId }),
    entryPoint: copyEntryPoint(options.entryPoint),
    ...(options.label === undefined ? {} : { label: options.label })
  })

  const directoryState = options.directoryReadStates.get(node.directoryId)

  if (isConfirmedDirectoryLeaf(node)) {
    return {
      id: node.id,
      role: 'literalDirectory',
      label: node.label,
      icon: 'folder',
      detail: formatDirectoryDetail(node.presence),
      children: { kind: 'none' }
    }
  }

  if (isUnknownDirectoryChildReadiness(node)) {
    const readState = unknownDirectoryChildReadinessState(node)
    const detail = formatUnknownDirectoryChildReadinessDetail(node)
    return {
      id: node.id,
      role: 'literalDirectory',
      label: node.label,
      icon: 'folder',
      detail,
      children: unknownChildren(
        {
          ownerId: node.id,
          state: readState,
          label: formatUnknownDirectoryChildReadinessLabel(node),
          detail
        },
        options.bindingsById
      )
    }
  }

  return {
    id: node.id,
    role: 'literalDirectory',
    label: node.label,
    icon: 'folder',
    detail: formatDirectoryDetail(node.presence),
    ...projectDirectoryChildren({
      ownerId: node.id,
      state: directoryState,
      profile: options.profile,
      directoryReadStates: options.directoryReadStates,
      bindingsById: options.bindingsById
    })
  }
}

function projectLiteralFileNode(options: {
  readonly node: Extract<ChildRow, { readonly kind: 'file' }>
  readonly entryPoint: EntryPoint
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const node = options.node

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
    icon: literalFileIcon(node.fileClass),
    detail: formatLiteralFileDetail(node),
    children: { kind: 'none' }
  }
}

export function isConfirmedDirectoryLeaf(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): boolean {
  return node.navigableChildScopeState === 'noNavigableChildScopes'
}

export function hasDisclosureAffordance(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): boolean {
  return node.navigableChildScopeState === 'hasNavigableChildScopes'
}

export function isUnknownDirectoryChildReadiness(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): boolean {
  return node.navigableChildScopeState === 'unknown'
}

const sourceUnavailableErrorCodes = new Set([
  'notFound',
  'hostUnavailable',
  'hostStopped',
  'hostStopping',
  'hostFailed',
  'hostNotStarted'
])

function sourceFailedLabel(errorCode: string): string {
  if (sourceUnavailableErrorCodes.has(errorCode)) {
    return 'Source unavailable'
  }

  switch (errorCode) {
    case 'readFailed':
    case 'windowMismatch':
      return 'Hierarchy read failed'
    case 'noTarget':
      return 'No source available'
    case 'invalidRequest':
      return 'Invalid read request'
    default:
      return 'Read error'
  }
}

function directoryFailedLabel(errorCode: string): string {
  if (sourceUnavailableErrorCodes.has(errorCode)) {
    return 'Folder unavailable'
  }

  switch (errorCode) {
    case 'readFailed':
    case 'windowMismatch':
      return 'Directory read failed'
    default:
      return 'Read error'
  }
}

function isRetryableChildrenReadError(errorCode: string): boolean {
  return errorCode === 'readFailed' || errorCode === 'windowMismatch'
}

function childrenForProjectedNodes(nodes: readonly BrowserTreeNode[]): BrowserTreeChildren {
  return nodes.length === 0 ? { kind: 'none' } : { kind: 'loaded', nodes }
}

function unknownChildren(
  options: {
    readonly ownerId: string
    readonly state: BrowserTreeReadState
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeChildren {
  return {
    kind: 'unknown',
    stateNode: trackedReadStateNode(options, bindingsById)
  }
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
  readonly profile: LibraryBrowseProfile
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
          label: directoryFailedLabel(state.errorCode),
          detail: state.detail
        },
        options.bindingsById
      ),
      ...(isRetryableChildrenReadError(state.errorCode)
        ? { action: loadChildrenAction('failed', state.detail) }
        : {})
    }
  }

  if (state.kind === 'refreshing' || state.kind === 'loaded') {
    return {
      children: childrenForProjectedNodes(
        projectLoadedHierarchyChildren({
          ownerId: options.ownerId,
          children: state.children,
          profile: options.profile,
          directoryReadStates: options.directoryReadStates,
          bindingsById: options.bindingsById,
          suppressFileOnlyTerminalState: state.kind === 'refreshing'
        })
      )
    }
  }

  return {
    children: childrenForProjectedNodes([])
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

function emptyProjectionWithBindings(
  options: {
    readonly ownerId: string
    readonly state: BrowserTreeReadState
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserProjection {
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
    id: `read-state:${options.ownerId}`,
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

function formatNavigationSourceDetail(
  row: NavigationRow,
  readiness: SourceReadiness | undefined
): string {
  const lifecycleDetail = readiness?.detail ?? 'Library source.'
  return `${lifecycleDetail} Updated ${formatRowFreshness(row)}.`
}

function formatNavigationDetail(row: NavigationRow): string {
  return `${formatNavigationRowKind(row.rowKind)}. Updated ${formatRowFreshness(row)}.`
}

function formatNavigationRowKind(rowKind: NavigationRow['rowKind']): string {
  switch (rowKind) {
    case 'view':
      return 'View'
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

function formatLiteralFileDetail(node: Extract<ChildRow, { readonly kind: 'file' }>): string {
  if (node.presence === 'missing') {
    return 'File missing'
  }

  if (node.presence === 'removed') {
    return 'File removed'
  }

  switch (node.fileClass) {
    case 'audio':
      return 'Audio file'
    case 'video':
      return 'Video file'
    case 'image':
      return 'Image file'
    case 'unsupported':
      return 'Unsupported file'
    case 'none':
      return 'File'
  }
}

function literalFileIcon(
  fileClass: Extract<ChildRow, { readonly kind: 'file' }>['fileClass']
): BrowserTreeIcon {
  switch (fileClass) {
    case 'audio':
      return 'music'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'unsupported':
      return 'metadata'
    case 'none':
      return 'file'
  }
}

function formatUnknownDirectoryChildReadinessDetail(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): string {
  switch (node.directoryScanState) {
    case 'pending':
      return 'Folder child scopes have not been proven yet.'
    case 'scanning':
      return 'Folder child scopes are being probed.'
    case 'blocked':
      return 'Folder child scopes are blocked.'
    case 'failed':
      return 'Folder child scope read failed.'
    case 'complete':
      return 'Hierarchy read model completed without proving folder child-scope readiness.'
  }
}

function formatUnknownDirectoryChildReadinessLabel(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): string {
  switch (node.directoryScanState) {
    case 'pending':
      return 'Folder child scopes pending'
    case 'scanning':
      return 'Probing folder child scopes'
    case 'blocked':
      return 'Folder child scopes blocked'
    case 'failed':
      return 'Folder child scope read failed'
    case 'complete':
      return 'Hierarchy readiness unresolved'
  }
}

function unknownDirectoryChildReadinessState(
  node: Extract<ChildRow, { readonly kind: 'directory' }>
): BrowserTreeReadState {
  switch (node.directoryScanState) {
    case 'pending':
    case 'scanning':
      return 'loading'
    case 'blocked':
      return 'unavailable'
    case 'failed':
      return 'error'
    case 'complete':
      return 'error'
  }
}
