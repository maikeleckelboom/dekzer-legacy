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
import { libraryBrowseProfileLabel, type LibraryBrowseProfile } from '../libraryBrowseProfile/types'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserState, RowBinding } from '../state'
import type { BrowserTreeNodeId } from '../tree/types'
import type { ProjectedSourceActivity } from '../runtime/sourceActivity'
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

export type ContentRowSubject =
  | {
      readonly kind: 'sourceFile'
      readonly sourceId: string
      readonly sourceFileId: string
      readonly label: string
      readonly detail?: string
      readonly relativePath?: string
      readonly presence: Presence
    }
  | {
      readonly kind: 'playableMedia'
      readonly sourceId: string
      readonly sourceFileId: string
      readonly playableMediaId: string
      readonly attachmentId: string
      readonly label: string
      readonly detail?: string
      readonly relativePath?: string
      readonly mediaKind: PlayableMedia['mediaKind']
      readonly mimeType?: string
      readonly codec?: string
      readonly presence: Presence
    }

export type ContentRow = {
  readonly id: string
  readonly kind: ContentRowKind
  readonly label: string
  readonly presence?: Presence
  readonly detail?: string
  readonly icon?: ContentRowIcon
  readonly state?:
    | 'empty'
    | 'notLoaded'
    | 'loading'
    | 'attention'
    | 'failed'
    | 'unsupported'
    | 'file'
  readonly fileClass?: 'audio' | 'video' | 'image' | 'unsupported'
  readonly action?: ContentRowAction
  readonly subject?: ContentRowSubject
}

export type ContentScopeHealth = {
  readonly label:
    | 'Ready'
    | 'Loading'
    | 'Still indexing'
    | 'Unavailable'
    | 'Missing'
    | 'Blocked'
    | 'Partial'
    | 'Empty'
    | 'Needs scan'
    | 'Maintenance needed'
  readonly tone: 'ready' | 'active' | 'warning' | 'danger' | 'muted'
}

export type ContentScopeHeader = {
  readonly surfaceLabel: 'Library Browse' | 'Add Source'
  readonly scopeLabel: string
  readonly profileLabel?: string
  readonly searchLabel?: string
  readonly searchScopeLabel?: string
  readonly health: ContentScopeHealth
}

export type ContentProjection = {
  readonly surfaceKind: ContentSurfaceKind
  readonly surfaceLabel: string
  readonly header: ContentScopeHeader
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
  readonly sourceActivityBySourceId?: ReadonlyMap<string, ProjectedSourceActivity>
}

type ContentSurface = {
  readonly surfaceKind: ContentSurfaceKind
  readonly surfaceLabel: string
  readonly workstationSurfaceLabel: ContentScopeHeader['surfaceLabel']
}

const indexedContentsSurface = {
  surfaceKind: 'indexedContents',
  surfaceLabel: 'Contents',
  workstationSurfaceLabel: 'Library Browse'
} satisfies ContentSurface

const librarySurface = {
  surfaceKind: 'libraryStart',
  surfaceLabel: 'Library',
  workstationSurfaceLabel: 'Library Browse'
} satisfies ContentSurface

const libraryHomeSurface = {
  surfaceKind: 'libraryHome',
  surfaceLabel: 'Library',
  workstationSurfaceLabel: 'Library Browse'
} satisfies ContentSurface

const sourceStatusSurface = {
  surfaceKind: 'sourceStatus',
  surfaceLabel: 'Source Status',
  workstationSurfaceLabel: 'Library Browse'
} satisfies ContentSurface

function contentProjection(
  surface: ContentSurface,
  projection: Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel' | 'header'> & {
    readonly header?: ContentScopeHeader
  }
): ContentProjection {
  return {
    surfaceKind: surface.surfaceKind,
    surfaceLabel: surface.surfaceLabel,
    header: projection.header ?? contentScopeHeader(surface, projection),
    ...projection
  }
}

