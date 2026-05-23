import type { ChildRow, Presence } from '../../../shared/libraryHierarchy/readChildren'
import type { BrowserProjection } from './tree'
import type { BrowserState, LoadedChildren, RowBinding } from '../runtime/state'
import type { BrowserTreeNodeId } from '../tree/types'
import { sameEntryPoint } from '../runtime/entryPoint'
import { classifyLibraryEntryName, type LibraryEntryRole } from './entryPresentation'

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
}

export function projectContents(options: ProjectContentsOptions): ContentProjection {
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
        state: options.state,
        selectedNodeId,
        binding,
        bindingsById: options.bindingsById
      })
    case 'directory':
      return projectDirectoryContents({
        state: options.state,
        selectedNodeId,
        binding,
        bindingsById: options.bindingsById
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

function projectSourceContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'source' }>
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
}): ContentProjection {
  const sourceState = options.state.sourceReadStates.get(options.selectedNodeId)
  const title = options.binding.target.label

  if (sourceState === undefined || sourceState.kind === 'unloaded') {
    const detail = sourceState?.detail ?? 'Contents not loaded yet.'

    return loadableStateProjection({
      kind: 'notLoaded',
      ownerId: options.selectedNodeId,
      title,
      state: 'notLoaded',
      label: 'Contents not loaded',
      detail,
      action: {
        kind: 'loadChildren',
        nodeId: options.selectedNodeId,
        label: 'Load contents'
      }
    })
  }

  if (sourceState.kind === 'loading') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.selectedNodeId,
      title,
      state: 'loading',
      label: 'Loading contents',
      detail: sourceState.detail ?? 'Loading folder contents.'
    })
  }

  if (sourceState.kind === 'failed') {
    return loadableStateProjection({
      kind: 'failed',
      ownerId: options.selectedNodeId,
      title,
      state: 'failed',
      label: 'Contents unavailable',
      detail: sourceState.detail,
      action: {
        kind: 'loadChildren',
        nodeId: options.selectedNodeId,
        label: 'Retry'
      }
    })
  }

  return projectLoadedContents({
    ownerNodeId: options.selectedNodeId,
    title,
    children: sourceState.children,
    bindingsById: options.bindingsById
  })
}

function projectDirectoryContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'directory' }>
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
}): ContentProjection {
  const directoryRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const directoryState = options.state.directoryReadStates.get(options.binding.directoryId)
  const title = directoryRow?.label ?? 'Selected folder'

  if (directoryState === undefined || directoryState.kind === 'unloaded') {
    const detail = directoryState?.detail ?? 'Contents not loaded yet.'

    return loadableStateProjection({
      kind: 'notLoaded',
      ownerId: options.selectedNodeId,
      title,
      state: 'notLoaded',
      label: 'Contents not loaded',
      detail,
      action: {
        kind: 'loadChildren',
        nodeId: options.selectedNodeId,
        label: 'Load contents'
      }
    })
  }

  if (directoryState.kind === 'loading') {
    return stateProjection({
      kind: 'loading',
      ownerId: options.selectedNodeId,
      title,
      state: 'loading',
      label: 'Loading contents',
      detail: directoryState.detail ?? 'Loading contents.'
    })
  }

  if (directoryState.kind === 'failed') {
    return loadableStateProjection({
      kind: 'failed',
      ownerId: options.selectedNodeId,
      title,
      state: 'failed',
      label: 'Contents unavailable',
      detail: directoryState.detail,
      action: {
        kind: 'loadChildren',
        nodeId: options.selectedNodeId,
        label: 'Retry'
      }
    })
  }

  return projectLoadedContents({
    ownerNodeId: options.selectedNodeId,
    title,
    children: directoryState.children,
    bindingsById: options.bindingsById
  })
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

function projectLoadedContents(options: {
  readonly ownerNodeId: BrowserTreeNodeId
  readonly title: string
  readonly children: LoadedChildren
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
}): ContentProjection {
  const rows: ContentRow[] = options.children.rows
    .filter((row) => isVisibleContentRow(row))
    .map((row) => contentChildRow(row))

  if (options.children.nextOffset !== undefined) {
    rows.push(contentMoreRowForLoadedChildren(options))
  }

  if (rows.length === 0) {
    rows.push(
      stateRow({
        ownerId: options.ownerNodeId,
        state: 'empty',
        label: 'Empty folder',
        detail: 'No items are available here.'
      })
    )
  }

  return {
    kind: 'ready',
    title: options.title,
    detail: formatLoadedDetail(options.children),
    rows
  }
}

function contentChildRow(row: ChildRow): ContentRow {
  const icon = contentChildRowIcon(row)

  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    presence: row.presence,
    detail: formatPresenceDetail(row),
    icon
  }
}

