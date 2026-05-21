import type {
  LibraryHierarchyReadChildrenEntryPoint,
  LibraryHierarchyReadChildrenNode
} from '../../shared/libraryHierarchy/readChildren'
import type { LibraryNavigationRow } from '../../shared/libraryNavigation/readRows'
import type {
  ContinuationReadTarget,
  DirectoryReadState,
  DirectoryReadTarget,
  HierarchyProjectionRow,
  HierarchyState,
  LoadedHierarchyChildrenState,
  SourceReadState,
  SourceReadTarget
} from './hierarchyState'
import type { BrowserTreeNode, BrowserTreeNodeId } from './tree/types'

export type HierarchyProjection = {
  readonly kind: 'tree'
  readonly nodes: readonly BrowserTreeNode[]
  readonly rowsByNodeId: ReadonlyMap<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly sourceReadTargetsByNodeId: ReadonlyMap<BrowserTreeNodeId, SourceReadTarget>
  readonly directoryReadTargetsByNodeId: ReadonlyMap<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: ReadonlyMap<BrowserTreeNodeId, ContinuationReadTarget>
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export function projectState(state: HierarchyState): HierarchyProjection | undefined {
  if (state.navigationReadResult === undefined) {
    return undefined
  }

  return projectNavigationResult(state)
}

function projectNavigationResult(state: HierarchyState): HierarchyProjection {
  const rowsByNodeId = new Map<BrowserTreeNodeId, HierarchyProjectionRow>()
  const sourceReadTargetsByNodeId = new Map<BrowserTreeNodeId, SourceReadTarget>()
  const directoryReadTargetsByNodeId = new Map<BrowserTreeNodeId, DirectoryReadTarget>()
  const continuationReadTargetsByNodeId = new Map<BrowserTreeNodeId, ContinuationReadTarget>()
  const result = state.navigationReadResult

  if (result === undefined) {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'loading',
      label: 'Loading navigation',
      detail: 'Loading library navigation rows.'
    })
  }

  if (result.state !== 'ready') {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'error',
      label: 'Navigation unavailable',
      detail: result.error.message
    })
  }

  if (result.rows.length === 0) {
    return emptyProjection({
      ownerId: 'navigation',
      state: 'empty',
      label: 'No library sources',
      detail: 'No persisted library navigation rows are available.'
    })
  }

  return {
    kind: 'tree',
    nodes: result.rows.map((row) =>
      projectNavigationRow({
        row,
        sourceReadStates: state.sourceReadStates,
        directoryReadStates: state.directoryReadStates,
        rowsByNodeId,
        sourceReadTargetsByNodeId,
        directoryReadTargetsByNodeId,
        continuationReadTargetsByNodeId
      })
    ),
    rowsByNodeId,
    sourceReadTargetsByNodeId,
    directoryReadTargetsByNodeId,
    continuationReadTargetsByNodeId
  }
}

function projectNavigationRow(options: {
  readonly row: LibraryNavigationRow
  readonly sourceReadStates: ReadonlyMap<string, SourceReadState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly sourceReadTargetsByNodeId: Map<BrowserTreeNodeId, SourceReadTarget>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
}): BrowserTreeNode {
  const nodeId = navigationNodeId(options.row)
  const sourceTarget = sourceReadTargetFor(options.row)

  if (sourceTarget !== undefined) {
    options.rowsByNodeId.set(nodeId, {
      kind: 'sourceEntry',
      navigationRow: options.row,
      target: sourceTarget
    })
    options.sourceReadTargetsByNodeId.set(nodeId, sourceTarget)

    return {
      id: nodeId,
      label: options.row.displayName,
      badgeLabel: 'Source',
      detail: formatNavigationSourceDetail(options.row),
      childrenState: projectSourceChildren({
        ownerId: nodeId,
        target: sourceTarget,
        state: options.sourceReadStates.get(nodeId),
        directoryReadStates: options.directoryReadStates,
        rowsByNodeId: options.rowsByNodeId,
        directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
        continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
      })
    }
  }

  options.rowsByNodeId.set(nodeId, {
    kind: 'navigation',
    navigationRow: options.row
  })

  return {
    id: nodeId,
    label: options.row.displayName,
    badgeLabel: 'Navigation',
    detail: formatNavigationDetail(options.row),
    childrenState: {
      kind: 'loaded',
      children: [
        trackedReadStateNode(
          {
            ownerId: nodeId,
            state: 'unavailable',
            label: 'Unavailable',
            detail: 'This navigation row does not expose a literal hierarchy entry point yet.'
          },
          options.rowsByNodeId
        )
      ]
    }
  }
}

