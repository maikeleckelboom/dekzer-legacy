import type {
  LocalBrowseEntryPoint,
  LocalBrowseEntryPointStatus,
  LocalBrowseOperation
} from '../../../shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  LocalBrowseItemKind,
  LocalBrowseItemStatus
} from '../../../shared/library/localBrowse/items'
import type { RowBinding } from '../state'
import type {
  BrowserTreeAction,
  BrowserTreeActionState,
  BrowserTreeChildren,
  BrowserTreeIcon,
  BrowserTreeNode,
  BrowserTreeNodeId
} from '../tree/types'
import {
  localBrowseRootTarget,
  localBrowseWindowKey,
  targetForEntryPoint,
  type LoadedLocalBrowseItems,
  type LocalBrowseDirectoryTarget,
  type LocalBrowseEntryPointTarget,
  type LocalBrowseEntryPointsState,
  type LocalBrowseItemState,
  type LocalBrowseMoreTarget
} from './types'
import type { ProfileKey } from '../browseProfile/types'

export const localBrowseSectionNodeId = 'local-browse:section'

type LocalBrowseProjectionOptions = {
  readonly profile: ProfileKey
  readonly entryPointsState?: LocalBrowseEntryPointsState
  readonly itemStates?: ReadonlyMap<string, LocalBrowseItemState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}

type BrowserTreeReadState = 'notLoaded' | 'loading' | 'empty' | 'unavailable' | 'error'

export function projectLocalBrowseSection(
  options: LocalBrowseProjectionOptions
): BrowserTreeNode | undefined {
  const state = options.entryPointsState

  if (state === undefined || state.kind === 'unread') {
    return undefined
  }

  if (state.kind === 'loading') {
    options.bindingsById.set(localBrowseSectionNodeId, {
      kind: 'localBrowseSection'
    })
    return localBrowseSectionNode(
      loadingChildren(
        {
          ownerId: localBrowseSectionNodeId,
          label: 'Loading local folders',
          detail: state.detail ?? 'Loading local browse entry points.'
        },
        options.bindingsById
      )
    )
  }

  if (state.kind === 'failed') {
    options.bindingsById.set(localBrowseSectionNodeId, {
      kind: 'localBrowseSection'
    })
    return localBrowseSectionNode(
      failedChildren(
        {
          ownerId: localBrowseSectionNodeId,
          label: 'Local folders unavailable',
          detail: state.detail
        },
        options.bindingsById
      )
    )
  }

  const entries = displayableEntryPoints(state.result.entries)

  if (entries.length === 0) {
    return undefined
  }

  options.bindingsById.set(localBrowseSectionNodeId, {
    kind: 'localBrowseSection'
  })

  return localBrowseSectionNode(
    childrenForProjectedNodes(
      entries.map((entry) =>
        projectEntryPoint({
          entry,
          profile: options.profile,
          itemStates: options.itemStates ?? new Map(),
          bindingsById: options.bindingsById
        })
      )
    ),
    state.kind === 'refreshing'
      ? (state.detail ?? 'Refreshing local browse entry points.')
      : state.refreshError
  )
}

function localBrowseSectionNode(
  children: BrowserTreeChildren,
  detail = 'Music, Downloads, Desktop, and Home are starting points for adding sources.'
): BrowserTreeNode {
  return {
    id: localBrowseSectionNodeId,
    role: 'collectionView',
    label: 'Local Files',
    detail,
    icon: 'navigation',
    children
  }
}

