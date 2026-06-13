import type { ChildRow, Presence } from '../../../shared/library/hierarchy/read'
import type {
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'
import type {
  ContentsReadResult,
  ContentsResult,
  ContentsFileRow,
  PlayableMedia
} from '../../../shared/library/contents/read'
import type { LibraryPanelSurface } from '../../../shared/library/viewState/persistence'
import type { ContentsBoundaryState } from '../boundary/contentsRead'
import type { LocalBrowseOperation } from '../../../shared/library/localBrowse/entryPoints'
import {
  projectAddSourceProjection,
  type AddSourceAction,
  type AddSourceProjection,
  type AddSourceRow
} from '../addSource/projection'
import {
  libraryBrowseEmptyStateLabel,
  projectLibraryHome,
  type LibraryHomeAction,
  type LibraryHomeProjection,
  type LibraryHomeRow
} from '../libraryHome/projection'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserState, RowBinding } from '../state'
import type { BrowserTreeNodeId } from '../tree/types'
import { formatSourceDisplayName } from '../tree/sourcePresentation'

export type ContentProjectionKind =
  | 'libraryHome'
  | 'libraryStart'
  | 'emptySelection'
  | 'unsupported'
  | 'notLoaded'
  | 'loading'
  | 'failed'
  | 'ready'

export type ContentSurfaceKind =
  | 'indexedContents'
  | 'libraryHome'
  | 'libraryStart'
  | 'addSource'
  | 'sourcePreview'
  | 'sourceInventory'
  | 'sourceStatus'
  | 'readState'

export type ContentRowKind = 'directory' | 'file' | 'state' | 'more'

export type ContentRowIcon =
  | 'folder'
  | 'music'
  | 'video'
  | 'image'
  | 'cueSheet'
  | 'metadata'
  | 'more'
  | 'loading'
  | 'warning'
  | 'state'

export type ContentRowAction =
  | {
      readonly kind: 'loadChildren'
      readonly nodeId: BrowserTreeNodeId
      readonly label: string
    }
  | {
      readonly kind: 'loadContentsPage'
      readonly nodeId: BrowserTreeNodeId
      readonly label: string
      readonly cursor: string
    }
  | {
      readonly kind: 'loadLocalBrowseChildren'
      readonly nodeId: BrowserTreeNodeId
      readonly label: string
    }
  | {
      readonly kind: 'loadLocalBrowseMore'
      readonly nodeId: BrowserTreeNodeId
      readonly label: string
    }
  | {
      readonly kind: 'loadSearchPage'
      readonly label: string
      readonly cursor: string
    }
  | {
      readonly kind: 'requestLocalBrowseAdmission'
      readonly resolvedPath: string
      readonly requestKind: Extract<
        LocalBrowseOperation,
        { readonly kind: 'requestSourceAdmission' }
      >['requestKind']
      readonly label: string
    }
  | {
      readonly kind: 'chooseMusicFolder'
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
  readonly fileClass?: 'audio' | 'video' | 'image' | 'unsupported'
  readonly action?: ContentRowAction
}

export type ContentProjection = {
  readonly surfaceKind: ContentSurfaceKind
  readonly surfaceLabel: string
  readonly kind: ContentProjectionKind
  readonly title: string
  readonly detail?: string
  readonly rows: readonly ContentRow[]
}

export type ProjectContentsOptions = {
  readonly state: BrowserState
  readonly surface?: LibraryPanelSurface
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly bindingsById?: BrowserProjection['bindingsById']
  readonly contentsState?: ContentsBoundaryState
  readonly sourceIntegrityBySourceId?: ReadonlyMap<string, ReadSourceIntegrityReply>
  readonly sourceMaintenanceBySourceId?: ReadonlyMap<string, ReadSourceMaintenanceReply>
}

type ContentSurface = {
  readonly surfaceKind: ContentSurfaceKind
  readonly surfaceLabel: string
}

const indexedContentsSurface = {
  surfaceKind: 'indexedContents',
  surfaceLabel: 'Contents'
} satisfies ContentSurface

