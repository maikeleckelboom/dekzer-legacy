import type {
  LocalBrowseEntryPointKind,
  LocalBrowseEntryPointStatus,
  LocalBrowseOperation
} from '../../../shared/library/localBrowse/entryPoints'
import type { LocalBrowseItem } from '../../../shared/library/localBrowse/items'
import { sourceAdmissionOperation } from '../localBrowse/projection'
import {
  localBrowseRootTarget,
  localBrowseWindowKey,
  type LoadedLocalBrowseItems,
  type LocalBrowseItemState
} from '../localBrowse/types'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import { defaultAddSourceView, type AddSourceView } from './view'

export type AddSourceProductState =
  | 'start'
  | 'loadingSuggestions'
  | 'suggestionsReady'
  | 'selectedFolderPreview'
  | 'alreadyAdded'
  | 'restoreSource'
  | 'protectedOrBlocked'
  | 'tooBroad'
  | 'notUseful'
  | 'inventory'
  | 'unavailable'

export type AddSourceSurfaceKind = 'addSource' | 'preview' | 'inventory'

export type AddSourceRowKind = 'directory' | 'file' | 'state' | 'more'

export type AddSourceRowIcon =
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

export type AddSourceAction =
  | {
      readonly kind: 'chooseMusicFolder'
      readonly label: 'Add music folder'
    }
  | {
      readonly kind: 'loadLocalBrowseChildren'
      readonly nodeId: BrowserTreeNodeId
      readonly label: 'Load preview' | 'Load inventory' | 'Retry'
    }
  | {
      readonly kind: 'loadLocalBrowseMore'
      readonly nodeId: BrowserTreeNodeId
      readonly label: 'Load more' | 'Retry'
    }
  | {
      readonly kind: 'requestLocalBrowseAdmission'
      readonly resolvedPath: string
      readonly requestKind: Extract<
        LocalBrowseOperation,
        { readonly kind: 'requestSourceAdmission' }
      >['requestKind']
      readonly label: 'Add as music source' | 'Add parent as music source' | 'Restore source'
    }

export type AddSourceRow = {
  readonly id: string
  readonly kind: AddSourceRowKind
  readonly label: string
  readonly detail?: string
  readonly icon?: AddSourceRowIcon
  readonly state?: 'empty' | 'notLoaded' | 'loading' | 'failed' | 'unsupported' | 'file'
  readonly fileClass?: 'audio' | 'video' | 'image' | 'unsupported'
  readonly action?: AddSourceAction
}

export type AddSourceProjection = {
  readonly productState: AddSourceProductState
  readonly surfaceKind: AddSourceSurfaceKind
  readonly surfaceLabel: 'Add Source' | 'Preview' | 'Inventory'
  readonly title: string
  readonly detail?: string
  readonly rows: readonly AddSourceRow[]
}

export type AddSourceStatusText = {
  readonly badge?:
    | 'Ready to add'
    | 'Restore source'
    | 'Already added'
    | 'Choose a specific folder'
    | 'Protected location'
    | 'Blocked'
    | 'Choose a music folder'
  readonly detail: string
}

export type ProjectAddSourceProjectionOptions = {
  readonly addSourceView?: AddSourceView
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly projection?: BrowserProjection
  readonly entryPointsState?: {
    readonly kind: 'unread' | 'loading' | 'ready' | 'refreshing' | 'failed'
    readonly detail?: string
    readonly refreshError?: string
  }
  readonly itemStates?: ReadonlyMap<string, LocalBrowseItemState>
}

