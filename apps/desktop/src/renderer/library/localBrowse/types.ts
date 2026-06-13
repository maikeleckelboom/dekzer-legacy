import type {
  LocalBrowseEntryPoint,
  LocalBrowseEntryPointKind,
  ReadLocalBrowseEntryPointsResult
} from '../../../shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  LocalBrowseItemFilter,
  LocalBrowseWindowIdentity,
  ReadLocalBrowseItemsResult
} from '../../../shared/library/localBrowse/items'
import { mapAddSourceViewToLocalBrowseItemFilter, type AddSourceView } from '../addSource/view'

export type LocalBrowseEntryPointsState =
  | {
      readonly kind: 'unread'
    }
  | {
      readonly kind: 'loading'
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'ready'
      readonly result: ReadLocalBrowseEntryPointsResult
      readonly refreshError?: string
    }
  | {
      readonly kind: 'refreshing'
      readonly result: ReadLocalBrowseEntryPointsResult
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
    }

export type LocalBrowseEntryPointTarget = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly resolvedRootPath: string
  readonly label: string
}

export type LocalBrowseDirectoryTarget = {
  readonly addSourceView: AddSourceView
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly resolvedRootPath: string
  readonly resolvedParentPath: string
  readonly label: string
}

export type LocalBrowseMoreTarget = LocalBrowseDirectoryTarget & {
  readonly ownerNodeId: string
  readonly offset: number
  readonly limit: number
}

export type LocalBrowseMoreState =
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type LoadedLocalBrowseItems = {
  readonly addSourceView: AddSourceView
  readonly identity: LocalBrowseWindowIdentity
  readonly label: string
  readonly items: readonly LocalBrowseItem[]
  readonly totalItems: number
  readonly status: ReadLocalBrowseItemsResult['status']
  readonly failure: ReadLocalBrowseItemsResult['failure']
  readonly nextOffset?: number
  readonly limit: number
  readonly more?: LocalBrowseMoreState
}

export type LocalBrowseItemState =
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'loaded'
      readonly window: LoadedLocalBrowseItems
    }
  | {
      readonly kind: 'refreshing'
      readonly window: LoadedLocalBrowseItems
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
    }

export function targetForEntryPoint(
  entry: LocalBrowseEntryPoint
): LocalBrowseEntryPointTarget | undefined {
  const resolvedPath = entry.identity.resolvedPath

  if (resolvedPath === null || resolvedPath.trim().length === 0) {
    return undefined
  }

  return {
    entryPointKind: entry.identity.entryPointKind,
    resolvedRootPath: resolvedPath,
    label: entry.displayName
  }
}

export function localBrowseRootTarget(
  target: LocalBrowseEntryPointTarget,
  addSourceView: AddSourceView
): LocalBrowseDirectoryTarget {
  return {
    addSourceView,
    entryPointKind: target.entryPointKind,
    resolvedRootPath: target.resolvedRootPath,
    resolvedParentPath: target.resolvedRootPath,
    label: target.label
  }
}

export function localBrowseWindowKey(target: LocalBrowseDirectoryTarget): string {
  return [
    target.addSourceView,
    target.entryPointKind,
    encodeURIComponent(target.resolvedRootPath),
    encodeURIComponent(target.resolvedParentPath)
  ].join(':')
}

export function localBrowseWindowKeyFromIdentity(
  identity: LocalBrowseWindowIdentity,
  addSourceView: AddSourceView
): string {
  return localBrowseWindowKey({
    addSourceView,
    entryPointKind: identity.entryPointKind,
    resolvedRootPath: identity.resolvedRootPath,
    resolvedParentPath: identity.resolvedParentPath,
    label: ''
  })
}

export function localBrowseItemFilterForTarget(
  target: LocalBrowseDirectoryTarget
): LocalBrowseItemFilter {
  return mapAddSourceViewToLocalBrowseItemFilter(target.addSourceView)
}
