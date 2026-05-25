import type { LocalRoot } from '../../../shared/libraryRoots/readLocalRoots'
import type { EntryPoint } from '../../../shared/libraryHierarchy/readChildren'
import type {
  LocalRootScanStatus,
  LocalRootsReadState,
  RemoveSourceStatus
} from '../boundary/localRootActions'
import type { BrowserProjection } from '../tree/projection'
import type { RootLifecycleRefreshStatus } from './rootLifecycle'
import type { RowBinding } from '../state'
import type { BrowserTreeNodeId } from '../tree/types'

export type VisibleSourceRow = {
  readonly nodeId: BrowserTreeNodeId
  readonly rootId: string
  readonly localRoot?: LocalRoot
}

export type SourceActionModel = {
  readonly visibleSourceRows: readonly VisibleSourceRow[]
  readonly selectedRemovableSourceRootId?: string
  readonly removeVisible: boolean
  readonly removeEnabled: boolean
  readonly reasonUnavailable?: string
}

export type SourceActionModelInput = {
  readonly projection: BrowserProjection | undefined
  readonly selectedNodeId: BrowserTreeNodeId | undefined
  readonly localRootsReadState: LocalRootsReadState
  readonly scanStatus: LocalRootScanStatus
  readonly removeSourceStatus: RemoveSourceStatus
  readonly refreshStatus: RootLifecycleRefreshStatus
}

export function deriveSourceActionModel(input: SourceActionModelInput): SourceActionModel {
  const visibleSourceRows = visibleLocalSourceRows(input.projection, input.localRootsReadState)
  const visibleRootIds = new Set(visibleSourceRows.map((row) => row.rootId))
  const removableRootIds = localRootIds(input.localRootsReadState)
  const visibleRemovableRows = visibleSourceRows.filter((row) => removableRootIds.has(row.rootId))
  const selectedSourceRootId = sourceRootIdForSelectedBinding(input)
  const selectedRemovableSourceRootId =
    selectedSourceRootId !== undefined &&
    visibleRootIds.has(selectedSourceRootId) &&
    removableRootIds.has(selectedSourceRootId)
      ? selectedSourceRootId
      : selectedFallbackRootId(input.selectedNodeId, visibleRemovableRows)
  const removeVisible = selectedRemovableSourceRootId !== undefined
  const reasonUnavailable = removeUnavailableReason({
    ...input,
    visibleSourceRows,
    visibleRemovableRows,
    visibleRootIds,
    removableRootIds,
    selectedSourceRootId,
    removeVisible
  })

  return {
    visibleSourceRows,
    ...(selectedRemovableSourceRootId === undefined ? {} : { selectedRemovableSourceRootId }),
    removeVisible,
    removeEnabled:
      removeVisible &&
      input.scanStatus !== 'scanning' &&
      input.removeSourceStatus !== 'removing' &&
      input.refreshStatus !== 'refreshing',
    ...(reasonUnavailable === undefined ? {} : { reasonUnavailable })
  }
}

export function hasVisibleSourceRootBinding(
  projection: BrowserProjection | undefined,
  rootId: string
): boolean {
  return visibleLocalSourceRows(projection, { kind: 'unread' }).some((row) => row.rootId === rootId)
}

function visibleLocalSourceRows(
  projection: BrowserProjection | undefined,
  localRootsReadState: LocalRootsReadState
): readonly VisibleSourceRow[] {
  if (projection === undefined) {
    return []
  }

  const rootsById = localRootsById(localRootsReadState)
  const rows: VisibleSourceRow[] = []

  for (const [nodeId, binding] of projection.bindingsById) {
    if (binding.kind !== 'source') {
      continue
    }

    const rootId = localRootIdForEntryPoint(binding.target.entryPoint)

    if (rootId === undefined) {
      continue
    }

    const localRoot = rootsById.get(rootId)
    rows.push({
      nodeId,
      rootId,
      ...(localRoot === undefined ? {} : { localRoot })
    })
  }

  return rows
}

function sourceRootIdForSelectedBinding(input: SourceActionModelInput): string | undefined {
  if (input.projection === undefined || input.selectedNodeId === undefined) {
    return undefined
  }

  return localRootIdForBinding(input.projection.bindingsById.get(input.selectedNodeId))
}

function selectedFallbackRootId(
  selectedNodeId: BrowserTreeNodeId | undefined,
  visibleRemovableRows: readonly VisibleSourceRow[]
): string | undefined {
  if (selectedNodeId !== undefined || visibleRemovableRows.length !== 1) {
    return undefined
  }

  return visibleRemovableRows[0]?.rootId
}

function removeUnavailableReason(
  input: SourceActionModelInput & {
    readonly visibleSourceRows: readonly VisibleSourceRow[]
    readonly visibleRemovableRows: readonly VisibleSourceRow[]
    readonly visibleRootIds: ReadonlySet<string>
    readonly removableRootIds: ReadonlySet<string>
    readonly selectedSourceRootId: string | undefined
    readonly removeVisible: boolean
  }
): string | undefined {
  if (input.removeVisible) {
    if (input.removeSourceStatus === 'removing') {
      return 'A source removal is already in progress.'
    }

    if (input.scanStatus === 'scanning') {
      return 'A source scan is still running.'
    }

    if (input.refreshStatus === 'refreshing') {
      return 'The library view is refreshing.'
    }

    return undefined
  }

  if (input.localRootsReadState.kind === 'failed') {
    return input.localRootsReadState.message
  }

  if (input.localRootsReadState.kind === 'unread') {
    return 'Local source actions are not ready yet.'
  }

  if (input.visibleSourceRows.length === 0) {
    return 'No removable local source is visible.'
  }

  if (input.selectedNodeId === undefined) {
    return input.visibleRemovableRows.length > 1
      ? 'Select a local source to remove it.'
      : 'No removable local source is visible.'
  }

  if (input.selectedSourceRootId === undefined) {
    return 'The selected row does not belong to a removable local source.'
  }

  if (!input.visibleRootIds.has(input.selectedSourceRootId)) {
    return 'The selected source is no longer visible.'
  }

  if (!input.removableRootIds.has(input.selectedSourceRootId)) {
    return 'The selected source is not a removable local root.'
  }

  return undefined
}

function localRootIdForBinding(binding: RowBinding | undefined): string | undefined {
  if (binding === undefined) {
    return undefined
  }

  switch (binding.kind) {
    case 'source':
      return localRootIdForEntryPoint(binding.target.entryPoint)
    case 'directory':
    case 'file':
      return localRootIdForEntryPoint(binding.entryPoint)
    case 'more':
      return localRootIdForEntryPoint(binding.target.entryPoint)
    case 'navigation':
    case 'readState':
      return undefined
  }
}

function localRootIdForEntryPoint(entryPoint: EntryPoint): string | undefined {
  // Absolute-path local root ids are source ids in this slice; do not infer from labels or paths.
  return entryPoint.kind === 'source' ? entryPoint.sourceId : undefined
}

function localRootIds(localRootsReadState: LocalRootsReadState): ReadonlySet<string> {
  if (localRootsReadState.kind !== 'ready') {
    return new Set()
  }

  return new Set(localRootsReadState.roots.map((root) => root.rootId))
}

function localRootsById(localRootsReadState: LocalRootsReadState): ReadonlyMap<string, LocalRoot> {
  if (localRootsReadState.kind !== 'ready') {
    return new Map()
  }

  return new Map(localRootsReadState.roots.map((root) => [root.rootId, root]))
}