export function projectAddSourceProjection(
  options: ProjectAddSourceProjectionOptions
): AddSourceProjection {
  const addSourceView = options.addSourceView ?? defaultAddSourceView
  const selectedNodeId = options.selectedNodeId

  if (selectedNodeId === undefined) {
    return startProjection(options.entryPointsState, addSourceView)
  }

  const binding = options.projection?.bindingsById.get(selectedNodeId)

  if (binding === undefined) {
    return startProjection(options.entryPointsState, addSourceView)
  }

  switch (binding.kind) {
    case 'addSourceSection':
      return startProjection(options.entryPointsState, addSourceView)
    case 'localBrowseEntryPoint':
      return folderProjection({
        selectedNodeId,
        title: binding.entry.displayName,
        targetNodeId: selectedNodeId,
        folderDetail: binding.entry.identity.resolvedPath ?? binding.entry.displayName,
        addSourceView,
        entryPointKind: binding.entry.identity.entryPointKind,
        localStatus: binding.entry.status,
        admission: sourceAdmissionOperation(binding.entry.availableOperations),
        windowState: options.itemStates?.get(
          localBrowseWindowKey(localBrowseRootTarget(binding.target, addSourceView))
        )
      })
    case 'localBrowseItem':
      if (binding.target === undefined) {
        return itemProjection(selectedNodeId, binding.item, addSourceView)
      }

      return folderProjection({
        selectedNodeId,
        title: binding.item.displayName,
        targetNodeId: selectedNodeId,
        folderDetail: localBrowseItemDetail(binding.item),
        addSourceView: binding.target.addSourceView,
        entryPointKind: binding.item.identity.entryPointKind,
        localStatus: binding.item.status,
        admission: sourceAdmissionOperation(binding.item.availableOperations),
        windowState: options.itemStates?.get(localBrowseWindowKey(binding.target))
      })
    case 'localBrowseMore':
      return {
        productState: addSourceView === 'inventory' ? 'inventory' : 'selectedFolderPreview',
        surfaceKind: addSourceView === 'inventory' ? 'inventory' : 'preview',
        surfaceLabel: addSourceView === 'inventory' ? 'Inventory' : 'Preview',
        title: 'More items',
        detail: binding.detail,
        rows: [localBrowseMoreRow(selectedNodeId, binding.detail)]
      }
    default:
      return startProjection(options.entryPointsState, addSourceView)
  }
}

export function projectAddSourceStatusText(input: {
  readonly localState:
    | 'notCandidate'
    | 'eligible'
    | 'broadRoot'
    | 'protected'
    | 'resolutionFailed'
    | 'alreadyAdded'
    | 'restorable'
  readonly detail?: string
}): AddSourceStatusText {
  switch (input.localState) {
    case 'eligible':
      return {
        badge: 'Ready to add',
        detail: 'This folder can be added as a managed music source.'
      }
    case 'restorable':
      return {
        badge: 'Restore source',
        detail: 'This source was removed. Restore it to manage this folder again.'
      }
    case 'broadRoot':
      return {
        badge: 'Choose a specific folder',
        detail: 'Choose a specific folder inside this drive.'
      }
    case 'protected':
      return {
        badge: 'Protected location',
        detail: input.detail ?? 'Choose another folder that Dekzer can read.'
      }
    case 'resolutionFailed':
      return {
        badge: 'Blocked',
        detail: input.detail ?? 'This folder cannot be checked right now.'
      }
    case 'alreadyAdded':
      return {
        badge: 'Already added',
        detail: 'Already added as a music source.'
      }
    case 'notCandidate':
      return {
        badge: 'Choose a music folder',
        detail: 'Choose a folder that contains music files.'
      }
  }
}

function startProjection(
  entryPointsState: ProjectAddSourceProjectionOptions['entryPointsState'],
  addSourceView: AddSourceView
): AddSourceProjection {
  if (addSourceView === 'inventory') {
    return {
      productState: 'inventory',
      surfaceKind: 'inventory',
      surfaceLabel: 'Inventory',
      title: 'Inventory',
      detail:
        'Inventory is for inspection. It shows local files without adding or indexing a folder.',
      rows: [
        stateRow({
          rowId: 'add-source-inventory',
          state: 'empty',
          label: 'Select a suggested folder',
          detail: 'Choose a folder to inspect its local files.'
        })
      ]
    }
  }

  if (entryPointsState?.kind === 'loading' || entryPointsState?.kind === 'unread') {
    return {
      productState: 'loadingSuggestions',
      surfaceKind: 'addSource',
      surfaceLabel: 'Add Source',
      title: 'Add Source',
      detail: 'Add a folder to manage it as a music source.',
      rows: [
        stateRow({
          rowId: 'add-source-suggestions',
          state: 'loading',
          label: 'Loading suggested folders',
          detail: entryPointsState.detail ?? 'Loading suggested folders.'
        })
      ]
    }
  }

  if (entryPointsState?.kind === 'failed') {
    return {
      productState: 'unavailable',
      surfaceKind: 'addSource',
      surfaceLabel: 'Add Source',
      title: 'Add Source',
      detail: 'Add a folder to manage it as a music source.',
      rows: [
        stateRow({
          rowId: 'add-source-suggestions',
          state: 'failed',
          label: 'Suggested folders unavailable',
          detail: entryPointsState.detail ?? 'Suggested folders are not available right now.',
          action: {
            kind: 'chooseMusicFolder',
            label: 'Add music folder'
          }
        })
      ]
    }
  }

  return {
    productState:
      entryPointsState?.kind === 'ready' || entryPointsState?.kind === 'refreshing'
        ? 'suggestionsReady'
        : 'start',
    surfaceKind: 'addSource',
    surfaceLabel: 'Add Source',
    title: 'Add Source',
    detail:
      'Suggested folders are shortcuts into local browse. Add a folder to manage it as a music source. Inventory is for inspection.',
    rows: [
      stateRow({
        rowId: 'add-source',
        state: 'empty',
        label: 'Suggested folders',
        detail:
          entryPointsState?.refreshError ??
          'Open a suggested folder, or use the folder picker to choose another location.',
        action: {
          kind: 'chooseMusicFolder',
          label: 'Add music folder'
        }
      })
    ]
  }
}