function contentChildRowIcon(row: ChildRow): ContentRowIcon {
  if (row.kind === 'directory') {
    return 'folder'
  }

  const presentation = classifyLibraryEntryName(row.label)

  return presentationRoleToContentRowIcon(presentation.role)
}

function presentationRoleToContentRowIcon(role: LibraryEntryRole): ContentRowIcon {
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
      return 'music'
  }
}

function isVisibleContentRow(row: ChildRow): boolean {
  if (row.kind === 'directory') {
    return true
  }

  const presentation = classifyLibraryEntryName(row.label)

  return presentation.visibility !== 'hiddenNonMedia'
}

function contentMoreRowForLoadedChildren(options: {
  readonly ownerNodeId: BrowserTreeNodeId
  readonly children: LoadedChildren
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
}): ContentRow {
  const moreBinding = findMoreBinding({
    ownerNodeId: options.ownerNodeId,
    children: options.children,
    bindingsById: options.bindingsById
  })

  if (moreBinding === undefined) {
    return stateRow({
      ownerId: options.ownerNodeId,
      state: 'unsupported',
      label: 'More items unavailable',
      detail: 'More items are available.'
    })
  }

  return contentMoreRow(moreBinding.nodeId, moreBinding.binding)
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

function loadableStateProjection(options: {
  readonly kind: ContentProjectionKind
  readonly ownerId: string
  readonly title: string
  readonly state: Exclude<ContentRow['state'], undefined>
  readonly label: string
  readonly detail: string
  readonly action: ContentRowAction
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
        action: options.action
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

function findMoreBinding(options: {
  readonly ownerNodeId: BrowserTreeNodeId
  readonly children: LoadedChildren
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
}):
  | {
      readonly nodeId: BrowserTreeNodeId
      readonly binding: Extract<RowBinding, { readonly kind: 'more' }>
    }
  | undefined {
  const nextOffset = options.children.nextOffset

  if (nextOffset === undefined || options.bindingsById === undefined) {
    return undefined
  }

  for (const [nodeId, binding] of options.bindingsById) {
    if (binding.kind !== 'more') {
      continue
    }

    if (binding.ownerId !== options.ownerNodeId) {
      continue
    }

    if (binding.target.offset !== nextOffset) {
      continue
    }

    if (binding.target.limit !== options.children.limit) {
      continue
    }

    if ((binding.target.parentDirectoryId ?? undefined) !== options.children.parentDirectoryId) {
      continue
    }

    if (!sameEntryPoint(binding.target.entryPoint, options.children.entryPoint)) {
      continue
    }

    return { nodeId, binding }
  }

  return undefined
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

function formatLoadedDetail(children: LoadedChildren): string {
  if (children.nextOffset === undefined) {
    return `${children.rows.length} ${children.rows.length === 1 ? 'item' : 'items'} loaded.`
  }

  return `${children.rows.length} of ${children.totalRows} items loaded.`
}

function formatPresenceDetail(row: ChildRow): string {
  const subject = row.kind === 'directory' ? 'Folder' : 'File'

  switch (row.presence) {
    case 'present':
      return `${subject} available.`
    case 'missing':
      return `${subject} missing.`
    case 'removed':
      return `${subject} removed.`
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
