import type { ChildRow, Presence } from '../../../shared/libraryHierarchy/readChildren'
import type {
  SelectedContentsReadResult,
  SelectedContentsResult,
  SelectedContentsRow
} from '../../../shared/librarySelectedContents/read'
import type { SelectedContentsBoundaryState } from '../boundary/selectedContentsRead'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserState, RowBinding } from '../state'
import type { BrowserTreeNodeId } from '../tree/types'
import { formatSourceDisplayName } from '../tree/sourcePresentation'

export type ContentProjectionKind =
  | 'emptySelection'
  | 'unsupported'
  | 'notLoaded'
  | 'loading'
  | 'failed'
  | 'ready'

export type ContentRowKind = 'directory' | 'file' | 'state' | 'more'

export type ContentRowIcon =
  | 'folder'
  | 'music'
  | 'video'
  | 'image'
  | 'cueSheet'
  | 'playlist'
  | 'metadata'
  | 'more'
  | 'loading'
  | 'warning'
  | 'state'

export type ContentRowAction = {
  readonly kind: 'loadChildren' | 'loadMore'
  readonly nodeId: BrowserTreeNodeId
  readonly label: string
}

export type ContentRow = {
  readonly id: string
  readonly kind: ContentRowKind
  readonly label: string
  readonly presence?: Presence
  readonly detail?: string
  readonly icon?: ContentRowIcon
  readonly state?: 'empty' | 'notLoaded' | 'loading' | 'failed' | 'unsupported' | 'file'
  readonly mediaClass?: 'audio' | 'video'
  readonly availabilityState?: 'available' | 'unavailable' | 'degraded'
  readonly action?: ContentRowAction
}

export type ContentProjection = {
  readonly kind: ContentProjectionKind
  readonly title: string
  readonly detail?: string
  readonly rows: readonly ContentRow[]
}

export type ProjectContentsOptions = {
  readonly state: BrowserState
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly bindingsById?: BrowserProjection['bindingsById']
  readonly selectedContentsState?: SelectedContentsBoundaryState
}

export function projectContents(options: ProjectContentsOptions): ContentProjection {
  const hostProjection = projectHostContents(options.state.hostStatus)

  if (hostProjection !== undefined) {
    return hostProjection
  }

  const selectedNodeId = options.selectedNodeId

  if (selectedNodeId === undefined) {
    return stateProjection({
      kind: 'emptySelection',
      ownerId: 'selection',
      title: 'Library contents',
      state: 'empty',
      label: 'Nothing selected',
      detail: 'Select a source or folder to see its contents.'
    })
  }

  const binding = options.bindingsById?.get(selectedNodeId)

  if (binding === undefined) {
    return stateProjection({
      kind: 'unsupported',
      ownerId: selectedNodeId,
      title: 'Selection unavailable',
      state: 'unsupported',
      label: 'Selection unavailable',
      detail: 'This item is not available right now.'
    })
  }

  switch (binding.kind) {
    case 'source':
      return projectSourceContents({
        selectedNodeId,
        binding,
        selectedContentsState: options.selectedContentsState
      })
    case 'directory':
      return projectDirectoryContents({
        state: options.state,
        selectedNodeId,
        binding,
        selectedContentsState: options.selectedContentsState
      })
    case 'file':
      return projectFileContents({
        state: options.state,
        selectedNodeId
      })
    case 'navigation':
      return stateProjection({
        kind: 'unsupported',
        ownerId: selectedNodeId,
        title: binding.navigationRow.displayName,
        state: 'unsupported',
        label: 'Contents unavailable',
        detail: 'Contents for this item are not available yet.'
      })
    case 'readState':
      return stateProjection({
        kind:
          binding.state === 'loading' ? 'loading' : binding.state === 'error' ? 'failed' : 'ready',
        ownerId: selectedNodeId,
        title: 'Status',
        state: contentStateFromReadState(binding.state),
        label: formatReadStateLabel(binding.state),
        detail: binding.detail
      })
    case 'more':
      return {
        kind: 'ready',
        title: 'More items',
        detail: binding.detail,
        rows: [contentMoreRow(selectedNodeId, binding)]
      }
  }
}

function projectHostContents(
  hostStatus: BrowserState['hostStatus']
): ContentProjection | undefined {
  if (hostStatus === undefined) {
    return undefined
  }

  if (hostStatus.state === 'failed') {
    return stateProjection({
      kind: 'failed',
      ownerId: 'host',
      title: 'Library engine unavailable',
      state: 'failed',
      label: 'Library engine failed to start',
      detail: hostStatus.lastError?.message ?? 'The library engine is unavailable.'
    })
  }

  if (hostStatus.state === 'stopping' || hostStatus.state === 'stopped') {
    return stateProjection({
      kind: 'unsupported',
      ownerId: 'host',
      title: 'Library engine unavailable',
      state: 'unsupported',
      label: 'Library engine unavailable',
      detail: 'The library engine is not running.'
    })
  }

  return undefined
}

