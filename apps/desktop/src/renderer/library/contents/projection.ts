import type { ChildRow, Presence } from '../../../shared/libraryHierarchy/readChildren'
import type {
  ContentsReadResult,
  ContentsResult,
  ContentsFileRow,
  PrimaryMediaSummary
} from '../../../shared/libraryContents/read'
import type { ContentsBoundaryState } from '../boundary/contentsRead'
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
  readonly mediaClass?: 'audio' | 'video' | 'image'
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
  readonly contentsState?: ContentsBoundaryState
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
        contentsState: options.contentsState
      })
    case 'directory':
      return projectDirectoryContents({
        state: options.state,
        selectedNodeId,
        binding,
        contentsState: options.contentsState
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
          binding.state === 'loading'
            ? 'loading'
            : binding.state === 'error'
              ? 'failed'
              : binding.state === 'notLoaded'
                ? 'notLoaded'
                : 'ready',
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
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const title = formatSourceDisplayName(options.binding.target.label)
  return projectContentsState({
    ownerId: options.selectedNodeId,
    title,
    contentsState: options.contentsState
  })
}

function projectDirectoryContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'directory' }>
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const directoryRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const title = directoryRow?.label ?? 'Selected folder'
  return projectContentsState({
    ownerId: options.selectedNodeId,
    title,
    contentsState: options.contentsState
  })
}

function projectContentsState(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const state = options.contentsState

  if (state === undefined || state.kind === 'idle') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.ownerId,
      title: options.title,
      state: 'loading',
      label: 'Loading contents',
      detail: state?.detail ?? 'Loading contents.'
    })
  }

  if (state.kind === 'loading') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.ownerId,
      title: options.title,
      state: 'loading',
      label: 'Loading contents',
      detail: state.detail ?? 'Loading contents.'
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

  return projectContentsReadResult({
    ownerId: options.ownerId,
    title: options.title,
    result: state.result
  })
}

function projectContentsReadResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsReadResult
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

  return projectContentsResult({
    ownerId: options.ownerId,
    title: options.title,
    result: options.result.result
  })
}

function projectContentsResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsResult
}): ContentProjection {
  const result = options.result
  const rows = result.rows.map(contentsRow)

  if (rows.length === 0) {
    return {
      kind: contentsProjectionKind(result),
      title: options.title,
      detail: contentsDetail(result),
      rows: [
        stateRow({
          ownerId: options.ownerId,
          state: contentsStateRowState(result),
          label: contentsStateLabel(result),
          detail: contentsDetail(result)
        })
      ]
    }
  }

  return {
    kind: contentsProjectionKind(result),
    title: options.title,
    detail: contentsDetail(result),
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

function contentsRow(row: ContentsFileRow): ContentRow {
  const icon = row.mediaClass === 'image' ? 'image' : row.mediaClass === 'video' ? 'video' : 'music'
  const detail =
    row.primaryMedia === undefined
      ? sourceFileRowDetail(row)
      : primaryMediaRowDetail(row.primaryMedia, row)
  return {
    id: row.id,
    kind: 'file',
    label: row.label,
    presence: row.presence,
    detail,
    icon,
    mediaClass: row.mediaClass,
    ...(row.availabilityState === undefined ? {} : { availabilityState: row.availabilityState })
  }
}

function sourceFileRowDetail(row: ContentsFileRow): string {
  if (row.presence === 'missing') {
    return 'File missing'
  }

  if (row.presence === 'removed') {
    return 'File removed'
  }

  return row.relativePath ?? sourceFileMediaLabel(row.mediaClass)
}

function primaryMediaRowDetail(primaryMedia: PrimaryMediaSummary, row: ContentsFileRow): string {
  const parts = [primaryMedia.artist, primaryMedia.album].filter(
    (value): value is string => value !== undefined && value.trim().length > 0
  )
  const base =
    parts.length > 0
      ? parts.join(' - ')
      : (row.relativePath ?? sourceFileMediaLabel(row.mediaClass))

  switch (row.availabilityState) {
    case 'available':
      return base
    case 'degraded':
      return `Degraded - ${base}`
    case 'unavailable':
      return `Unavailable - ${base}`
    case undefined:
      return base
  }

  return base
}

function sourceFileMediaLabel(mediaClass: ContentsFileRow['mediaClass']): string {
  switch (mediaClass) {
    case 'audio':
      return 'Audio file'
    case 'video':
      return 'Video file'
    case 'image':
      return 'Image file'
  }

  return 'File'
}

function contentsProjectionKind(result: ContentsResult): ContentProjectionKind {
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

function contentsStateRowState(result: ContentsResult): Exclude<ContentRow['state'], undefined> {
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

function contentsStateLabel(result: ContentsResult): string {
  const rowSubject = contentsRowSubject(result)

  switch (result.state) {
    case 'ready':
    case 'empty':
      return result.coverage.emptyResultAuthoritative
        ? `No ${rowSubject} found`
        : `No ${rowSubject} found yet`
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

function contentsDetail(result: ContentsResult): string {
  if (result.detail !== undefined) {
    return result.detail
  }

  if (result.rows.length === 0 && (result.state === 'ready' || result.state === 'empty')) {
    return contentsStateLabel(result)
  }

  if (result.rows.length === 1) {
    return contentsCoveragePrefix(result) ?? `1 ${contentsCountSubject(result, 1)} loaded.`
  }

  const prefix = contentsCoveragePrefix(result)
  const count = `${result.rows.length} ${contentsCountSubject(result, result.rows.length)} loaded.`
  return prefix === undefined ? count : `${prefix} ${count}`
}

function contentsRowSubject(result: ContentsResult): 'primary media' | 'visible files' {
  return result.policy.rowProfile.kind === 'primaryMedia' ? 'primary media' : 'visible files'
}

function contentsCountSubject(result: ContentsResult, count: number): string {
  if (result.policy.rowProfile.kind === 'primaryMedia') {
    return count === 1 ? 'primary media item' : 'primary media items'
  }

  return count === 1 ? 'visible file' : 'visible files'
}

function contentsCoveragePrefix(result: ContentsResult): string | undefined {
  if (result.state === 'partial') {
    return 'Still indexing. Results may be incomplete.'
  }

  if (result.coverage.state === 'pending' || result.coverage.state === 'scanning') {
    return 'Indexing is incomplete.'
  }

  if (result.coverage.state === 'incomplete') {
    return 'One or more accepted source locations are missing. Results may be incomplete.'
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
  readonly action?: ContentRowAction
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
        detail: options.detail,
        ...(options.action === undefined ? {} : { action: options.action })
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
    case 'notLoaded':
      return 'notLoaded'
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
    case 'notLoaded':
      return 'Not loaded'
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