function projectSourceChildren(options: {
  readonly ownerId: string
  readonly target: SourceReadTarget
  readonly state: SourceReadState | undefined
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
}): BrowserTreeNode['childrenState'] {
  const state = options.state

  if (state === undefined || state.kind === 'unloaded') {
    return {
      kind: 'unloaded',
      detail: state?.detail ?? 'Literal hierarchy not loaded yet.'
    }
  }

  if (state.kind === 'loading') {
    return {
      kind: 'loaded',
      children: [
        trackedReadStateNode(
          {
            ownerId: options.ownerId,
            state: 'loading',
            label: 'Loading hierarchy',
            detail: state.detail ?? 'Loading literal hierarchy children.'
          },
          options.rowsByNodeId
        )
      ]
    }
  }

  if (state.kind === 'failed') {
    return {
      kind: 'loaded',
      children: [
        trackedReadStateNode(
          {
            ownerId: options.ownerId,
            state: 'error',
            label: 'Hierarchy unavailable',
            detail: state.detail
          },
          options.rowsByNodeId
        )
      ]
    }
  }

  return {
    kind: 'loaded',
    children: projectLoadedHierarchyChildren({
      ownerId: options.ownerId,
      children: state.children,
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
      continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
    })
  }
}

function projectLoadedHierarchyChildren(options: {
  readonly ownerId: string
  readonly children: LoadedHierarchyChildrenState
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
}): readonly BrowserTreeNode[] {
  if (options.children.rows.length === 0 && options.children.nextOffset === undefined) {
    return [
      trackedReadStateNode(
        {
          ownerId: options.ownerId,
          state: 'empty',
          label: 'Empty folder',
          detail: 'No literal hierarchy rows are available here.'
        },
        options.rowsByNodeId
      )
    ]
  }

  const projectedNodes = projectLiteralNodes({
    nodes: options.children.rows,
    entryPoint: options.children.entryPoint,
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    directoryReadStates: options.directoryReadStates,
    rowsByNodeId: options.rowsByNodeId,
    directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
    continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
  })

  if (options.children.nextOffset === undefined) {
    return projectedNodes
  }

  return [
    ...projectedNodes,
    trackedContinuationNode(
      {
        ownerId: options.ownerId,
        children: options.children
      },
      options.rowsByNodeId,
      options.continuationReadTargetsByNodeId
    )
  ]
}

function projectLiteralNodes(options: {
  readonly nodes: readonly LibraryHierarchyReadChildrenNode[]
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
}): readonly BrowserTreeNode[] {
  return options.nodes.map((node) =>
    projectLiteralNode({
      node,
      entryPoint: options.entryPoint,
      ...(options.label === undefined ? {} : { label: options.label }),
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
      continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
    })
  )
}