function projectEntryPoint(options: {
  readonly entry: LocalBrowseEntryPoint
  readonly profile: ProfileKey
  readonly itemStates: ReadonlyMap<string, LocalBrowseItemState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): BrowserTreeNode {
  const target = targetForEntryPoint(options.entry)
  const nodeId = localBrowseEntryPointNodeId(options.entry)

  if (target !== undefined) {
    options.bindingsById.set(nodeId, {
      kind: 'localBrowseEntryPoint',
      entry: options.entry,
      target
    })
  }

  const detail = formatEntryPointDetail(options.entry)

  return {
    id: nodeId,
    role: 'localBrowseRoot',
    label: options.entry.displayName,
    icon: entryPointIcon(options.entry),
    detail,
    ...projectEntryPointChildren({
      ownerId: nodeId,
      entry: options.entry,
      profile: options.profile,
      target,
      itemStates: options.itemStates,
      bindingsById: options.bindingsById
    })
  }
}

function projectEntryPointChildren(options: {
  readonly ownerId: string
  readonly entry: LocalBrowseEntryPoint
  readonly profile: ProfileKey
  readonly target: LocalBrowseEntryPointTarget | undefined
  readonly itemStates: ReadonlyMap<string, LocalBrowseItemState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): { readonly children: BrowserTreeChildren; readonly action?: BrowserTreeAction } {
  const target = options.target

  if (
    target === undefined ||
    !hasLocalBrowseOperation(options.entry.availableOperations, 'browseChildren')
  ) {
    return { children: { kind: 'none' } }
  }

  const rootTarget = localBrowseRootTarget(target, options.profile)

  return projectLocalBrowseChildren({
    ownerId: options.ownerId,
    target: rootTarget,
    state: options.itemStates.get(localBrowseWindowKey(rootTarget)),
    itemStates: options.itemStates,
    bindingsById: options.bindingsById
  })
}

function projectLocalBrowseItem(options: {
  readonly item: LocalBrowseItem
  readonly profile: ProfileKey
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
  readonly itemStates: ReadonlyMap<string, LocalBrowseItemState>
}): BrowserTreeNode {
  const item = options.item
  const nodeId = localBrowseItemNodeId(item)
  const target = hasLocalBrowseOperation(item.availableOperations, 'browseChildren')
    ? directoryTargetForItem(item, options.profile)
    : undefined

  options.bindingsById.set(nodeId, {
    kind: 'localBrowseItem',
    item,
    ...(target === undefined ? {} : { target })
  })

  return {
    id: nodeId,
    role:
      item.itemKind === 'directory' || item.itemKind === 'rejectedRoot'
        ? 'localBrowseDirectory'
        : 'localBrowseFile',
    label: item.displayName,
    icon: itemIcon(item),
    detail: formatItemDetail(item),
    ...projectLocalBrowseChildren({
      ownerId: nodeId,
      target,
      state:
        target === undefined ? undefined : options.itemStates.get(localBrowseWindowKey(target)),
      itemStates: options.itemStates,
      bindingsById: options.bindingsById
    })
  }
}

function projectLocalBrowseChildren(options: {
  readonly ownerId: string
  readonly target: LocalBrowseDirectoryTarget | undefined
  readonly state: LocalBrowseItemState | undefined
  readonly itemStates: ReadonlyMap<string, LocalBrowseItemState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): { readonly children: BrowserTreeChildren; readonly action?: BrowserTreeAction } {
  const target = options.target

  if (target === undefined) {
    return { children: { kind: 'none' } }
  }

  const state = options.state

  if (state === undefined) {
    const detail = 'Local folder contents not loaded yet.'

    return {
      children: deferredChildren(
        {
          ownerId: options.ownerId,
          label: 'Local folder contents not loaded',
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
          label: 'Loading local folder contents',
          detail: state.detail ?? 'Loading local folder contents.'
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
          label: 'Local folder unavailable',
          detail: state.detail
        },
        options.bindingsById
      ),
      action: loadChildrenAction('failed', state.detail)
    }
  }

  return {
    children: childrenForProjectedNodes(
      projectLoadedLocalBrowseWindow({
        ownerId: options.ownerId,
        window: state.window,
        itemStates: options.itemStates,
        bindingsById: options.bindingsById
      })
    )
  }
}

function projectLoadedLocalBrowseWindow(options: {
  readonly ownerId: string
  readonly window: LoadedLocalBrowseItems
  readonly itemStates: ReadonlyMap<string, LocalBrowseItemState>
  readonly bindingsById: Map<BrowserTreeNodeId, RowBinding>
}): readonly BrowserTreeNode[] {
  if (options.window.items.length === 0 && options.window.nextOffset === undefined) {
    return [
      localBrowseWindowStateNode(options.ownerId, options.window, options.bindingsById)
    ].filter((node): node is BrowserTreeNode => node !== undefined)
  }

  const projectedItems = options.window.items.map((item) =>
    projectLocalBrowseItem({
      item,
      profile: options.window.profile,
      itemStates: options.itemStates,
      bindingsById: options.bindingsById
    })
  )

  if (options.window.nextOffset === undefined) {
    return projectedItems
  }

  return [...projectedItems, trackedMoreNode(options, options.bindingsById)]
}

function localBrowseWindowStateNode(
  ownerId: string,
  window: LoadedLocalBrowseItems,
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode | undefined {
  switch (window.status) {
    case 'complete':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'empty',
          label: 'No local items',
          detail: 'No items are available in this local folder.'
        },
        bindingsById
      )
    case 'partialFailure':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'error',
          label: 'Local items partially unavailable',
          detail: window.failure?.detail ?? 'Some local items could not be read.'
        },
        bindingsById
      )
    case 'failed':
    case 'unavailable':
    case 'permissionBlocked':
    case 'missing':
      return trackedReadStateNode(
        {
          ownerId,
          state: window.status === 'permissionBlocked' ? 'unavailable' : 'error',
          label: localBrowseReadStatusLabel(window.status),
          detail: window.failure?.detail ?? 'Local folder contents could not be read.'
        },
        bindingsById
      )
    case 'unsupportedPlatform':
      return trackedReadStateNode(
        {
          ownerId,
          state: 'unavailable',
          label: 'Local browse unsupported',
          detail: window.failure?.detail ?? 'Local browse is not supported on this platform.'
        },
        bindingsById
      )
  }
}