const librarySurface = {
  surfaceKind: 'libraryStart',
  surfaceLabel: 'Library'
} satisfies ContentSurface

const libraryHomeSurface = {
  surfaceKind: 'libraryHome',
  surfaceLabel: 'Library'
} satisfies ContentSurface

const sourceStatusSurface = {
  surfaceKind: 'sourceStatus',
  surfaceLabel: 'Source Status'
} satisfies ContentSurface

function contentProjection(
  surface: ContentSurface,
  projection: Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel'>
): ContentProjection {
  return {
    surfaceKind: surface.surfaceKind,
    surfaceLabel: surface.surfaceLabel,
    ...projection
  }
}

export function projectContents(options: ProjectContentsOptions): ContentProjection {
  const hostProjection = projectHostContents(options.state.hostStatus)

  if (hostProjection !== undefined) {
    return hostProjection
  }

  const selectedNodeId = options.selectedNodeId

  if (selectedNodeId === undefined) {
    if (options.surface === 'addSource') {
      return contentProjectionFromAddSource(
        projectAddSourceProjection({
          ...(options.state.addSourceView === undefined
            ? {}
            : { addSourceView: options.state.addSourceView }),
          ...(options.state.localBrowseEntryPointsState === undefined
            ? {}
            : { entryPointsState: options.state.localBrowseEntryPointsState })
        })
      )
    }

    if (options.surface === 'libraryBrowse') {
      return contentProjectionFromLibraryHome(
        projectLibraryHome({
          state: options.state,
          ...(options.bindingsById === undefined ? {} : { bindingsById: options.bindingsById }),
          ...(options.sourceIntegrityBySourceId === undefined
            ? {}
            : { sourceIntegrityBySourceId: options.sourceIntegrityBySourceId }),
          ...(options.sourceMaintenanceBySourceId === undefined
            ? {}
            : { sourceMaintenanceBySourceId: options.sourceMaintenanceBySourceId })
        })
      )
    }

    return stateProjection({
      surface: librarySurface,
      kind: 'libraryStart',
      ownerId: 'selection',
      title: 'Start your library',
      state: 'empty',
      label: 'Add a music folder',
      detail: 'Add a music folder to begin, or select an existing source.',
      action: {
        kind: 'chooseMusicFolder',
        label: 'Add music folder'
      }
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
        bindingsById: options.bindingsById,
        contentsState: options.contentsState
      })
    case 'directory':
      return projectDirectoryContents({
        state: options.state,
        selectedNodeId,
        binding,
        bindingsById: options.bindingsById,
        contentsState: options.contentsState
      })
    case 'file':
      return projectFileContents({
        state: options.state,
        selectedNodeId
      })
    case 'navigation':
      return stateProjection({
        surface: librarySurface,
        kind: 'unsupported',
        ownerId: selectedNodeId,
        title: binding.navigationRow.displayName,
        state: 'unsupported',
        label: 'Contents unavailable',
        detail: 'Contents for this item are not available yet.'
      })
    case 'readState':
      return stateProjection({
        surface: sourceStatusSurface,
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
      return contentProjection(indexedContentsSurface, {
        kind: 'ready',
        title: 'More items',
        detail: binding.detail,
        rows: [contentMoreRow(selectedNodeId, binding)]
      })
    case 'addSourceSection':
    case 'localBrowseEntryPoint':
    case 'localBrowseItem':
    case 'localBrowseMore':
      return contentProjectionFromAddSource(
        projectAddSourceProjection({
          selectedNodeId,
          ...(options.state.addSourceView === undefined
            ? {}
            : { addSourceView: options.state.addSourceView }),
          ...(options.bindingsById === undefined
            ? {}
            : {
                projection: {
                  kind: 'tree',
                  nodes: [],
                  bindingsById: options.bindingsById
                } satisfies BrowserProjection
              }),
          ...(options.state.localBrowseEntryPointsState === undefined
            ? {}
            : { entryPointsState: options.state.localBrowseEntryPointsState }),
          ...(options.state.localBrowseItemStates === undefined
            ? {}
            : { itemStates: options.state.localBrowseItemStates })
        })
      )
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
      surface: librarySurface,
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
      surface: librarySurface,
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

function contentProjectionFromAddSource(projection: AddSourceProjection): ContentProjection {
  return contentProjection(addSourceContentSurface(projection), {
    kind: contentKindFromAddSource(projection),
    title: projection.title,
    ...(projection.detail === undefined ? {} : { detail: projection.detail }),
    rows: projection.rows.map(contentRowFromAddSource)
  })
}

function contentProjectionFromLibraryHome(projection: LibraryHomeProjection): ContentProjection {
  return contentProjection(libraryHomeSurface, {
    kind: 'libraryHome',
    title: projection.title,
    detail: projection.detail,
    rows: projection.rows.map(contentRowFromLibraryHome)
  })
}

function contentRowFromLibraryHome(row: LibraryHomeRow): ContentRow {
  return {
    id: row.id,
    kind: 'state',
    label: row.label,
    detail: row.detail,
    state: row.state,
    icon:
      row.state === 'loading'
        ? 'loading'
        : row.state === 'failed' || row.state === 'unsupported'
          ? 'warning'
          : 'state',
    ...(row.action === undefined ? {} : { action: contentActionFromLibraryHome(row.action) })
  }
}

function contentActionFromLibraryHome(action: LibraryHomeAction): ContentRowAction {
  switch (action.kind) {
    case 'chooseMusicFolder':
      return action
  }
}

function addSourceContentSurface(projection: AddSourceProjection): ContentSurface {
  switch (projection.surfaceKind) {
    case 'inventory':
      return {
        surfaceKind: 'sourceInventory',
        surfaceLabel: projection.surfaceLabel
      }
    case 'preview':
      return {
        surfaceKind: 'sourcePreview',
        surfaceLabel: projection.surfaceLabel
      }
    case 'addSource':
      return {
        surfaceKind: 'addSource',
        surfaceLabel: projection.surfaceLabel
      }
  }
}

function contentKindFromAddSource(projection: AddSourceProjection): ContentProjectionKind {
  if (projection.rows.some((row) => row.state === 'loading')) {
    return 'loading'
  }

  if (projection.rows.some((row) => row.state === 'failed')) {
    return 'failed'
  }

  if (projection.rows.some((row) => row.state === 'unsupported')) {
    return 'unsupported'
  }

  if (projection.rows.some((row) => row.state === 'notLoaded')) {
    return 'notLoaded'
  }

  return 'ready'
}

function contentRowFromAddSource(row: AddSourceRow): ContentRow {
  return {
    id: row.id,
    kind: row.kind,
    label: row.label,
    ...(row.detail === undefined ? {} : { detail: row.detail }),
    ...(row.icon === undefined ? {} : { icon: row.icon }),
    ...(row.state === undefined ? {} : { state: row.state }),
    ...(row.fileClass === undefined ? {} : { fileClass: row.fileClass }),
    ...(row.action === undefined ? {} : { action: contentActionFromAddSource(row.action) })
  }
}

function contentActionFromAddSource(action: AddSourceAction): ContentRowAction {
  switch (action.kind) {
    case 'chooseMusicFolder':
      return action
    case 'loadLocalBrowseChildren':
      return action
    case 'loadLocalBrowseMore':
      return action
    case 'requestLocalBrowseAdmission':
      return action
  }
}

function projectSourceContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'source' }>
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const title = formatSourceDisplayName(options.binding.target.label)
  const acceptedSnapshot = retainedAcceptedSnapshotProjection({
    state: options.state,
    bindingsById: options.bindingsById,
    contentsState: options.contentsState
  })
  const useAcceptedFallback =
    acceptedSnapshot === undefined && hasCrossScopePending(options.contentsState)
  return projectContentsState({
    ownerId:
      acceptedSnapshot?.ownerId ??
      (useAcceptedFallback ? 'accepted-contents' : options.selectedNodeId),
    title: acceptedSnapshot?.title ?? (useAcceptedFallback ? 'Library contents' : title),
    contentsState: options.contentsState
  })
}

function projectDirectoryContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'directory' }>
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const directoryRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const title = directoryRow?.label ?? 'Selected folder'
  const acceptedSnapshot = retainedAcceptedSnapshotProjection({
    state: options.state,
    bindingsById: options.bindingsById,
    contentsState: options.contentsState
  })
  const useAcceptedFallback =
    acceptedSnapshot === undefined && hasCrossScopePending(options.contentsState)
  return projectContentsState({
    ownerId:
      acceptedSnapshot?.ownerId ??
      (useAcceptedFallback ? 'accepted-contents' : options.selectedNodeId),
    title: acceptedSnapshot?.title ?? (useAcceptedFallback ? 'Library contents' : title),
    contentsState: options.contentsState
  })
}