function projectLiteralNode(options: {
  readonly node: LibraryHierarchyReadChildrenNode
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label?: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
}): BrowserTreeNode {
  const node = options.node

  if (node.kind === 'directory') {
    const target = {
      entryPoint: copyReadEntryPoint(options.entryPoint),
      ...(options.label === undefined ? {} : { label: options.label }),
      sourceDirectoryId: node.sourceDirectoryId
    }
    options.directoryReadTargetsByNodeId.set(node.id, target)
    options.rowsByNodeId.set(node.id, {
      kind: 'literalDirectory',
      sourceDirectoryId: node.sourceDirectoryId,
      ...(node.parentSourceDirectoryId === undefined
        ? {}
        : { parentSourceDirectoryId: node.parentSourceDirectoryId }),
      entryPoint: target.entryPoint
    })

    return {
      id: node.id,
      label: node.label,
      badgeLabel: 'Folder',
      detail: formatDirectoryDetail(node.presenceState),
      childrenState: projectDirectoryChildren({
        ownerId: node.id,
        target,
        state: options.directoryReadStates.get(node.sourceDirectoryId),
        directoryReadStates: options.directoryReadStates,
        rowsByNodeId: options.rowsByNodeId,
        directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
        continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
      })
    }
  }

  options.rowsByNodeId.set(node.id, {
    kind: 'literalFile',
    sourceFileId: node.sourceFileId,
    ...(node.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: node.parentSourceDirectoryId }),
    entryPoint: copyReadEntryPoint(options.entryPoint)
  })

  return {
    id: node.id,
    label: node.label,
    badgeLabel: 'File',
    detail: formatFileDetail(node.presenceState),
    childrenState: { kind: 'leaf' }
  }
}

function projectDirectoryChildren(options: {
  readonly ownerId: string
  readonly target: DirectoryReadTarget
  readonly state: DirectoryReadState | undefined
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
  readonly continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
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
      kind: 'loaded',
      children: [
        trackedReadStateNode(
          {
            ownerId: options.ownerId,
            state: 'loading',
            label: 'Loading children',
            detail: state.detail ?? 'Loading children.'
          },
          options.rowsByNodeId
        )
      ]
    }
  }

  if (state.kind === 'failed') {
    return {
      kind: 'loaded',
      children: [
        trackedReadStateNode(
          {
            ownerId: options.ownerId,
            state: 'error',
            label: 'Children unavailable',
            detail: state.detail
          },
          options.rowsByNodeId
        )
      ]
    }
  }

  return {
    kind: 'loaded',
    children: projectLoadedHierarchyChildren({
      ownerId: options.ownerId,
      children: state.children,
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId,
      continuationReadTargetsByNodeId: options.continuationReadTargetsByNodeId
    })
  }
}

function emptyProjection(options: {
  readonly ownerId: string
  readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
  readonly label: string
  readonly detail: string
}): HierarchyProjection {
  const rowsByNodeId = new Map<BrowserTreeNodeId, HierarchyProjectionRow>()
  const node = trackedReadStateNode(options, rowsByNodeId)

  return {
    kind: 'tree',
    nodes: [node],
    rowsByNodeId,
    sourceReadTargetsByNodeId: new Map(),
    directoryReadTargetsByNodeId: new Map(),
    continuationReadTargetsByNodeId: new Map()
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
    label: options.label,
    badgeLabel: 'State',
    detail: options.detail,
    childrenState: { kind: 'leaf' }
  }
}

function trackedReadStateNode(
  options: {
    readonly ownerId: string
    readonly state: 'loading' | 'empty' | 'unavailable' | 'error'
    readonly label: string
    readonly detail: string
  },
  rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
): BrowserTreeNode {
  const node = readStateNode(options)

  rowsByNodeId.set(node.id, {
    kind: 'readState',
    state: options.state,
    ownerId: options.ownerId,
    detail: options.detail
  })

  return node
}