function contentScopeHeader(
  surface: ContentSurface,
  projection: Pick<ContentProjection, 'kind' | 'title' | 'rows'> & {
    readonly detail?: string
  }
): ContentScopeHeader {
  return {
    surfaceLabel: surface.workstationSurfaceLabel,
    scopeLabel: projection.title,
    health: contentHealthFromProjection(projection)
  }
}

function libraryBrowseHeader(
  projection: Pick<ContentProjection, 'kind' | 'title' | 'rows'> & {
    readonly detail?: string
  },
  profile: LibraryBrowseProfile | undefined
): ContentScopeHeader {
  return {
    surfaceLabel: 'Library Browse',
    scopeLabel: projection.title,
    profileLabel: libraryBrowseProfileLabel(profile ?? 'audio'),
    health: contentHealthFromProjection(projection)
  }
}

function contentHealthFromProjection(
  projection: Pick<ContentProjection, 'kind' | 'rows'> & { readonly detail?: string }
): ContentScopeHealth {
  switch (projection.kind) {
    case 'ready':
    case 'libraryHome':
    case 'libraryStart':
      return rowsIndicateEmpty(projection.rows)
        ? { label: 'Empty', tone: 'muted' }
        : { label: 'Ready', tone: 'ready' }
    case 'loading':
    case 'notLoaded':
      return { label: 'Loading', tone: 'active' }
    case 'failed':
      return { label: 'Unavailable', tone: 'danger' }
    case 'unsupported':
      return unavailableHealth(projection)
    case 'emptySelection':
      return { label: 'Empty', tone: 'muted' }
  }
}

function rowsIndicateEmpty(rows: readonly ContentRow[]): boolean {
  return rows.length === 1 && rows[0]?.kind === 'state' && rows[0].state === 'empty'
}

function unavailableHealth(projection: {
  readonly rows: readonly ContentRow[]
  readonly detail?: string
}): ContentScopeHealth {
  const text = `${projection.detail ?? ''} ${projection.rows.map((row) => row.label).join(' ')}`

  if (/missing/i.test(text)) {
    return { label: 'Missing', tone: 'danger' }
  }

  if (/blocked/i.test(text)) {
    return { label: 'Blocked', tone: 'danger' }
  }

  return { label: 'Unavailable', tone: 'danger' }
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
            : { sourceMaintenanceBySourceId: options.sourceMaintenanceBySourceId }),
          ...(options.sourceActivityBySourceId === undefined
            ? {}
            : { sourceActivityBySourceId: options.sourceActivityBySourceId })
        }),
        options.state.libraryBrowseProfile
      )
    }

    return stateProjection({
      surface: librarySurface,
      kind: 'libraryStart',
      stateRowId: 'selection',
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
      stateRowId: selectedNodeId,
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
        contentsState: options.contentsState,
        profile: options.state.libraryBrowseProfile
      })
    case 'directory':
      return projectDirectoryContents({
        state: options.state,
        selectedNodeId,
        binding,
        bindingsById: options.bindingsById,
        contentsState: options.contentsState,
        profile: options.state.libraryBrowseProfile
      })
    case 'file':
      return projectFileContents({
        state: options.state,
        selectedNodeId,
        profile: options.state.libraryBrowseProfile
      })
    case 'navigation':
      return stateProjection({
        surface: librarySurface,
        kind: 'unsupported',
        stateRowId: selectedNodeId,
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
        stateRowId: selectedNodeId,
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
      stateRowId: 'host',
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
      stateRowId: 'host',
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

function contentProjectionFromLibraryHome(
  projection: LibraryHomeProjection,
  profile: LibraryBrowseProfile | undefined
): ContentProjection {
  const content = {
    kind: 'libraryHome',
    title: projection.title,
    detail: projection.detail,
    rows: projection.rows.map(contentRowFromLibraryHome)
  } satisfies Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel' | 'header'>

  return contentProjection(libraryHomeSurface, {
    ...content,
    header: libraryBrowseHeaderWithHealth(
      content,
      profile,
      libraryHomeHealthFromProductState(projection.productState)
    )
  })
}

function libraryHomeHealthFromProductState(
  productState: LibraryHomeProjection['productState']
): ContentScopeHealth {
  switch (productState) {
    case 'noSources':
    case 'chooseSource':
      return { label: 'Empty', tone: 'muted' }
    case 'indexing':
      return { label: 'Loading', tone: 'active' }
    case 'ready':
      return { label: 'Ready', tone: 'ready' }
    case 'needsScan':
      return { label: 'Needs scan', tone: 'warning' }
    case 'maintenanceNeeded':
      return { label: 'Maintenance needed', tone: 'warning' }
    case 'emptyCurrentView':
      return { label: 'Empty', tone: 'warning' }
    case 'missing':
      return { label: 'Missing', tone: 'danger' }
    case 'blocked':
      return { label: 'Blocked', tone: 'danger' }
    case 'unavailable':
      return { label: 'Unavailable', tone: 'danger' }
  }
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
        : row.state === 'attention' || row.state === 'failed' || row.state === 'unsupported'
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
        surfaceLabel: projection.surfaceLabel,
        workstationSurfaceLabel: 'Add Source'
      }
    case 'preview':
      return {
        surfaceKind: 'sourcePreview',
        surfaceLabel: projection.surfaceLabel,
        workstationSurfaceLabel: 'Add Source'
      }
    case 'addSource':
      return {
        surfaceKind: 'addSource',
        surfaceLabel: projection.surfaceLabel,
        workstationSurfaceLabel: 'Add Source'
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
  readonly profile: LibraryBrowseProfile | undefined
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
    projectionId:
      acceptedSnapshot?.retainedNodeId ??
      (useAcceptedFallback ? 'accepted-contents' : options.selectedNodeId),
    title: acceptedSnapshot?.title ?? (useAcceptedFallback ? 'Library contents' : title),
    contentsState: options.contentsState,
    profile: options.profile
  })
}

function projectDirectoryContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly binding: Extract<RowBinding, { readonly kind: 'directory' }>
  readonly bindingsById: BrowserProjection['bindingsById'] | undefined
  readonly contentsState: ContentsBoundaryState | undefined
  readonly profile: LibraryBrowseProfile | undefined
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
    projectionId:
      acceptedSnapshot?.retainedNodeId ??
      (useAcceptedFallback ? 'accepted-contents' : options.selectedNodeId),
    title: acceptedSnapshot?.title ?? (useAcceptedFallback ? 'Library contents' : title),
    contentsState: options.contentsState,
    profile: options.profile
  })
}

type AcceptedSnapshotProjection = {
  readonly retainedNodeId: BrowserTreeNodeId
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
          retainedNodeId: nodeId,
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
          retainedNodeId: nodeId,
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
        retainedNodeId: nodeId,
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
        retainedNodeId: `navigation-row:${row.navigationRowId}`,
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
    retainedNodeId: row.id,
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
  readonly projectionId: BrowserTreeNodeId
  readonly title: string
  readonly contentsState: ContentsBoundaryState | undefined
  readonly profile: LibraryBrowseProfile | undefined
}): ContentProjection {
  const state = options.contentsState

  if (state === undefined) {
    return withLibraryBrowseHeader(
      stateProjection({
        kind: 'loading',
        stateRowId: options.projectionId,
        title: options.title,
        state: 'loading',
        label: 'Loading contents',
        detail: 'Loading contents.'
      }),
      options.profile
    )
  }

  if (state.kind === 'idle') {
    if (state.pending !== undefined) {
      return withLibraryBrowseHeader(
        stateProjection({
          kind: 'notLoaded',
          stateRowId: options.projectionId,
          title: options.title,
          state: 'notLoaded',
          label: 'Contents pending',
          detail: state.detail ?? 'Contents request is pending.'
        }),
        options.profile
      )
    }

    return withLibraryBrowseHeader(
      stateProjection({
        kind: 'notLoaded',
        stateRowId: options.projectionId,
        title: options.title,
        state: 'notLoaded',
        label: 'Contents not loaded',
        detail: state.detail ?? 'Contents have not been loaded.'
      }),
      options.profile
    )
  }

  if (state.kind === 'loading') {
    return withLibraryBrowseHeader(
      stateProjection({
        kind: 'loading',
        stateRowId: options.projectionId,
        title: options.title,
        state: 'loading',
        label: 'Loading contents',
        detail: state.detail ?? 'Loading contents.'
      }),
      options.profile
    )
  }

  if (state.kind === 'failed') {
    return withLibraryBrowseHeader(
      stateProjection({
        kind: 'failed',
        stateRowId: options.projectionId,
        title: options.title,
        state: 'failed',
        label: 'Contents unavailable',
        detail: state.detail
      }),
      options.profile
    )
  }

  const allowContinuation = retainedSnapshotCanLoadMore(state)
  const projection = projectContentsReadResult({
    projectionId: options.projectionId,
    title: options.title,
    result: state.result,
    profile: options.profile,
    allowContinuation,
    ...(allowContinuation && state.nextCursor !== undefined
      ? { nextCursor: state.nextCursor }
      : {}),
    ...(state.accumulatedRows !== undefined ? { accumulatedRows: state.accumulatedRows } : {}),
    ...(state.pending?.requestKey === state.requestKey && state.pending.cursor !== undefined
      ? { loadingCursor: state.pending.cursor }
      : {})
  })

  if (state.pending?.presentation === 'visible') {
    return withPendingDetail(
      projection,
      state.pending.requestKey === state.requestKey
        ? 'Refreshing contents.'
        : 'Loading selected contents. Showing previous contents until the selected scope is ready.'
    )
  }

  if (state.pending !== undefined && state.pending.requestKey !== state.requestKey) {
    return withPendingDetail(
      projection,
      'Loading selected contents. Showing previous contents until the selected scope is ready.'
    )
  }

  if (state.refreshError !== undefined) {
    return {
      ...projection,
      detail: refreshingDetail('Showing previous contents.', state.refreshError)
    }
  }

  return projection
}