type AcceptedSnapshotProjection = {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
}

type AcceptedContentsScope =
  | {
      readonly kind: 'source'
      readonly sourceId: string
    }
  | {
      readonly kind: 'sourceLocation'
      readonly sourceLocationId: string
    }
  | {
      readonly kind: 'directory'
      readonly sourceId: string
      readonly directoryId: string
    }

function retainedAcceptedSnapshotProjection(options: {
  readonly state: BrowserState
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
  readonly contentsState: ContentsBoundaryState | undefined
}): AcceptedSnapshotProjection | undefined {
  const contentsState = options.contentsState

  if (
    contentsState?.kind !== 'ready' ||
    contentsState.pending === undefined ||
    contentsState.pending.requestKey === contentsState.requestKey
  ) {
    return undefined
  }

  return acceptedSnapshotProjectionForRequestKey({
    state: options.state,
    bindingsById: options.bindingsById,
    requestKey: contentsState.requestKey
  })
}

function hasCrossScopePending(contentsState: ContentsBoundaryState | undefined): boolean {
  return (
    contentsState?.kind === 'ready' &&
    contentsState.pending !== undefined &&
    contentsState.pending.requestKey !== contentsState.requestKey
  )
}

function acceptedSnapshotProjectionForRequestKey(options: {
  readonly state: BrowserState
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
  readonly requestKey: string
}): AcceptedSnapshotProjection | undefined {
  const scope = parseAcceptedContentsScope(options.requestKey)

  if (scope === undefined) {
    return undefined
  }

  if (scope.kind === 'directory') {
    const stateProjection = acceptedSnapshotProjectionFromState(scope, options.state)

    if (stateProjection !== undefined) {
      return stateProjection
    }
  }

  const bindingProjection = acceptedSnapshotProjectionFromBindings(scope, options.bindingsById)

  if (bindingProjection !== undefined) {
    return bindingProjection
  }

  return acceptedSnapshotProjectionFromState(scope, options.state)
}