function continuationNode(options: {
  readonly ownerId: string
  readonly children: LoadedHierarchyChildrenState
}): {
  readonly node: BrowserTreeNode
  readonly target: ContinuationReadTarget
  readonly detail: string
} {
  const offset = options.children.nextOffset

  if (offset === undefined) {
    throw new Error('Continuation rows require a next offset.')
  }

  const target = {
    ownerNodeId: options.ownerId,
    entryPoint: copyReadEntryPoint(options.children.entryPoint),
    ...(options.children.parentSourceDirectoryId === undefined
      ? {}
      : { parentSourceDirectoryId: options.children.parentSourceDirectoryId }),
    ...(options.children.label === undefined ? {} : { label: options.children.label }),
    offset,
    limit: options.children.limit
  }
  const continuation = options.children.continuation
  const detail =
    continuation?.kind === 'failed'
      ? continuation.detail
      : continuation?.kind === 'loading'
        ? (continuation.detail ?? 'Loading more literal hierarchy rows.')
        : formatContinuationDetail(offset, options.children.limit, options.children.totalRows)
  const childrenState =
    continuation?.kind === 'failed'
      ? ({
          kind: 'failed',
          detail
        } as const)
      : continuation?.kind === 'loading'
        ? ({
            kind: 'loading',
            detail
          } as const)
        : ({
            kind: 'unloaded',
            detail
          } as const)

  return {
    node: {
      id: `continuation:${options.ownerId}:${offset}`,
      label:
        continuation?.kind === 'failed'
          ? 'Retry loading more rows'
          : continuation?.kind === 'loading'
            ? 'Loading more rows'
            : 'Load more rows',
      badgeLabel: 'More',
      childrenState
    },
    target,
    detail
  }
}

function trackedContinuationNode(
  options: {
    readonly ownerId: string
    readonly children: LoadedHierarchyChildrenState
  },
  rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>,
  continuationReadTargetsByNodeId: Map<BrowserTreeNodeId, ContinuationReadTarget>
): BrowserTreeNode {
  const { node, target, detail } = continuationNode(options)
  const continuation = options.children.continuation

  rowsByNodeId.set(node.id, {
    kind: 'continuation',
    state:
      continuation?.kind === 'failed'
        ? 'error'
        : continuation?.kind === 'loading'
          ? 'loading'
          : 'available',
    ownerId: options.ownerId,
    target,
    detail
  })
  continuationReadTargetsByNodeId.set(node.id, target)

  return node
}

function sourceReadTargetFor(row: LibraryNavigationRow): SourceReadTarget | undefined {
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

function navigationNodeId(row: LibraryNavigationRow): BrowserTreeNodeId {
  return `navigation-row:${row.navigationRowId}`
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function copyReadEntryPoint(
  entryPoint: LibraryHierarchyReadChildrenEntryPoint
): LibraryHierarchyReadChildrenEntryPoint {
  switch (entryPoint.kind) {
    case 'source':
      return {
        kind: 'source',
        sourceId: entryPoint.sourceId
      }
    case 'sourceLocation':
      return {
        kind: 'sourceLocation',
        sourceLocationId: entryPoint.sourceLocationId
      }
  }
}

function formatNavigationSourceDetail(row: LibraryNavigationRow): string {
  return `Navigation source row. ${formatRowFreshness(row)}`
}

function formatNavigationDetail(row: LibraryNavigationRow): string {
  return `Navigation ${formatNavigationRowKind(row.rowKind)} row. ${formatRowFreshness(row)}`
}

function formatNavigationRowKind(rowKind: LibraryNavigationRow['rowKind']): string {
  switch (rowKind) {
    case 'view':
      return 'view'
    case 'collectionGroup':
      return 'collection group'
    case 'playlist':
      return 'playlist'
    case 'prepPolicyGroup':
      return 'preparation group'
    case 'prepPolicyScope':
      return 'preparation scope'
    case 'source':
      return 'source'
    case 'locationGroup':
      return 'location group'
    case 'location':
      return 'location'
  }
}

function formatRowFreshness(row: LibraryNavigationRow): string {
  return `Updated ${row.updatedAtMs}.`
}

function formatContinuationDetail(offset: number, limit: number, totalRows: number): string {
  return `Rows ${offset + 1}-${Math.min(offset + limit, totalRows)} of ${totalRows} are available.`
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