function retainedSnapshotCanLoadMore(state: ContentsBoundaryState): boolean {
  return (
    state.kind === 'ready' &&
    (state.pending === undefined || state.pending.requestKey === state.requestKey)
  )
}

function refreshingDetail(prefix: string, detail: string | undefined): string {
  return detail === undefined ? prefix : `${prefix} ${detail}`
}

function withPendingDetail(projection: ContentProjection, prefix: string): ContentProjection {
  return {
    ...projection,
    detail: refreshingDetail(prefix, projection.detail),
    header: {
      ...projection.header,
      health: { label: 'Loading', tone: 'active' }
    }
  }
}

function withLibraryBrowseHeader(
  projection: ContentProjection,
  profile: LibraryBrowseProfile | undefined
): ContentProjection {
  return {
    ...projection,
    header: libraryBrowseHeader(projection, profile)
  }
}

function libraryBrowseHeaderWithHealth(
  projection: Pick<ContentProjection, 'kind' | 'title' | 'rows'> & {
    readonly detail?: string
  },
  profile: LibraryBrowseProfile | undefined,
  health: ContentScopeHealth
): ContentScopeHeader {
  return {
    ...libraryBrowseHeader(projection, profile),
    health
  }
}

function contentHealthFromContentsResult(
  result: ContentsResult,
  rowCount: number
): ContentScopeHealth {
  switch (result.state) {
    case 'ready':
    case 'empty':
      if (!isVerifiedEmptyResult(result, rowCount)) {
        return contentHealthFromCoverage(result.scopeCoverage.state)
      }

      return rowCount === 0 ? { label: 'Empty', tone: 'muted' } : { label: 'Ready', tone: 'ready' }
    case 'partial':
      return { label: 'Still indexing', tone: 'active' }
    case 'sourceUnavailable':
      return { label: 'Unavailable', tone: 'danger' }
    case 'locationMissing':
      return { label: 'Missing', tone: 'danger' }
    case 'blocked':
      return { label: 'Blocked', tone: 'danger' }
    case 'failed':
      return { label: 'Unavailable', tone: 'danger' }
  }
}