function acceptedSnapshotProjectionFromBindings(
  scope: AcceptedContentsScope,
  bindingsById: BrowserProjection['bindingsById'] | undefined
): AcceptedSnapshotProjection | undefined {
  if (bindingsById === undefined) {
    return undefined
  }

  for (const [nodeId, binding] of bindingsById) {
    if (scope.kind === 'source' && binding.kind === 'source') {
      if (
        binding.target.entryPoint.kind === 'source' &&
        binding.target.entryPoint.sourceId === scope.sourceId
      ) {
        return {
          ownerId: nodeId,
          title: formatSourceDisplayName(binding.target.label)
        }
      }
    }

    if (scope.kind === 'sourceLocation' && binding.kind === 'source') {
      if (
        binding.target.entryPoint.kind === 'sourceLocation' &&
        binding.target.entryPoint.sourceLocationId === scope.sourceLocationId
      ) {
        return {
          ownerId: nodeId,
          title: formatSourceDisplayName(binding.target.label)
        }
      }
    }

    if (
      scope.kind === 'directory' &&
      binding.kind === 'directory' &&
      binding.sourceId === scope.sourceId &&
      binding.directoryId === scope.directoryId
    ) {
      return {
        ownerId: nodeId,
        title: binding.label ?? 'Selected folder'
      }
    }
  }

  return undefined
}