function folderProjection(options: {
  readonly selectedNodeId: BrowserTreeNodeId
  readonly title: string
  readonly targetNodeId: BrowserTreeNodeId
  readonly folderDetail: string
  readonly addSourceView: AddSourceView
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly localStatus: LocalBrowseEntryPointStatus | LocalBrowseItem['status']
  readonly admission:
    | Extract<LocalBrowseOperation, { readonly kind: 'requestSourceAdmission' }>
    | undefined
  readonly windowState: LocalBrowseItemState | undefined
}): AddSourceProjection {
  const selection = folderSelectionState(options)
  const surface = surfaceForView(options.addSourceView)
  const state = options.windowState

  if (state === undefined) {
    return {
      productState: selection,
      ...surface,
      title: options.title,
      detail: folderSelectionDetail(options, selection),
      rows: [
        stateRow({
          rowId: options.selectedNodeId,
          state: 'notLoaded',
          label:
            options.addSourceView === 'inventory' ? 'Inventory not loaded' : 'Preview not loaded',
          detail:
            options.addSourceView === 'inventory'
              ? 'Load inventory to inspect local files without adding or indexing this folder.'
              : 'Load preview to inspect this folder before adding it as a managed music source.',
          action: {
            kind: 'loadLocalBrowseChildren',
            nodeId: options.targetNodeId,
            label: options.addSourceView === 'inventory' ? 'Load inventory' : 'Load preview'
          }
        })
      ]
    }
  }

  if (state.kind === 'loading') {
    return {
      productState: selection,
      ...surface,
      title: options.title,
      detail: folderSelectionDetail(options, selection),
      rows: [
        stateRow({
          rowId: options.selectedNodeId,
          state: 'loading',
          label: options.addSourceView === 'inventory' ? 'Loading inventory' : 'Loading preview',
          detail: state.detail ?? 'Loading folder contents.'
        })
      ]
    }
  }

  if (state.kind === 'failed') {
    return {
      productState: 'unavailable',
      ...surface,
      title: options.title,
      detail: folderSelectionDetail(options, selection),
      rows: [
        stateRow({
          rowId: options.selectedNodeId,
          state: 'failed',
          label: 'Folder unavailable',
          detail: state.detail,
          action: {
            kind: 'loadLocalBrowseChildren',
            nodeId: options.targetNodeId,
            label: 'Retry'
          }
        })
      ]
    }
  }

  const window = state.window
  const contentRows = window.items
    .filter((item) => isVisibleAddSourcePreviewItem(item, window))
    .map(localBrowseItemRow)
  const moreRow =
    window.nextOffset === undefined
      ? undefined
      : localBrowseMoreContentRow(options.selectedNodeId, window)
  const rows = [...contentRows, ...(moreRow === undefined ? [] : [moreRow])]

  if (rows.length > 0) {
    return {
      productState: selection,
      ...surfaceForView(window.addSourceView),
      title: options.title,
      detail:
        state.kind === 'refreshing'
          ? (state.detail ?? 'Refreshing folder contents.')
          : localBrowseWindowDetail(window, contentRows.length, selection),
      rows
    }
  }

  return {
    productState: selection,
    ...surfaceForView(window.addSourceView),
    title: options.title,
    detail: localBrowseWindowDetail(window, contentRows.length, selection),
    rows: [
      stateRow({
        rowId: options.selectedNodeId,
        state: localBrowseWindowState(window, selection),
        label: localBrowseWindowStateLabel(window, selection),
        detail:
          window.failure?.detail ?? localBrowseWindowDetail(window, contentRows.length, selection)
      })
    ]
  }
}