function projectSourceContents(options: {
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'source' }>
  readonly selectedContentsState: SelectedContentsBoundaryState | undefined
}): ContentProjection {
  const title = formatSourceDisplayName(options.binding.target.label)
  return projectSelectedContentsState({
    ownerId: options.selectedNodeId,
    title,
    selectedContentsState: options.selectedContentsState
  })
}

function projectDirectoryContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'directory' }>
  readonly selectedContentsState: SelectedContentsBoundaryState | undefined
}): ContentProjection {
  const directoryRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const title = directoryRow?.label ?? 'Selected folder'
  return projectSelectedContentsState({
    ownerId: options.selectedNodeId,
    title,
    selectedContentsState: options.selectedContentsState
  })
}

function projectSelectedContentsState(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly selectedContentsState: SelectedContentsBoundaryState | undefined
}): ContentProjection {
  const state = options.selectedContentsState

  if (state === undefined || state.kind === 'idle') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.ownerId,
      title: options.title,
      state: 'loading',
      label: 'Loading selected contents',
      detail: state?.detail ?? 'Loading selected contents.'
    })
  }

  if (state.kind === 'loading') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.ownerId,
      title: options.title,
      state: 'loading',
      label: 'Loading selected contents',
      detail: state.detail ?? 'Loading selected contents.'
    })
  }

  if (state.kind === 'failed') {
    return stateProjection({
      kind: 'failed',
      ownerId: options.ownerId,
      title: options.title,
      state: 'failed',
      label: 'Contents unavailable',
      detail: state.detail
    })
  }

  return projectSelectedContentsReadResult({
    ownerId: options.ownerId,
    title: options.title,
    result: state.result
  })
}

function projectSelectedContentsReadResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: SelectedContentsReadResult
}): ContentProjection {
  if (options.result.state !== 'ready') {
    return stateProjection({
      kind: options.result.state === 'readFailed' ? 'failed' : 'unsupported',
      ownerId: options.ownerId,
      title: options.title,
      state: options.result.state === 'readFailed' ? 'failed' : 'unsupported',
      label: 'Contents unavailable',
      detail: options.result.error.message
    })
  }

  return projectSelectedContentsResult({
    ownerId: options.ownerId,
    title: options.title,
    result: options.result.result
  })
}

function projectSelectedContentsResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: SelectedContentsResult
}): ContentProjection {
  const result = options.result
  const rows = result.rows.map(selectedContentsRow)

  if (rows.length === 0) {
    return {
      kind: selectedContentsProjectionKind(result),
      title: options.title,
      detail: selectedContentsDetail(result),
      rows: [
        stateRow({
          ownerId: options.ownerId,
          state: selectedContentsStateRowState(result),
          label: selectedContentsStateLabel(result),
          detail: selectedContentsDetail(result)
        })
      ]
    }
  }

  return {
    kind: selectedContentsProjectionKind(result),
    title: options.title,
    detail: selectedContentsDetail(result),
    rows
  }
}

function projectFileContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
}): ContentProjection {
  const fileRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const title = fileRow?.label ?? 'Selected file'
  const detail =
    fileRow === undefined ? 'File details are not available.' : formatPresenceDetail(fileRow)

  return stateProjection({
    kind: 'ready',
    ownerId: options.selectedNodeId,
    title,
    state: 'file',
    label: 'File selected',
    detail
  })
}

function selectedContentsRow(row: SelectedContentsRow): ContentRow {
  return {
    id: row.stableId,
    kind: 'file',
    label: row.label,
    detail: selectedContentsRowDetail(row),
    icon: row.mediaClass === 'video' ? 'video' : 'music',
    mediaClass: row.mediaClass,
    availabilityState: row.availabilityState
  }
}

function selectedContentsRowDetail(row: SelectedContentsRow): string {
  const parts = [row.artist, row.album].filter(
    (value): value is string => value !== undefined && value.trim().length > 0
  )
  const base = parts.length > 0 ? parts.join(' - ') : row.relativePath

  switch (row.availabilityState) {
    case 'available':
      return base
    case 'degraded':
      return `Degraded - ${base}`
    case 'unavailable':
      return `Unavailable - ${base}`
  }
}

function selectedContentsProjectionKind(result: SelectedContentsResult): ContentProjectionKind {
  switch (result.state) {
    case 'ready':
    case 'empty':
    case 'partial':
      return 'ready'
    case 'blocked':
    case 'failed':
      return 'failed'
    case 'sourceUnavailable':
    case 'locationMissing':
      return 'unsupported'
  }
}

function selectedContentsStateRowState(
  result: SelectedContentsResult
): Exclude<ContentRow['state'], undefined> {
  switch (result.state) {
    case 'ready':
    case 'empty':
      return 'empty'
    case 'partial':
      return 'loading'
    case 'blocked':
    case 'failed':
      return 'failed'
    case 'sourceUnavailable':
    case 'locationMissing':
      return 'unsupported'
  }
}