function acceptedSnapshotProjectionFromState(
  scope: AcceptedContentsScope,
  state: BrowserState
): AcceptedSnapshotProjection | undefined {
  if (scope.kind === 'source' || scope.kind === 'sourceLocation') {
    const row =
      state.navigationReadResult?.state === 'ready'
        ? state.navigationReadResult.rows.find((candidate) =>
            scope.kind === 'source'
              ? candidate.selectorKind === 'source' && candidate.selectorPayload === scope.sourceId
              : candidate.selectorKind === 'sourceLocation' &&
                candidate.selectorPayload === scope.sourceLocationId
          )
        : undefined

    if (row !== undefined) {
      return {
        ownerId: `navigation-row:${row.navigationRowId}`,
        title: formatSourceDisplayName(row.displayName)
      }
    }

    return undefined
  }

  const row = findLoadedDirectoryRow(state, scope.sourceId, scope.directoryId)

  if (row === undefined) {
    return undefined
  }

  return {
    ownerId: row.id,
    title: row.label
  }
}

function parseAcceptedContentsScope(requestKey: string): AcceptedContentsScope | undefined {
  const parts = requestKey.split(':')

  if (parts[0] === 'source' && parts[1] !== undefined) {
    return {
      kind: 'source',
      sourceId: parts[1]
    }
  }

  if (parts[0] === 'source-location' && parts[1] !== undefined) {
    return {
      kind: 'sourceLocation',
      sourceLocationId: parts[1]
    }
  }

  if (parts[0] === 'directory' && parts[1] !== undefined && parts[2] !== undefined) {
    return {
      kind: 'directory',
      sourceId: parts[1],
      directoryId: parts[2]
    }
  }

  return undefined
}

function projectContentsState(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly contentsState: ContentsBoundaryState | undefined
}): ContentProjection {
  const state = options.contentsState

  if (state === undefined) {
    return stateProjection({
      kind: 'loading',
      ownerId: options.ownerId,
      title: options.title,
      state: 'loading',
      label: 'Loading contents',
      detail: 'Loading contents.'
    })
  }

  if (state.kind === 'idle') {
    if (state.pending !== undefined) {
      return stateProjection({
        kind: 'notLoaded',
        ownerId: options.ownerId,
        title: options.title,
        state: 'notLoaded',
        label: 'Contents pending',
        detail: state.detail ?? 'Contents request is pending.'
      })
    }

    return stateProjection({
      kind: 'notLoaded',
      ownerId: options.ownerId,
      title: options.title,
      state: 'notLoaded',
      label: 'Contents not loaded',
      detail: state.detail ?? 'Contents have not been loaded.'
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

  const projection = projectContentsReadResult({
    ownerId: options.ownerId,
    title: options.title,
    result: state.result,
    ...(state.nextCursor !== undefined ? { nextCursor: state.nextCursor } : {}),
    ...(state.accumulatedRows !== undefined ? { accumulatedRows: state.accumulatedRows } : {})
  })

  if (state.pending?.presentation === 'visible') {
    return {
      ...projection,
      detail:
        state.pending.requestKey === state.requestKey
          ? refreshingDetail('Refreshing contents.', projection.detail)
          : refreshingDetail('Updating selected contents.', projection.detail)
    }
  }

  if (state.refreshError !== undefined) {
    return {
      ...projection,
      detail: refreshingDetail('Showing previous contents.', state.refreshError)
    }
  }

  return projection
}

function refreshingDetail(prefix: string, detail: string | undefined): string {
  return detail === undefined ? prefix : `${prefix} ${detail}`
}

function projectContentsReadResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsReadResult
  readonly nextCursor?: string
  readonly accumulatedRows?: readonly ContentsFileRow[]
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
    result: options.result.result,
    ...(options.nextCursor !== undefined ? { nextCursor: options.nextCursor } : {}),
    ...(options.accumulatedRows !== undefined ? { accumulatedRows: options.accumulatedRows } : {})
  })
}