function itemProjection(
  selectedNodeId: BrowserTreeNodeId,
  item: LocalBrowseItem,
  addSourceView: AddSourceView
): AddSourceProjection {
  if (isTerminalLocalBrowseItem(item)) {
    return {
      productState: 'protectedOrBlocked',
      ...surfaceForView(addSourceView),
      title: item.displayName,
      detail: item.failure?.detail ?? localBrowseItemDetail(item),
      rows: [
        stateRow({
          rowId: selectedNodeId,
          state: localBrowseTerminalRowState(item),
          label: localBrowseTerminalLabel(item),
          detail: item.failure?.detail ?? localBrowseItemDetail(item)
        })
      ]
    }
  }

  return {
    productState:
      addSourceView === 'inventory'
        ? 'inventory'
        : folderSelectionState({
            addSourceView,
            entryPointKind: item.identity.entryPointKind,
            localStatus: item.status,
            admission: sourceAdmissionOperation(item.availableOperations)
          }),
    ...surfaceForView(addSourceView),
    title: item.displayName,
    detail: localBrowseItemDetail(item),
    rows: [localBrowseItemRow(item)]
  }
}

function folderSelectionState(options: {
  readonly addSourceView: AddSourceView
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly localStatus: LocalBrowseEntryPointStatus | LocalBrowseItem['status']
  readonly admission:
    | Extract<LocalBrowseOperation, { readonly kind: 'requestSourceAdmission' }>
    | undefined
}): AddSourceProductState {
  if (options.addSourceView === 'inventory') {
    return 'inventory'
  }

  if (options.localStatus === 'duplicateOfAdmittedSource') {
    return 'alreadyAdded'
  }

  if (options.localStatus === 'restorableSource') {
    return 'restoreSource'
  }

  if (options.localStatus === 'permissionBlocked' || options.localStatus === 'rejected') {
    return 'protectedOrBlocked'
  }

  if (options.entryPointKind === 'systemDriveRoot') {
    return 'tooBroad'
  }

  if (options.localStatus !== 'available') {
    return 'protectedOrBlocked'
  }

  return options.admission === undefined ? 'notUseful' : 'selectedFolderPreview'
}

function folderSelectionDetail(
  options: {
    readonly folderDetail: string
    readonly addSourceView: AddSourceView
  },
  state: AddSourceProductState
): string {
  switch (state) {
    case 'alreadyAdded':
      return 'Already added as a music source.'
    case 'restoreSource':
      return 'This source was removed. Restore it to manage this folder again.'
    case 'tooBroad':
      return 'Choose a specific folder inside this drive.'
    case 'protectedOrBlocked':
      return 'Choose another folder that Dekzer can read.'
    case 'notUseful':
      return `This folder can be browsed, but it does not look useful as a music source yet. ${options.folderDetail}`
    case 'inventory':
      return `Inventory is for inspection and does not add or index this folder. ${options.folderDetail}`
    default:
      return `Inspect this folder before adding it as a managed music source. ${options.folderDetail}`
  }
}

function surfaceForView(
  addSourceView: AddSourceView
): Pick<AddSourceProjection, 'surfaceKind' | 'surfaceLabel'> {
  return addSourceView === 'inventory'
    ? { surfaceKind: 'inventory', surfaceLabel: 'Inventory' }
    : { surfaceKind: 'preview', surfaceLabel: 'Preview' }
}