function childrenForProjectedNodes(nodes: readonly BrowserTreeNode[]): BrowserTreeChildren {
  return nodes.length === 0 ? { kind: 'none' } : { kind: 'loaded', nodes }
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

function trackedReadStateNode(
  options: {
    readonly ownerId: string
    readonly state: BrowserTreeReadState
    readonly label: string
    readonly detail: string
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode {
  const node: BrowserTreeNode = {
    id: `read-state:${options.ownerId}`,
    role: 'state',
    label: options.label,
    detail: options.detail,
    icon: readStateIcon(options.state),
    children: { kind: 'none' }
  }

  bindingsById.set(node.id, {
    kind: 'readState',
    state: options.state,
    ownerId: options.ownerId,
    detail: options.detail
  })

  return node
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

function trackedMoreNode(
  options: {
    readonly ownerId: string
    readonly window: LoadedLocalBrowseItems
  },
  bindingsById: Map<BrowserTreeNodeId, RowBinding>
): BrowserTreeNode {
  const offset = options.window.nextOffset

  if (offset === undefined) {
    throw new Error('More local browse rows require a next offset.')
  }

  const target: LocalBrowseMoreTarget = {
    ownerNodeId: options.ownerId,
    profile: options.window.profile,
    entryPointKind: options.window.identity.entryPointKind,
    resolvedRootPath: options.window.identity.resolvedRootPath,
    resolvedParentPath: options.window.identity.resolvedParentPath,
    label: options.window.label,
    offset,
    limit: options.window.limit
  }
  const more = options.window.more
  const detail =
    more?.kind === 'failed'
      ? more.detail
      : more?.kind === 'loading'
        ? (more.detail ?? 'Loading more local items.')
        : `Items ${offset + 1}-${Math.min(offset + options.window.limit, options.window.totalItems)} of ${options.window.totalItems} are available.`
  const node: BrowserTreeNode = {
    id: `local-browse-more:${options.ownerId}:${offset}`,
    role: 'action',
    label:
      more?.kind === 'failed'
        ? 'Retry loading more'
        : more?.kind === 'loading'
          ? 'Loading more'
          : 'Load more',
    icon: moreIcon(more),
    detail,
    children: { kind: 'none' },
    action: {
      kind: 'loadMore',
      state:
        more?.kind === 'failed'
          ? { kind: 'failed', detail }
          : more?.kind === 'loading'
            ? { kind: 'loading', detail }
            : { kind: 'idle', detail }
    }
  }

  bindingsById.set(node.id, {
    kind: 'localBrowseMore',
    state: more?.kind === 'failed' ? 'error' : more?.kind === 'loading' ? 'loading' : 'available',
    ownerId: options.ownerId,
    target,
    detail
  })

  return node
}

function displayableEntryPoints(
  entries: readonly LocalBrowseEntryPoint[]
): readonly LocalBrowseEntryPoint[] {
  return [
    ...entries.filter((entry) => {
      if (entry.status === 'unsupportedPlatform') {
        return false
      }

      return targetForEntryPoint(entry) !== undefined
    })
  ].sort(
    (left, right) =>
      entryPointPriority(left).localeCompare(entryPointPriority(right)) ||
      left.displayName.localeCompare(right.displayName)
  )
}

function entryPointPriority(entry: LocalBrowseEntryPoint): string {
  switch (entry.identity.entryPointKind) {
    case 'music':
      return '0'
    case 'downloads':
      return '1'
    case 'desktop':
      return '2'
    case 'userHome':
      return '3'
    case 'removableVolumeRoot':
      return '4'
    case 'localDataVolumeRoot':
      return '5'
    case 'systemDriveRoot':
      return '6'
  }
}

function directoryTargetForItem(
  item: LocalBrowseItem,
  profile: ProfileKey
): LocalBrowseDirectoryTarget | undefined {
  if (!hasLocalBrowseOperation(item.availableOperations, 'browseChildren')) {
    return undefined
  }

  return {
    profile,
    entryPointKind: item.identity.entryPointKind,
    resolvedRootPath: item.identity.resolvedRootPath,
    resolvedParentPath: item.identity.resolvedItemPath,
    label: item.displayName
  }
}

function localBrowseEntryPointNodeId(entry: LocalBrowseEntryPoint): BrowserTreeNodeId {
  return `local-browse-entry:${entry.identity.entryPointKind}:${encodeURIComponent(
    entry.identity.resolvedPath ?? entry.displayName
  )}`
}

function localBrowseItemNodeId(item: LocalBrowseItem): BrowserTreeNodeId {
  return `local-browse-item:${item.identity.entryPointKind}:${encodeURIComponent(
    item.identity.resolvedRootPath
  )}:${encodeURIComponent(item.identity.resolvedItemPath)}`
}

function entryPointIcon(entry: LocalBrowseEntryPoint): BrowserTreeIcon {
  return isEntryPointUnavailable(entry.status) ? 'warning' : 'folder'
}

function itemIcon(item: LocalBrowseItem): BrowserTreeIcon {
  if (item.status === 'rejected' || isItemUnavailable(item.status)) {
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

function moreIcon(more: LoadedLocalBrowseItems['more']): BrowserTreeIcon {
  switch (more?.kind) {
    case 'loading':
      return 'loading'
    case 'failed':
      return 'warning'
    default:
      return 'more'
  }
}

function formatEntryPointDetail(entry: LocalBrowseEntryPoint): string {
  if (entry.status === 'duplicateOfAdmittedSource') {
    return 'Already added as a library source.'
  }

  if (entry.failure !== null) {
    return entry.failure.detail
  }

  if (entry.status !== 'available') {
    return entryPointStatusLabel(entry.status)
  }

  if (entry.identity.entryPointKind === 'systemDriveRoot') {
    return 'Choose a narrower folder.'
  }

  return (
    sourceAdmissionOperationDetail(entry.availableOperations) ?? 'Not a music-source candidate.'
  )
}

function formatItemDetail(item: LocalBrowseItem): string {
  if (item.status === 'duplicateOfAdmittedSource') {
    return 'Already added as a library source.'
  }

  if (item.failure !== null) {
    return item.failure.detail
  }

  if (item.status !== 'available') {
    return itemStatusLabel(item.status)
  }

  return sourceAdmissionOperationDetail(item.availableOperations) ?? itemKindLabel(item.itemKind)
}

function sourceAdmissionOperationDetail(
  operations: readonly LocalBrowseOperation[]
): string | undefined {
  const operation = sourceAdmissionOperation(operations)

  if (operation === undefined) {
    return undefined
  }

  switch (operation.requestKind) {
    case 'defaultMusicFolder':
      return 'Not in library yet.'
    case 'selectedDirectory':
      return 'Not in library yet.'
    case 'parentDirectory':
      return 'Parent folder can be added as a music source.'
  }
}

export function hasLocalBrowseOperation(
  operations: readonly LocalBrowseOperation[],
  kind: LocalBrowseOperation['kind']
): boolean {
  return operations.some((operation) => operation.kind === kind)
}

export function sourceAdmissionOperation(
  operations: readonly LocalBrowseOperation[]
): Extract<LocalBrowseOperation, { readonly kind: 'requestSourceAdmission' }> | undefined {
  return operations.find(
    (
      operation
    ): operation is Extract<LocalBrowseOperation, { readonly kind: 'requestSourceAdmission' }> =>
      operation.kind === 'requestSourceAdmission'
  )
}

function itemKindLabel(kind: LocalBrowseItemKind): string {
  switch (kind) {
    case 'directory':
      return 'Not a music-source candidate.'
    case 'mediaFile':
      return 'Media file.'
    case 'unsupportedFile':
      return 'Unsupported file.'
    case 'rejectedRoot':
      return 'Protected location.'
    case 'inaccessible':
      return 'Local item inaccessible.'
    case 'unknown':
      return 'Could not fully resolve this location.'
  }
}

function localBrowseReadStatusLabel(status: LoadedLocalBrowseItems['status']): string {
  switch (status) {
    case 'complete':
      return 'Local folder empty'
    case 'partialFailure':
      return 'Local items partially unavailable'
    case 'failed':
      return 'Local folder read failed'
    case 'unsupportedPlatform':
      return 'Local browse unsupported'
    case 'missing':
      return 'Local folder missing'
    case 'permissionBlocked':
      return 'Local folder access blocked'
    case 'unavailable':
      return 'Local folder unavailable'
  }
}

function entryPointStatusLabel(status: LocalBrowseEntryPointStatus): string {
  switch (status) {
    case 'resolving':
      return 'Resolving local folder.'
    case 'available':
      return 'Local folder.'
    case 'unavailable':
      return 'Local folder unavailable.'
    case 'permissionBlocked':
      return 'Local folder access blocked.'
    case 'missing':
      return 'Local folder missing.'
    case 'unsupportedPlatform':
      return 'Local browse unsupported.'
    case 'duplicateOfAdmittedSource':
      return 'Already added as a library source.'
  }
}

function itemStatusLabel(status: LocalBrowseItemStatus): string {
  switch (status) {
    case 'available':
      return 'Not a music-source candidate.'
    case 'unavailable':
      return 'Local item unavailable.'
    case 'permissionBlocked':
      return 'Protected location.'
    case 'missing':
      return 'Local item missing.'
    case 'unsupportedPlatform':
      return 'Local browse unsupported.'
    case 'duplicateOfAdmittedSource':
      return 'Already added as a library source.'
    case 'rejected':
      return 'Protected location.'
    case 'unknown':
      return 'Could not fully resolve this location.'
  }
}

function isEntryPointUnavailable(status: LocalBrowseEntryPointStatus): boolean {
  return status === 'unavailable' || status === 'permissionBlocked' || status === 'missing'
}

function isItemUnavailable(status: LocalBrowseItemStatus): boolean {
  return (
    status === 'unavailable' ||
    status === 'permissionBlocked' ||
    status === 'missing' ||
    status === 'unsupportedPlatform'
  )
}