function contentHealthFromCoverage(
  state: ContentsResult['scopeCoverage']['state']
): ContentScopeHealth {
  switch (state) {
    case 'complete':
      return { label: 'Ready', tone: 'ready' }
    case 'pending':
    case 'scanning':
      return { label: 'Still indexing', tone: 'active' }
    case 'incomplete':
      return { label: 'Partial', tone: 'warning' }
    case 'sourceUnavailable':
      return { label: 'Unavailable', tone: 'danger' }
    case 'locationMissing':
      return { label: 'Missing', tone: 'danger' }
    case 'blocked':
      return { label: 'Blocked', tone: 'danger' }
    case 'failed':
      return { label: 'Unavailable', tone: 'danger' }
  }
}

function projectContentsReadResult(options: {
  readonly projectionId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsReadResult
  readonly profile: LibraryBrowseProfile | undefined
  readonly allowContinuation?: boolean
  readonly nextCursor?: string
  readonly accumulatedRows?: readonly ContentsFileRow[]
  readonly loadingCursor?: string
}): ContentProjection {
  if (options.result.state !== 'ready') {
    return withLibraryBrowseHeader(
      stateProjection({
        kind: options.result.state === 'readFailed' ? 'failed' : 'unsupported',
        stateRowId: options.projectionId,
        title: options.title,
        state: options.result.state === 'readFailed' ? 'failed' : 'unsupported',
        label: 'Contents unavailable',
        detail: options.result.error.message
      }),
      options.profile
    )
  }

  return projectContentsResult({
    projectionId: options.projectionId,
    title: options.title,
    result: options.result.result,
    profile: options.profile,
    ...(options.allowContinuation === undefined
      ? {}
      : { allowContinuation: options.allowContinuation }),
    ...(options.nextCursor !== undefined ? { nextCursor: options.nextCursor } : {}),
    ...(options.accumulatedRows !== undefined ? { accumulatedRows: options.accumulatedRows } : {}),
    ...(options.loadingCursor !== undefined ? { loadingCursor: options.loadingCursor } : {})
  })
}

function projectContentsResult(options: {
  readonly projectionId: BrowserTreeNodeId
  readonly title: string
  readonly result: ContentsResult
  readonly profile: LibraryBrowseProfile | undefined
  readonly allowContinuation?: boolean
  readonly nextCursor?: string
  readonly accumulatedRows?: readonly ContentsFileRow[]
  readonly loadingCursor?: string
}): ContentProjection {
  const result = options.result
  const rows = (options.accumulatedRows ?? result.rows).map(contentsRow)
  const nextCursor =
    options.allowContinuation === false ? undefined : (options.nextCursor ?? result.nextCursor)
  const hasMore = nextCursor !== undefined

  if (rows.length === 0 && nextCursor !== undefined) {
    const projection = {
      kind: contentsProjectionKind(result),
      title: options.title,
      detail: contentsDetailWithContinuation(result, hasMore, options.accumulatedRows),
      rows: [loadMoreRow(options.projectionId, result, nextCursor, options.loadingCursor)]
    } satisfies Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel' | 'header'>

    return contentProjection(indexedContentsSurface, {
      ...projection,
      header: libraryBrowseHeaderWithHealth(
        projection,
        options.profile,
        contentHealthFromContentsResult(result, rows.length)
      )
    })
  }

  if (rows.length === 0) {
    const projection = {
      kind: contentsProjectionKind(result),
      title: options.title,
      detail: contentsDetail(result, options.accumulatedRows),
      rows: [
        stateRow({
          rowId: options.projectionId,
          state: contentsStateRowState(result, rows.length),
          label: contentsStateLabel(result, rows.length),
          detail: contentsDetail(result, options.accumulatedRows)
        })
      ]
    } satisfies Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel' | 'header'>

    return contentProjection(indexedContentsSurface, {
      ...projection,
      header: libraryBrowseHeaderWithHealth(
        projection,
        options.profile,
        contentHealthFromContentsResult(result, rows.length)
      )
    })
  }

  const contentRows =
    nextCursor !== undefined
      ? [...rows, loadMoreRow(options.projectionId, result, nextCursor, options.loadingCursor)]
      : rows

  const projection = {
    kind: contentsProjectionKind(result),
    title: options.title,
    detail: contentsDetailWithContinuation(result, hasMore, options.accumulatedRows),
    rows: contentRows
  } satisfies Omit<ContentProjection, 'surfaceKind' | 'surfaceLabel' | 'header'>

  return contentProjection(indexedContentsSurface, {
    ...projection,
    header: libraryBrowseHeaderWithHealth(
      projection,
      options.profile,
      contentHealthFromContentsResult(result, rows.length)
    )
  })
}