function localBrowseItemRow(item: LocalBrowseItem): AddSourceRow {
  const fileClass = localBrowseFileClass(item)

  return {
    id: `local-browse-content:${item.identity.entryPointKind}:${encodeURIComponent(
      item.identity.resolvedRootPath
    )}:${encodeURIComponent(item.identity.resolvedItemPath)}`,
    kind: item.itemKind === 'directory' || item.itemKind === 'rejectedRoot' ? 'directory' : 'file',
    label: item.displayName,
    detail: localBrowseItemDetail(item),
    icon: localBrowseItemIcon(item),
    ...(fileClass === undefined ? {} : { fileClass })
  }
}

function isVisibleAddSourcePreviewItem(
  item: LocalBrowseItem,
  window: LoadedLocalBrowseItems
): boolean {
  if (window.addSourceView === 'inventory') {
    return true
  }

  if (isSystemDriveRootWindow(window)) {
    return (
      item.status === 'duplicateOfAdmittedSource' ||
      (item.itemKind === 'directory' &&
        sourceAdmissionOperation(item.availableOperations) !== undefined)
    )
  }

  if (isTerminalLocalBrowseItem(item)) {
    return true
  }

  if (item.itemKind === 'directory') {
    return true
  }

  return (
    item.itemKind === 'mediaFile' &&
    item.mediaRelevance !== 'unsupported' &&
    (item.fileKind === 'audio' || item.fileKind === 'cueSheet')
  )
}

function localBrowseMoreContentRow(
  selectedNodeId: BrowserTreeNodeId,
  window: LoadedLocalBrowseItems
): AddSourceRow | undefined {
  const offset = window.nextOffset

  if (offset === undefined) {
    return undefined
  }

  return {
    id: `contents-local-browse-load-more:${selectedNodeId}:${offset}`,
    kind: 'more',
    label:
      window.more?.kind === 'failed'
        ? 'Retry loading more'
        : window.more?.kind === 'loading'
          ? 'Loading more'
          : 'Load more',
    detail:
      window.more?.kind === 'failed'
        ? window.more.detail
        : `Items ${offset + 1}-${Math.min(offset + window.limit, window.totalItems)} of ${window.totalItems} are available.`,
    icon:
      window.more?.kind === 'loading'
        ? 'loading'
        : window.more?.kind === 'failed'
          ? 'warning'
          : 'more',
    ...(window.more?.kind === 'loading'
      ? {}
      : {
          action: {
            kind: 'loadLocalBrowseMore',
            nodeId: selectedNodeId,
            label: window.more?.kind === 'failed' ? 'Retry' : 'Load more'
          }
        })
  }
}

function localBrowseMoreRow(rowId: BrowserTreeNodeId, detail: string): AddSourceRow {
  return {
    id: rowId,
    kind: 'more',
    label: 'Load more',
    detail,
    icon: 'more'
  }
}

function localBrowseWindowDetail(
  window: LoadedLocalBrowseItems,
  visibleRowCount: number,
  selectionState: AddSourceProductState
): string {
  if (window.failure !== null) {
    return window.failure.detail
  }

  if (window.addSourceView === 'inventory') {
    if (window.totalItems === 0) {
      return 'Inventory is for inspection. No local files are shown for this folder.'
    }

    return `Inventory is for inspection. ${visibleRowCount} of ${window.totalItems} local files shown.`
  }

  if (selectionState === 'tooBroad' || isSystemDriveRootWindow(window)) {
    return 'Choose a specific folder inside this drive.'
  }

  if (window.totalItems === 0 || visibleRowCount === 0) {
    return 'This folder can be browsed, but it does not look useful as a music source yet.'
  }

  return `${visibleRowCount} of ${window.totalItems} preview items shown.`
}

function localBrowseWindowState(
  window: LoadedLocalBrowseItems,
  selectionState: AddSourceProductState
): Exclude<AddSourceRow['state'], undefined> {
  if (selectionState === 'tooBroad') {
    return 'empty'
  }

  switch (window.status) {
    case 'complete':
      return 'empty'
    case 'permissionBlocked':
    case 'unsupportedPlatform':
      return 'unsupported'
    case 'partialFailure':
    case 'failed':
    case 'missing':
    case 'unavailable':
      return 'failed'
  }
}