function projectContentsResult(options: {
  readonly ownerId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsResult
  readonly nextCursor?: string
  readonly accumulatedRows?: readonly ContentsFileRow[]
}): ContentProjection {
  const result = options.result
  const rows = (options.accumulatedRows ?? result.rows).map(contentsRow)
  const nextCursor = options.nextCursor ?? result.nextCursor
  const hasMore = nextCursor !== undefined

  if (rows.length === 0 && nextCursor !== undefined) {
    return contentProjection(indexedContentsSurface, {
      kind: contentsProjectionKind(result),
      title: options.title,
      detail: contentsDetailWithContinuation(result, hasMore, options.accumulatedRows),
      rows: [loadMoreRow(options.ownerId, result, nextCursor)]
    })
  }

  if (rows.length === 0) {
    return contentProjection(indexedContentsSurface, {
      kind: contentsProjectionKind(result),
      title: options.title,
      detail: contentsDetail(result, options.accumulatedRows),
      rows: [
        stateRow({
          ownerId: options.ownerId,
          state: contentsStateRowState(result, rows.length),
          label: contentsStateLabel(result, rows.length),
          detail: contentsDetail(result, options.accumulatedRows)
        })
      ]
    })
  }

  const contentRows =
    nextCursor !== undefined ? [...rows, loadMoreRow(options.ownerId, result, nextCursor)] : rows

  return contentProjection(indexedContentsSurface, {
    kind: contentsProjectionKind(result),
    title: options.title,
    detail: contentsDetailWithContinuation(result, hasMore, options.accumulatedRows),
    rows: contentRows
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

function contentsRow(row: ContentsFileRow): ContentRow {
  const icon = contentsRowIcon(row)
  const detail =
    row.playableMedia === undefined
      ? sourceFileRowDetail(row)
      : playableMediaRowDetail(row.playableMedia, row)
  return {
    id: row.id,
    kind: 'file',
    label: row.label,
    presence: row.presence,
    detail,
    icon,
    fileClass: row.fileClass
  }
}

function contentsRowIcon(row: ContentsFileRow): ContentRowIcon {
  switch (row.fileClass) {
    case 'audio':
      return 'music'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'unsupported':
      return row.fileKind === 'cueSheet' ? 'cueSheet' : 'metadata'
  }
}

function sourceFileRowDetail(row: ContentsFileRow): string {
  if (row.presence === 'missing') {
    return 'File missing'
  }

  if (row.presence === 'removed') {
    return 'File removed'
  }

  return row.relativePath ?? sourceFileClassLabel(row.fileClass, row.fileKind)
}

function playableMediaRowDetail(playableMedia: PlayableMedia, row: ContentsFileRow): string {
  const observations = [playableMedia.mimeType, playableMedia.codec]
    .filter((value): value is string => value !== undefined && value.trim().length > 0)
    .join(' - ')
  return observations.length > 0
    ? observations
    : (row.relativePath ?? sourceFileClassLabel(row.fileClass, row.fileKind))
}

function sourceFileClassLabel(
  fileClass: ContentsFileRow['fileClass'],
  fileKind?: ContentsFileRow['fileKind']
): string {
  switch (fileClass) {
    case 'audio':
      return 'Audio file'
    case 'video':
      return 'Video file'
    case 'image':
      return 'Image file'
    case 'unsupported':
      return fileKind === 'cueSheet' ? 'Cue sheet' : 'Unsupported file'
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

function contentsStateRowState(
  result: ContentsResult,
  rowCount: number
): Exclude<ContentRow['state'], undefined> {
  switch (result.state) {
    case 'ready':
    case 'empty':
      return isVerifiedEmptyResult(result, rowCount) ? 'empty' : 'loading'
    case 'partial':
      // Partial coverage can legitimately contain zero known rows while scanning is incomplete.
      return 'loading'
    case 'blocked':
    case 'failed':
      return 'failed'
    case 'sourceUnavailable':
    case 'locationMissing':
      return 'unsupported'
  }
}

function contentsStateLabel(result: ContentsResult, rowCount: number): string {
  switch (result.state) {
    case 'ready':
    case 'empty':
      if (!isVerifiedEmptyResult(result, rowCount)) {
        return unverifiedEmptyLabel(result)
      }
      return result.hasPolicyOmittedRows
        ? policyEmptyLabel(result.policy)
        : trueEmptyLabel(result.policy)
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

function isVerifiedEmptyResult(result: ContentsResult, rowCount: number): boolean {
  if (rowCount !== 0) {
    return false
  }

  if (result.nextCursor !== undefined) {
    return false
  }

  if (result.state !== 'ready' && result.state !== 'empty') {
    return false
  }

  if (result.scopeCoverage.state !== 'complete' || !result.scopeCoverage.subtreeCoverageComplete) {
    return false
  }

  return result.scopeCoverage.emptyResultAuthoritative || result.hasPolicyOmittedRows
}

function unverifiedEmptyLabel(result: ContentsResult): string {
  switch (result.scopeCoverage.state) {
    case 'complete':
      return 'Contents coverage unverified'
    case 'pending':
    case 'scanning':
    case 'incomplete':
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

function contentsDetail(
  result: ContentsResult,
  accumulatedRows?: readonly ContentsFileRow[]
): string {
  const rowCount = accumulatedRows?.length ?? result.rows.length

  if (result.detail !== undefined) {
    return result.detail
  }

  if (rowCount === 0) {
    return contentsCoveragePrefix(result) ?? contentsStateLabel(result, rowCount)
  }

  if (rowCount === 1) {
    return contentsCoveragePrefix(result) ?? `1 ${contentsCountSubject(result, 1)} loaded.`
  }

  const prefix = contentsCoveragePrefix(result)
  const count = `${rowCount} ${contentsCountSubject(result, rowCount)} loaded.`
  return prefix === undefined ? count : `${prefix} ${count}`
}

function contentsDetailWithContinuation(
  result: ContentsResult,
  hasMore: boolean,
  accumulatedRows?: readonly ContentsFileRow[]
): string {
  const rowCount = accumulatedRows?.length ?? result.rows.length

  if (result.detail !== undefined && !hasMore) {
    return result.detail
  }

  if (hasMore) {
    const subject = contentsCountSubject(result, rowCount)
    const prefix = contentsCoveragePrefix(result)
    const count = `${rowCount} ${subject} loaded. More available.`
    return prefix === undefined ? count : `${prefix} ${count}`
  }

  return contentsDetail(result, accumulatedRows)
}

function contentsCountSubject(result: ContentsResult, count: number): string {
  switch (result.policy.kind) {
    case 'playableMediaBrowse':
      return count === 1 ? 'playable media item' : 'playable media items'
    case 'audioBrowse':
      return count === 1 ? 'audio track' : 'audio tracks'
    case 'playableMedia':
      return count === 1 ? 'playable media item' : 'playable media items'
    case 'sourceFileInventory':
      return count === 1 ? 'requested file' : 'requested files'
  }
}

function policyEmptyLabel(policy: ContentsResult['policy']): string {
  switch (policy.kind) {
    case 'playableMediaBrowse':
      return libraryBrowseEmptyStateLabel('playable')
    case 'audioBrowse':
      return libraryBrowseEmptyStateLabel('audio')
    case 'sourceFileInventory':
      return sourceFileInventoryEmptyLabel(policy)
    case 'playableMedia':
      return playableMediaEmptyLabel(policy)
  }
}

function trueEmptyLabel(policy: ContentsResult['policy']): string {
  switch (policy.kind) {
    case 'playableMediaBrowse':
      return libraryBrowseEmptyStateLabel('playable')
    case 'audioBrowse':
      return libraryBrowseEmptyStateLabel('audio')
    case 'sourceFileInventory':
      return sourceFileInventoryEmptyLabel(policy)
    case 'playableMedia':
      return playableMediaEmptyLabel(policy)
  }
}

function playableMediaEmptyLabel(
  policy: Extract<ContentsResult['policy'], { kind: 'playableMedia' }>
): string {
  return policy.mediaKinds.length === 1 && policy.mediaKinds[0] === 'video'
    ? 'No video items in this scope.'
    : 'No playable media in this scope.'
}

function sourceFileInventoryEmptyLabel(
  policy: Extract<ContentsResult['policy'], { kind: 'sourceFileInventory' }>
): string {
  const fileClasses = policy.fileClasses.join(',')

  if (fileClasses === 'unsupported') {
    return 'No companion files in this scope.'
  }

  if (fileClasses === 'audio,video,image,unsupported') {
    return libraryBrowseEmptyStateLabel('allFiles')
  }

  return 'No requested files in this scope.'
}

function contentsCoveragePrefix(result: ContentsResult): string | undefined {
  if (result.state === 'partial') {
    return indexingDetail(result)
  }

  if (result.scopeCoverage.state === 'pending' || result.scopeCoverage.state === 'scanning') {
    return indexingDetail(result)
  }

  if (result.scopeCoverage.state === 'incomplete') {
    return 'One or more source locations are missing. Results may be incomplete.'
  }

  return undefined
}

function indexingDetail(result: ContentsResult): string {
  switch (result.policy.kind) {
    case 'audioBrowse':
      return 'Still indexing. More tracks may appear as scanning finishes.'
    case 'playableMediaBrowse':
    case 'playableMedia':
      return 'Still indexing. More playable media may appear as scanning finishes.'
    case 'sourceFileInventory':
      return 'Still indexing. More files may appear as scanning finishes.'
  }
}

function loadMoreRow(
  ownerId: BrowserTreeNodeId,
  result: ContentsResult,
  nextCursor: string
): ContentRow {
  const subject = contentsCountSubject(result, result.rows.length)
  return {
    id: `contents-load-more:${ownerId}`,
    kind: 'more',
    label: `More ${subject} available`,
    detail: `Load more`,
    icon: 'more',
    action: {
      kind: 'loadContentsPage',
      nodeId: ownerId,
      label: `Load more ${subject}`,
      cursor: nextCursor
    }
  }
}

function contentMoreRow(
  nodeId: BrowserTreeNodeId,
  binding: Extract<RowBinding, { readonly kind: 'more' }>
): ContentRow {
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
    ...(binding.state === 'loading'
      ? {}
      : {
          action: {
            kind: 'loadLocalBrowseMore',
            nodeId,
            label: binding.state === 'error' ? 'Retry' : 'Load more'
          }
        })
  }
}

function stateProjection(options: {
  readonly surface?: ContentSurface
  readonly kind: ContentProjectionKind
  readonly ownerId: string
  readonly title: string
  readonly state: Exclude<ContentRow['state'], undefined>
  readonly label: string
  readonly detail: string
  readonly action?: ContentRowAction
}): ContentProjection {
  return contentProjection(options.surface ?? indexedContentsSurface, {
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
  })
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

function findLoadedDirectoryRow(
  state: BrowserState,
  sourceId: string,
  directoryId: string
): Extract<ChildRow, { readonly kind: 'directory' }> | undefined {
  for (const sourceState of state.sourceReadStates.values()) {
    if (sourceState.kind !== 'loaded' && sourceState.kind !== 'refreshing') {
      continue
    }

    const row = sourceState.children.rows.find(
      (candidate): candidate is Extract<ChildRow, { readonly kind: 'directory' }> =>
        candidate.kind === 'directory' &&
        candidate.sourceId === sourceId &&
        candidate.directoryId === directoryId
    )

    if (row !== undefined) {
      return row
    }
  }

  for (const directoryState of state.directoryReadStates.values()) {
    if (directoryState.kind !== 'loaded' && directoryState.kind !== 'refreshing') {
      continue
    }

    const row = directoryState.children.rows.find(
      (candidate): candidate is Extract<ChildRow, { readonly kind: 'directory' }> =>
        candidate.kind === 'directory' &&
        candidate.sourceId === sourceId &&
        candidate.directoryId === directoryId
    )

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