function projectFileContents(options: {
  readonly state: BrowserState
  readonly selectedNodeId: BrowserTreeNodeId
  readonly profile: LibraryBrowseProfile | undefined
}): ContentProjection {
  const fileRow = findLoadedChildRow(options.state, options.selectedNodeId)
  const title = fileRow?.label ?? 'Selected file'
  const detail =
    fileRow === undefined ? 'File details are not available.' : formatPresenceDetail(fileRow)

  return withLibraryBrowseHeader(
    stateProjection({
      kind: 'ready',
      stateRowId: options.selectedNodeId,
      title,
      state: 'file',
      label: 'File selected',
      detail
    }),
    options.profile
  )
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
    fileClass: row.fileClass,
    subject: contentRowSubject(row, detail)
  }
}

function contentRowSubject(row: ContentsFileRow, detail: string): ContentRowSubject {
  const playableMedia = row.playableMedia

  if (playableMedia !== undefined) {
    return {
      kind: 'playableMedia',
      sourceId: row.sourceId,
      sourceFileId: row.sourceFileId,
      playableMediaId: playableMedia.playableMediaId,
      attachmentId: playableMedia.attachmentId,
      label: row.label,
      detail,
      ...(row.relativePath === undefined ? {} : { relativePath: row.relativePath }),
      mediaKind: playableMedia.mediaKind,
      ...(playableMedia.mimeType === undefined ? {} : { mimeType: playableMedia.mimeType }),
      ...(playableMedia.codec === undefined ? {} : { codec: playableMedia.codec }),
      presence: row.presence
    }
  }

  return {
    kind: 'sourceFile',
    sourceId: row.sourceId,
    sourceFileId: row.sourceFileId,
    label: row.label,
    detail,
    ...(row.relativePath === undefined ? {} : { relativePath: row.relativePath }),
    presence: row.presence
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
  selectedNodeId: BrowserTreeNodeId,
  result: ContentsResult,
  nextCursor: string,
  loadingCursor?: string
): ContentRow {
  const subject = contentsCountSubject(result, result.rows.length)
  const isLoading = loadingCursor === nextCursor
  return {
    id: `contents-load-more:${selectedNodeId}`,
    kind: 'more',
    label: isLoading ? `Loading more ${subject}` : `More ${subject} available`,
    detail: isLoading ? 'Loading more' : 'Load more',
    icon: isLoading ? 'loading' : 'more',
    ...(isLoading
      ? {}
      : {
          action: {
            kind: 'loadContentsPage' as const,
            nodeId: selectedNodeId,
            label: `Load more ${subject}`,
            cursor: nextCursor
          }
        })
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
  readonly stateRowId: string
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
        rowId: options.stateRowId,
        state: options.state,
        label: options.label,
        detail: options.detail,
        ...(options.action === undefined ? {} : { action: options.action })
      })
    ]
  })
}

function stateRow(options: {
  readonly rowId: string
  readonly state: Exclude<ContentRow['state'], undefined>
  readonly label: string
  readonly detail: string
  readonly action?: ContentRowAction
}): ContentRow {
  return {
    id: `contents-state:${options.rowId}:${options.state}`,
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
    case 'attention':
      return 'warning'
    case 'empty':
    case 'notLoaded':
    case 'file':
      return 'state'
  }
}