function localBrowseWindowStateLabel(
  window: LoadedLocalBrowseItems,
  selectionState: AddSourceProductState
): string {
  if (selectionState === 'tooBroad') {
    return 'Choose a specific folder'
  }

  switch (window.status) {
    case 'complete':
      return window.addSourceView === 'inventory'
        ? 'No local files shown'
        : 'Choose a folder that contains music files'
    case 'partialFailure':
      return 'Some items could not be read'
    case 'failed':
      return 'Folder unavailable'
    case 'unsupportedPlatform':
      return 'Local browsing unavailable'
    case 'missing':
      return 'Folder missing'
    case 'permissionBlocked':
      return 'Protected location'
    case 'unavailable':
      return 'Folder unavailable'
  }
}

function localBrowseItemDetail(item: LocalBrowseItem): string {
  if (item.status === 'duplicateOfAdmittedSource') {
    return 'Already added as a music source.'
  }

  if (item.status === 'restorableSource') {
    return 'Source was removed. Restore it to use this folder again.'
  }

  if (item.failure !== null) {
    return item.failure.detail
  }

  return item.identity.resolvedItemPath
}

function isTerminalLocalBrowseItem(item: LocalBrowseItem): boolean {
  return (
    item.itemKind === 'rejectedRoot' ||
    item.itemKind === 'inaccessible' ||
    item.status === 'rejected' ||
    item.status === 'permissionBlocked' ||
    item.failure?.code === 'reparsePointSkipped'
  )
}

function localBrowseTerminalRowState(
  item: LocalBrowseItem
): Exclude<AddSourceRow['state'], undefined> {
  return item.status === 'permissionBlocked' || item.status === 'rejected'
    ? 'unsupported'
    : 'failed'
}

function localBrowseTerminalLabel(item: LocalBrowseItem): string {
  if (item.failure?.code === 'reparsePointSkipped') {
    return 'Protected location'
  }

  switch (item.status) {
    case 'permissionBlocked':
    case 'rejected':
      return 'Protected location'
    case 'missing':
      return 'Folder missing'
    default:
      return 'Folder unavailable'
  }
}

function localBrowseItemIcon(item: LocalBrowseItem): AddSourceRowIcon {
  if (
    item.status !== 'available' &&
    item.status !== 'duplicateOfAdmittedSource' &&
    item.status !== 'restorableSource'
  ) {
    return 'warning'
  }

  switch (item.itemKind) {
    case 'directory':
    case 'rejectedRoot':
      return 'folder'
    case 'mediaFile':
      switch (item.fileKind) {
        case 'audio':
          return 'music'
        case 'video':
          return 'video'
        case 'image':
          return 'image'
        case 'cueSheet':
          return 'cueSheet'
        default:
          return 'metadata'
      }
    case 'unsupportedFile':
      return item.fileKind === 'cueSheet' ? 'cueSheet' : 'metadata'
    case 'inaccessible':
      return 'warning'
    case 'unknown':
      return 'state'
  }
}

function localBrowseFileClass(item: LocalBrowseItem): AddSourceRow['fileClass'] | undefined {
  switch (item.fileKind) {
    case 'audio':
      return 'audio'
    case 'video':
      return 'video'
    case 'image':
      return 'image'
    case 'cueSheet':
    case 'logDoc':
    case 'textDoc':
    case 'archive':
    case 'other':
    case 'unknown':
      return 'unsupported'
    case null:
      return undefined
  }
}

function isSystemDriveRootWindow(window: LoadedLocalBrowseItems): boolean {
  return (
    window.identity.entryPointKind === 'systemDriveRoot' &&
    localBrowsePathKey(window.identity.resolvedRootPath) ===
      localBrowsePathKey(window.identity.resolvedParentPath)
  )
}

function localBrowsePathKey(path: string): string {
  return path.replaceAll('/', '\\').replace(/\\+$/, '').toLowerCase()
}

function stateRow(options: {
  readonly rowId: string
  readonly state: Exclude<AddSourceRow['state'], undefined>
  readonly label: string
  readonly detail: string
  readonly action?: AddSourceAction
}): AddSourceRow {
  return {
    id: `add-source-state:${options.rowId}:${options.state}`,
    kind: 'state',
    label: options.label,
    detail: options.detail,
    state: options.state,
    icon:
      options.state === 'loading'
        ? 'loading'
        : options.state === 'failed' || options.state === 'unsupported'
          ? 'warning'
          : 'state',
    ...(options.action === undefined ? {} : { action: options.action })
  }
}
