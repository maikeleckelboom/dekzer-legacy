import type {
  LibraryHierarchyReadChildrenEntryPoint,
  LibraryHierarchyReadChildrenNode,
  LibraryHierarchyReadChildrenWindow
} from '../../shared/libraryHierarchy/readChildren'
import type { LibraryNavigationRow } from '../../shared/libraryNavigation/readRows'
import type {
  DirectoryReadState,
  DirectoryReadTarget,
  HierarchyProjectionRow,
  HierarchyState,
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
        directoryReadTargetsByNodeId
      })
    ),
    rowsByNodeId,
    sourceReadTargetsByNodeId,
    directoryReadTargetsByNodeId
  }
}

function projectNavigationRow(options: {
  readonly row: LibraryNavigationRow
  readonly sourceReadStates: ReadonlyMap<string, SourceReadState>
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly sourceReadTargetsByNodeId: Map<BrowserTreeNodeId, SourceReadTarget>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
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
        directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
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
      window: state.window,
      entryPoint: options.target.entryPoint,
      label: options.target.label,
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
    })
  }
}

function projectLoadedHierarchyChildren(options: {
  readonly ownerId: string
  readonly window: LibraryHierarchyReadChildrenWindow
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
}): readonly BrowserTreeNode[] {
  if (!isCompleteWindow(options.window)) {
    return [
      trackedReadStateNode(
        {
          ownerId: options.ownerId,
          state: 'error',
          label: 'Partial hierarchy',
          detail: 'The hierarchy read returned a partial child window.'
        },
        options.rowsByNodeId
      )
    ]
  }

  if (options.window.nodes.length === 0) {
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

  return projectLiteralNodes({
    nodes: options.window.nodes,
    entryPoint: options.entryPoint,
    label: options.label,
    directoryReadStates: options.directoryReadStates,
    rowsByNodeId: options.rowsByNodeId,
    directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
  })
}

function projectLiteralNodes(options: {
  readonly nodes: readonly LibraryHierarchyReadChildrenNode[]
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
}): readonly BrowserTreeNode[] {
  return options.nodes.map((node) =>
    projectLiteralNode({
      node,
      entryPoint: options.entryPoint,
      label: options.label,
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
    })
  )
}

function projectLiteralNode(options: {
  readonly node: LibraryHierarchyReadChildrenNode
  readonly entryPoint: LibraryHierarchyReadChildrenEntryPoint
  readonly label: string
  readonly directoryReadStates: ReadonlyMap<string, DirectoryReadState>
  readonly rowsByNodeId: Map<BrowserTreeNodeId, HierarchyProjectionRow>
  readonly directoryReadTargetsByNodeId: Map<BrowserTreeNodeId, DirectoryReadTarget>
}): BrowserTreeNode {
  const node = options.node

  if (node.kind === 'directory') {
    const target = {
      entryPoint: copyReadEntryPoint(options.entryPoint),
      label: options.label,
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
        directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
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
      window: state.window,
      entryPoint: options.target.entryPoint,
      label: options.target.label ?? 'Source',
      directoryReadStates: options.directoryReadStates,
      rowsByNodeId: options.rowsByNodeId,
      directoryReadTargetsByNodeId: options.directoryReadTargetsByNodeId
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
    directoryReadTargetsByNodeId: new Map()
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

function isCompleteWindow(window: {
  readonly offset: number
  readonly nodes: readonly unknown[]
  readonly totalRows: number
}): boolean {
  return window.offset === 0 && window.nodes.length === window.totalRows
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