function selectedContentsStateLabel(result: SelectedContentsResult): string {
  switch (result.state) {
    case 'ready':
    case 'empty':
      return result.coverage.emptyResultAuthoritative
        ? 'No media found'
        : 'No media found yet'
    case 'partial':
      return 'Still indexing'
    case 'sourceUnavailable':
      return 'Source unavailable'
    case 'locationMissing':
      return 'Folder missing'
    case 'blocked':
      return 'Contents blocked'
    case 'failed':
      return 'Contents failed'
  }
}

function selectedContentsDetail(result: SelectedContentsResult): string {
  if (result.detail !== undefined) {
    return result.detail
  }

  if (result.rows.length === 1) {
    return selectedContentsCoveragePrefix(result) ?? '1 media item loaded.'
  }

  const prefix = selectedContentsCoveragePrefix(result)
  const count = `${result.rows.length} media items loaded.`
  return prefix === undefined ? count : `${prefix} ${count}`
}

function selectedContentsCoveragePrefix(result: SelectedContentsResult): string | undefined {
  if (result.state === 'partial') {
    return 'Still indexing. Results may be incomplete.'
  }

  if (result.coverage.state === 'pending' || result.coverage.state === 'scanning') {
    return 'Indexing is incomplete.'
  }

  return undefined
}

function contentMoreRow(
  nodeId: BrowserTreeNodeId,
  binding: Extract<RowBinding, { readonly kind: 'more' }>
): ContentRow {
  const action =
    binding.state === 'loading'
      ? undefined
      : {
          kind: 'loadMore' as const,
          nodeId,
          label: binding.state === 'error' ? 'Retry' : 'Load more'
        }

  return {
    id: nodeId,
    kind: 'more',
    label:
      binding.state === 'error'
        ? 'Retry loading more'
        : binding.state === 'loading'
          ? 'Loading more'
          : 'Load more',
    detail: binding.detail,
    icon: binding.state === 'loading' ? 'loading' : binding.state === 'error' ? 'warning' : 'more',
    ...(action === undefined ? {} : { action })
  }
}

function stateProjection(options: {
  readonly kind: ContentProjectionKind
  readonly ownerId: string
  readonly title: string
  readonly state: Exclude<ContentRow['state'], undefined>
  readonly label: string
  readonly detail: string
}): ContentProjection {
  return {
    kind: options.kind,
    title: options.title,
    detail: options.detail,
    rows: [
      stateRow({
        ownerId: options.ownerId,
        state: options.state,
        label: options.label,
        detail: options.detail
      })
    ]
  }
}

function stateRow(options: {
  readonly ownerId: string
  readonly state: Exclude<ContentRow['state'], undefined>
  readonly label: string
  readonly detail: string
  readonly action?: ContentRowAction
}): ContentRow {
  return {
    id: `contents-state:${options.ownerId}:${options.state}`,
    kind: 'state',
    label: options.label,
    detail: options.detail,
    state: options.state,
    icon: contentStateIcon(options.state),
    ...(options.action === undefined ? {} : { action: options.action })
  }
}

function findLoadedChildRow(state: BrowserState, nodeId: BrowserTreeNodeId): ChildRow | undefined {
  for (const sourceState of state.sourceReadStates.values()) {
    if (sourceState.kind !== 'loaded') {
      continue
    }

    const row = sourceState.children.rows.find((candidate) => candidate.id === nodeId)

    if (row !== undefined) {
      return row
    }
  }

  for (const directoryState of state.directoryReadStates.values()) {
    if (directoryState.kind !== 'loaded') {
      continue
    }

    const row = directoryState.children.rows.find((candidate) => candidate.id === nodeId)

    if (row !== undefined) {
      return row
    }
  }

  return undefined
}

function formatPresenceDetail(row: ChildRow): string {
  const subject = row.kind === 'directory' ? 'Folder' : 'File'

  switch (row.presence) {
    case 'present':
      return subject
    case 'missing':
      return `${subject} missing`
    case 'removed':
      return `${subject} removed`
  }
}

function contentStateFromReadState(
  state: Extract<RowBinding, { readonly kind: 'readState' }>['state']
): Exclude<ContentRow['state'], undefined> {
  switch (state) {
    case 'loading':
      return 'loading'
    case 'empty':
      return 'empty'
    case 'unavailable':
      return 'unsupported'
    case 'error':
      return 'failed'
  }
}

function formatReadStateLabel(
  state: Extract<RowBinding, { readonly kind: 'readState' }>['state']
): string {
  switch (state) {
    case 'loading':
      return 'Loading'
    case 'empty':
      return 'Empty'
    case 'unavailable':
      return 'Unavailable'
    case 'error':
      return 'Unavailable'
  }
}

function contentStateIcon(state: Exclude<ContentRow['state'], undefined>): ContentRowIcon {
  switch (state) {
    case 'loading':
      return 'loading'
    case 'failed':
    case 'unsupported':
      return 'warning'
    case 'empty':
    case 'notLoaded':
    case 'file':
      return 'state'
  }
}
